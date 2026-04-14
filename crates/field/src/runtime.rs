//! `RoutingEngine` and `RouterManagedEngine` implementations for `FieldEngine`.
//!
//! `materialize_route` decodes the backend token, validates the destination
//! state, and records an `ActiveFieldRoute` keyed by route ID. `engine_tick`
//! drives the per-tick control loop: seeding destinations from topology,
//! advancing the PI control plane, refreshing destination observers, stepping
//! protocol sessions, and deriving the attractor view. `maintain_route`
//! evaluates the installed route on each maintenance trigger, checking
//! corridor support, congestion price, corridor-envelope realization drift,
//! and frontier freshness to decide between hold fallback, replacement, or
//! continuation. Routes expire when delivery support falls below 250 permille
//! or the frontier has been stale for more than four ticks.
// long-file-exception: runtime owns the synchronous field control loop,
// maintenance path, protocol bridge integration, and router-facing hooks; the
// file stays together so those transitions can be audited end to end.

use jacquard_core::{
    Configuration, DestinationId, Fact, FactBasis, HealthScore, Limit, NodeId,
    PublishedRouteRecord, ReachabilityState, RouteBinding, RouteCommitment, RouteCommitmentFailure,
    RouteCommitmentId, RouteCommitmentResolution, RouteError, RouteHealth, RouteId,
    RouteInstallation, RouteInvalidationReason, RouteLifecycleEvent, RouteMaintenanceFailure,
    RouteMaintenanceOutcome, RouteMaintenanceResult, RouteMaintenanceTrigger,
    RouteMaterializationInput, RouteMaterializationProof, RouteOperationId, RouteProgressContract,
    RouteProgressState, RouteRuntimeError, RouteRuntimeState, RouteSelectionError,
    RoutingTickChange, RoutingTickContext, RoutingTickHint, RoutingTickOutcome, Tick,
    TimeoutPolicy, TransportObservation,
};
use jacquard_traits::{Blake3Hashing, Hashing, RouterManagedEngine, RoutingEngine};

use crate::{
    attractor::{derive_local_attractor_view, rank_frontier_by_attractor},
    choreography::{
        FieldChoreographyAdvance, FieldHostWaitStatus, FieldProtocolCheckpoint, FieldProtocolKind,
        FieldProtocolReconfigurationCause, FieldProtocolSessionKey, QueuedProtocolSend,
        FIELD_PROTOCOL_SESSION_MAX,
    },
    control::{advance_control_plane, ControlMeasurements},
    engine::FieldRuntimeRoundArtifact,
    observer::{update_destination_observer, ObserverInputs},
    planner::{promotion_assessment_for_route, FieldBootstrapDecision},
    recovery::{FieldPromotionBlocker, FieldRouteRecoveryTrigger, StoredFieldRouteRecovery},
    route::{decode_backend_token, ActiveFieldRoute, FieldBootstrapClass},
    state::{
        HopBand, NeighborContinuation, ObserverInputSignature, SupportBucket,
        SUMMARY_HEARTBEAT_TICKS,
    },
    summary::{
        summary_divergence, DirectEvidence, EvidenceContributionClass, FieldSummary,
        LocalOriginTrace, SummaryDestinationKey, SummaryUncertaintyClass,
        FIELD_SUMMARY_ENCODING_BYTES,
    },
    FieldEngine,
};

const FIELD_COMMITMENT_ATTEMPT_COUNT_MAX: u32 = 2;
const FIELD_COMMITMENT_INITIAL_BACKOFF_MS: u32 = 25;
const FIELD_COMMITMENT_BACKOFF_MS_MAX: u32 = 25;
const FIELD_COMMITMENT_OVERALL_TIMEOUT_MS: u32 = 50;
const FIELD_COMMITMENT_ID_DOMAIN: &[u8] = b"field-route-commitment";
pub(crate) const FIELD_ROUTE_FAILURE_SUPPORT_FLOOR: u16 = 180;
pub(crate) const FIELD_ROUTE_WEAK_SUPPORT_FLOOR: u16 = 220;
const FIELD_BOOTSTRAP_FAILURE_SUPPORT_FLOOR: u16 = 140;
const FIELD_BOOTSTRAP_STALE_TICKS_MAX: u64 = 6;
const FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX: u16 = 180;

impl<Transport, Effects> RoutingEngine for FieldEngine<Transport, Effects>
where
    Transport: jacquard_traits::TransportSenderEffects,
{
    // long-block-exception: route materialization intentionally keeps backend
    // decoding, witness validation, active-route installation, and
    // route-scoped protocol session startup in one fail-closed path.
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
        let bootstrap_class = detail.bootstrap_class;
        let mut recovery = StoredFieldRouteRecovery::default();
        if bootstrap_class == FieldBootstrapClass::Bootstrap {
            recovery.note_bootstrap_activated();
        }

        self.active_routes.insert(
            *input.handle.route_id(),
            ActiveFieldRoute {
                destination: token.destination,
                selected_neighbor: token.selected_neighbor,
                continuation_neighbors: token.continuation_neighbors,
                corridor_envelope: corridor_envelope.clone(),
                witness_detail: detail,
                bootstrap_class,
                backend_route_id: input.admission.backend_ref.backend_route_id.clone(),
                topology_epoch: input.handle.topology_epoch(),
                installed_at_tick: input.handle.materialized_at_tick(),
                bootstrap_confirmation_streak: 0,
                coordination_capability: None,
                recovery,
            },
        );
        self.install_route_protocol_session(
            input.handle.route_id(),
            input.handle.topology_epoch(),
            input.handle.materialized_at_tick(),
        )?;

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

    fn route_commitments(&self, route: &jacquard_core::MaterializedRoute) -> Vec<RouteCommitment> {
        let resolution = if !route
            .identity
            .lease
            .is_valid_at(self.state.last_tick_processed)
        {
            RouteCommitmentResolution::Invalidated(RouteInvalidationReason::LeaseExpired)
        } else if let Some(active) = self.active_routes.get(route.identity.route_id()) {
            let current_support = self
                .state
                .destinations
                .get(&active.destination)
                .map(|state| state.corridor_belief.delivery_support.value())
                .unwrap_or(active.corridor_envelope.delivery_support.value());
            if active.topology_epoch != route.identity.topology_epoch() {
                RouteCommitmentResolution::Invalidated(RouteInvalidationReason::TopologySuperseded)
            } else if current_support < 250 {
                RouteCommitmentResolution::Invalidated(RouteInvalidationReason::EvidenceWithdrawn)
            } else {
                RouteCommitmentResolution::Pending
            }
        } else {
            RouteCommitmentResolution::Failed(RouteCommitmentFailure::BackendUnavailable)
        };

        vec![RouteCommitment {
            commitment_id: field_commitment_id_for_route(route.identity.route_id()),
            operation_id: RouteOperationId(route.identity.route_id().0),
            route_binding: RouteBinding::Bound(*route.identity.route_id()),
            owner_node_id: route.identity.lease.owner_node_id,
            deadline_tick: route.identity.lease.valid_for.end_tick(),
            retry_policy: TimeoutPolicy {
                attempt_count_max: FIELD_COMMITMENT_ATTEMPT_COUNT_MAX,
                initial_backoff_ms: jacquard_core::DurationMs(FIELD_COMMITMENT_INITIAL_BACKOFF_MS),
                retry_multiplier_permille: jacquard_core::RatioPermille(1000),
                backoff_ms_max: jacquard_core::DurationMs(FIELD_COMMITMENT_BACKOFF_MS_MAX),
                overall_timeout_ms: jacquard_core::DurationMs(FIELD_COMMITMENT_OVERALL_TIMEOUT_MS),
            },
            resolution,
        }]
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
    // over posture, support, freshness, continuation envelopes, and hold
    // fallback.
    fn maintain_route(
        &mut self,
        identity: &PublishedRouteRecord,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, RouteError> {
        let Some(destination_key) = self
            .active_routes
            .get(identity.route_id())
            .map(|active| active.destination.clone())
        else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let Some(destination_state) = self.state.destinations.get(&destination_key).cloned() else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        let ranked = rank_frontier_by_attractor(
            &destination_state,
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
        let current_corridor_envelope = destination_state.corridor_belief.clone();
        let current_witness_detail = self.witness_detail_from_state(&destination_state);
        let current_bootstrap_class = current_witness_detail.bootstrap_class;

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

        let corridor_support = destination_state.corridor_belief.delivery_support.value();
        let mut pending_coordination_shift = None;
        let mut maintenance_outcome = RouteMaintenanceOutcome::Continued;
        let previous_posture;
        let previous_bootstrap_class;
        let promotion_assessment;
        let bootstrap_decision;
        {
            let active = self
                .active_routes
                .get_mut(identity.route_id())
                .expect("active route remains present during maintenance");
            previous_posture = active.witness_detail.posture;
            previous_bootstrap_class = active.bootstrap_class;
            promotion_assessment = promotion_assessment_for_route(
                active,
                &destination_state,
                best,
                self.state.last_tick_processed,
            );
            let blocker = promotion_assessment.primary_blocker();
            let projected_confirmation_streak = if previous_bootstrap_class
                == FieldBootstrapClass::Bootstrap
                && promotion_assessment.anti_entropy_confirmed
                && promotion_assessment.continuation_coherent
            {
                active.bootstrap_confirmation_streak.saturating_add(1)
            } else {
                0
            };
            bootstrap_decision = if previous_bootstrap_class == FieldBootstrapClass::Bootstrap {
                promotion_assessment
                    .decision_for_bootstrap(&destination_state, projected_confirmation_streak)
            } else {
                FieldBootstrapDecision::Hold
            };
            active.corridor_envelope = current_corridor_envelope;
            active.witness_detail = current_witness_detail;
            active.bootstrap_class = match (previous_bootstrap_class, bootstrap_decision) {
                (FieldBootstrapClass::Bootstrap, FieldBootstrapDecision::Promote) => {
                    FieldBootstrapClass::Steady
                }
                (FieldBootstrapClass::Bootstrap, _) => FieldBootstrapClass::Bootstrap,
                (_, _) => current_bootstrap_class,
            };
            active.witness_detail.bootstrap_class = active.bootstrap_class;

            match (
                previous_bootstrap_class,
                active.bootstrap_class,
                bootstrap_decision,
            ) {
                (FieldBootstrapClass::Steady, FieldBootstrapClass::Bootstrap, _) => {
                    active.recovery.note_bootstrap_activated();
                }
                (
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapDecision::Narrow,
                ) => {
                    active.recovery.note_bootstrap_narrowed(blocker);
                    active.bootstrap_confirmation_streak = 0;
                }
                (
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapDecision::Hold,
                ) => {
                    active.recovery.note_bootstrap_held(blocker);
                    if promotion_assessment.anti_entropy_confirmed
                        && promotion_assessment.continuation_coherent
                    {
                        active.bootstrap_confirmation_streak =
                            active.bootstrap_confirmation_streak.saturating_add(1);
                    } else {
                        active.bootstrap_confirmation_streak = 0;
                    }
                }
                (
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapClass::Steady,
                    FieldBootstrapDecision::Promote,
                ) => {
                    active.recovery.note_bootstrap_upgraded();
                    active.bootstrap_confirmation_streak = 0;
                }
                (
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapDecision::Withdraw,
                ) => {
                    active.recovery.note_bootstrap_withdrawn(blocker);
                    active.bootstrap_confirmation_streak = 0;
                }
                (FieldBootstrapClass::Steady, FieldBootstrapClass::Steady, _) => {}
                (
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                    FieldBootstrapDecision::Promote,
                ) => {
                    active
                        .recovery
                        .note_bootstrap_held(FieldPromotionBlocker::SupportTrend);
                }
                (FieldBootstrapClass::Bootstrap, FieldBootstrapClass::Steady, _) => {}
            }

            if !active.continuation_neighbors.contains(&best.neighbor_id) {
                let selected_entry = ranked
                    .iter()
                    .find(|(entry, _)| entry.neighbor_id == active.selected_neighbor)
                    .map(|(entry, _)| entry);
                let support_delta = selected_entry
                    .map(|entry| best.net_value.value().abs_diff(entry.net_value.value()))
                    .unwrap_or(0);
                if corridor_support >= FIELD_ROUTE_WEAK_SUPPORT_FLOOR
                    && support_delta <= FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX
                {
                    active.continuation_neighbors = ranked
                        .iter()
                        .take(crate::state::MAX_CONTINUATION_NEIGHBOR_COUNT + 1)
                        .map(|(entry, _)| entry.neighbor_id)
                        .collect();
                    active.selected_neighbor = best.neighbor_id;
                    pending_coordination_shift = Some(best.neighbor_id);
                } else {
                    if previous_bootstrap_class == FieldBootstrapClass::Bootstrap
                        && promotion_assessment.degraded_but_coherent(&destination_state)
                    {
                        active.continuation_neighbors = ranked
                            .iter()
                            .take(2)
                            .map(|(entry, _)| entry.neighbor_id)
                            .collect();
                        if let Some(first) = active.continuation_neighbors.first().copied() {
                            active.selected_neighbor = first;
                        }
                        active.recovery.note_corridor_narrowed();
                        active.recovery.note_bootstrap_narrowed(blocker);
                        maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                            trigger,
                            retained_object_count: jacquard_core::HoldItemCount(1),
                        };
                    } else {
                        return Ok(RouteMaintenanceResult {
                            event: RouteLifecycleEvent::Replaced,
                            outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
                        });
                    }
                }
            }

            if active.selected_neighbor != best.neighbor_id {
                active.selected_neighbor = best.neighbor_id;
                pending_coordination_shift = Some(best.neighbor_id);
            }
        }

        let failure_support_floor = if previous_bootstrap_class == FieldBootstrapClass::Bootstrap
            && promotion_assessment.degraded_but_coherent(&destination_state)
        {
            FIELD_BOOTSTRAP_FAILURE_SUPPORT_FLOOR
        } else {
            FIELD_ROUTE_FAILURE_SUPPORT_FLOOR
        };

        if corridor_support < failure_support_floor {
            if previous_bootstrap_class == FieldBootstrapClass::Bootstrap
                && promotion_assessment.degraded_but_coherent(&destination_state)
            {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    active
                        .recovery
                        .note_bootstrap_narrowed(FieldPromotionBlocker::SupportTrend);
                }
                return Ok(RouteMaintenanceResult {
                    event: RouteLifecycleEvent::Activated,
                    outcome: RouteMaintenanceOutcome::HoldFallback {
                        trigger,
                        retained_object_count: jacquard_core::HoldItemCount(1),
                    },
                });
            }
            if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                if active.bootstrap_class == FieldBootstrapClass::Bootstrap {
                    if active.recovery.state.last_bootstrap_transition
                        != Some(crate::FieldBootstrapTransition::Withdrawn)
                    {
                        active
                            .recovery
                            .note_bootstrap_withdrawn(FieldPromotionBlocker::SupportTrend);
                    }
                }
            }
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded),
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

        if self.state.posture.current != previous_posture
            && self.state.posture.current == crate::state::RoutingPosture::RiskSuppressed
        {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::PolicyShift,
                },
            });
        }

        if let Some(new_neighbor) = pending_coordination_shift {
            self.reconfigure_route_protocol_session(
                identity.route_id(),
                new_neighbor,
                FieldProtocolReconfigurationCause::ContinuationShift,
                self.state.last_tick_processed,
            )?;
        }

        let is_stale = self
            .state
            .last_tick_processed
            .0
            .saturating_sub(best.freshness.0)
            > if previous_bootstrap_class == FieldBootstrapClass::Bootstrap
                && promotion_assessment.degraded_but_coherent(&destination_state)
            {
                FIELD_BOOTSTRAP_STALE_TICKS_MAX
            } else {
                4
            };
        if is_stale {
            if corridor_support < FIELD_ROUTE_WEAK_SUPPORT_FLOOR {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    if active.bootstrap_class == FieldBootstrapClass::Bootstrap {
                        if promotion_assessment.degraded_but_coherent(&destination_state) {
                            active
                                .recovery
                                .note_bootstrap_narrowed(FieldPromotionBlocker::Freshness);
                            active.recovery.note_corridor_narrowed();
                        } else {
                            if active.recovery.state.last_bootstrap_transition
                                != Some(crate::FieldBootstrapTransition::Withdrawn)
                            {
                                active
                                    .recovery
                                    .note_bootstrap_withdrawn(FieldPromotionBlocker::Freshness);
                            }
                        }
                    }
                }
                if promotion_assessment.degraded_but_coherent(&destination_state) {
                    maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                        trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                        retained_object_count: jacquard_core::HoldItemCount(1),
                    };
                } else {
                    return Ok(RouteMaintenanceResult {
                        event: RouteLifecycleEvent::Activated,
                        outcome: RouteMaintenanceOutcome::ReplacementRequired {
                            trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                        },
                    });
                }
            }
            if maintenance_outcome == RouteMaintenanceOutcome::Continued {
                maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                    trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                    retained_object_count: jacquard_core::HoldItemCount(1),
                };
            }
        }

        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: if corridor_support < FIELD_ROUTE_WEAK_SUPPORT_FLOOR {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    if active.bootstrap_class == FieldBootstrapClass::Bootstrap {
                        active
                            .recovery
                            .note_bootstrap_held(promotion_assessment.primary_blocker());
                    }
                }
                RouteMaintenanceOutcome::HoldFallback {
                    trigger,
                    retained_object_count: jacquard_core::HoldItemCount(1),
                }
            } else {
                maintenance_outcome
            },
        })
    }

    fn teardown(&mut self, route_id: &RouteId) {
        // allow-ignored-result: route teardown must continue even if the private coordination session has already been invalidated or removed.
        let _ = self.close_route_protocol_session(route_id);
        self.active_routes.remove(route_id);
    }
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    fn protocol_neighbor_targets(&self, preferred_neighbor: NodeId) -> Vec<NodeId> {
        let mut neighbors = self
            .state
            .neighbor_endpoints
            .keys()
            .copied()
            .collect::<Vec<_>>();
        if neighbors.is_empty() {
            neighbors.push(preferred_neighbor);
            return neighbors;
        }
        neighbors.sort();
        if let Some(index) = neighbors
            .iter()
            .position(|neighbor| *neighbor == preferred_neighbor)
        {
            neighbors.swap(0, index);
        }
        neighbors
    }

    fn dispatch_protocol_sends(&mut self, sends: &[QueuedProtocolSend]) -> Result<usize, RouteError>
    where
        Transport: jacquard_traits::TransportSenderEffects,
    {
        let mut sent = 0usize;
        for send in sends {
            let Some(endpoint) = self.state.neighbor_endpoints.get(&send.to_neighbor) else {
                continue;
            };
            self.transport.send_transport(endpoint, &send.payload)?;
            sent = sent.saturating_add(1);
        }
        Ok(sent)
    }

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
    ) -> bool
    where
        Transport: jacquard_traits::TransportSenderEffects,
    {
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
            let Some((
                session_destination,
                summary,
                anti_entropy_summary,
                should_publish,
                leading_neighbor,
                delivery_support,
                is_stale,
                runtime_route_artifact,
            )) = self
                .state
                .destinations
                .get(&destination_key)
                .and_then(|destination_state| {
                    let primary = destination_state.frontier.as_slice().first()?;
                    let destination = DestinationId::from(&destination_state.destination);
                    let summary = summary_for_destination(
                        destination_state,
                        topology_epoch,
                        now_tick,
                        &destination,
                    );
                    let anti_entropy_summary =
                        anti_entropy_summary_for_destination(destination_state, &summary, now_tick);
                    Some((
                        SummaryDestinationKey::from(&destination),
                        summary.clone(),
                        anti_entropy_summary,
                        should_transmit_summary(destination_state, &summary, now_tick),
                        primary.neighbor_id,
                        destination_state.corridor_belief.delivery_support.value(),
                        now_tick.0.saturating_sub(primary.freshness.0) > 4,
                        self.runtime_route_artifact_for_destination(
                            &destination,
                            destination_state,
                            topology_epoch,
                        ),
                    ))
                })
            else {
                continue;
            };

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
                    let _summary_queue_failed = self
                        .protocol_runtime
                        .queue_summary_flow(
                            &capability,
                            self.protocol_neighbor_targets(leading_neighbor)
                                .into_iter()
                                .map(|neighbor| QueuedProtocolSend {
                                    protocol: FieldProtocolKind::SummaryDissemination,
                                    to_neighbor: neighbor,
                                    payload: summary.encode(),
                                }),
                        )
                        .is_err();
                    let published = self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .map(|advance| {
                            let dispatched = self
                                .dispatch_protocol_sends(&advance.flushed_sends)
                                .unwrap_or(0);
                            self.record_protocol_round(
                                &dissemination_key,
                                &advance,
                                Some(runtime_route_artifact.clone()),
                                now_tick,
                            );
                            dispatched > 0
                        })
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
                    let _summary_queue_failed = self
                        .protocol_runtime
                        .queue_summary_flow(
                            &capability,
                            self.protocol_neighbor_targets(leading_neighbor)
                                .into_iter()
                                .map(|neighbor| QueuedProtocolSend {
                                    protocol: FieldProtocolKind::AntiEntropy,
                                    to_neighbor: neighbor,
                                    payload: anti_entropy_summary.encode(),
                                }),
                        )
                        .is_err();
                    changed |= self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .map(|advance| {
                            let _ = self.dispatch_protocol_sends(&advance.flushed_sends);
                            self.record_protocol_round(
                                &anti_entropy_key,
                                &advance,
                                Some(runtime_route_artifact.clone()),
                                now_tick,
                            );
                            true
                        })
                        .unwrap_or(false);
                }
            }

            let should_replay_retention = (self.state.posture.current
                == crate::state::RoutingPosture::RetentionBiased
                && (delivery_support < 450 || is_stale))
                || (delivery_support < 520 && summary.uncertainty_penalty.value() >= 350);
            if should_replay_retention {
                let replay_key = FieldProtocolSessionKey {
                    protocol: FieldProtocolKind::RetentionReplay,
                    route_id: None,
                    topology_epoch,
                    destination: Some(session_destination),
                };
                if let Ok(capability) = self.protocol_runtime.open_session(&replay_key, 0, None) {
                    let _branch_choice_failed = self
                        .protocol_runtime
                        .queue_branch_choice(&capability, 1)
                        .is_err();
                    let _summary_queue_failed = self
                        .protocol_runtime
                        .queue_summary_flow(
                            &capability,
                            self.protocol_neighbor_targets(leading_neighbor)
                                .into_iter()
                                .map(|neighbor| QueuedProtocolSend {
                                    protocol: FieldProtocolKind::RetentionReplay,
                                    to_neighbor: neighbor,
                                    payload: anti_entropy_summary.encode(),
                                }),
                        )
                        .is_err();
                    changed |= self
                        .protocol_runtime
                        .advance_host_bridged_round(
                            &capability,
                            None,
                            FieldHostWaitStatus::Idle,
                            now_tick,
                        )
                        .map(|advance| {
                            let dispatched = self
                                .dispatch_protocol_sends(&advance.flushed_sends)
                                .unwrap_or(0);
                            self.record_protocol_round(
                                &replay_key,
                                &advance,
                                Some(runtime_route_artifact.clone()),
                                now_tick,
                            );
                            dispatched > 0
                        })
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
                        .map(|advance| {
                            let _ = self.dispatch_protocol_sends(&advance.flushed_sends);
                            self.record_protocol_round(
                                &coordination_key,
                                &advance,
                                Some(runtime_route_artifact.clone()),
                                now_tick,
                            );
                            true
                        })
                        .unwrap_or(false);
                }
            }
        }

        changed
    }

    fn record_protocol_round(
        &self,
        session_key: &FieldProtocolSessionKey,
        advance: &FieldChoreographyAdvance,
        router_artifact: Option<crate::engine::FieldRuntimeRouteArtifact>,
        now_tick: Tick,
    ) {
        let search_snapshot_epoch = self
            .search_snapshot_state
            .borrow()
            .as_ref()
            .map(|state| state.epoch.clone());
        let last_search_record = self.last_search_record.borrow();
        self.record_runtime_round_artifact(FieldRuntimeRoundArtifact {
            protocol: session_key.protocol(),
            destination: session_key.destination(),
            destination_class: session_key
                .destination()
                .as_ref()
                .map(destination_objective_class),
            blocked_receive: advance.round.blocked_receive,
            disposition: advance.round.disposition,
            host_wait_status: advance.round.host_wait_status,
            emitted_count: advance.round.emitted_send_count,
            step_budget_remaining: advance.round.step_budget_remaining,
            execution_policy: advance.round.execution_policy,
            search_snapshot_epoch,
            search_selected_result_present: last_search_record
                .as_ref()
                .is_some_and(|record| record.selected_continuation.is_some()),
            search_reconfiguration_present: last_search_record.as_ref().is_some_and(|record| {
                record
                    .run
                    .as_ref()
                    .is_some_and(|run| run.reconfiguration.is_some())
            }),
            router_artifact,
            observed_at_tick: now_tick,
        });
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
            let forward_evidence = destination_state.pending_forward_evidence.clone();
            let reverse_feedback = destination_state.pending_reverse_feedback.clone();
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
            let forward_evidence = std::mem::take(&mut destination_state.pending_forward_evidence);
            let reverse_feedback = std::mem::take(&mut destination_state.pending_reverse_feedback);
            let had_state = (
                destination_state.posterior.clone(),
                destination_state.progress_belief.clone(),
                destination_state.corridor_belief.clone(),
                destination_state.frontier.clone(),
            );
            update_destination_observer(
                destination_state,
                &ObserverInputs {
                    destination,
                    topology_epoch,
                    now_tick,
                    direct_evidence: direct_evidence.clone(),
                    forward_evidence: forward_evidence.clone(),
                    reverse_feedback: reverse_feedback.clone(),
                    local_origin_trace,
                    regime,
                    control_state: control_state.clone(),
                },
            );
            destination_state.frontier = refresh_frontier_from_evidence(
                destination_state.frontier.clone(),
                destination_state.corridor_belief.expected_hop_band,
                destination_state.corridor_belief.delivery_support,
                destination_state.corridor_belief.retention_affinity,
                &direct_evidence,
                &forward_evidence,
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
                destination_state.frontier.clone(),
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

fn anti_entropy_summary_for_destination(
    destination_state: &crate::state::DestinationFieldState,
    summary: &FieldSummary,
    now_tick: Tick,
) -> FieldSummary {
    let publication_support = destination_state
        .publication
        .last_summary
        .as_ref()
        .map(|published| published.delivery_support.value())
        .unwrap_or(0);
    let publication_retention = destination_state
        .publication
        .last_summary
        .as_ref()
        .map(|published| published.retention_support.value())
        .unwrap_or(0);
    let bootstrap_delivery_floor = destination_state
        .progress_belief
        .posterior_support
        .value()
        .min(destination_state.posterior.top_corridor_mass.value())
        .max(
            destination_state
                .progress_belief
                .posterior_support
                .value()
                .saturating_add(destination_state.posterior.top_corridor_mass.value())
                / 2,
        );
    let replay_support = summary
        .delivery_support
        .value()
        .max(bootstrap_delivery_floor)
        .max(publication_support.saturating_sub(20))
        .max(
            publication_support
                .saturating_add(publication_retention / 6)
                .min(1000),
        )
        .min(1000);
    let replay_retention = summary
        .retention_support
        .value()
        .max(publication_retention)
        .max((replay_support.saturating_mul(4)) / 5)
        .max(
            destination_state
                .posterior
                .top_corridor_mass
                .value()
                .saturating_add(replay_support)
                / 2,
        )
        .min(1000);
    let replay_uncertainty = summary
        .uncertainty_penalty
        .value()
        .saturating_sub(if replay_retention >= 700 {
            190
        } else if replay_retention >= 560 {
            150
        } else {
            100
        })
        .saturating_sub(if publication_retention >= 320 { 40 } else { 0 });
    FieldSummary {
        freshness_tick: now_tick,
        delivery_support: SupportBucket::new(replay_support),
        retention_support: SupportBucket::new(replay_retention),
        uncertainty_penalty: crate::state::EntropyBucket::new(replay_uncertainty),
        uncertainty_class: match replay_uncertainty {
            0..=249 => SummaryUncertaintyClass::Low,
            250..=599 => SummaryUncertaintyClass::Medium,
            _ => SummaryUncertaintyClass::High,
        },
        ..summary.clone()
    }
}

fn refresh_frontier_from_evidence(
    mut frontier: crate::state::ContinuationFrontier,
    corridor_hops: HopBand,
    corridor_support: SupportBucket,
    corridor_retention: SupportBucket,
    direct_evidence: &[DirectEvidence],
    forward_evidence: &[crate::summary::ForwardPropagatedEvidence],
    now_tick: Tick,
) -> crate::state::ContinuationFrontier {
    let prune_horizon = if corridor_retention.value() >= 300 && corridor_support.value() >= 220 {
        8
    } else if corridor_retention.value() >= 240 {
        6
    } else {
        4
    };
    frontier = frontier.prune_stale(now_tick, prune_horizon);
    for evidence in direct_evidence {
        frontier = frontier.insert(NeighborContinuation {
            neighbor_id: evidence.neighbor_id,
            net_value: corridor_support,
            downstream_support: corridor_support,
            expected_hop_band: HopBand::new(1, corridor_hops.max_hops.max(1)),
            freshness: now_tick,
        });
    }
    for evidence in forward_evidence {
        frontier = frontier.insert(NeighborContinuation {
            neighbor_id: evidence.from_neighbor,
            net_value: SupportBucket::new(
                corridor_support
                    .value()
                    .max(evidence.summary.delivery_support.value()),
            ),
            downstream_support: evidence.summary.delivery_support,
            expected_hop_band: HopBand::new(
                evidence.summary.hop_band.min_hops.saturating_add(1),
                evidence.summary.hop_band.max_hops.saturating_add(1),
            ),
            freshness: evidence.observed_at_tick,
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
    if summary_divergence(previous_summary, summary).value() >= 100 {
        return true;
    }
    destination_state.corridor_belief.delivery_support.value() < 320
        && destination_state.corridor_belief.retention_affinity.value() >= 260
        && now_tick.0.saturating_sub(last_sent_at.0) >= SUMMARY_HEARTBEAT_TICKS.saturating_sub(1)
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

fn field_commitment_id_for_route(route_id: &RouteId) -> RouteCommitmentId {
    let digest = Blake3Hashing.hash_tagged(FIELD_COMMITMENT_ID_DOMAIN, &route_id.0);
    RouteCommitmentId::from(&digest)
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, Environment,
        FactSourceClass, LinkEndpoint, Observation, OriginAuthenticationClass, PublicationId,
        RatioPermille, RouteEpoch, RouteHandle, RouteLease, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RouteSelectionError, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick, TimeWindow,
        TransportError,
    };
    use jacquard_mem_link_profile::InMemoryTransport;
    use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
    use jacquard_traits::{
        effect_handler, RouterManagedEngine, RoutingEngine, RoutingEnginePlanner,
        TransportSenderEffects,
    };

    use super::*;
    use crate::state::{
        DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket,
        MAX_ACTIVE_DESTINATIONS,
    };
    use crate::summary::{
        EvidenceContributionClass, FieldSummary, SummaryDestinationKey, SummaryUncertaintyClass,
    };

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct NoopTransport;

    #[effect_handler]
    impl TransportSenderEffects for NoopTransport {
        fn send_transport(
            &mut self,
            _endpoint: &LinkEndpoint,
            _payload: &[u8],
        ) -> Result<(), TransportError> {
            Ok(())
        }
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

    fn seeded_engine() -> FieldEngine<NoopTransport, ()> {
        let mut engine = FieldEngine::new(node(1), NoopTransport, ());
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
        assert_eq!(active.selected_neighbor, node(2));
        assert_eq!(active.continuation_neighbors, vec![node(2)]);
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
    fn forward_payload_uses_selected_corridor_realization() {
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
    fn forward_payload_switches_realization_inside_continuation_envelope() {
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
        active.continuation_neighbors = vec![node(2), node(3)];
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
                .selected_neighbor,
            node(3)
        );
    }

    #[test]
    fn maintenance_requires_replacement_when_best_neighbor_leaves_corridor_envelope() {
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
        state.corridor_belief.delivery_support = SupportBucket::new(210);
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
    fn maintenance_expands_corridor_envelope_for_close_stronger_neighbor() {
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
        assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
        let active = engine.active_routes.get(&route_id).expect("active route");
        assert_eq!(active.selected_neighbor, node(3));
        assert!(active.continuation_neighbors.contains(&node(3)));
        assert!(active.continuation_neighbors.contains(&node(2)));
    }

    #[test]
    fn bootstrap_route_materialization_records_activation() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(240);
        state.corridor_belief.retention_affinity = SupportBucket::new(320);

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

        let active = engine.active_routes.get(&route_id).expect("active route");
        assert_eq!(active.bootstrap_class, FieldBootstrapClass::Bootstrap);
        assert!(active.recovery.state.bootstrap_active);
        assert_eq!(
            active.recovery.state.last_bootstrap_transition,
            Some(crate::FieldBootstrapTransition::Activated)
        );
        assert_eq!(active.recovery.state.bootstrap_activation_count, 1);
    }

    #[test]
    fn bootstrap_route_upgrades_to_steady_without_replacement() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
        {
            let state = engine
                .state
                .destinations
                .get_mut(&destination)
                .expect("destination");
            state.corridor_belief.delivery_support = SupportBucket::new(240);
            state.corridor_belief.retention_affinity = SupportBucket::new(320);
        }

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
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(360);

        let result = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");

        assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
        let active = engine.active_routes.get(&route_id).expect("active route");
        assert_eq!(active.bootstrap_class, FieldBootstrapClass::Steady);
        assert!(!active.recovery.state.bootstrap_active);
        assert_eq!(
            active.recovery.state.last_bootstrap_transition,
            Some(crate::FieldBootstrapTransition::Upgraded)
        );
        assert_eq!(active.recovery.state.bootstrap_upgrade_count, 1);
    }

    #[test]
    fn bootstrap_route_promotes_after_confirmed_bridge_streak() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
        {
            let state = engine
                .state
                .destinations
                .get_mut(&destination)
                .expect("destination");
            state.corridor_belief.delivery_support = SupportBucket::new(240);
            state.corridor_belief.retention_affinity = SupportBucket::new(340);
            state.posterior.top_corridor_mass = SupportBucket::new(320);
            state.posterior.usability_entropy = crate::state::EntropyBucket::new(900);
        }

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

        {
            let active = engine.active_routes.get_mut(&route_id).expect("active");
            active.bootstrap_confirmation_streak = 1;
        }
        engine.state.note_tick(Tick(5));
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.posterior.top_corridor_mass = SupportBucket::new(340);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(835);
        state.corridor_belief.delivery_support = SupportBucket::new(275);
        state.corridor_belief.retention_affinity = SupportBucket::new(360);
        state.publication.last_summary = Some(super::summary_for_destination(
            state,
            topology.value.epoch,
            Tick(4),
            &objective.destination,
        ));
        state.publication.last_sent_at = Some(Tick(4));

        let result = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
                RouteMaintenanceTrigger::AntiEntropyRequired,
            )
            .expect("maintenance");

        assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
        let active = engine.active_routes.get(&route_id).expect("active route");
        assert_eq!(active.bootstrap_class, FieldBootstrapClass::Steady);
        assert_eq!(
            active.recovery.state.last_bootstrap_transition,
            Some(crate::FieldBootstrapTransition::Upgraded)
        );
        assert_eq!(active.recovery.state.bootstrap_upgrade_count, 1);
    }

    #[test]
    fn bootstrap_route_withdrawal_is_recorded_before_failure() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
        {
            let state = engine
                .state
                .destinations
                .get_mut(&destination)
                .expect("destination");
            state.corridor_belief.delivery_support = SupportBucket::new(240);
            state.corridor_belief.retention_affinity = SupportBucket::new(320);
        }

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
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(120);

        let result = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");

        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded)
        );
        let active = engine.active_routes.get(&route_id).expect("active route");
        assert!(!active.recovery.state.bootstrap_active);
        assert_eq!(
            active.recovery.state.last_bootstrap_transition,
            Some(crate::FieldBootstrapTransition::Withdrawn)
        );
        assert_eq!(active.recovery.state.bootstrap_withdraw_count, 1);
    }

    // long-block-exception: this test keeps materialization, frontier shift,
    // maintenance, and reduced protocol replay assertions together so the
    // continuation-shift reconfiguration path stays readable as one scenario.
    #[test]
    fn maintenance_switches_realization_inside_corridor_envelope() {
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
        let active = engine
            .active_routes
            .get_mut(&route_id)
            .expect("active route");
        active.continuation_neighbors = vec![node(2), node(3)];
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
        assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
        assert_eq!(
            engine
                .active_routes
                .get(&route_id)
                .expect("active route")
                .selected_neighbor,
            node(3)
        );
        let recovery = &engine
            .active_routes
            .get(&route_id)
            .expect("active route")
            .recovery
            .state;
        assert_eq!(recovery.continuation_shift_count, 1);
        assert_eq!(
            recovery.last_outcome,
            Some(crate::FieldRouteRecoveryOutcome::ContinuationRetained)
        );
        let protocol_replay = engine
            .replay_snapshot(std::slice::from_ref(&materialized))
            .reduced_protocol_replay();
        assert!(protocol_replay
            .reconfigurations
            .iter()
            .any(|reconfiguration| {
                reconfiguration.prior_session.route_id == Some(route_id)
                    && reconfiguration.next_session.route_id == Some(route_id)
                    && reconfiguration.cause
                        == crate::choreography::FieldProtocolReconfigurationCause::ContinuationShift
                    && reconfiguration.prior_owner_tag != reconfiguration.next_owner_tag
            }));
    }

    #[test]
    fn exported_replay_bundle_captures_continuation_shift_fixture() {
        use std::path::PathBuf;

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
        let active = engine
            .active_routes
            .get_mut(&route_id)
            .expect("active route");
        active.continuation_neighbors = vec![node(2), node(3)];
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
        // allow-ignored-result: this fixture needs only the route-local shift side effects before exporting the replay bundle.
        let _ = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");

        let actual = engine
            .replay_snapshot(std::slice::from_ref(&materialized))
            .exported_bundle_json()
            .expect("export replay json");
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("../fixtures/replay/continuation-shift.json");
        if std::env::var_os("JACQUARD_UPDATE_FIELD_REPLAY_FIXTURES").is_some() {
            std::fs::write(&fixture_path, format!("{actual}\n"))
                .expect("write continuation shift replay fixture");
        }
        let expected =
            std::fs::read_to_string(&fixture_path).expect("read continuation shift replay fixture");
        assert_eq!(actual, expected.trim_end());
    }

    #[test]
    fn suspend_route_runtime_captures_checkpoint_and_marks_recovery_surface() {
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
        let materialized = jacquard_core::MaterializedRoute::from_installation(input, installation);

        assert!(engine
            .suspend_route_runtime_for_recovery(&route_id)
            .expect("suspend"));

        let active = engine.active_routes.get(&route_id).expect("active route");
        assert!(active.coordination_capability.is_none());
        assert!(active.recovery.checkpoint.is_some());

        let replay = engine.replay_snapshot(std::slice::from_ref(&materialized));
        let entry = replay
            .recovery
            .entries
            .into_iter()
            .find(|entry| entry.route_id == route_id)
            .expect("recovery entry");
        assert!(entry.state.checkpoint_available);
        assert_eq!(entry.state.checkpoint_capture_count, 1);
        assert_eq!(
            entry.state.last_outcome,
            Some(crate::FieldRouteRecoveryOutcome::CheckpointStored)
        );
    }

    #[test]
    fn restore_route_runtime_prefers_checkpoint_restore() {
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
        let input = materialization_input(route_id, admission);
        let installation = engine
            .materialize_route(input.clone())
            .expect("installation");
        let materialized = jacquard_core::MaterializedRoute::from_installation(input, installation);

        engine
            .suspend_route_runtime_for_recovery(&route_id)
            .expect("suspend");
        assert!(engine
            .restore_route_runtime_for_router(&route_id)
            .expect("restore"));

        let active = engine.active_routes.get(&route_id).expect("active route");
        assert!(active.coordination_capability.is_some());
        assert!(!active.recovery.state.checkpoint_available);
        assert_eq!(active.recovery.state.checkpoint_capture_count, 1);
        assert_eq!(active.recovery.state.checkpoint_restore_count, 1);
        assert_eq!(
            active.recovery.state.last_outcome,
            Some(crate::FieldRouteRecoveryOutcome::CheckpointRestored)
        );

        let protocol_replay = engine
            .replay_snapshot(std::slice::from_ref(&materialized))
            .reduced_protocol_replay();
        assert!(protocol_replay
            .reconfigurations
            .iter()
            .any(|reconfiguration| {
                reconfiguration.prior_session.route_id == Some(route_id)
                    && reconfiguration.cause
                        == crate::choreography::FieldProtocolReconfigurationCause::CheckpointRestore
            }));
    }

    #[test]
    fn restore_route_runtime_fails_closed_for_stale_checkpoint_owner() {
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
        let input = materialization_input(route_id, admission);
        engine.materialize_route(input).expect("installation");

        engine
            .suspend_route_runtime_for_recovery(&route_id)
            .expect("suspend");
        let active = engine
            .active_routes
            .get_mut(&route_id)
            .expect("active route");
        active.selected_neighbor = node(9);

        let error = engine
            .restore_route_runtime_for_router(&route_id)
            .expect_err("stale restore must fail closed");
        assert!(matches!(
            error,
            RouteError::Runtime(RouteRuntimeError::Invalidated)
        ));
        let recovery = &engine
            .active_routes
            .get(&route_id)
            .expect("active route")
            .recovery
            .state;
        assert_eq!(
            recovery.last_trigger,
            Some(crate::FieldRouteRecoveryTrigger::RestoreRuntime)
        );
        assert_eq!(
            recovery.last_outcome,
            Some(crate::FieldRouteRecoveryOutcome::RecoveryFailed)
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
    fn weak_corridor_support_prefers_hold_fallback_before_failure() {
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
        state.corridor_belief.delivery_support = SupportBucket::new(210);
        let result = engine
            .maintain_route(
                &materialized.identity,
                &mut materialized.runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance");
        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::HoldFallback {
                trigger: RouteMaintenanceTrigger::LinkDegraded,
                retained_object_count: jacquard_core::HoldItemCount(1),
            }
        );
    }

    #[test]
    fn summary_transmission_heartbeats_each_tick_for_active_destinations() {
        let mut engine = seeded_engine();
        assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(4)));
        assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(5)));
        assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(6)));
    }

    #[test]
    fn observer_refresh_is_incremental_and_sparse_under_load() {
        let topology = supported_topology();
        let mut engine = FieldEngine::new(node(1), NoopTransport, ());
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

    #[test]
    fn observer_refresh_consumes_pending_evidence_once() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        engine.record_forward_summary(
            &DestinationId::Node(node(2)),
            node(2),
            crate::engine::FieldForwardSummaryObservation::new(RouteEpoch(4), Tick(4), 850, 1, 2),
        );
        engine.record_reverse_feedback(&DestinationId::Node(node(2)), node(2), 900, Tick(4));

        let destination = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Node(node(2)))
            .expect("destination before refresh");
        assert_eq!(destination.pending_forward_evidence.len(), 1);
        assert_eq!(destination.pending_reverse_feedback.len(), 1);

        assert!(engine.refresh_destination_observers(&topology.value, Tick(4)));
        let destination = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Node(node(2)))
            .expect("destination after refresh");
        assert!(destination.pending_forward_evidence.is_empty());
        assert!(destination.pending_reverse_feedback.is_empty());

        assert!(engine.refresh_destination_observers(&topology.value, Tick(5)));
        assert!(!engine.refresh_destination_observers(&topology.value, Tick(6)));
    }

    #[test]
    fn evidence_accumulates_across_ticks_and_changes_observer_state() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        engine.record_forward_summary(
            &DestinationId::Node(node(2)),
            node(2),
            crate::engine::FieldForwardSummaryObservation::new(RouteEpoch(4), Tick(4), 650, 1, 2),
        );
        assert!(engine.refresh_destination_observers(&topology.value, Tick(4)));
        let after_forward = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Node(node(2)))
            .expect("destination after forward evidence")
            .posterior
            .clone();
        assert_eq!(
            after_forward.predicted_observation_class,
            crate::state::ObservationClass::ForwardPropagated,
        );

        engine.record_reverse_feedback(&DestinationId::Node(node(2)), node(2), 900, Tick(5));
        assert!(engine.refresh_destination_observers(&topology.value, Tick(5)));
        let after_reverse = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Node(node(2)))
            .expect("destination after reverse feedback")
            .posterior
            .clone();
        assert_eq!(
            after_reverse.predicted_observation_class,
            crate::state::ObservationClass::ReverseValidated,
        );
        assert!(after_reverse.top_corridor_mass.value() >= after_forward.top_corridor_mass.value());
    }

    #[test]
    fn route_commitments_follow_live_evidence_withdrawal() {
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
        let route = jacquard_core::MaterializedRoute::from_installation(input, installation);
        engine.state.note_tick(Tick(4));
        assert_eq!(
            engine.route_commitments(&route)[0].resolution,
            RouteCommitmentResolution::Pending,
        );

        let destination = engine
            .state
            .destinations
            .get_mut(&crate::state::DestinationKey::Node(node(2)))
            .expect("tracked destination");
        destination.corridor_belief.delivery_support = SupportBucket::new(100);

        assert_eq!(
            engine.route_commitments(&route)[0].resolution,
            RouteCommitmentResolution::Invalidated(RouteInvalidationReason::EvidenceWithdrawn,),
        );
    }

    #[test]
    fn retention_replay_is_observational_until_explicit_evidence_intake_changes_state() {
        let topology = supported_topology();
        let mut engine = seeded_engine();
        engine.state.posture.current = crate::state::RoutingPosture::RetentionBiased;
        let destination = engine
            .state
            .destinations
            .get_mut(&crate::state::DestinationKey::Node(node(2)))
            .expect("tracked destination");
        destination.corridor_belief.delivery_support = SupportBucket::new(350);
        destination.posterior.top_corridor_mass = SupportBucket::new(320);

        let before = destination.posterior.clone();
        assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(4)));
        assert!(engine
            .protocol_artifacts()
            .iter()
            .any(|artifact| artifact.protocol == FieldProtocolKind::RetentionReplay));
        let after_protocol_only = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Node(node(2)))
            .expect("destination after protocol-only replay")
            .posterior
            .clone();
        assert_eq!(after_protocol_only, before);

        let replay_summary = FieldSummary {
            destination: SummaryDestinationKey::from(&DestinationId::Node(node(2))),
            topology_epoch: RouteEpoch(4),
            freshness_tick: Tick(5),
            hop_band: HopBand::new(1, 2),
            delivery_support: SupportBucket::new(900),
            congestion_penalty: crate::state::EntropyBucket::default(),
            retention_support: SupportBucket::new(600),
            uncertainty_penalty: crate::state::EntropyBucket::default(),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Low,
        };
        engine
            .ingest_forward_summary(node(2), replay_summary.encode(), Tick(5))
            .expect("ingest replayed summary");
        assert!(engine.refresh_destination_observers(&topology.value, Tick(5)));
        let after_ingest = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Node(node(2)))
            .expect("destination after replay intake")
            .posterior
            .clone();
        assert!(
            after_ingest.top_corridor_mass.value() > after_protocol_only.top_corridor_mass.value()
        );
    }
}

impl<Transport, Effects> RouterManagedEngine for FieldEngine<Transport, Effects>
where
    Transport: jacquard_traits::TransportSenderEffects,
{
    fn local_node_id_for_router(&self) -> NodeId {
        self.local_node_id
    }

    fn ingest_transport_observation_for_router(
        &mut self,
        observation: &TransportObservation,
    ) -> Result<(), RouteError> {
        let TransportObservation::PayloadReceived {
            from_node_id,
            payload,
            observed_at_tick,
            ..
        } = observation
        else {
            return Ok(());
        };
        if payload.len() != FIELD_SUMMARY_ENCODING_BYTES {
            return Ok(());
        }
        let payload: [u8; FIELD_SUMMARY_ENCODING_BYTES] = payload
            .as_slice()
            .try_into()
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        let _ignored_non_field_payload = self
            .ingest_forward_summary(*from_node_id, payload, *observed_at_tick)
            .is_err();
        Ok(())
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
        let mut candidates =
            Vec::with_capacity(active.continuation_neighbors.len().saturating_add(1));
        candidates.push(active.selected_neighbor);
        candidates.extend(
            active
                .continuation_neighbors
                .iter()
                .copied()
                .filter(|neighbor| *neighbor != active.selected_neighbor),
        );
        for neighbor in candidates {
            let Some(endpoint) = self.state.neighbor_endpoints.get(&neighbor) else {
                continue;
            };
            self.transport.send_transport(endpoint, payload)?;
            if neighbor != active.selected_neighbor {
                active.selected_neighbor = neighbor;
                // allow-ignored-result: forwarding stays productive even if the observational protocol reconfiguration marker cannot be retained.
                let _ = self.reconfigure_route_protocol_session(
                    route_id,
                    neighbor,
                    FieldProtocolReconfigurationCause::ContinuationShift,
                    self.state.last_tick_processed,
                );
            }
            return Ok(());
        }
        Err(RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(&mut self, route_id: &RouteId) -> Result<bool, RouteError> {
        if !self.active_routes.contains_key(route_id) {
            return Ok(false);
        }
        let needs_restore = self
            .active_routes
            .get(route_id)
            .is_some_and(|active| active.coordination_capability.is_none());
        if needs_restore {
            if let Some(checkpoint) = self.take_route_checkpoint(route_id) {
                self.restore_route_protocol_session(route_id, checkpoint)?;
            } else {
                self.note_route_without_checkpoint(route_id)?;
                let topology_epoch = self
                    .active_routes
                    .get(route_id)
                    .expect("route presence checked")
                    .topology_epoch;
                self.install_route_protocol_session(
                    route_id,
                    topology_epoch,
                    self.state.last_tick_processed,
                )?;
                self.note_fresh_route_runtime_install(route_id)?;
            }
        }
        Ok(true)
    }

    fn analysis_snapshot_for_router(
        &self,
        active_routes: &[jacquard_core::MaterializedRoute],
    ) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(self.exported_replay_bundle(active_routes)))
    }
}

fn destination_objective_class(
    destination: &DestinationId,
) -> crate::engine::FieldReducedObjectiveClass {
    match destination {
        DestinationId::Node(_) => crate::engine::FieldReducedObjectiveClass::Node,
        DestinationId::Gateway(_) => crate::engine::FieldReducedObjectiveClass::Gateway,
        DestinationId::Service(_) => crate::engine::FieldReducedObjectiveClass::Service,
    }
}

fn owner_tag_for_neighbor(neighbor: NodeId) -> u64 {
    u64::from_le_bytes(
        neighbor.0[..8]
            .try_into()
            .expect("node id prefix is 8 bytes"),
    )
}

fn bound_task_for_route(route_id: &RouteId) -> u64 {
    u64::from_le_bytes(
        route_id.0[..8]
            .try_into()
            .expect("route id prefix is 8 bytes"),
    )
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    pub fn suspend_route_runtime_for_recovery(
        &mut self,
        route_id: &RouteId,
    ) -> Result<bool, RouteError> {
        let Some(capability) = self
            .active_routes
            .get(route_id)
            .and_then(|active| active.coordination_capability.clone())
        else {
            return Ok(self.active_routes.contains_key(route_id));
        };
        let checkpoint = match self.protocol_runtime.checkpoint_session(&capability) {
            Ok(checkpoint) => checkpoint,
            Err(_) => {
                self.note_route_recovery_failed(
                    route_id,
                    FieldRouteRecoveryTrigger::SuspendForRuntimeLoss,
                )?;
                return Err(RouteRuntimeError::Invalidated.into());
            }
        };
        let _closed = match self.protocol_runtime.close_session(&capability) {
            Ok(closed) => closed,
            Err(_) => {
                self.note_route_recovery_failed(
                    route_id,
                    FieldRouteRecoveryTrigger::SuspendForRuntimeLoss,
                )?;
                return Err(RouteRuntimeError::Invalidated.into());
            }
        };
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during recovery suspend");
        active.coordination_capability = None;
        active.recovery.note_checkpoint_stored(checkpoint);
        Ok(true)
    }

    fn install_route_protocol_session(
        &mut self,
        route_id: &RouteId,
        topology_epoch: jacquard_core::RouteEpoch,
        _now_tick: Tick,
    ) -> Result<(), RouteError> {
        let Some(active) = self.active_routes.get(route_id) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let destination = DestinationId::from(&active.destination);
        let session_key = FieldProtocolSessionKey {
            protocol: FieldProtocolKind::ExplicitCoordination,
            route_id: Some(*route_id),
            topology_epoch,
            destination: Some(SummaryDestinationKey::from(&destination)),
        };
        let capability = self
            .protocol_runtime
            .open_session(
                &session_key,
                owner_tag_for_neighbor(active.selected_neighbor),
                Some(bound_task_for_route(route_id)),
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during session install");
        active.coordination_capability = Some(capability);
        Ok(())
    }

    fn restore_route_protocol_session(
        &mut self,
        route_id: &RouteId,
        checkpoint: FieldProtocolCheckpoint,
    ) -> Result<(), RouteError> {
        self.validate_route_checkpoint(route_id, &checkpoint)?;
        let capability = match self.protocol_runtime.restore_session(checkpoint) {
            Ok(capability) => capability,
            Err(_) => {
                self.note_route_recovery_failed(
                    route_id,
                    FieldRouteRecoveryTrigger::RestoreRuntime,
                )?;
                return Err(RouteRuntimeError::Invalidated.into());
            }
        };
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during checkpoint restore");
        active.coordination_capability = Some(capability);
        active.recovery.note_checkpoint_restored();
        Ok(())
    }

    fn reconfigure_route_protocol_session(
        &mut self,
        route_id: &RouteId,
        new_neighbor: NodeId,
        cause: FieldProtocolReconfigurationCause,
        now_tick: Tick,
    ) -> Result<(), RouteError> {
        let capability = self
            .active_routes
            .get(route_id)
            .and_then(|active| active.coordination_capability.clone())
            .ok_or(RouteRuntimeError::Invalidated)?;
        let updated = self
            .protocol_runtime
            .transfer_owner_with_cause(
                &capability,
                owner_tag_for_neighbor(new_neighbor),
                Some(bound_task_for_route(route_id)),
                cause,
                now_tick,
            )
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("route remains present during session reconfiguration");
        active.coordination_capability = Some(updated);
        if cause == FieldProtocolReconfigurationCause::ContinuationShift {
            active.recovery.note_continuation_retained();
        }
        Ok(())
    }

    fn close_route_protocol_session(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        let Some(capability) = self
            .active_routes
            .get(route_id)
            .and_then(|active| active.coordination_capability.clone())
        else {
            return Ok(());
        };
        let _closed = self
            .protocol_runtime
            .close_session(&capability)
            .map_err(|_| RouteRuntimeError::Invalidated)?;
        Ok(())
    }

    fn take_route_checkpoint(&mut self, route_id: &RouteId) -> Option<FieldProtocolCheckpoint> {
        self.active_routes
            .get_mut(route_id)
            .and_then(|active| active.recovery.checkpoint.take())
    }

    fn note_route_without_checkpoint(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active.recovery.note_no_checkpoint_available();
        Ok(())
    }

    fn note_fresh_route_runtime_install(&mut self, route_id: &RouteId) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active.recovery.note_fresh_session_installed();
        Ok(())
    }

    fn note_route_recovery_failed(
        &mut self,
        route_id: &RouteId,
        trigger: FieldRouteRecoveryTrigger,
    ) -> Result<(), RouteError> {
        let active = self
            .active_routes
            .get_mut(route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
        active.recovery.note_recovery_failed(trigger);
        Ok(())
    }

    fn validate_route_checkpoint(
        &mut self,
        route_id: &RouteId,
        checkpoint: &FieldProtocolCheckpoint,
    ) -> Result<(), RouteError> {
        let Some(active) = self.active_routes.get_mut(route_id) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let destination = DestinationId::from(&active.destination);
        let expected_session = FieldProtocolSessionKey {
            protocol: FieldProtocolKind::ExplicitCoordination,
            route_id: Some(*route_id),
            topology_epoch: active.topology_epoch,
            destination: Some(SummaryDestinationKey::from(&destination)),
        };
        let expected_owner = owner_tag_for_neighbor(active.selected_neighbor);
        if checkpoint.session != expected_session || checkpoint.owner_tag != expected_owner {
            active
                .recovery
                .note_recovery_failed(FieldRouteRecoveryTrigger::RestoreRuntime);
            return Err(RouteRuntimeError::Invalidated.into());
        }
        Ok(())
    }
}
