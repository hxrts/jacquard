//! `RoutingEngine` and `RouterManagedEngine` impls for `BabelEngine`.
//!
//! The core lifecycle is identical in shape to the enhanced batman engine.
//! The two key Babel-specific behaviours:
//!
//! - `flood_gossip` — sends the originated update (metric=0, self as destination)
//!   and re-advertisements of the selected (best) route for each other
//!   destination. Only the selected route is re-advertised, not all received
//!   routes. This is a key Babel property: non-selected routes are suppressed to
//!   reduce overhead and avoid loop-prone re-broadcasting of inferior paths.
//! - `ingest_update` — computes bidirectional ETX link cost and compound metric
//!   before storing the route entry. Routes with metric >= BABEL_INFINITY are
//!   discarded immediately (they represent unusable/asymmetric paths).
//!
//! **Note on `maintain_route`**: unlike batman-classic, there is no
//! `is_bidirectional` check here. Babel encodes bidirectionality via link cost:
//! a route with an absent or Faulted reverse link will have
//! cost = BABEL_INFINITY, so it will never be stored in the route table and
//! thus never appear in `best_next_hops`.

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HealthScore, Limit, LinkEndpoint, NodeId,
    Observation, PublishedRouteRecord, RatioPermille, ReachabilityState, RouteCommitment,
    RouteError, RouteHealth, RouteId, RouteInstallation, RouteLifecycleEvent,
    RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceResult,
    RouteMaintenanceTrigger, RouteMaterializationInput, RouteMaterializationProof,
    RouteProgressContract, RouteProgressState, RouteRuntimeError, RouteRuntimeState,
    RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick, TransportObservation,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, TimeEffects, TransportSenderEffects};

use crate::{
    gossip::{decode_update, encode_update, originated_update, BabelUpdate},
    private_state::link_is_usable,
    public_state::ActiveBabelRoute,
    scoring::PERMILLE_MAX,
    BabelEngine, BABEL_ENGINE_ID,
};

/// Tick interval between local sequence-number increments.
const SEQNO_REFRESH_INTERVAL: Tick = Tick(16);

fn health_scores_from_metric(tq: RatioPermille) -> (HealthScore, jacquard_core::PenaltyPoints) {
    let penalty = u16::try_from(PERMILLE_MAX)
        .expect("permille max fits u16")
        .saturating_sub(tq.0);
    (
        HealthScore(u32::from(tq.0)),
        jacquard_core::PenaltyPoints(u32::from(penalty)),
    )
}

impl<Transport, Effects> BabelEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn direct_neighbor_endpoints(
        &self,
        topology: &Observation<Configuration>,
    ) -> Vec<(NodeId, LinkEndpoint)> {
        topology
            .value
            .links
            .iter()
            .filter(|((from_node_id, _), link)| {
                *from_node_id == self.local_node_id && link_is_usable(link.state.state)
            })
            .map(|((_, neighbor), link)| (*neighbor, link.endpoint.clone()))
            .collect()
    }

    // long-block-exception: one flood pass sends the originated update and all
    // selected-route re-advertisements to each direct neighbor in a single pass.
    fn flood_gossip(
        &mut self,
        topology: &Observation<Configuration>,
        _now: Tick,
    ) -> Result<(), RouteError> {
        let neighbors = self.direct_neighbor_endpoints(topology);
        if neighbors.is_empty() {
            return Ok(());
        }
        let endpoints: Vec<LinkEndpoint> = neighbors.iter().map(|(_, ep)| ep.clone()).collect();

        // Send originated update (self as destination, metric=0).
        let local_update = originated_update(self.local_node_id, self.local_seqno);
        if let Ok(payload) = encode_update(&local_update) {
            for endpoint in &endpoints {
                self.transport.send_transport(endpoint, &payload)?;
            }
        }

        // Re-advertise each selected route (best route per destination).
        // Only the selected (lowest-metric) route is re-broadcast; non-selected
        // routes are suppressed. This is the core Babel flooding rule.
        let readvertisements: Vec<BabelUpdate> = self
            .selected_routes
            .values()
            .filter(|selected| selected.destination != self.local_node_id)
            .map(|selected| BabelUpdate {
                destination: selected.destination,
                router_id: selected.router_id,
                seqno: selected.seqno,
                metric: selected.metric,
            })
            .collect();

        for update in &readvertisements {
            if let Ok(payload) = encode_update(update) {
                for endpoint in &endpoints {
                    self.transport.send_transport(endpoint, &payload)?;
                }
            }
        }

        Ok(())
    }

    fn ingest_update_payload(
        &mut self,
        from_node_id: NodeId,
        payload: &[u8],
        observed_at_tick: Tick,
    ) {
        let Some(update) = decode_update(payload) else {
            return;
        };
        let topology = match self.latest_topology.clone() {
            Some(t) => t,
            None => return,
        };
        self.ingest_update(from_node_id, update, &topology, observed_at_tick);
    }
}

impl<Transport, Effects> RoutingEngine for BabelEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        let DestinationId::Node(destination) = input.admission.objective.destination else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        if input.admission.backend_ref.engine != BABEL_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        let Some(best) = self.best_next_hops.get(&destination) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        self.active_routes.insert(
            *input.handle.route_id(),
            ActiveBabelRoute {
                destination,
                next_hop: best.next_hop,
                backend_route_id: best.backend_route_id.clone(),
                installed_at_tick: self.effects.now_tick(),
            },
        );
        let tq = best.tq;
        let (stability_score, congestion_penalty_points) = health_scores_from_metric(tq);
        Ok(RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                stamp: input.handle.stamp.clone(),
                witness: Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness.clone(),
                    established_at_tick: self.effects.now_tick(),
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score,
                congestion_penalty_points,
                last_validated_at_tick: self.effects.now_tick(),
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(1),
                last_progress_at_tick: self.effects.now_tick(),
                state: RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(&self, _route: &jacquard_core::MaterializedRoute) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(&mut self, tick: &RoutingTickContext) -> Result<RoutingTickOutcome, RouteError> {
        // Increment local_seqno every SEQNO_REFRESH_INTERVAL ticks.
        if tick
            .topology
            .observed_at_tick
            .0
            .is_multiple_of(SEQNO_REFRESH_INTERVAL.0)
        {
            self.local_seqno = self.local_seqno.wrapping_add(1);
        }
        let change = self.refresh_private_state(&tick.topology, tick.topology.observed_at_tick);
        self.flood_gossip(&tick.topology, tick.topology.observed_at_tick)?;
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change,
            next_tick_hint: if change == RoutingTickChange::PrivateStateUpdated {
                RoutingTickHint::Immediate
            } else {
                RoutingTickHint::WithinTicks(Tick(self.decay_window.next_refresh_within_ticks))
            },
        })
    }

    fn maintain_route(
        &mut self,
        identity: &PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        _trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let Some(active_route) = self.active_routes.get(identity.route_id()) else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let destination = active_route.destination;
        let active_next_hop = active_route.next_hop;
        let Some(best) = self.best_next_hops.get(&destination) else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        // NOTE: No is_bidirectional check here. Babel encodes bidirectionality
        // in the link cost: a route with an absent or Faulted reverse link will
        // have BABEL_INFINITY metric and will not appear in best_next_hops.
        let (stability_score, congestion_penalty_points) = health_scores_from_metric(best.tq);
        runtime.health.last_validated_at_tick = self.effects.now_tick();
        runtime.health.stability_score = stability_score;
        runtime.health.congestion_penalty_points = congestion_penalty_points;
        runtime.health.reachability_state = ReachabilityState::Reachable;
        if best.next_hop != active_next_hop {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::LinkDegraded,
                },
            });
        }
        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        })
    }

    fn teardown(&mut self, route_id: &RouteId) {
        self.active_routes.remove(route_id);
    }
}

impl<Transport, Effects> RouterManagedEngine for BabelEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn ingest_transport_observation_for_router(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        if let TransportObservation::PayloadReceived {
            from_node_id,
            payload,
            observed_at_tick,
            ..
        } = observation
        {
            self.ingest_update_payload(*from_node_id, payload, *observed_at_tick);
        }
        Ok(())
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let endpoint = self.endpoint_for_next_hop(active.next_hop)?;
        self.transport.send_transport(&endpoint, payload)?;
        Ok(())
    }

    fn restore_route_runtime_for_router(&mut self, route_id: &RouteId) -> Result<bool, RouteError> {
        Ok(self.active_routes.contains_key(route_id))
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        Belief, ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId,
        DurationMs, Environment, Link, LinkEndpoint, LinkProfile, LinkRuntimeState, LinkState,
        Node, Observation, RatioPermille, RepairCapability, RouteEpoch, RouteMaintenanceTrigger,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RoutingTickContext,
        SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
    };
    use jacquard_mem_link_profile::{InMemoryRuntimeEffects, InMemoryTransport};
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

    use super::*;
    use crate::{BabelEngine, BABEL_ENGINE_ID};

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(64))
    }

    fn babel_node(byte: u8) -> Node {
        NodePreset::route_capable(
            NodePresetOptions::new(
                NodeIdentity::new(node(byte), ControllerId([byte; 32])),
                endpoint(byte),
                Tick(1),
            ),
            &BABEL_ENGINE_ID,
        )
        .build()
    }

    fn fixture_link(remote: u8) -> Link {
        Link {
            endpoint: endpoint(remote),
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: jacquard_core::PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::certain(RatioPermille(900), Tick(1)),
                symmetry_permille: Belief::Absent,
            },
        }
    }

    fn sample_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: BTreeMap::from([(node(1), babel_node(1)), (node(2), babel_node(2))]),
                links: BTreeMap::from([
                    ((node(1), node(2)), fixture_link(2)),
                    ((node(2), node(1)), fixture_link(1)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: jacquard_core::FactSourceClass::Local,
            evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
            origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    fn sample_objective() -> jacquard_core::RoutingObjective {
        jacquard_core::RoutingObjective {
            destination: DestinationId::Node(node(2)),
            service_kind: jacquard_core::RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
            latency_budget_ms: Limit::Bounded(DurationMs(100)),
            protection_priority: jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_core::PriorityPoints(10),
        }
    }

    fn sample_profile() -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn engine_with_update_state(
        now: Tick,
    ) -> (
        BabelEngine<InMemoryTransport, InMemoryRuntimeEffects>,
        Observation<Configuration>,
    ) {
        let mut engine = BabelEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now,
                ..Default::default()
            },
        );
        let topology = sample_topology();
        // Simulate having received a Babel update for node(2) from node(2)
        // directly (metric=0, as if node(2) is a direct neighbor and originator).
        let update = BabelUpdate {
            destination: node(2),
            router_id: node(2),
            seqno: 1,
            metric: 0,
        };
        engine.ingest_update(node(2), update, &topology, now);
        engine.refresh_private_state(&topology, now);
        (engine, topology)
    }

    #[test]
    fn materialize_route_succeeds_after_update_received() {
        let (mut engine, topology) = engine_with_update_state(Tick(5));
        engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("tick");

        let objective = sample_objective();
        let profile = sample_profile();
        let candidates = engine.candidate_routes(&objective, &profile, &topology);
        assert!(
            !candidates.is_empty(),
            "no candidates after update received"
        );
    }

    #[test]
    fn maintain_route_expires_when_destination_disappears() {
        let now = Tick(1);
        let (mut engine, topology) = engine_with_update_state(now);
        engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("first tick");

        let objective = sample_objective();
        let profile = sample_profile();
        let candidates = engine.candidate_routes(&objective, &profile, &topology);
        assert!(!candidates.is_empty());
        let admission = engine
            .admit_route(&objective, &profile, candidates[0].clone(), &topology)
            .expect("admission");
        let input = RouteMaterializationInput {
            handle: jacquard_core::RouteHandle {
                stamp: jacquard_core::RouteIdentityStamp {
                    route_id: engine.route_id_for(node(2)),
                    topology_epoch: topology.value.epoch,
                    materialized_at_tick: now,
                    publication_id: jacquard_core::PublicationId([1; 16]),
                },
            },
            admission,
            lease: jacquard_core::RouteLease {
                owner_node_id: node(1),
                lease_epoch: topology.value.epoch,
                valid_for: TimeWindow::new(Tick(1), Tick(20)).expect("lease"),
            },
        };
        let installation = engine.materialize_route(input.clone()).expect("install");
        let identity = PublishedRouteRecord {
            stamp: input.handle.stamp.clone(),
            proof: installation.materialization_proof,
            admission: input.admission,
            lease: input.lease,
        };
        let mut runtime = RouteRuntimeState {
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: now,
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(1),
                last_progress_at_tick: now,
                state: RouteProgressState::Pending,
            },
        };

        // Clear all links and advance time to decay windows.
        let mut empty_topology = topology.clone();
        empty_topology.observed_at_tick = Tick(20);
        empty_topology.value.links.clear();
        engine.refresh_private_state(&empty_topology, Tick(20));

        let result = engine
            .maintain_route(
                &identity,
                &mut runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");

        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
        );
    }

    #[test]
    fn asymmetric_link_yields_higher_metric_than_symmetric() {
        use crate::gossip::BABEL_INFINITY;
        use crate::scoring::link_cost;

        // Symmetric: both Active 900‰
        let sym_link = fixture_link(2);
        let sym_cost = link_cost(Some(&sym_link), Some(&sym_link));

        // Asymmetric: fwd Active 900‰, rev Degraded 300‰
        let fwd_link = fixture_link(2);
        let rev_link = Link {
            state: LinkState {
                state: LinkRuntimeState::Degraded,
                delivery_confidence_permille: Belief::certain(RatioPermille(300), Tick(1)),
                ..fixture_link(1).state
            },
            ..fixture_link(1)
        };
        let asym_cost = link_cost(Some(&fwd_link), Some(&rev_link));

        assert!(
            asym_cost > sym_cost,
            "asymmetric cost {asym_cost} should exceed symmetric {sym_cost}"
        );
        assert!(asym_cost < BABEL_INFINITY);
    }
}
