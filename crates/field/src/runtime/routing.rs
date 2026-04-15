use super::*;

#[derive(Clone, Debug)]
struct PreparedMaintenance {
    policy: crate::policy::FieldPolicy,
    destination_context: FieldDestinationDecisionContext,
    destination_state: crate::state::DestinationFieldState,
    active_route: ActiveFieldRoute,
    search_config: crate::FieldSearchConfig,
    ranked: Vec<(NeighborContinuation, SupportBucket)>,
    best: NeighborContinuation,
    current_corridor_envelope: CorridorBeliefEnvelope,
    current_witness_detail: FieldWitnessDetail,
    current_bootstrap_class: FieldBootstrapClass,
    current_continuity_band: FieldContinuityBand,
    corridor_support: u16,
}

#[derive(Clone, Debug)]
struct TransitionEvaluation {
    promotion_assessment: crate::planner::promotion::FieldPromotionAssessment,
    bootstrap_decision: FieldBootstrapDecision,
    effective_continuity_band: FieldContinuityBand,
    projected_promotion_window_score: u8,
    projected_confirmation_streak: u8,
    policy_events: Vec<FieldPolicyEvent>,
}

#[derive(Clone, Debug)]
struct AppliedTransition {
    previous_posture: crate::state::RoutingPosture,
    pending_coordination_shift: Option<NodeId>,
    maintenance_outcome: RouteMaintenanceOutcome,
}

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
        recovery.note_continuity_band(detail.continuity_band);
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
                witness_detail: detail.clone(),
                bootstrap_class,
                continuity_band: detail.continuity_band,
                backend_route_id: input.admission.backend_ref.backend_route_id.clone(),
                topology_epoch: input.handle.topology_epoch(),
                installed_at_tick: input.handle.materialized_at_tick(),
                bootstrap_confirmation_streak: 0,
                promotion_window_score: 0,
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

    // long-block-exception: route commitments keep lease validity, continuity,
    // and downgrade handling in one publication-facing resolution path.
    fn route_commitments(&self, route: &jacquard_core::MaterializedRoute) -> Vec<RouteCommitment> {
        let resolution = if !route
            .identity
            .lease
            .is_valid_at(self.state.last_tick_processed)
        {
            RouteCommitmentResolution::Invalidated(RouteInvalidationReason::LeaseExpired)
        } else if let Some(active) = self.active_routes.get(route.identity.route_id()) {
            let destination_state = self.state.destinations.get(&active.destination);
            let current_support = destination_state
                .map(|state| state.corridor_belief.delivery_support.value())
                .unwrap_or(active.corridor_envelope.delivery_support.value());
            let destination_context =
                FieldDestinationDecisionContext::new(&active.destination, &self.search_config);
            let service_corridor_viable = destination_context.service_bias()
                && destination_state.is_some_and(|state| service_corridor_viable(active, state));
            let node_corridor_viable = destination_context.discovery_node_route()
                && destination_state.is_some_and(|state| node_corridor_viable(active, state));
            let commitment_support_floor = commitment_support_floor(
                active.bootstrap_class,
                active.continuity_band,
                destination_context.service_bias(),
                destination_context.discovery_node_route(),
            );
            if active.topology_epoch != route.identity.topology_epoch() {
                RouteCommitmentResolution::Invalidated(RouteInvalidationReason::TopologySuperseded)
            } else if current_support < commitment_support_floor
                && !service_corridor_viable
                && !node_corridor_viable
            {
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
        let policy = *self.policy();
        let topology_seeded = self.seed_destinations_from_topology(&tick.topology.value, now_tick);
        let measurements =
            ControlMeasurements::from_topology(&tick.topology.value, self.local_node_id);
        let slow_path =
            advance_control_plane_with_policy(&mut self.state, measurements, now_tick, &policy);
        if let Some(event) = slow_path.policy_event.clone() {
            self.record_policy_event(event);
        }
        let observer_changed = self.refresh_destination_observers(&tick.topology.value, now_tick);
        let protocol_changed = self.advance_protocol_sessions(tick.topology.value.epoch, now_tick);
        let attractor_view =
            derive_local_attractor_view_with_policy(&self.state, &policy.evidence.attractor);
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
        let Some(active_route) = self.active_routes.get(identity.route_id()).cloned() else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let Some(prepared) = self.prepare_maintenance(active_route.clone())? else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        let service_bias = prepared.destination_context.service_bias();
        let discovery_node_route = prepared.destination_context.discovery_node_route();

        runtime.health = route_health_for(
            &prepared.destination_state.corridor_belief,
            self.state.last_tick_processed,
        );
        runtime.progress.last_progress_at_tick = self.state.last_tick_processed;
        let evaluation = evaluate_transition(
            identity.route_id(),
            &prepared,
            self.state.last_tick_processed,
        );

        if self.state.controller.congestion_price.value()
            >= self
                .policy()
                .continuity
                .runtime
                .retention_biased_hold_congestion_price_floor_permille
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
        let transition =
            self.apply_transition(identity.route_id(), &prepared, &evaluation, trigger)?;
        if matches!(
            transition.maintenance_outcome,
            RouteMaintenanceOutcome::ReplacementRequired { .. }
        ) {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: transition.maintenance_outcome,
            });
        }
        let mut maintenance_outcome = transition.maintenance_outcome;

        let degraded_continuity = matches!(
            evaluation.effective_continuity_band,
            FieldContinuityBand::DegradedSteady | FieldContinuityBand::Bootstrap
        ) && evaluation
            .promotion_assessment
            .degraded_but_coherent(&prepared.destination_state);
        let post_shift_grace = self
            .active_routes
            .get(identity.route_id())
            .is_some_and(|active| {
                continuation_shift_grace_active(active, &evaluation.promotion_assessment)
            });
        let failure_support_floor = maintenance_failure_support_floor(
            &prepared.policy.continuity.runtime,
            prepared.current_bootstrap_class,
            evaluation.effective_continuity_band,
            degraded_continuity,
            service_bias,
            discovery_node_route,
            post_shift_grace,
        );

        if prepared.corridor_support < failure_support_floor {
            if degraded_continuity {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    if active.continuity_band == FieldContinuityBand::DegradedSteady {
                        active.recovery.note_corridor_narrowed();
                    } else {
                        active
                            .recovery
                            .note_bootstrap_narrowed(FieldPromotionBlocker::SupportTrend);
                    }
                }
                self.record_policy_event(route_policy_event(
                    identity.route_id(),
                    &prepared.destination_context.destination,
                    FieldPolicyGate::CarryForward,
                    FieldPolicyReason::EmittedByEvidenceGate,
                    self.state.last_tick_processed,
                ));
                return Ok(RouteMaintenanceResult {
                    event: RouteLifecycleEvent::Activated,
                    outcome: RouteMaintenanceOutcome::HoldFallback {
                        trigger,
                        retained_object_count: jacquard_core::HoldItemCount(1),
                    },
                });
            }
            if post_shift_grace {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    active.recovery.note_continuation_retained();
                }
                self.record_policy_event(route_policy_event(
                    identity.route_id(),
                    &prepared.destination_context.destination,
                    FieldPolicyGate::CarryForward,
                    FieldPolicyReason::EmittedByEvidenceGate,
                    self.state.last_tick_processed,
                ));
                return Ok(RouteMaintenanceResult {
                    event: RouteLifecycleEvent::Activated,
                    outcome: RouteMaintenanceOutcome::HoldFallback {
                        trigger,
                        retained_object_count: jacquard_core::HoldItemCount(1),
                    },
                });
            }
            if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                if active.bootstrap_class == FieldBootstrapClass::Bootstrap
                    && active.recovery.state.last_bootstrap_transition
                        != Some(crate::FieldBootstrapTransition::Withdrawn)
                {
                    active
                        .recovery
                        .note_bootstrap_withdrawn(FieldPromotionBlocker::SupportTrend);
                }
            }
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded),
            });
        }

        if self.state.controller.congestion_price.value()
            >= self
                .policy()
                .continuity
                .runtime
                .retention_biased_hold_congestion_price_floor_permille
        {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::CapacityExceeded,
                },
            });
        }

        if self.state.posture.current != transition.previous_posture
            && self.state.posture.current == crate::state::RoutingPosture::RiskSuppressed
        {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Replaced,
                outcome: RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::PolicyShift,
                },
            });
        }

        if let Some(new_neighbor) = transition.pending_coordination_shift {
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
            .saturating_sub(prepared.best.freshness.0)
            > maintenance_stale_ticks_max(
                &prepared.policy.continuity.runtime,
                prepared.current_bootstrap_class,
                evaluation.effective_continuity_band,
                degraded_continuity,
                service_bias,
                discovery_node_route,
                post_shift_grace,
            );
        if is_stale {
            if prepared.corridor_support
                < weak_support_hold_floor(&prepared.policy.continuity.runtime, discovery_node_route)
            {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    if active.bootstrap_class == FieldBootstrapClass::Bootstrap
                        || active.continuity_band == FieldContinuityBand::DegradedSteady
                    {
                        if degraded_continuity {
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
                if degraded_continuity {
                    self.record_policy_event(route_policy_event(
                        identity.route_id(),
                        &prepared.destination_context.destination,
                        FieldPolicyGate::CarryForward,
                        FieldPolicyReason::EmittedByEvidenceGate,
                        self.state.last_tick_processed,
                    ));
                    maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                        trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                        retained_object_count: jacquard_core::HoldItemCount(1),
                    };
                } else if post_shift_grace {
                    self.record_policy_event(route_policy_event(
                        identity.route_id(),
                        &prepared.destination_context.destination,
                        FieldPolicyGate::CarryForward,
                        FieldPolicyReason::EmittedByEvidenceGate,
                        self.state.last_tick_processed,
                    ));
                    maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                        trigger,
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
                self.record_policy_event(route_policy_event(
                    identity.route_id(),
                    &prepared.destination_context.destination,
                    FieldPolicyGate::CarryForward,
                    FieldPolicyReason::EmittedByEvidenceGate,
                    self.state.last_tick_processed,
                ));
                maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                    trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                    retained_object_count: jacquard_core::HoldItemCount(1),
                };
            }
        }

        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: if prepared.corridor_support
                < weak_support_hold_floor(&prepared.policy.continuity.runtime, discovery_node_route)
            {
                if let Some(active) = self.active_routes.get_mut(identity.route_id()) {
                    if active.bootstrap_class == FieldBootstrapClass::Bootstrap {
                        active
                            .recovery
                            .note_bootstrap_held(evaluation.promotion_assessment.primary_blocker());
                    }
                }
                self.record_policy_event(route_policy_event(
                    identity.route_id(),
                    &prepared.destination_context.destination,
                    FieldPolicyGate::CarryForward,
                    FieldPolicyReason::EmittedByEvidenceGate,
                    self.state.last_tick_processed,
                ));
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

fn commitment_support_floor(
    bootstrap_class: FieldBootstrapClass,
    continuity_band: FieldContinuityBand,
    service_bias: bool,
    discovery_node_route: bool,
) -> u16 {
    match (bootstrap_class, continuity_band) {
        (_, FieldContinuityBand::DegradedSteady) => {
            if service_bias || discovery_node_route {
                FIELD_COMMITMENT_DEGRADED_SUPPORT_FLOOR
                    .saturating_sub(FIELD_COMMITMENT_DEGRADED_SERVICE_RELIEF_PERMILLE)
            } else {
                FIELD_COMMITMENT_DEGRADED_SUPPORT_FLOOR
            }
        }
        (FieldBootstrapClass::Bootstrap, _) => {
            if service_bias || discovery_node_route {
                FIELD_COMMITMENT_BOOTSTRAP_SUPPORT_FLOOR
                    .saturating_sub(FIELD_COMMITMENT_BOOTSTRAP_SERVICE_RELIEF_PERMILLE)
            } else {
                FIELD_COMMITMENT_BOOTSTRAP_SUPPORT_FLOOR
            }
        }
        _ if discovery_node_route => FIELD_COMMITMENT_DISCOVERY_SUPPORT_FLOOR,
        _ => FIELD_COMMITMENT_STEADY_SUPPORT_FLOOR,
    }
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    fn prepare_maintenance(
        &self,
        active_route: ActiveFieldRoute,
    ) -> Result<Option<PreparedMaintenance>, RouteError> {
        let destination_key = active_route.destination.clone();
        let Some(destination_state) = self.state.destinations.get(&destination_key).cloned() else {
            return Ok(None);
        };
        let search_config = self.search_config().clone();
        let destination_context =
            FieldDestinationDecisionContext::new(&destination_key, &search_config);
        let mut ranked = rank_frontier_by_attractor_with_policy(
            &destination_state,
            &self.state.mean_field,
            self.state.regime.current,
            self.state.posture.current,
            &self.state.controller,
            &self.policy().evidence.attractor,
        );
        merge_pending_forward_continuations(&mut ranked, &destination_state);
        if ranked.is_empty() && destination_context.discovery_node_route() {
            ranked = synthesized_node_carry_forward_ranked(
                &active_route,
                &destination_state,
                &self.state.neighbor_endpoints,
                self.state.last_tick_processed,
                &search_config,
            );
        }
        let Some((ranked_best, _)) = ranked.first() else {
            return Ok(None);
        };
        let preferred_service_neighbor = if destination_context.service_bias() {
            preferred_service_shift_neighbor(&active_route, &ranked, ranked_best, &search_config)
        } else {
            None
        };
        let preferred_node_neighbor = if destination_context.discovery_node_route() {
            preferred_node_shift_neighbor(
                &active_route,
                &ranked,
                &destination_state,
                &self.state.neighbor_endpoints,
                &search_config,
            )
        } else {
            None
        };
        let best = preferred_service_neighbor
            .or(preferred_node_neighbor)
            .and_then(|neighbor_id| {
                ranked.iter().find_map(|(entry, _)| {
                    (entry.neighbor_id == neighbor_id).then_some(entry.clone())
                })
            })
            .unwrap_or_else(|| ranked_best.clone());
        let current_corridor_envelope = destination_state.corridor_belief.clone();
        let current_witness_detail = self.witness_detail_from_state(&destination_state);
        let current_bootstrap_class = current_witness_detail.bootstrap_class;
        let current_continuity_band =
            continuity_band_for_state_with_config(&destination_state, &search_config);

        Ok(Some(PreparedMaintenance {
            policy: *self.policy(),
            destination_context,
            destination_state,
            active_route,
            search_config,
            ranked,
            best,
            current_corridor_envelope: current_corridor_envelope.clone(),
            current_witness_detail,
            current_bootstrap_class,
            current_continuity_band,
            corridor_support: current_corridor_envelope.delivery_support.value(),
        }))
    }

    fn apply_transition(
        &mut self,
        route_id: &RouteId,
        prepared: &PreparedMaintenance,
        evaluation: &TransitionEvaluation,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<AppliedTransition, RouteError> {
        let runtime_context = FieldRuntimeDecisionContext::new(
            &prepared.destination_context,
            evaluation.effective_continuity_band,
        );
        let blocker = evaluation.promotion_assessment.primary_blocker();
        let mut pending_coordination_shift = None;
        let mut maintenance_outcome = RouteMaintenanceOutcome::Continued;
        let mut pending_policy_events = Vec::new();
        let active = self
            .active_routes
            .get_mut(route_id)
            .expect("active route remains present during maintenance");
        let previous_posture = active.witness_detail.posture;
        let previous_bootstrap_class = active.bootstrap_class;
        let previous_continuity_band = active.continuity_band;

        active.corridor_envelope = prepared.current_corridor_envelope.clone();
        active.witness_detail = prepared.current_witness_detail.clone();
        active.bootstrap_class = match (previous_bootstrap_class, evaluation.bootstrap_decision) {
            (FieldBootstrapClass::Bootstrap, FieldBootstrapDecision::Promote) => {
                FieldBootstrapClass::Steady
            }
            (FieldBootstrapClass::Bootstrap, _)
                if prepared.current_bootstrap_class == FieldBootstrapClass::Steady
                    && evaluation.effective_continuity_band != FieldContinuityBand::Bootstrap =>
            {
                FieldBootstrapClass::Steady
            }
            (FieldBootstrapClass::Bootstrap, _) => FieldBootstrapClass::Bootstrap,
            (_, _) => prepared.current_bootstrap_class,
        };
        active.witness_detail.bootstrap_class = active.bootstrap_class;
        active.continuity_band = evaluation.effective_continuity_band;
        active.witness_detail.continuity_band = evaluation.effective_continuity_band;
        active.promotion_window_score = evaluation.projected_promotion_window_score;

        match (previous_continuity_band, active.continuity_band) {
            (FieldContinuityBand::Steady, FieldContinuityBand::DegradedSteady)
            | (FieldContinuityBand::Bootstrap, FieldContinuityBand::DegradedSteady) => {
                active.recovery.note_degraded_steady_entered();
            }
            (FieldContinuityBand::DegradedSteady, FieldContinuityBand::Steady) => {
                active.recovery.note_degraded_steady_recovered();
            }
            (FieldContinuityBand::DegradedSteady, FieldContinuityBand::Bootstrap) => {
                active.recovery.note_degraded_to_bootstrap();
            }
            (_, band) => active.recovery.note_continuity_band(band),
        }

        match (
            previous_bootstrap_class,
            active.bootstrap_class,
            evaluation.bootstrap_decision,
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
                active.promotion_window_score = active.promotion_window_score.saturating_sub(1);
            }
            (
                FieldBootstrapClass::Bootstrap,
                FieldBootstrapClass::Bootstrap,
                FieldBootstrapDecision::Hold,
            ) => {
                active.recovery.note_bootstrap_held(blocker);
                if evaluation.promotion_assessment.anti_entropy_confirmed
                    && evaluation.promotion_assessment.continuation_coherent
                {
                    active.bootstrap_confirmation_streak = evaluation.projected_confirmation_streak;
                } else {
                    active.bootstrap_confirmation_streak = 0;
                }
            }
            (FieldBootstrapClass::Bootstrap, FieldBootstrapClass::Steady, _) => {
                active.recovery.note_bootstrap_upgraded();
                active.bootstrap_confirmation_streak = 0;
                active.promotion_window_score = 0;
            }
            (
                FieldBootstrapClass::Bootstrap,
                FieldBootstrapClass::Bootstrap,
                FieldBootstrapDecision::Withdraw,
            ) => {
                active.recovery.note_bootstrap_withdrawn(blocker);
                active.bootstrap_confirmation_streak = 0;
                active.promotion_window_score = 0;
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
        }

        if !active
            .continuation_neighbors
            .contains(&prepared.best.neighbor_id)
        {
            let selected_entry = prepared
                .ranked
                .iter()
                .find(|(entry, _)| entry.neighbor_id == active.selected_neighbor)
                .map(|(entry, _)| entry);
            let degraded_continuity = matches!(
                active.continuity_band,
                FieldContinuityBand::DegradedSteady | FieldContinuityBand::Bootstrap
            ) && evaluation
                .promotion_assessment
                .degraded_but_coherent(&prepared.destination_state);
            let support_delta = selected_entry
                .map(|entry| {
                    prepared
                        .best
                        .net_value
                        .value()
                        .abs_diff(entry.net_value.value())
                })
                .unwrap_or(0);
            let shift_delta_max = runtime_shift_delta_max(
                &prepared.policy.continuity.runtime,
                runtime_context.service_bias,
                runtime_context.discovery_node_route,
                active.continuity_band,
            );
            let support_floor = runtime_shift_support_floor(
                &prepared.policy.continuity.runtime,
                runtime_context.service_bias,
                runtime_context.discovery_node_route,
                active.continuity_band,
            );
            if prepared.corridor_support >= support_floor && support_delta <= shift_delta_max {
                active.continuation_neighbors = if runtime_context.service_bias {
                    service_runtime_continuation_neighbors(
                        &prepared.ranked,
                        &prepared.destination_state,
                        prepared.best.neighbor_id,
                        &prepared.search_config,
                    )
                } else if runtime_context.discovery_node_route {
                    node_runtime_continuation_neighbors(
                        &prepared.ranked,
                        &prepared.destination_state,
                        prepared.best.neighbor_id,
                        &prepared.search_config,
                    )
                } else {
                    prepared
                        .ranked
                        .iter()
                        .take(crate::state::MAX_CONTINUATION_NEIGHBOR_COUNT + 1)
                        .map(|(entry, _)| entry.neighbor_id)
                        .collect()
                };
                active.selected_neighbor = prepared.best.neighbor_id;
                pending_coordination_shift = Some(prepared.best.neighbor_id);
                if runtime_context.service_bias {
                    active.recovery.note_service_retention_carry_forward();
                    pending_policy_events.push(route_policy_event(
                        route_id,
                        &prepared.destination_context.destination,
                        FieldPolicyGate::CarryForward,
                        FieldPolicyReason::EmittedByContinuityGate,
                        self.state.last_tick_processed,
                    ));
                }
                if degraded_continuity {
                    active.recovery.note_asymmetric_shift_success();
                }
            } else if degraded_continuity || runtime_context.discovery_node_route {
                active.continuation_neighbors = if runtime_context.discovery_node_route {
                    node_runtime_continuation_neighbors(
                        &prepared.ranked,
                        &prepared.destination_state,
                        active.selected_neighbor,
                        &prepared.search_config,
                    )
                } else {
                    prepared
                        .ranked
                        .iter()
                        .take(2)
                        .map(|(entry, _)| entry.neighbor_id)
                        .collect()
                };
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
                return Ok(AppliedTransition {
                    previous_posture,
                    pending_coordination_shift: None,
                    maintenance_outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
                });
            }
        }

        if active.selected_neighbor != prepared.best.neighbor_id {
            if active.continuity_band == FieldContinuityBand::DegradedSteady {
                active.recovery.note_asymmetric_shift_success();
            }
            if runtime_context.service_bias {
                active.continuation_neighbors = service_runtime_continuation_neighbors(
                    &prepared.ranked,
                    &prepared.destination_state,
                    prepared.best.neighbor_id,
                    &prepared.search_config,
                );
                active.recovery.note_service_retention_carry_forward();
                pending_policy_events.push(route_policy_event(
                    route_id,
                    &prepared.destination_context.destination,
                    FieldPolicyGate::CarryForward,
                    FieldPolicyReason::EmittedByContinuityGate,
                    self.state.last_tick_processed,
                ));
            } else if runtime_context.discovery_node_route {
                active.continuation_neighbors = node_runtime_continuation_neighbors(
                    &prepared.ranked,
                    &prepared.destination_state,
                    prepared.best.neighbor_id,
                    &prepared.search_config,
                );
            }
            active.selected_neighbor = prepared.best.neighbor_id;
            pending_coordination_shift = Some(prepared.best.neighbor_id);
        }

        for event in &evaluation.policy_events {
            pending_policy_events.push(event.clone());
        }
        let _ = active;
        for event in pending_policy_events {
            self.record_policy_event(event);
        }

        Ok(AppliedTransition {
            previous_posture,
            pending_coordination_shift,
            maintenance_outcome,
        })
    }
}

fn evaluate_transition(
    route_id: &RouteId,
    prepared: &PreparedMaintenance,
    now_tick: Tick,
) -> TransitionEvaluation {
    let previous_continuity_band = prepared.active_route.continuity_band;
    let promotion_assessment = promotion_assessment_for_route_with_policy(
        &prepared.active_route,
        &prepared.destination_state,
        &prepared.best,
        now_tick,
        &prepared.policy.promotion,
    );
    let blocker = promotion_assessment.primary_blocker();
    let projected_promotion_window_score = updated_promotion_window_score(
        prepared.active_route.promotion_window_score,
        &promotion_assessment,
        &prepared.destination_state,
        prepared.destination_context.service_bias(),
    );
    let projected_confirmation_streak = if prepared.current_bootstrap_class
        == FieldBootstrapClass::Bootstrap
        && promotion_assessment.anti_entropy_confirmed
        && promotion_assessment.continuation_coherent
    {
        prepared
            .active_route
            .bootstrap_confirmation_streak
            .saturating_add(1)
    } else {
        0
    };
    let mut bootstrap_decision =
        if prepared.current_bootstrap_class == FieldBootstrapClass::Bootstrap {
            promotion_assessment.decision_for_bootstrap(
                &prepared.destination_state,
                projected_confirmation_streak,
                projected_promotion_window_score,
                &prepared.search_config,
            )
        } else {
            FieldBootstrapDecision::Hold
        };
    if prepared.destination_context.discovery_node_route()
        && previous_continuity_band == FieldContinuityBand::DegradedSteady
        && bootstrap_decision == FieldBootstrapDecision::Withdraw
        && node_corridor_viable(&prepared.active_route, &prepared.destination_state)
    {
        bootstrap_decision = FieldBootstrapDecision::Hold;
    }
    let effective_continuity_band = match (
        previous_continuity_band,
        prepared.current_continuity_band,
        prepared.current_bootstrap_class,
    ) {
        (_, FieldContinuityBand::Steady, _) => FieldContinuityBand::Steady,
        (_, FieldContinuityBand::DegradedSteady, _) => FieldContinuityBand::DegradedSteady,
        (
            FieldContinuityBand::Steady | FieldContinuityBand::DegradedSteady,
            FieldContinuityBand::Bootstrap,
            FieldBootstrapClass::Bootstrap,
        ) if promotion_assessment.degraded_but_coherent(&prepared.destination_state) => {
            FieldContinuityBand::DegradedSteady
        }
        (
            FieldContinuityBand::Bootstrap,
            FieldContinuityBand::Bootstrap,
            FieldBootstrapClass::Bootstrap,
        ) if prepared.destination_context.discovery_node_route()
            && promotion_assessment.degraded_but_coherent(&prepared.destination_state) =>
        {
            FieldContinuityBand::DegradedSteady
        }
        (
            FieldContinuityBand::DegradedSteady,
            FieldContinuityBand::Bootstrap,
            FieldBootstrapClass::Bootstrap,
        ) if prepared.destination_context.discovery_node_route()
            && bootstrap_decision == FieldBootstrapDecision::Hold
            && node_corridor_viable(&prepared.active_route, &prepared.destination_state) =>
        {
            FieldContinuityBand::DegradedSteady
        }
        _ => FieldContinuityBand::Bootstrap,
    };

    let mut policy_events = Vec::new();
    if prepared.current_bootstrap_class == FieldBootstrapClass::Bootstrap
        && bootstrap_decision != FieldBootstrapDecision::Promote
    {
        policy_events.push(route_policy_event(
            route_id,
            &prepared.destination_context.destination,
            FieldPolicyGate::Promotion,
            policy_reason_for_promotion_blocker(blocker),
            now_tick,
        ));
    }
    if continuity_softened(previous_continuity_band, effective_continuity_band) {
        policy_events.push(route_policy_event(
            route_id,
            &prepared.destination_context.destination,
            FieldPolicyGate::Continuity,
            continuity_softening_reason(blocker),
            now_tick,
        ));
    }
    TransitionEvaluation {
        promotion_assessment,
        bootstrap_decision,
        effective_continuity_band,
        projected_promotion_window_score,
        projected_confirmation_streak,
        policy_events,
    }
}

fn runtime_shift_delta_max(
    policy: &crate::policy::FieldRuntimeContinuityPolicy,
    service_bias: bool,
    discovery_node_route: bool,
    continuity_band: FieldContinuityBand,
) -> u16 {
    let base = policy.envelope_shift_support_delta_max_permille;
    if service_bias {
        base.saturating_add(policy.service_shift_delta_bonus_permille)
    } else if discovery_node_route && continuity_band == FieldContinuityBand::DegradedSteady {
        base.saturating_add(policy.discovery_shift_delta_bonus_permille)
    } else if continuity_band == FieldContinuityBand::DegradedSteady {
        base.saturating_add(policy.degraded_shift_delta_bonus_permille)
    } else {
        base
    }
}

fn runtime_shift_support_floor(
    policy: &crate::policy::FieldRuntimeContinuityPolicy,
    service_bias: bool,
    discovery_node_route: bool,
    continuity_band: FieldContinuityBand,
) -> u16 {
    let base = if continuity_band == FieldContinuityBand::DegradedSteady {
        policy.degraded_steady_failure_support_floor_permille
    } else {
        policy.route_weak_support_floor_permille
    };
    base.saturating_sub(if service_bias || discovery_node_route {
        policy.support_floor_relief_permille
    } else {
        0
    })
}

fn maintenance_failure_support_floor(
    policy: &crate::policy::FieldRuntimeContinuityPolicy,
    bootstrap_class: FieldBootstrapClass,
    continuity_band: FieldContinuityBand,
    degraded_continuity: bool,
    service_bias: bool,
    discovery_node_route: bool,
    post_shift_grace: bool,
) -> u16 {
    let base = if bootstrap_class == FieldBootstrapClass::Bootstrap && degraded_continuity {
        if service_bias {
            policy
                .bootstrap_failure_support_floor_permille
                .saturating_sub(FIELD_COMMITMENT_BOOTSTRAP_SERVICE_RELIEF_PERMILLE)
        } else {
            policy.bootstrap_failure_support_floor_permille
        }
    } else if continuity_band == FieldContinuityBand::DegradedSteady {
        if service_bias {
            policy
                .degraded_steady_failure_support_floor_permille
                .saturating_sub(FIELD_COMMITMENT_DEGRADED_SERVICE_RELIEF_PERMILLE)
        } else {
            policy.degraded_steady_failure_support_floor_permille
        }
    } else {
        policy.route_failure_support_floor_permille
    };
    base.saturating_sub(if post_shift_grace {
        FIELD_FAILURE_POST_SHIFT_GRACE_RELIEF_PERMILLE
    } else if discovery_node_route {
        FIELD_FAILURE_DISCOVERY_RELIEF_PERMILLE
    } else {
        0
    })
}

fn maintenance_stale_ticks_max(
    policy: &crate::policy::FieldRuntimeContinuityPolicy,
    bootstrap_class: FieldBootstrapClass,
    continuity_band: FieldContinuityBand,
    degraded_continuity: bool,
    service_bias: bool,
    discovery_node_route: bool,
    post_shift_grace: bool,
) -> u64 {
    let base = if bootstrap_class == FieldBootstrapClass::Bootstrap && degraded_continuity {
        if service_bias {
            policy
                .bootstrap_stale_ticks_max
                .saturating_add(FIELD_STALE_SERVICE_RELIEF_TICKS)
        } else {
            policy.bootstrap_stale_ticks_max
        }
    } else if continuity_band == FieldContinuityBand::DegradedSteady {
        if service_bias {
            policy
                .degraded_steady_stale_ticks_max
                .saturating_add(FIELD_STALE_SERVICE_RELIEF_TICKS)
        } else if discovery_node_route {
            policy
                .degraded_steady_stale_ticks_max
                .saturating_add(FIELD_STALE_DISCOVERY_RELIEF_TICKS)
        } else {
            policy.degraded_steady_stale_ticks_max
        }
    } else if discovery_node_route {
        FIELD_STALE_STEADY_DISCOVERY_TICKS_MAX
    } else {
        FIELD_STALE_STEADY_TICKS_MAX
    };
    base.saturating_add(if post_shift_grace {
        FIELD_STALE_POST_SHIFT_GRACE_TICKS
    } else {
        0
    })
}

fn weak_support_hold_floor(
    policy: &crate::policy::FieldRuntimeContinuityPolicy,
    discovery_node_route: bool,
) -> u16 {
    policy
        .route_weak_support_floor_permille
        .saturating_sub(if discovery_node_route {
            FIELD_WEAK_SUPPORT_DISCOVERY_RELIEF_PERMILLE
        } else {
            0
        })
}

fn continuity_softened(
    previous_continuity_band: FieldContinuityBand,
    effective_continuity_band: FieldContinuityBand,
) -> bool {
    matches!(
        (previous_continuity_band, effective_continuity_band),
        (
            FieldContinuityBand::Steady,
            FieldContinuityBand::DegradedSteady
        ) | (FieldContinuityBand::Steady, FieldContinuityBand::Bootstrap)
            | (
                FieldContinuityBand::DegradedSteady,
                FieldContinuityBand::Bootstrap
            )
    )
}

fn continuity_softening_reason(blocker: FieldPromotionBlocker) -> FieldPolicyReason {
    match blocker {
        FieldPromotionBlocker::Uncertainty | FieldPromotionBlocker::Freshness => {
            FieldPolicyReason::SoftenedByEntropy
        }
        FieldPromotionBlocker::SupportTrend
        | FieldPromotionBlocker::AntiEntropyConfirmation
        | FieldPromotionBlocker::ContinuationCoherence => FieldPolicyReason::SoftenedBySupport,
    }
}

fn policy_reason_for_promotion_blocker(blocker: FieldPromotionBlocker) -> FieldPolicyReason {
    match blocker {
        FieldPromotionBlocker::SupportTrend => FieldPolicyReason::BlockedBySupportTrend,
        FieldPromotionBlocker::Uncertainty => FieldPolicyReason::BlockedByUncertainty,
        FieldPromotionBlocker::AntiEntropyConfirmation => {
            FieldPolicyReason::BlockedByAntiEntropyConfirmation
        }
        FieldPromotionBlocker::ContinuationCoherence => {
            FieldPolicyReason::BlockedByContinuationCoherence
        }
        FieldPromotionBlocker::Freshness => FieldPolicyReason::BlockedByFreshness,
    }
}

fn route_policy_event(
    route_id: &RouteId,
    destination_key: &crate::state::DestinationKey,
    gate: FieldPolicyGate,
    reason: FieldPolicyReason,
    observed_at_tick: Tick,
) -> FieldPolicyEvent {
    FieldPolicyEvent {
        gate,
        reason,
        destination: Some(DestinationId::from(destination_key)),
        route_id: Some(*route_id),
        observed_at_tick,
    }
}
