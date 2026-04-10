//! `RoutingEngine` and `RouterManagedEngine` impls for `BatmanEngine`.
//!
//! Provides the full lifecycle surface for installed BATMAN routes:
//!
//! - `materialize_route` — resolves the admitted backend route against the
//!   current best next-hop table, records an `ActiveBatmanRoute`, and returns a
//!   `RouteInstallation` with a `RouteMaterializationProof` and initial route
//!   health derived from the best-next-hop TQ score.
//! - `engine_tick` — delegates to `refresh_private_state` and returns the
//!   appropriate `RoutingTickHint`: `Immediate` when private state changed,
//!   otherwise `WithinTicks` bounded by
//!   `decay_window.next_refresh_within_ticks`.
//! - `maintain_route` — checks whether the active route's next-hop has been
//!   superseded by a better neighbor; returns `ReplacementRequired` if so, or
//!   `Failed(LostReachability)` if the originator has become unreachable.
//! - `teardown` — removes the route from the active table.
//! - `RouterManagedEngine` — provides `local_node_id_for_router`,
//!   `forward_payload_for_router` (sends to the next-hop endpoint via
//!   `TransportSenderEffects`), and `restore_route_runtime_for_router`.

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
    gossip::{
        decode_advertisement, encode_advertisement, local_advertisement, LearnedAdvertisement,
    },
    private_state::link_is_usable,
    public_state::ActiveBatmanRoute,
    scoring, BatmanEngine, BATMAN_ENGINE_ID,
};

fn health_scores_from_tq(tq: RatioPermille) -> (HealthScore, jacquard_core::PenaltyPoints) {
    let penalty = u16::try_from(scoring::PERMILLE_MAX)
        .expect("permille max fits u16")
        .saturating_sub(tq.0);
    (
        HealthScore(u32::from(tq.0)),
        jacquard_core::PenaltyPoints(u32::from(penalty)),
    )
}

impl<Transport, Effects> BatmanEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn direct_neighbor_endpoints(
        &self,
        topology: &Observation<Configuration>,
    ) -> Vec<LinkEndpoint> {
        topology
            .value
            .links
            .iter()
            .filter(|((from_node_id, _), link)| {
                *from_node_id == self.local_node_id && link_is_usable(link.state.state)
            })
            .map(|((_, _), link)| link.endpoint.clone())
            .collect()
    }

    fn flood_gossip(
        &mut self,
        topology: &Observation<Configuration>,
        observed_at_tick: Tick,
    ) -> Result<(), RouteError> {
        let neighbor_endpoints = self.direct_neighbor_endpoints(topology);
        if neighbor_endpoints.is_empty() {
            return Ok(());
        }

        let mut advertisements = self
            .learned_advertisements
            .values()
            .cloned()
            .collect::<Vec<_>>();
        advertisements.push(LearnedAdvertisement::new(
            local_advertisement(self.local_node_id, topology, observed_at_tick.0),
            observed_at_tick,
        ));

        for neighbor in &neighbor_endpoints {
            for learned in &advertisements {
                let Ok(payload) = encode_advertisement(&learned.advertisement) else {
                    continue;
                };
                self.transport.send_transport(neighbor, &payload)?;
            }
        }

        Ok(())
    }

    fn ingest_advertisement(&mut self, payload: &[u8], observed_at_tick: Tick) {
        let Some(advertisement) = decode_advertisement(payload) else {
            return;
        };
        if advertisement.originator == self.local_node_id {
            return;
        }

        let Some(is_newer) = self
            .learned_advertisements
            .get(&advertisement.originator)
            .map(|known| advertisement.sequence > known.advertisement.sequence)
            .or(Some(true))
        else {
            return;
        };
        if !is_newer {
            return;
        }

        self.learned_advertisements.insert(
            advertisement.originator,
            LearnedAdvertisement::new(advertisement, observed_at_tick),
        );
    }
}

impl<Transport, Effects> RoutingEngine for BatmanEngine<Transport, Effects>
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
        if input.admission.backend_ref.engine != BATMAN_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        let Some(best) = self.best_next_hops.get(&destination) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        self.active_routes.insert(
            *input.handle.route_id(),
            ActiveBatmanRoute {
                destination,
                next_hop: best.next_hop,
                backend_route_id: best.backend_route_id.clone(),
                installed_at_tick: self.effects.now_tick(),
            },
        );
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
            health: {
                let (stability_score, congestion_penalty_points) = health_scores_from_tq(best.tq);
                RouteHealth {
                    reachability_state: ReachabilityState::Reachable,
                    stability_score,
                    congestion_penalty_points,
                    last_validated_at_tick: self.effects.now_tick(),
                }
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
        let Some(best) = self.best_next_hops.get(&active_route.destination) else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        let (stability_score, congestion_penalty_points) = health_scores_from_tq(best.tq);
        runtime.health.last_validated_at_tick = self.effects.now_tick();
        runtime.health.stability_score = stability_score;
        runtime.health.congestion_penalty_points = congestion_penalty_points;
        if best.next_hop != active_route.next_hop {
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

impl<Transport, Effects> RouterManagedEngine for BatmanEngine<Transport, Effects>
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
            payload,
            observed_at_tick,
            ..
        } = observation
        {
            self.ingest_advertisement(payload, *observed_at_tick);
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
    use crate::public_state::DecayWindow;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(64))
    }

    fn batman_node(byte: u8) -> Node {
        NodePreset::route_capable(
            NodePresetOptions::new(
                NodeIdentity::new(node(byte), ControllerId([byte; 32])),
                endpoint(byte),
                Tick(1),
            ),
            &BATMAN_ENGINE_ID,
        )
        .build()
    }

    fn link(remote: u8, delivery: u16, symmetry: u16, loss: u16) -> Link {
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
                transfer_rate_bytes_per_sec: Belief::certain(128_000, Tick(1)),
                stability_horizon_ms: Belief::certain(DurationMs(4_000), Tick(1)),
                loss_permille: RatioPermille(loss),
                delivery_confidence_permille: Belief::certain(RatioPermille(delivery), Tick(1)),
                symmetry_permille: Belief::certain(RatioPermille(symmetry), Tick(1)),
            },
        }
    }

    fn sample_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(1),
                nodes: BTreeMap::from([
                    (node(1), batman_node(1)),
                    (node(2), batman_node(2)),
                    (node(3), batman_node(3)),
                    (node(4), batman_node(4)),
                ]),
                links: BTreeMap::from([
                    ((node(1), node(2)), link(2, 960, 950, 5)),
                    ((node(2), node(4)), link(4, 940, 930, 10)),
                    ((node(1), node(3)), link(3, 910, 900, 20)),
                    ((node(3), node(4)), link(4, 800, 790, 80)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 2,
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
            destination: DestinationId::Node(node(4)),
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

    fn sample_runtime() -> RouteRuntimeState {
        RouteRuntimeState {
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: Tick(1),
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(1),
                last_progress_at_tick: Tick(1),
                state: RouteProgressState::Pending,
            },
        }
    }

    fn install_route(
        engine: &mut BatmanEngine<InMemoryTransport, InMemoryRuntimeEffects>,
        topology: &Observation<Configuration>,
    ) -> (PublishedRouteRecord, RouteRuntimeState) {
        let objective = sample_objective();
        let profile = sample_profile();
        let candidate = engine.candidate_routes(&objective, &profile, topology)[0].clone();
        let admission = engine
            .admit_route(&objective, &profile, candidate, topology)
            .expect("admission");
        let input = RouteMaterializationInput {
            handle: jacquard_core::RouteHandle {
                stamp: jacquard_core::RouteIdentityStamp {
                    route_id: engine.route_id_for(node(4)),
                    topology_epoch: topology.value.epoch,
                    materialized_at_tick: Tick(1),
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
        (
            PublishedRouteRecord {
                stamp: input.handle.stamp.clone(),
                proof: installation.materialization_proof,
                admission: input.admission,
                lease: input.lease,
            },
            sample_runtime(),
        )
    }

    #[test]
    fn maintain_route_recommends_replacement_for_better_next_hop() {
        let mut engine = BatmanEngine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        let topology = sample_topology();
        engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("populate table");
        let (identity, mut runtime) = install_route(&mut engine, &topology);

        let mut changed_topology = sample_topology();
        changed_topology
            .value
            .links
            .insert((node(1), node(2)), link(2, 600, 600, 250));
        changed_topology
            .value
            .links
            .insert((node(1), node(3)), link(3, 980, 970, 5));
        changed_topology
            .value
            .links
            .insert((node(3), node(4)), link(4, 960, 950, 5));
        engine
            .engine_tick(&RoutingTickContext::new(changed_topology))
            .expect("re-rank next hop");

        let result = engine
            .maintain_route(
                &identity,
                &mut runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");

        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::ReplacementRequired {
                trigger: RouteMaintenanceTrigger::LinkDegraded,
            }
        );
    }

    #[test]
    fn maintain_route_expires_when_originator_disappears() {
        let mut engine = BatmanEngine::with_decay_window(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
            DecayWindow {
                stale_after_ticks: 1,
                next_refresh_within_ticks: 2,
            },
        );
        let topology = sample_topology();
        engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("populate table");
        let (identity, mut runtime) = install_route(&mut engine, &topology);

        let mut changed_topology = sample_topology();
        changed_topology.observed_at_tick = Tick(4);
        changed_topology.value.links.clear();
        engine
            .engine_tick(&RoutingTickContext::new(changed_topology))
            .expect("remove reachability");

        let result = engine
            .maintain_route(
                &identity,
                &mut runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");

        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability)
        );
    }
}
