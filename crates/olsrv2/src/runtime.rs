//! `RoutingEngine` and `RouterManagedEngine` impls for `OlsrV2Engine`.

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HealthScore, Limit, MaterializedRoute, NodeId,
    PublishedRouteRecord, ReachabilityState, RouteCommitment, RouteError, RouteHealth, RouteId,
    RouteInstallation, RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RouteMaterializationProof, RouteProgressContract, RouteProgressState, RouteRuntimeError,
    RouteRuntimeState, RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick, TransportObservation,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, TimeEffects, TransportSenderEffects};

use crate::{
    gossip::{decode_message, encode_message, originated_hello, originated_tc, OlsrMessage},
    public_state::ActiveOlsrRoute,
    OlsrV2Engine, OLSRV2_ENGINE_ID,
};

fn health_scores_from_cost(path_cost: u32) -> (HealthScore, jacquard_core::PenaltyPoints) {
    let quality = 1000_u32.saturating_sub(path_cost.saturating_sub(1).min(1000));
    (
        HealthScore(quality),
        jacquard_core::PenaltyPoints(1000_u32.saturating_sub(quality)),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OlsrMaintenanceInput {
    runtime: RouteRuntimeState,
    active_route: ActiveOlsrRoute,
    best_next_hop: Option<crate::public_state::OlsrBestNextHop>,
    now_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OlsrMaintenanceTransition {
    next_runtime: RouteRuntimeState,
    result: RouteMaintenanceResult,
}

fn reduce_maintenance(input: OlsrMaintenanceInput) -> OlsrMaintenanceTransition {
    let mut next_runtime = input.runtime;
    let Some(best) = input.best_next_hop else {
        return OlsrMaintenanceTransition {
            next_runtime,
            result: RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            },
        };
    };
    let (stability_score, congestion_penalty_points) = health_scores_from_cost(best.path_cost);
    next_runtime.health.last_validated_at_tick = input.now_tick;
    next_runtime.health.stability_score = stability_score;
    next_runtime.health.congestion_penalty_points = congestion_penalty_points;
    next_runtime.health.reachability_state = ReachabilityState::Reachable;
    let result = if best.next_hop != input.active_route.next_hop {
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Replaced,
            outcome: RouteMaintenanceOutcome::ReplacementRequired {
                trigger: RouteMaintenanceTrigger::LinkDegraded,
            },
        }
    } else {
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        }
    };
    OlsrMaintenanceTransition {
        next_runtime,
        result,
    }
}

pub(crate) fn restored_active_route(route: &MaterializedRoute) -> Option<ActiveOlsrRoute> {
    let DestinationId::Node(destination) = route.identity.admission.objective.destination else {
        return None;
    };
    let backend = &route.identity.admission.backend_ref.backend_route_id.0;
    if backend.len() != 68 {
        return None;
    }
    let mut next_hop = [0_u8; 32];
    next_hop.copy_from_slice(&backend[32..64]);
    let next_hop = NodeId(next_hop);
    Some(ActiveOlsrRoute {
        destination,
        next_hop,
        backend_route_id: route
            .identity
            .admission
            .backend_ref
            .backend_route_id
            .clone(),
        installed_at_tick: route.identity.stamp.materialized_at_tick,
    })
}

impl<Transport, Effects> OlsrV2Engine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn flood_control(
        &mut self,
        topology: &jacquard_core::Observation<Configuration>,
    ) -> Result<(), RouteError> {
        let neighbors = self.direct_neighbor_endpoints(topology);
        if neighbors.is_empty() {
            return Ok(());
        }

        self.hello_sequence = self.hello_sequence.saturating_add(1);
        let advertised_neighbors = self.local_tc_advertised_neighbors(topology);
        let hello = originated_hello(
            self.local_node_id,
            self.hello_sequence,
            advertised_neighbors.iter().copied(),
            self.local_mprs(),
        );
        if let Ok(payload) = encode_message(&hello) {
            for (_, endpoint) in &neighbors {
                self.transport.send_transport(endpoint, &payload)?;
            }
        }

        if !advertised_neighbors.is_empty() {
            self.tc_sequence = self.tc_sequence.saturating_add(1);
            self.last_originated_tc_neighbors = advertised_neighbors.clone();
            let tc = originated_tc(
                self.local_node_id,
                self.tc_sequence,
                advertised_neighbors.iter().copied(),
            );
            if let Ok(payload) = encode_message(&tc) {
                for (_, endpoint) in &neighbors {
                    self.transport.send_transport(endpoint, &payload)?;
                }
            }
        }

        let pending_forwards: Vec<_> = self.pending_tc_forwards.values().cloned().collect();
        self.pending_tc_forwards.clear();
        for pending in pending_forwards {
            let payload = encode_message(&OlsrMessage::Tc(pending.tc.clone()))
                .map_err(|_| RouteRuntimeError::Invalidated)?;
            for (neighbor, endpoint) in &neighbors {
                if *neighbor == pending.received_from {
                    continue;
                }
                self.transport.send_transport(endpoint, &payload)?;
            }
            self.last_forwarded_tc_sequences
                .insert(pending.tc.originator, pending.tc.sequence);
        }

        Ok(())
    }

    fn ingest_message_payload(
        &mut self,
        from_node_id: NodeId,
        payload: &[u8],
        observed_at_tick: Tick,
    ) {
        let Some(message) = decode_message(payload) else {
            return;
        };
        let Some(topology) = self.latest_topology.clone() else {
            return;
        };
        match message {
            OlsrMessage::Hello(hello) => {
                self.ingest_hello(from_node_id, hello, &topology, observed_at_tick)
            }
            OlsrMessage::Tc(tc) => self.ingest_tc(from_node_id, tc, &topology, observed_at_tick),
        }
    }
}

impl<Transport, Effects> RoutingEngine for OlsrV2Engine<Transport, Effects>
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
        if input.admission.backend_ref.engine != OLSRV2_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        let Some(best) = self.best_next_hops.get(&destination) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        self.active_routes.insert(
            *input.handle.route_id(),
            ActiveOlsrRoute {
                destination,
                next_hop: best.next_hop,
                backend_route_id: best.backend_route_id.clone(),
                installed_at_tick: self.effects.now_tick(),
            },
        );
        let (stability_score, congestion_penalty_points) = health_scores_from_cost(best.path_cost);
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
        let change = self.refresh_private_state(&tick.topology, tick.topology.observed_at_tick);
        self.flood_control(&tick.topology)?;
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change,
            next_tick_hint: if change == RoutingTickChange::PrivateStateUpdated
                || !self.pending_tc_forwards.is_empty()
            {
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
        let Some(active) = self.active_routes.get(identity.route_id()).cloned() else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let transition = reduce_maintenance(OlsrMaintenanceInput {
            runtime: runtime.clone(),
            active_route: active,
            best_next_hop: match identity.admission.objective.destination {
                DestinationId::Node(destination) => self.best_next_hops.get(&destination).cloned(),
                _ => None,
            },
            now_tick: self.effects.now_tick(),
        });
        *runtime = transition.next_runtime;
        Ok(transition.result)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        self.active_routes.remove(route_id);
    }
}

impl<Transport, Effects> RouterManagedEngine for OlsrV2Engine<Transport, Effects>
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
            self.ingest_message_payload(*from_node_id, payload, *observed_at_tick);
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

    fn restore_route_runtime_with_record_for_router(
        &mut self,
        route: &MaterializedRoute,
        topology: &jacquard_core::Observation<Configuration>,
    ) -> Result<bool, RouteError> {
        let Some(active_route) = restored_active_route(route) else {
            return Ok(false);
        };
        self.latest_topology = Some(topology.clone());
        self.active_routes
            .insert(route.identity.stamp.route_id, active_route);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use jacquard_core::{
        ByteCount, Configuration, ControllerId, Environment, FactSourceClass, LinkEndpoint,
        Observation, OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass,
        RoutingTickContext, Tick, TransportError, TransportKind,
    };
    use jacquard_host_support::opaque_endpoint;
    use jacquard_mem_link_profile::{
        InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
    };
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::{effect_handler, RoutingEngine, TransportSenderEffects};

    use super::*;
    use crate::gossip::OlsrMessage;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
    }

    fn fixture_node(byte: u8) -> jacquard_core::Node {
        NodePreset::route_capable(
            NodePresetOptions::new(
                NodeIdentity::new(node(byte), ControllerId([byte; 32])),
                endpoint(byte),
                Tick(1),
            ),
            &OLSRV2_ENGINE_ID,
        )
        .build()
    }

    fn fixture_link(byte: u8) -> jacquard_core::Link {
        LinkPreset::active(LinkPresetOptions::new(endpoint(byte), Tick(1))).build()
    }

    fn sample_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(3),
                nodes: BTreeMap::from([
                    (node(1), fixture_node(1)),
                    (node(2), fixture_node(2)),
                    (node(3), fixture_node(3)),
                ]),
                links: BTreeMap::from([
                    ((node(1), node(2)), fixture_link(2)),
                    ((node(2), node(1)), fixture_link(1)),
                    ((node(1), node(3)), fixture_link(3)),
                    ((node(3), node(1)), fixture_link(1)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 2,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    fn sample_objective() -> jacquard_core::RoutingObjective {
        jacquard_core::RoutingObjective {
            destination: jacquard_core::DestinationId::Node(node(2)),
            service_kind: jacquard_core::RouteServiceKind::Move,
            target_protection: jacquard_core::RouteProtectionClass::LinkProtected,
            protection_floor: jacquard_core::RouteProtectionClass::LinkProtected,
            target_connectivity: jacquard_core::ConnectivityPosture {
                repair: jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
            latency_budget_ms: jacquard_core::Limit::Bounded(jacquard_core::DurationMs(100)),
            protection_priority: jacquard_core::PriorityPoints(10),
            connectivity_priority: jacquard_core::PriorityPoints(10),
        }
    }

    fn sample_profile() -> jacquard_core::SelectedRoutingParameters {
        jacquard_core::SelectedRoutingParameters {
            selected_protection: jacquard_core::RouteProtectionClass::LinkProtected,
            selected_connectivity: jacquard_core::ConnectivityPosture {
                repair: jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn materialized_route_record(
        engine: &mut OlsrV2Engine<InMemoryTransport, InMemoryRuntimeEffects>,
        topology: &Observation<Configuration>,
        admission: jacquard_core::RouteAdmission,
        now: Tick,
    ) -> (PublishedRouteRecord, RouteRuntimeState) {
        let input = jacquard_core::RouteMaterializationInput {
            handle: jacquard_core::RouteHandle {
                stamp: jacquard_core::RouteIdentityStamp {
                    route_id: jacquard_core::RouteId(*b"olsr-route-00001"),
                    topology_epoch: topology.value.epoch,
                    materialized_at_tick: now,
                    publication_id: jacquard_core::PublicationId(*b"publication-0001"),
                },
            },
            admission,
            lease: jacquard_core::RouteLease {
                owner_node_id: node(1),
                lease_epoch: topology.value.epoch,
                valid_for: jacquard_core::TimeWindow::new(now, Tick(10)).expect("lease window"),
            },
        };
        let installation = engine
            .materialize_route(input.clone())
            .expect("materialize route");
        (
            PublishedRouteRecord {
                stamp: input.handle.stamp,
                proof: installation.materialization_proof,
                admission: input.admission,
                lease: input.lease,
            },
            RouteRuntimeState {
                last_lifecycle_event: installation.last_lifecycle_event,
                health: installation.health,
                progress: installation.progress,
            },
        )
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct SentFrame {
        endpoint: LinkEndpoint,
        payload: Vec<u8>,
    }

    #[derive(Clone)]
    struct RecordingSender {
        frames: Arc<Mutex<Vec<SentFrame>>>,
    }

    #[effect_handler]
    impl TransportSenderEffects for RecordingSender {
        fn send_transport(
            &mut self,
            endpoint: &LinkEndpoint,
            payload: &[u8],
        ) -> Result<(), TransportError> {
            self.frames.lock().expect("frames lock").push(SentFrame {
                endpoint: endpoint.clone(),
                payload: payload.to_vec(),
            });
            Ok(())
        }
    }

    #[test]
    fn engine_tick_sets_latest_topology_and_emits_no_error() {
        let topology = sample_topology();
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );

        let outcome = engine
            .engine_tick(&RoutingTickContext::new(topology.clone()))
            .expect("engine tick");

        assert_eq!(outcome.topology_epoch, topology.value.epoch);
        assert_eq!(engine.latest_topology, Some(topology));
    }

    #[test]
    fn engine_tick_emits_hello_and_tc_when_symmetric_neighbors_exist() {
        let topology = sample_topology();
        let frames = Arc::new(Mutex::new(Vec::new()));
        let mut engine = OlsrV2Engine::new(
            node(1),
            RecordingSender {
                frames: frames.clone(),
            },
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );

        engine
            .engine_tick(&RoutingTickContext::new(topology))
            .expect("engine tick");

        let sent = frames.lock().expect("frames lock");
        let decoded: Vec<_> = sent
            .iter()
            .map(|frame| decode_message(&frame.payload).expect("decode message"))
            .collect();
        assert_eq!(decoded.len(), 4);
        assert!(decoded
            .iter()
            .any(|message| matches!(message, OlsrMessage::Hello(_))));
        assert!(decoded
            .iter()
            .any(|message| matches!(message, OlsrMessage::Tc(_))));
    }

    #[test]
    fn tc_forward_requires_mpr_selector_state() {
        let topology = sample_topology();
        let frames = Arc::new(Mutex::new(Vec::new()));
        let mut engine = OlsrV2Engine::new(
            node(1),
            RecordingSender {
                frames: frames.clone(),
            },
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        );
        engine.latest_topology = Some(topology.clone());
        engine.ingest_hello(
            node(2),
            crate::gossip::HelloMessage {
                originator: node(2),
                sequence: 1,
                symmetric_neighbors: vec![node(1)],
                mprs: vec![node(1)],
            },
            &topology,
            Tick(1),
        );
        engine.ingest_tc(
            node(2),
            crate::gossip::TcMessage {
                originator: node(2),
                sequence: 7,
                advertised_neighbors: vec![node(3)],
            },
            &topology,
            Tick(1),
        );

        engine
            .engine_tick(&RoutingTickContext::new(topology))
            .expect("engine tick");

        let decoded: Vec<_> = frames
            .lock()
            .expect("frames lock")
            .iter()
            .map(|frame| decode_message(&frame.payload).expect("decode message"))
            .collect();
        let forwarded_tcs = decoded
            .iter()
            .filter(|message| matches!(message, OlsrMessage::Tc(tc) if tc.originator == node(2) && tc.sequence == 7))
            .count();
        assert_eq!(forwarded_tcs, 1);
    }

    #[test]
    fn maintain_route_requests_replacement_when_next_hop_changes() {
        let topology = sample_topology();
        let now = Tick(2);
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now,
                ..Default::default()
            },
        );
        engine.latest_topology = Some(topology.clone());
        engine.best_next_hops.insert(
            node(2),
            crate::public_state::OlsrBestNextHop {
                destination: node(2),
                next_hop: node(2),
                hop_count: 1,
                path_cost: 1,
                degradation: jacquard_core::RouteDegradation::None,
                transport_kind: TransportKind::WifiAware,
                updated_at_tick: now,
                topology_epoch: topology.value.epoch,
                backend_route_id: jacquard_core::BackendRouteId(vec![1]),
            },
        );
        let objective = sample_objective();
        let profile = sample_profile();
        let snapshot = engine.planner_snapshot();
        let candidate = crate::private_state::candidate_for_snapshot(
            &snapshot,
            &objective,
            &engine.best_next_hops[&node(2)],
        );
        let admission =
            crate::private_state::admission_for_candidate(&objective, &profile, &candidate);
        let (published, mut runtime_state) =
            materialized_route_record(&mut engine, &topology, admission.clone(), now);
        engine.best_next_hops.insert(
            node(2),
            crate::public_state::OlsrBestNextHop {
                next_hop: node(3),
                ..engine.best_next_hops[&node(2)].clone()
            },
        );

        let result = engine
            .maintain_route(
                &published,
                &mut runtime_state,
                jacquard_core::RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintain route");

        assert!(matches!(
            result.outcome,
            jacquard_core::RouteMaintenanceOutcome::ReplacementRequired { .. }
        ));
    }

    #[test]
    // long-block-exception: the parity test keeps the full reduced-vs-wrapper
    // maintenance setup in one place so route-state comparisons stay explicit.
    fn maintenance_reducer_matches_wrapper_projection() {
        let topology = sample_topology();
        let now = Tick(3);
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now,
                ..Default::default()
            },
        );
        engine.latest_topology = Some(topology.clone());
        engine.best_next_hops.insert(
            node(2),
            crate::public_state::OlsrBestNextHop {
                destination: node(2),
                next_hop: node(2),
                hop_count: 1,
                path_cost: 1,
                degradation: jacquard_core::RouteDegradation::None,
                transport_kind: TransportKind::WifiAware,
                updated_at_tick: now,
                topology_epoch: topology.value.epoch,
                backend_route_id: jacquard_core::BackendRouteId(vec![1]),
            },
        );
        let objective = sample_objective();
        let profile = sample_profile();
        let snapshot = engine.planner_snapshot();
        let candidate = crate::private_state::candidate_for_snapshot(
            &snapshot,
            &objective,
            &engine.best_next_hops[&node(2)],
        );
        let admission =
            crate::private_state::admission_for_candidate(&objective, &profile, &candidate);
        let (published, mut runtime_state) =
            materialized_route_record(&mut engine, &topology, admission.clone(), now);
        let reduced = reduce_maintenance(OlsrMaintenanceInput {
            runtime: runtime_state.clone(),
            active_route: engine
                .active_routes
                .get(published.route_id())
                .cloned()
                .expect("active route"),
            best_next_hop: engine.best_next_hops.get(&node(2)).cloned(),
            now_tick: now,
        });

        let wrapper_result = engine
            .maintain_route(
                &published,
                &mut runtime_state,
                jacquard_core::RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintain route");

        assert_eq!(runtime_state, reduced.next_runtime);
        assert_eq!(wrapper_result, reduced.result);
    }

    #[test]
    // long-block-exception: the restore test keeps route reconstruction,
    // router-led restore, and forward-after-restore checks together.
    fn restore_reconstructs_active_route_from_materialized_route() {
        let topology = sample_topology();
        let now = Tick(3);
        let backend_route_id = jacquard_core::BackendRouteId({
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&node(2).0);
            bytes.extend_from_slice(&node(2).0);
            bytes.extend_from_slice(&7_u32.to_le_bytes());
            bytes
        });
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now,
                ..Default::default()
            },
        );
        engine.latest_topology = Some(topology.clone());
        engine.best_next_hops.insert(
            node(2),
            crate::public_state::OlsrBestNextHop {
                destination: node(2),
                next_hop: node(2),
                hop_count: 1,
                path_cost: 7,
                degradation: jacquard_core::RouteDegradation::None,
                transport_kind: TransportKind::WifiAware,
                updated_at_tick: now,
                topology_epoch: topology.value.epoch,
                backend_route_id: backend_route_id.clone(),
            },
        );
        let objective = sample_objective();
        let profile = sample_profile();
        let snapshot = engine.planner_snapshot();
        let candidate = crate::private_state::candidate_for_snapshot(
            &snapshot,
            &objective,
            &engine.best_next_hops[&node(2)],
        );
        let mut admission =
            crate::private_state::admission_for_candidate(&objective, &profile, &candidate);
        admission.backend_ref.backend_route_id = backend_route_id;
        let (identity, runtime_state) =
            materialized_route_record(&mut engine, &topology, admission, now);
        let route = MaterializedRoute {
            identity,
            runtime: runtime_state,
        };

        engine.active_routes.clear();
        let restored = engine
            .restore_route_runtime_with_record_for_router(&route, &topology)
            .expect("restore route");

        assert!(restored);
        assert_eq!(
            engine.active_routes.get(&route.identity.stamp.route_id),
            Some(&ActiveOlsrRoute {
                destination: node(2),
                next_hop: node(2),
                backend_route_id: route
                    .identity
                    .admission
                    .backend_ref
                    .backend_route_id
                    .clone(),
                installed_at_tick: now,
            })
        );
    }
}
