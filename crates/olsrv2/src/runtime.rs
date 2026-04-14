//! `RoutingEngine` and `RouterManagedEngine` impls for `OlsrV2Engine`.

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HealthScore, Limit, NodeId,
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
        let Some(active) = self.active_routes.get(identity.route_id()) else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let Some(best) = self.best_next_hops.get(&active.destination) else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        let (stability_score, congestion_penalty_points) = health_scores_from_cost(best.path_cost);
        runtime.health.last_validated_at_tick = self.effects.now_tick();
        runtime.health.stability_score = stability_score;
        runtime.health.congestion_penalty_points = congestion_penalty_points;
        runtime.health.reachability_state = ReachabilityState::Reachable;
        if best.next_hop != active.next_hop {
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
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        ByteCount, Configuration, ControllerId, Environment, FactSourceClass, LinkEndpoint,
        Observation, OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass,
        RoutingTickContext, Tick, TransportError, TransportKind,
    };
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

    fn topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(3),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(1), ControllerId([1; 32])),
                                endpoint(1),
                                Tick(1),
                            ),
                            &OLSRV2_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(2),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(2), ControllerId([2; 32])),
                                endpoint(2),
                                Tick(1),
                            ),
                            &OLSRV2_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(3),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(3), ControllerId([3; 32])),
                                endpoint(3),
                                Tick(1),
                            ),
                            &OLSRV2_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links: BTreeMap::from([
                    (
                        (node(1), node(2)),
                        LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
                    ),
                    (
                        (node(2), node(1)),
                        LinkPreset::active(LinkPresetOptions::new(endpoint(1), Tick(1))).build(),
                    ),
                    (
                        (node(1), node(3)),
                        LinkPreset::active(LinkPresetOptions::new(endpoint(3), Tick(1))).build(),
                    ),
                    (
                        (node(3), node(1)),
                        LinkPreset::active(LinkPresetOptions::new(endpoint(1), Tick(1))).build(),
                    ),
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
        let topology = topology();
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
        let topology = topology();
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
        let topology = topology();
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
        let topology = topology();
        let mut engine = OlsrV2Engine::new(
            node(1),
            InMemoryTransport::new(),
            InMemoryRuntimeEffects {
                now: Tick(2),
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
                updated_at_tick: Tick(2),
                topology_epoch: topology.value.epoch,
                backend_route_id: jacquard_core::BackendRouteId(vec![1]),
            },
        );
        let objective = jacquard_core::RoutingObjective {
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
        };
        let profile = jacquard_core::SelectedRoutingParameters {
            selected_protection: jacquard_core::RouteProtectionClass::LinkProtected,
            selected_connectivity: jacquard_core::ConnectivityPosture {
                repair: jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        };
        let candidate = engine.candidate_for(&objective, &engine.best_next_hops[&node(2)]);
        let admission = engine.admission_for(&objective, &profile, &candidate);
        let stamp = jacquard_core::RouteIdentityStamp {
            route_id: jacquard_core::RouteId(*b"olsr-route-00001"),
            topology_epoch: topology.value.epoch,
            materialized_at_tick: Tick(2),
            publication_id: jacquard_core::PublicationId(*b"publication-0001"),
        };
        let handle = jacquard_core::RouteHandle {
            stamp: stamp.clone(),
        };
        let lease = jacquard_core::RouteLease {
            owner_node_id: node(1),
            lease_epoch: topology.value.epoch,
            valid_for: jacquard_core::TimeWindow::new(Tick(2), Tick(10)).expect("lease window"),
        };
        let installation = engine
            .materialize_route(jacquard_core::RouteMaterializationInput {
                handle: handle.clone(),
                admission: admission.clone(),
                lease: lease.clone(),
            })
            .expect("materialize route");
        let mut runtime_state = jacquard_core::RouteRuntimeState {
            last_lifecycle_event: installation.last_lifecycle_event,
            health: installation.health,
            progress: installation.progress,
        };
        let published = jacquard_core::PublishedRouteRecord {
            stamp,
            proof: installation.materialization_proof,
            admission: admission.clone(),
            lease,
        };
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
}
