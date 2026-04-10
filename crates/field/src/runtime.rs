//! `RoutingEngine` and `RouterManagedEngine` implementations for `FieldEngine`.
//!
//! `materialize_route` decodes the backend token, validates the destination
//! state, and records an `ActiveFieldRoute` keyed by route ID. `engine_tick`
//! drives the per-tick control loop: seeding destinations from topology,
//! advancing the PI control plane, refreshing destination observers, stepping
//! protocol sessions, and deriving the attractor view. `maintain_route`
//! evaluates the installed route on each maintenance trigger, checking
//! corridor support, congestion price, attractor drift, and frontier
//! freshness to decide between hold fallback, replacement, or continuation.
//! Routes expire when delivery support falls below 250 permille or the
//! frontier has been stale for more than four ticks.
// long-file-exception: runtime owns the synchronous field control loop,
// maintenance path, protocol bridge integration, and router-facing hooks; the
// file stays together so those transitions can be audited end to end.

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HealthScore, Limit, NodeId,
    PublishedRouteRecord, ReachabilityState, RouteCommitment, RouteError, RouteHealth, RouteId,
    RouteInstallation, RouteLifecycleEvent, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
    RouteMaterializationProof, RouteProgressContract, RouteProgressState, RouteRuntimeError,
    RouteRuntimeState, RouteSelectionError, RoutingTickChange, RoutingTickContext, RoutingTickHint,
    RoutingTickOutcome, Tick,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine};

use crate::{
    attractor::{derive_local_attractor_view, rank_frontier_by_attractor},
    choreography::{
        FieldHostWaitStatus, FieldProtocolKind, FieldProtocolSessionKey, QueuedProtocolSend,
        FIELD_PROTOCOL_SESSION_MAX,
    },
    control::{advance_control_plane, ControlMeasurements},
    observer::{update_destination_observer, ObserverInputs},
    route::{decode_backend_token, ActiveFieldRoute},
    state::{
        HopBand, NeighborContinuation, ObserverInputSignature, SupportBucket,
        SUMMARY_HEARTBEAT_TICKS,
    },
    summary::{
        summary_divergence, DirectEvidence, EvidenceContributionClass, FieldSummary,
        LocalOriginTrace, SummaryDestinationKey, SummaryUncertaintyClass,
    },
    FieldEngine,
};

impl<Transport, Effects> RoutingEngine for FieldEngine<Transport, Effects> {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, RouteError> {
        if input.admission.backend_ref.engine != crate::FIELD_ENGINE_ID {
            return Err(RouteRuntimeError::Invalidated.into());
        }
        let token = decode_backend_token(&input.admission.backend_ref.backend_route_id)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let detail = self
            .witness_detail_for_destination(&token.destination)
            .ok_or(RouteRuntimeError::Invalidated)?;
        let corridor_envelope = self
            .state
            .destinations
            .get(&token.destination)
            .map(|state| state.corridor_belief.clone())
            .ok_or(RouteRuntimeError::Invalidated)?;

        self.active_routes.insert(
            *input.handle.route_id(),
            ActiveFieldRoute {
                destination: token.destination,
                primary_neighbor: token.primary_neighbor,
                alternates: token.alternates,
                corridor_envelope: corridor_envelope.clone(),
                witness_detail: detail,
                backend_route_id: input.admission.backend_ref.backend_route_id.clone(),
                topology_epoch: input.handle.topology_epoch(),
                installed_at_tick: input.handle.materialized_at_tick(),
            },
        );

        Ok(RouteInstallation {
            materialization_proof: RouteMaterializationProof {
                stamp: input.handle.stamp.clone(),
                witness: Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness.clone(),
                    established_at_tick: input.handle.materialized_at_tick(),
                },
            },
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: route_health_for(&corridor_envelope, input.handle.materialized_at_tick()),
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(u32::from(
                    corridor_envelope.expected_hop_band.max_hops.max(1),
                )),
                total_step_count_max: Limit::Bounded(
                    u32::from(corridor_envelope.expected_hop_band.max_hops.max(1))
                        .saturating_add(2),
                ),
                last_progress_at_tick: input.handle.materialized_at_tick(),
                state: RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(&self, _route: &jacquard_core::MaterializedRoute) -> Vec<RouteCommitment> {
        Vec::new()
    }

    fn engine_tick(&mut self, tick: &RoutingTickContext) -> Result<RoutingTickOutcome, RouteError> {
        let now_tick = tick.topology.observed_at_tick;
        let topology_seeded = self.seed_destinations_from_topology(&tick.topology.value, now_tick);
        let measurements =
            ControlMeasurements::from_topology(&tick.topology.value, self.local_node_id);
        let slow_path = advance_control_plane(&mut self.state, measurements, now_tick);
        let observer_changed = self.refresh_destination_observers(&tick.topology.value, now_tick);
        let protocol_changed = self.advance_protocol_sessions(tick.topology.value.epoch, now_tick);
        let attractor_view = derive_local_attractor_view(&self.state);
        let attractor_active =
            !attractor_view.entries.is_empty() && attractor_view.coherence_score.value() > 0;
        let changed = self.state.last_tick_processed != now_tick
            || topology_seeded
            || slow_path.changed
            || observer_changed
            || protocol_changed;
        self.state.note_tick(now_tick);
        Ok(RoutingTickOutcome {
            topology_epoch: tick.topology.value.epoch,
            change: if changed {
                RoutingTickChange::PrivateStateUpdated
            } else {
                RoutingTickChange::NoChange
            },
            next_tick_hint: if observer_changed {
                RoutingTickHint::Immediate
            } else if slow_path.changed || attractor_active || protocol_changed {
                RoutingTickHint::WithinTicks(Tick(1))
            } else {
                RoutingTickHint::HostDefault
            },
        })
    }

    // long-block-exception: maintenance is a single fail-closed decision path
    // over posture, support, freshness, alternates, and hold fallback.
    fn maintain_route(
        &mut self,
        identity: &PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let Some(active) = self.active_routes.get_mut(identity.route_id()) else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let Some(destination_state) = self.state.destinations.get(&active.destination) else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        let ranked = rank_frontier_by_attractor(
            destination_state,
            &self.state.mean_field,
            self.state.regime.current,
            self.state.posture.current,
            &self.state.controller,
        );
        let Some((best, _)) = ranked.first() else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };

        runtime.health = route_health_for(
            &destination_state.corridor_belief,
            self.state.last_tick_processed,
        );
        runtime.progress.last_progress_at_tick = self.state.last_tick_processed;

        if self.state.controller.congestion_price.value() >= 850
            && self.state.posture.current == crate::state::RoutingPosture::RetentionBiased
        {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::EnteredPartitionMode,
                outcome: RouteMaintenanceOutcome::HoldFallback {
                    trigger: RouteMaintenanceTrigger::CapacityExceeded,
                    retained_object_count: jacquard_core::HoldItemCount(1),
                },
            });
        }

        if destination_state.corridor_belief.delivery_support.value() < 250 {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded),
            });
        }

        if active.primary_neighbor != best.neighbor_id {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
            });
        }

        if self.state.controller.congestion_price.value() >= 850 {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::CapacityExceeded,
                },
            });
        }

        if self.state.posture.current != active.witness_detail.posture {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::PolicyShift,
                },
            });
        }

        let is_stale = self
            .state
            .last_tick_processed
            .0
            .saturating_sub(best.freshness.0)
            > 4;
        if is_stale {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Activated,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
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

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    fn seed_destinations_from_topology(
        &mut self,
        topology: &Configuration,
        now_tick: Tick,
    ) -> bool {
        let mut changed = false;
        for (node_id, node) in &topology.nodes {
            if *node_id == self.local_node_id {
                continue;
            }
            let supports_field = node
                .profile
                .services
                .iter()
                .any(|service| service.routing_engines.contains(&crate::FIELD_ENGINE_ID));
            if !supports_field {
                continue;
            }
            let destination = DestinationId::Node(*node_id);
            let key = crate::state::DestinationKey::from(&destination);
            let existed = self.state.destinations.contains_key(&key);
            let interest_class = if topology.links.contains_key(&(self.local_node_id, *node_id)) {
                crate::state::DestinationInterestClass::Transit
            } else {
                crate::state::DestinationInterestClass::Propagated
            };
            self.state
                .upsert_destination_interest(&destination, interest_class, now_tick);
            changed |= !existed;
        }
        changed
    }

    // long-block-exception: protocol advancement intentionally batches all
    // bounded field cooperative flows in one deterministic per-round pass.
    fn advance_protocol_sessions(
        &mut self,
        topology_epoch: jacquard_core::RouteEpoch,
        now_tick: Tick,
    ) -> bool {
        let attractor_view = derive_local_attractor_view(&self.state);
        let low_coherence =
            !attractor_view.entries.is_empty() && attractor_view.coherence_score.value() < 200;
        let mut changed = false;

        let active_keys = self.state.active_destination_keys();
        for (index, destination_key) in active_keys
            .into_iter()
            .take(FIELD_PROTOCOL_SESSION_MAX)
            .enumerate()
        {
            let Some(destination_state) = self.state.destinations.get(&destination_key) else {
                continue;
            };
            let Some(primary) = destination_state.frontier.as_slice().first() else {
                continue;
            };
            let destination = DestinationId::from(&destination_state.destination);
            let session_destination = SummaryDestinationKey::from(&destination);
            let summary =
                summary_for_destination(destination_state, topology_epoch, now_tick, &destination);
            let should_publish = should_transmit_summary(destination_state, &summary, now_tick);
            let primary_neighbor = primary.neighbor_id;
            let delivery_support = destination_state.corridor_belief.delivery_support.value();
            let is_stale = now_tick.0.saturating_sub(primary.freshness.0) > 4;

            let dissemination_key = FieldProtocolSessionKey {
                protocol: FieldProtocolKind::SummaryDissemination,
                route_id: None,
                topology_epoch,
                destination: Some(session_destination),
            };
            if should_publish {
                if let Ok(capability) =
                    self.protocol_runtime
                        .open_session(&dissemination_key, 0, None)
                {
                    let _ = self.protocol_runtime.queue_summary_flow(
                        &capability,
                        [QueuedProtocolSend {
                            protocol: FieldProtocolKind::SummaryDissemination,
                            to_neighbor: primary_neighbor,
                            payload: summary.encode(),
                        }],
                    );
                    let published = self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .map(|advance| advance.round.emitted_send_count > 0)
                        .unwrap_or(false);
                    if published {
                        if let Some(state) = self.state.destinations.get_mut(&destination_key) {
                            state.publication.record(summary.clone(), now_tick);
                        }
                    }
                    changed |= published;
                }
            }

            if is_stale {
                let anti_entropy_key = FieldProtocolSessionKey {
                    protocol: FieldProtocolKind::AntiEntropy,
                    route_id: None,
                    topology_epoch,
                    destination: Some(session_destination),
                };
                if let Ok(capability) =
                    self.protocol_runtime
                        .open_session(&anti_entropy_key, 0, None)
                {
                    changed |= self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .is_ok();
                }
            }

            if self.state.posture.current == crate::state::RoutingPosture::RetentionBiased
                && delivery_support < 400
            {
                let replay_key = FieldProtocolSessionKey {
                    protocol: FieldProtocolKind::RetentionReplay,
                    route_id: None,
                    topology_epoch,
                    destination: Some(session_destination),
                };
                if let Ok(capability) = self.protocol_runtime.open_session(&replay_key, 0, None) {
                    let _ = self.protocol_runtime.queue_branch_choice(&capability, 1);
                    let _ = self.protocol_runtime.queue_summary_flow(
                        &capability,
                        [QueuedProtocolSend {
                            protocol: FieldProtocolKind::RetentionReplay,
                            to_neighbor: primary_neighbor,
                            payload: summary.encode(),
                        }],
                    );
                    changed |= self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .map(|advance| advance.round.emitted_send_count > 0)
                        .unwrap_or(false);
                }
            }

            if low_coherence && index == 0 {
                let coordination_key = FieldProtocolSessionKey {
                    protocol: FieldProtocolKind::ExplicitCoordination,
                    route_id: None,
                    topology_epoch,
                    destination: Some(session_destination),
                };
                if let Ok(capability) =
                    self.protocol_runtime
                        .open_session(&coordination_key, 0, None)
                {
                    changed |= self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .is_ok();
                }
            }
        }

        changed
    }

    // long-block-exception: observer refresh is one sparse incremental scan
    // that combines cache gating, evidence fusion, frontier refresh, and
    // endpoint capture.
    fn refresh_destination_observers(&mut self, topology: &Configuration, now_tick: Tick) -> bool {
        let topology_epoch = topology.epoch;
        let regime = self.state.regime.current;
        let control_state = self.state.controller.clone();
        let local_origin_trace = LocalOriginTrace {
            local_node_id: self.local_node_id,
            topology_epoch,
        };

        let mut changed = false;
        let active_keys = self.state.active_destination_keys();
        for destination_key in active_keys {
            let Some(destination_state) = self.state.destinations.get_mut(&destination_key) else {
                continue;
            };
            let destination = DestinationId::from(&destination_key);
            let direct_evidence = direct_evidence_for_destination(
                topology,
                self.local_node_id,
                &destination,
                now_tick,
            );
            let forward_evidence = Vec::new();
            let reverse_feedback = Vec::new();
            let signature = observer_input_signature(
                topology_epoch,
                regime,
                &control_state,
                &direct_evidence,
                &forward_evidence,
                &reverse_feedback,
            );
            if !destination_state
                .observer_cache
                .should_refresh(signature, now_tick)
            {
                continue;
            }
            let had_state = (
                destination_state.posterior.clone(),
                destination_state.progress_belief.clone(),
                destination_state.corridor_belief.clone(),
            );
            update_destination_observer(
                destination_state,
                &ObserverInputs {
                    destination,
                    topology_epoch,
                    now_tick,
                    direct_evidence: direct_evidence.clone(),
                    forward_evidence,
                    reverse_feedback,
                    local_origin_trace,
                    regime,
                    control_state: control_state.clone(),
                },
            );
            destination_state.frontier = refresh_frontier_from_direct_evidence(
                destination_state.frontier.clone(),
                destination_state.corridor_belief.expected_hop_band,
                destination_state.corridor_belief.delivery_support,
                &direct_evidence,
                now_tick,
            );
            destination_state.observer_cache.record(signature, now_tick);
            if !direct_evidence.is_empty() {
                for evidence in &direct_evidence {
                    self.state
                        .neighbor_endpoints
                        .insert(evidence.neighbor_id, evidence.link.endpoint.clone());
                }
            }
            let has_state = (
                destination_state.posterior.clone(),
                destination_state.progress_belief.clone(),
                destination_state.corridor_belief.clone(),
            );
            changed |= had_state != has_state;
        }
        changed
    }
}

fn direct_evidence_for_destination(
    topology: &Configuration,
    local_node_id: NodeId,
    destination: &DestinationId,
    now_tick: Tick,
) -> Vec<DirectEvidence> {
    let DestinationId::Node(node_id) = destination else {
        return Vec::new();
    };
    topology
        .links
        .get(&(local_node_id, *node_id))
        .cloned()
        .map(|link| {
            vec![DirectEvidence {
                neighbor_id: *node_id,
                link,
                observed_at_tick: now_tick,
            }]
        })
        .unwrap_or_default()
}

fn summary_for_destination(
    destination_state: &crate::state::DestinationFieldState,
    topology_epoch: jacquard_core::RouteEpoch,
    now_tick: Tick,
    destination: &DestinationId,
) -> FieldSummary {
    FieldSummary {
        destination: SummaryDestinationKey::from(destination),
        topology_epoch,
        freshness_tick: now_tick,
        hop_band: destination_state.corridor_belief.expected_hop_band,
        delivery_support: destination_state.corridor_belief.delivery_support,
        congestion_penalty: destination_state.corridor_belief.congestion_penalty,
        retention_support: destination_state.corridor_belief.retention_affinity,
        uncertainty_penalty: destination_state.posterior.usability_entropy,
        evidence_class: match destination_state.posterior.predicted_observation_class {
            crate::state::ObservationClass::DirectOnly => EvidenceContributionClass::Direct,
            crate::state::ObservationClass::ForwardPropagated
            | crate::state::ObservationClass::Mixed => EvidenceContributionClass::ForwardPropagated,
            crate::state::ObservationClass::ReverseValidated => {
                EvidenceContributionClass::ReverseFeedback
            }
        },
        uncertainty_class: match destination_state.posterior.usability_entropy.value() {
            0..=249 => SummaryUncertaintyClass::Low,
            250..=599 => SummaryUncertaintyClass::Medium,
            _ => SummaryUncertaintyClass::High,
        },
    }
}

fn refresh_frontier_from_direct_evidence(
    mut frontier: crate::state::ContinuationFrontier,
    corridor_hops: HopBand,
    corridor_support: SupportBucket,
    direct_evidence: &[DirectEvidence],
    now_tick: Tick,
) -> crate::state::ContinuationFrontier {
    frontier = frontier.prune_stale(now_tick, 4);
    for evidence in direct_evidence {
        frontier = frontier.insert(NeighborContinuation {
            neighbor_id: evidence.neighbor_id,
            net_value: corridor_support,
            downstream_support: corridor_support,
            expected_hop_band: HopBand::new(1, corridor_hops.max_hops.max(1)),
            freshness: now_tick,
        });
    }
    frontier
}

fn observer_input_signature(
    topology_epoch: jacquard_core::RouteEpoch,
    regime: crate::state::OperatingRegime,
    control_state: &crate::state::ControlState,
    direct_evidence: &[DirectEvidence],
    forward_evidence: &[crate::summary::ForwardPropagatedEvidence],
    reverse_feedback: &[crate::summary::ReverseFeedbackEvidence],
) -> ObserverInputSignature {
    ObserverInputSignature {
        topology_epoch,
        regime,
        direct_digest: direct_evidence_digest(direct_evidence),
        forward_digest: forward_evidence_digest(forward_evidence),
        reverse_digest: reverse_feedback_digest(reverse_feedback),
        control_signature: [
            control_state.congestion_price.value(),
            control_state.relay_price.value(),
            control_state.retention_price.value(),
            control_state.risk_price.value(),
            control_state.congestion_error_integral.value(),
            control_state.retention_error_integral.value(),
            control_state.relay_error_integral.value(),
            control_state.churn_error_integral.value(),
        ],
    }
}

fn should_transmit_summary(
    destination_state: &crate::state::DestinationFieldState,
    summary: &FieldSummary,
    now_tick: Tick,
) -> bool {
    let Some(previous_summary) = destination_state.publication.last_summary.as_ref() else {
        return true;
    };
    let Some(last_sent_at) = destination_state.publication.last_sent_at else {
        return true;
    };
    if now_tick.0.saturating_sub(last_sent_at.0) >= SUMMARY_HEARTBEAT_TICKS {
        return true;
    }
    summary_divergence(previous_summary, summary).value() >= 100
}

fn direct_evidence_digest(direct_evidence: &[DirectEvidence]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for evidence in direct_evidence {
        digest = mix_digest(digest, &evidence.neighbor_id.0);
        digest = mix_digest(
            digest,
            &evidence.link.profile.latency_floor_ms.0.to_le_bytes(),
        );
        digest = mix_digest(digest, &evidence.link.state.loss_permille.0.to_le_bytes());
    }
    digest
}

fn forward_evidence_digest(forward_evidence: &[crate::summary::ForwardPropagatedEvidence]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for evidence in forward_evidence {
        digest = mix_digest(digest, &evidence.from_neighbor.0);
        digest = mix_digest(digest, &evidence.summary.encode());
    }
    digest
}

fn reverse_feedback_digest(reverse_feedback: &[crate::summary::ReverseFeedbackEvidence]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for feedback in reverse_feedback {
        digest = mix_digest(digest, &feedback.from_neighbor.0);
        digest = mix_digest(digest, &feedback.delivery_feedback.value().to_le_bytes());
    }
    digest
}

fn mix_digest(mut digest: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
    }
    digest
}

fn route_health_for(
    corridor_envelope: &crate::state::CorridorBeliefEnvelope,
    now_tick: Tick,
) -> RouteHealth {
    RouteHealth {
        reachability_state: ReachabilityState::Reachable,
        stability_score: HealthScore(u32::from(corridor_envelope.delivery_support.value())),
        congestion_penalty_points: jacquard_core::PenaltyPoints(u32::from(
            corridor_envelope.congestion_penalty.value(),
        )),
        last_validated_at_tick: now_tick,
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, Environment,
        FactSourceClass, Observation, OriginAuthenticationClass, PublicationId, RatioPermille,
        RouteEpoch, RouteHandle, RouteLease, RoutePartitionClass, RouteProtectionClass,
        RouteRepairClass, RouteSelectionError, RouteServiceKind, RoutingEvidenceClass,
        RoutingObjective, SelectedRoutingParameters, Tick, TimeWindow,
    };
    use jacquard_mem_link_profile::InMemoryTransport;
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

    use super::*;
    use crate::state::{
        DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket,
        MAX_ACTIVE_DESTINATIONS,
    };

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn sample_objective(destination: NodeId) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(destination),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: jacquard_core::Limit::Bounded(jacquard_core::DurationMs(100)),
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

    fn supported_topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(4),
                nodes: BTreeMap::from([
                    (
                        node(1),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(1), ControllerId([1; 32])),
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![1],
                                    ByteCount(128),
                                ),
                                Tick(1),
                            ),
                            &crate::FIELD_ENGINE_ID,
                        )
                        .build(),
                    ),
                    (
                        node(2),
                        NodePreset::route_capable(
                            NodePresetOptions::new(
                                NodeIdentity::new(node(2), ControllerId([2; 32])),
                                jacquard_adapter::opaque_endpoint(
                                    jacquard_core::TransportKind::WifiAware,
                                    vec![2],
                                    ByteCount(128),
                                ),
                                Tick(1),
                            ),
                            &crate::FIELD_ENGINE_ID,
                        )
                        .build(),
                    ),
                ]),
                links: BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(100),
                    contention_permille: RatioPermille(100),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(4),
        }
    }

    fn seeded_engine() -> FieldEngine<(), ()> {
        let mut engine = FieldEngine::new(node(1), (), ());
        let state = engine.state.upsert_destination_interest(
            &DestinationId::Node(node(2)),
            DestinationInterestClass::Transit,
            Tick(4),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(850);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(200);
        state.posterior.predicted_observation_class = crate::state::ObservationClass::DirectOnly;
        state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
        state.corridor_belief.delivery_support = SupportBucket::new(800);
        state.corridor_belief.retention_affinity = SupportBucket::new(300);
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(900),
            downstream_support: SupportBucket::new(850),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(4),
        });
        engine.state.neighbor_endpoints.insert(
            node(2),
            jacquard_adapter::opaque_endpoint(
                jacquard_core::TransportKind::WifiAware,
                vec![2],
                ByteCount(128),
            ),
        );
        engine
    }

    fn seeded_transport_engine() -> FieldEngine<InMemoryTransport, ()> {
        let mut engine = FieldEngine::new(node(1), InMemoryTransport::default(), ());
        let state = engine.state.upsert_destination_interest(
            &DestinationId::Node(node(2)),
            DestinationInterestClass::Transit,
            Tick(4),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(850);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(200);
        state.posterior.predicted_observation_class = crate::state::ObservationClass::DirectOnly;
        state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
        state.corridor_belief.delivery_support = SupportBucket::new(800);
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(900),
            downstream_support: SupportBucket::new(850),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(4),
        });
        engine.state.neighbor_endpoints.insert(
            node(2),
            jacquard_adapter::opaque_endpoint(
                jacquard_core::TransportKind::WifiAware,
                vec![2],
                ByteCount(128),
            ),
        );
        engine
    }

    fn lease() -> RouteLease {
        RouteLease {
            owner_node_id: node(1),
            lease_epoch: RouteEpoch(4),
            valid_for: TimeWindow::new(Tick(4), Tick(10)).expect("lease window"),
        }
    }

    fn materialization_input(
        route_id: RouteId,
        admission: jacquard_core::RouteAdmission,
    ) -> RouteMaterializationInput {
        let lease = lease();
        RouteMaterializationInput {
            handle: RouteHandle {
                stamp: jacquard_core::RouteIdentityStamp {
                    route_id,
                    topology_epoch: lease.lease_epoch,
                    materialized_at_tick: lease.valid_for.start_tick(),
                    publication_id: PublicationId([7; 16]),
                },
            },
            admission,
            lease,
        }
    }

    #[test]
    fn materialize_route_installs_private_corridor_runtime_record() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let route_id = candidate.route_id;
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("admission");
        let installation = engine
            .materialize_route(materialization_input(route_id, admission))
            .expect("installation");
        assert_eq!(
            installation.last_lifecycle_event,
            RouteLifecycleEvent::Activated
        );
        let active = engine.active_routes.get(&route_id).expect("active route");
        assert_eq!(active.primary_neighbor, node(2));
        assert_eq!(
            active.destination,
            crate::state::DestinationKey::Node(node(2))
        );
    }

    #[test]
    fn materialize_route_fails_closed_for_invalid_backend_token() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let objective = sample_objective(node(2));
        let mut candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let route_id = candidate.route_id;
        let mut admission = engine
            .admit_route(&objective, &sample_profile(), candidate.clone(), &topology)
            .expect("admission");
        admission.backend_ref.backend_route_id.0 = vec![0xff, 0x00, 0xaa];
        candidate.backend_ref.backend_route_id = admission.backend_ref.backend_route_id.clone();
        let error = engine
            .materialize_route(materialization_input(route_id, admission))
            .expect_err("invalid backend must fail");
        assert!(matches!(
            error,
            RouteError::Runtime(RouteRuntimeError::Invalidated)
                | RouteError::Selection(RouteSelectionError::NoCandidate)
        ));
    }

    #[test]
    fn forward_payload_uses_active_corridor_primary_neighbor() {
        let topology = supported_topology();
        let mut engine = seeded_transport_engine();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let route_id = candidate.route_id;
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("admission");
        engine
            .materialize_route(materialization_input(route_id, admission))
            .expect("installation");
        engine
            .forward_payload_for_router(&route_id, b"payload")
            .expect("forward");
        assert_eq!(engine.transport.sent_frames.len(), 1);
        assert_eq!(engine.transport.sent_frames[0].1, b"payload".to_vec());
    }

    #[test]
    fn forward_payload_falls_back_to_alternate_when_primary_endpoint_missing() {
        let topology = supported_topology();
        let mut engine = seeded_transport_engine();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let route_id = candidate.route_id;
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("admission");
        engine
            .materialize_route(materialization_input(route_id, admission))
            .expect("installation");
        engine.state.neighbor_endpoints.remove(&node(2));
        engine.state.neighbor_endpoints.insert(
            node(3),
            jacquard_adapter::opaque_endpoint(
                jacquard_core::TransportKind::WifiAware,
                vec![3],
                ByteCount(128),
            ),
        );
        let active = engine
            .active_routes
            .get_mut(&route_id)
            .expect("active route");
        active.alternates = vec![node(3)];
        engine
            .forward_payload_for_router(&route_id, b"fallback")
            .expect("forward");
        assert_eq!(engine.transport.sent_frames.len(), 1);
        assert_eq!(engine.transport.sent_frames[0].1, b"fallback".to_vec());
        assert_eq!(
            engine
                .active_routes
                .get(&route_id)
                .expect("active")
                .primary_neighbor,
            node(3)
        );
    }

    #[test]
    fn maintenance_requires_replacement_when_frontier_primary_changes() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let route_id = candidate.route_id;
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("admission");
        let input = materialization_input(route_id, admission);
        let installation = engine
            .materialize_route(input.clone())
            .expect("installation");
        let mut materialized =
            jacquard_core::MaterializedRoute::from_installation(input, installation);
        engine.state.note_tick(Tick(5));
        let state = engine
            .state
            .destinations
            .get_mut(&crate::state::DestinationKey::from(&DestinationId::Node(
                node(2),
            )))
            .expect("destination");
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(3),
            net_value: SupportBucket::new(950),
            downstream_support: SupportBucket::new(900),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(5),
        });
        let result = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
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
    fn maintenance_enters_hold_fallback_under_retention_pressure() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        engine.state.posture.current = crate::state::RoutingPosture::RetentionBiased;
        engine.state.controller.congestion_price = crate::state::EntropyBucket::new(900);
        let objective = sample_objective(node(2));
        let candidate = engine
            .candidate_routes(&objective, &sample_profile(), &topology)
            .pop()
            .expect("candidate");
        let route_id = candidate.route_id;
        let admission = engine
            .admit_route(&objective, &sample_profile(), candidate, &topology)
            .expect("admission");
        let input = materialization_input(route_id, admission);
        let installation = engine
            .materialize_route(input.clone())
            .expect("installation");
        let mut materialized =
            jacquard_core::MaterializedRoute::from_installation(input, installation);
        if let Some(active) = engine.active_routes.get_mut(&route_id) {
            active.witness_detail.posture = crate::state::RoutingPosture::RetentionBiased;
        }
        let result = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
                RouteMaintenanceTrigger::PartitionDetected,
            )
            .expect("maintenance");
        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::HoldFallback {
                trigger: RouteMaintenanceTrigger::CapacityExceeded,
                retained_object_count: jacquard_core::HoldItemCount(1),
            }
        );
    }

    #[test]
    fn summary_transmission_is_suppressed_until_gain_or_heartbeat() {
        let mut engine = seeded_engine();
        assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(4)));
        assert!(!engine.advance_protocol_sessions(RouteEpoch(4), Tick(5)));
        assert!(engine.advance_protocol_sessions(
            RouteEpoch(4),
            Tick(4_u64.saturating_add(SUMMARY_HEARTBEAT_TICKS))
        ));
    }

    #[test]
    fn observer_refresh_is_incremental_and_sparse_under_load() {
        let topology = supported_topology();
        let mut engine = FieldEngine::new(node(1), (), ());
        for index in 0..MAX_ACTIVE_DESTINATIONS {
            let destination = DestinationId::Node(node(u8::try_from(index + 2).unwrap()));
            let state = engine.state.upsert_destination_interest(
                &destination,
                DestinationInterestClass::Transit,
                Tick(u64::try_from(index + 1).unwrap()),
            );
            state.posterior.top_corridor_mass =
                SupportBucket::new(u16::try_from(900 - index).unwrap());
            state.corridor_belief.delivery_support =
                SupportBucket::new(u16::try_from(800 - index).unwrap());
        }

        assert!(engine.refresh_destination_observers(&topology.value, Tick(10)));
        let refreshed = engine
            .state
            .destinations
            .values()
            .filter(|state| state.observer_cache.last_updated_at == Some(Tick(10)))
            .count();
        assert_eq!(refreshed, MAX_ACTIVE_DESTINATIONS);

        assert!(!engine.refresh_destination_observers(&topology.value, Tick(11)));
        assert!(engine.refresh_destination_observers(&topology.value, Tick(12)));
    }
}

impl<Transport, Effects> RouterManagedEngine for FieldEngine<Transport, Effects>
where
    Transport: jacquard_traits::TransportSenderEffects,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &RouteId,
        payload: &[u8],
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        let mut candidates = Vec::with_capacity(active.alternates.len().saturating_add(1));
        candidates.push(active.primary_neighbor);
        candidates.extend(active.alternates.iter().copied());
        for neighbor in candidates {
            let Some(endpoint) = self.state.neighbor_endpoints.get(&neighbor) else {
                continue;
            };
            self.transport.send_transport(endpoint, payload)?;
            if neighbor != active.primary_neighbor {
                active.primary_neighbor = neighbor;
            }
            return Ok(());
        }
        Err(RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(&mut self, route_id: &RouteId) -> Result<bool, RouteError> {
        Ok(self.active_routes.contains_key(route_id))
    }
}
