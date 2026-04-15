use super::*;

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
            let service_bias =
                matches!(active.destination, crate::state::DestinationKey::Service(_));
            let discovery_node_route = !service_bias && self.search_config.node_discovery_enabled();
            let service_corridor_viable = service_bias
                && destination_state.is_some_and(|state| service_corridor_viable(active, state));
            let node_corridor_viable = discovery_node_route
                && destination_state.is_some_and(|state| node_corridor_viable(active, state));
            let commitment_support_floor = match (active.bootstrap_class, active.continuity_band) {
                (_, FieldContinuityBand::DegradedSteady) => {
                    if service_bias || discovery_node_route {
                        140
                    } else {
                        160
                    }
                }
                (FieldBootstrapClass::Bootstrap, _) => {
                    if service_bias || discovery_node_route {
                        120
                    } else {
                        140
                    }
                }
                _ if discovery_node_route => 220,
                _ => 250,
            };
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
        let Some(active_route) = self.active_routes.get(identity.route_id()).cloned() else {
            return Err(RouteRuntimeError::Invalidated.into());
        };
        let destination_key = active_route.destination.clone();
        let Some(destination_state) = self.state.destinations.get(&destination_key).cloned() else {
            return Ok(RouteMaintenanceResult {
                event: RouteLifecycleEvent::Expired,
                outcome: RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability),
            });
        };
        let search_config = self.search_config().clone();
        let service_bias = matches!(destination_key, crate::state::DestinationKey::Service(_));
        let discovery_node_route = !service_bias && search_config.node_discovery_enabled();
        let mut ranked = rank_frontier_by_attractor(
            &destination_state,
            &self.state.mean_field,
            self.state.regime.current,
            self.state.posture.current,
            &self.state.controller,
        );
        merge_pending_forward_continuations(&mut ranked, &destination_state);
        if ranked.is_empty() && discovery_node_route {
            ranked = synthesized_node_carry_forward_ranked(
                &active_route,
                &destination_state,
                &self.state.neighbor_endpoints,
                self.state.last_tick_processed,
                &search_config,
            );
        }
        let Some((ranked_best, _)) = ranked.first() else {
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
        let current_continuity_band =
            continuity_band_for_state_with_config(&destination_state, &search_config);
        let preferred_service_neighbor = if service_bias {
            self.active_routes
                .get(identity.route_id())
                .and_then(|active| {
                    preferred_service_shift_neighbor(active, &ranked, ranked_best, &search_config)
                })
        } else {
            None
        };
        let preferred_node_neighbor = if discovery_node_route {
            self.active_routes
                .get(identity.route_id())
                .and_then(|active| {
                    preferred_node_shift_neighbor(
                        active,
                        &ranked,
                        &destination_state,
                        &self.state.neighbor_endpoints,
                        &search_config,
                    )
                })
        } else {
            None
        };
        let best = preferred_service_neighbor
            .or(preferred_node_neighbor)
            .and_then(|neighbor_id| {
                ranked
                    .iter()
                    .find(|(entry, _)| entry.neighbor_id == neighbor_id)
                    .map(|(entry, _)| entry)
            })
            .unwrap_or(ranked_best);

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
        let previous_continuity_band;
        let promotion_assessment;
        let bootstrap_decision;
        let effective_continuity_band;
        {
            let active = self
                .active_routes
                .get_mut(identity.route_id())
                .expect("active route remains present during maintenance");
            previous_posture = active.witness_detail.posture;
            previous_bootstrap_class = active.bootstrap_class;
            previous_continuity_band = active.continuity_band;
            promotion_assessment = promotion_assessment_for_route(
                active,
                &destination_state,
                best,
                self.state.last_tick_processed,
            );
            let blocker = promotion_assessment.primary_blocker();
            let projected_promotion_window_score = updated_promotion_window_score(
                active.promotion_window_score,
                &promotion_assessment,
                &destination_state,
                service_bias,
            );
            let projected_confirmation_streak = if current_bootstrap_class
                == FieldBootstrapClass::Bootstrap
                && promotion_assessment.anti_entropy_confirmed
                && promotion_assessment.continuation_coherent
            {
                active.bootstrap_confirmation_streak.saturating_add(1)
            } else {
                0
            };
            let mut local_bootstrap_decision =
                if current_bootstrap_class == FieldBootstrapClass::Bootstrap {
                    promotion_assessment.decision_for_bootstrap(
                        &destination_state,
                        projected_confirmation_streak,
                        projected_promotion_window_score,
                        &self.search_config,
                    )
                } else {
                    FieldBootstrapDecision::Hold
                };
            if discovery_node_route
                && previous_continuity_band == FieldContinuityBand::DegradedSteady
                && local_bootstrap_decision == FieldBootstrapDecision::Withdraw
                && node_corridor_viable(active, &destination_state)
            {
                local_bootstrap_decision = FieldBootstrapDecision::Hold;
            }
            bootstrap_decision = local_bootstrap_decision;
            effective_continuity_band = match (
                previous_continuity_band,
                current_continuity_band,
                current_bootstrap_class,
            ) {
                (_, FieldContinuityBand::Steady, _) => FieldContinuityBand::Steady,
                (_, FieldContinuityBand::DegradedSteady, _) => FieldContinuityBand::DegradedSteady,
                (
                    FieldContinuityBand::Steady | FieldContinuityBand::DegradedSteady,
                    FieldContinuityBand::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                ) if promotion_assessment.degraded_but_coherent(&destination_state) => {
                    FieldContinuityBand::DegradedSteady
                }
                (
                    FieldContinuityBand::Bootstrap,
                    FieldContinuityBand::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                ) if discovery_node_route
                    && promotion_assessment.degraded_but_coherent(&destination_state) =>
                {
                    FieldContinuityBand::DegradedSteady
                }
                (
                    FieldContinuityBand::DegradedSteady,
                    FieldContinuityBand::Bootstrap,
                    FieldBootstrapClass::Bootstrap,
                ) if discovery_node_route
                    && bootstrap_decision == FieldBootstrapDecision::Hold
                    && node_corridor_viable(active, &destination_state) =>
                {
                    FieldContinuityBand::DegradedSteady
                }
                _ => FieldContinuityBand::Bootstrap,
            };
            active.corridor_envelope = current_corridor_envelope;
            active.witness_detail = current_witness_detail;
            active.bootstrap_class = match (previous_bootstrap_class, bootstrap_decision) {
                (FieldBootstrapClass::Bootstrap, FieldBootstrapDecision::Promote) => {
                    FieldBootstrapClass::Steady
                }
                (FieldBootstrapClass::Bootstrap, _)
                    if current_bootstrap_class == FieldBootstrapClass::Steady
                        && effective_continuity_band != FieldContinuityBand::Bootstrap =>
                {
                    FieldBootstrapClass::Steady
                }
                (FieldBootstrapClass::Bootstrap, _) => FieldBootstrapClass::Bootstrap,
                (_, _) => current_bootstrap_class,
            };
            active.witness_detail.bootstrap_class = active.bootstrap_class;
            active.continuity_band = effective_continuity_band;
            active.witness_detail.continuity_band = effective_continuity_band;
            active.promotion_window_score = projected_promotion_window_score;

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
                    active.promotion_window_score = active.promotion_window_score.saturating_sub(1);
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

            if !active.continuation_neighbors.contains(&best.neighbor_id) {
                let selected_entry = ranked
                    .iter()
                    .find(|(entry, _)| entry.neighbor_id == active.selected_neighbor)
                    .map(|(entry, _)| entry);
                let degraded_continuity = matches!(
                    active.continuity_band,
                    FieldContinuityBand::DegradedSteady | FieldContinuityBand::Bootstrap
                ) && promotion_assessment
                    .degraded_but_coherent(&destination_state);
                let support_delta = selected_entry
                    .map(|entry| best.net_value.value().abs_diff(entry.net_value.value()))
                    .unwrap_or(0);
                let shift_delta_max = if service_bias {
                    FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX.saturating_add(240)
                } else if discovery_node_route
                    && active.continuity_band == FieldContinuityBand::DegradedSteady
                {
                    FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX.saturating_add(180)
                } else if active.continuity_band == FieldContinuityBand::DegradedSteady {
                    FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX.saturating_add(120)
                } else {
                    FIELD_ENVELOPE_SHIFT_SUPPORT_DELTA_MAX
                };
                let support_floor =
                    if active.continuity_band == FieldContinuityBand::DegradedSteady {
                        FIELD_DEGRADED_STEADY_FAILURE_SUPPORT_FLOOR
                    } else {
                        FIELD_ROUTE_WEAK_SUPPORT_FLOOR
                    }
                    .saturating_sub(if service_bias || discovery_node_route {
                        40
                    } else {
                        0
                    });
                if corridor_support >= support_floor && support_delta <= shift_delta_max {
                    active.continuation_neighbors = if service_bias {
                        service_runtime_continuation_neighbors(
                            &ranked,
                            &destination_state,
                            best.neighbor_id,
                            &search_config,
                        )
                    } else if discovery_node_route {
                        node_runtime_continuation_neighbors(
                            &ranked,
                            &destination_state,
                            best.neighbor_id,
                            &search_config,
                        )
                    } else {
                        ranked
                            .iter()
                            .take(crate::state::MAX_CONTINUATION_NEIGHBOR_COUNT + 1)
                            .map(|(entry, _)| entry.neighbor_id)
                            .collect()
                    };
                    active.selected_neighbor = best.neighbor_id;
                    pending_coordination_shift = Some(best.neighbor_id);
                    if service_bias {
                        active.recovery.note_service_retention_carry_forward();
                    }
                    if degraded_continuity {
                        active.recovery.note_asymmetric_shift_success();
                    }
                } else {
                    if degraded_continuity || discovery_node_route {
                        active.continuation_neighbors = if discovery_node_route {
                            node_runtime_continuation_neighbors(
                                &ranked,
                                &destination_state,
                                active.selected_neighbor,
                                &search_config,
                            )
                        } else {
                            ranked
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
                        return Ok(RouteMaintenanceResult {
                            event: RouteLifecycleEvent::Replaced,
                            outcome: RouteMaintenanceOutcome::ReplacementRequired { trigger },
                        });
                    }
                }
            }

            if active.selected_neighbor != best.neighbor_id {
                if active.continuity_band == FieldContinuityBand::DegradedSteady {
                    active.recovery.note_asymmetric_shift_success();
                }
                if service_bias {
                    active.continuation_neighbors = service_runtime_continuation_neighbors(
                        &ranked,
                        &destination_state,
                        best.neighbor_id,
                        &search_config,
                    );
                    active.recovery.note_service_retention_carry_forward();
                } else if discovery_node_route {
                    active.continuation_neighbors = node_runtime_continuation_neighbors(
                        &ranked,
                        &destination_state,
                        best.neighbor_id,
                        &search_config,
                    );
                }
                active.selected_neighbor = best.neighbor_id;
                pending_coordination_shift = Some(best.neighbor_id);
            }
        }

        let degraded_continuity = matches!(
            effective_continuity_band,
            FieldContinuityBand::DegradedSteady | FieldContinuityBand::Bootstrap
        ) && promotion_assessment
            .degraded_but_coherent(&destination_state);
        let post_shift_grace = self
            .active_routes
            .get(identity.route_id())
            .is_some_and(|active| continuation_shift_grace_active(active, &promotion_assessment));
        let failure_support_floor =
            if current_bootstrap_class == FieldBootstrapClass::Bootstrap && degraded_continuity {
                if service_bias {
                    FIELD_BOOTSTRAP_FAILURE_SUPPORT_FLOOR.saturating_sub(20)
                } else {
                    FIELD_BOOTSTRAP_FAILURE_SUPPORT_FLOOR
                }
            } else if effective_continuity_band == FieldContinuityBand::DegradedSteady {
                if service_bias {
                    FIELD_DEGRADED_STEADY_FAILURE_SUPPORT_FLOOR.saturating_sub(20)
                } else {
                    FIELD_DEGRADED_STEADY_FAILURE_SUPPORT_FLOOR
                }
            } else {
                FIELD_ROUTE_FAILURE_SUPPORT_FLOOR
            }
            .saturating_sub(if post_shift_grace {
                30
            } else if discovery_node_route {
                40
            } else {
                0
            });

        if corridor_support < failure_support_floor {
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
            > if current_bootstrap_class == FieldBootstrapClass::Bootstrap && degraded_continuity {
                if service_bias {
                    FIELD_BOOTSTRAP_STALE_TICKS_MAX.saturating_add(2)
                } else {
                    FIELD_BOOTSTRAP_STALE_TICKS_MAX
                }
            } else if effective_continuity_band == FieldContinuityBand::DegradedSteady {
                if service_bias {
                    FIELD_DEGRADED_STEADY_STALE_TICKS_MAX.saturating_add(2)
                } else if discovery_node_route {
                    FIELD_DEGRADED_STEADY_STALE_TICKS_MAX.saturating_add(5)
                } else {
                    FIELD_DEGRADED_STEADY_STALE_TICKS_MAX
                }
            } else {
                if discovery_node_route {
                    8
                } else {
                    4
                }
            }
            .saturating_add(if post_shift_grace { 2 } else { 0 });
        if is_stale {
            if corridor_support
                < if discovery_node_route {
                    FIELD_ROUTE_WEAK_SUPPORT_FLOOR.saturating_sub(50)
                } else {
                    FIELD_ROUTE_WEAK_SUPPORT_FLOOR
                }
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
                    maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                        trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                        retained_object_count: jacquard_core::HoldItemCount(1),
                    };
                } else if post_shift_grace {
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
                maintenance_outcome = RouteMaintenanceOutcome::HoldFallback {
                    trigger: RouteMaintenanceTrigger::AntiEntropyRequired,
                    retained_object_count: jacquard_core::HoldItemCount(1),
                };
            }
        }

        Ok(RouteMaintenanceResult {
            event: RouteLifecycleEvent::Activated,
            outcome: if corridor_support
                < if discovery_node_route {
                    FIELD_ROUTE_WEAK_SUPPORT_FLOOR.saturating_sub(50)
                } else {
                    FIELD_ROUTE_WEAK_SUPPORT_FLOOR
                } {
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
