use super::*;

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    pub(super) fn protocol_neighbor_targets(&self, preferred_neighbor: NodeId) -> Vec<NodeId> {
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

    pub(super) fn dispatch_protocol_sends(
        &mut self,
        sends: &[QueuedProtocolSend],
    ) -> Result<usize, RouteError>
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

    pub(super) fn seed_destinations_from_topology(
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
    pub(super) fn advance_protocol_sessions(
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
                            // allow-ignored-result: anti-entropy progression should continue even if observational transport flushing reports a non-fatal send error.
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
                            // allow-ignored-result: explicit-coordination progression should continue even if observational transport flushing reports a non-fatal send error.
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

    pub(super) fn record_protocol_round(
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
    pub(super) fn refresh_destination_observers(
        &mut self,
        topology: &Configuration,
        now_tick: Tick,
    ) -> bool {
        let topology_epoch = topology.epoch;
        let regime = self.state.regime.current;
        let control_state = self.state.controller.clone();
        let search_config = self.search_config().clone();
        let service_freshness_weight = search_config.service_freshness_weight();
        let local_origin_trace = LocalOriginTrace {
            local_node_id: self.local_node_id,
            topology_epoch,
        };
        let active_routes = self.active_routes.values().cloned().collect::<Vec<_>>();

        let mut changed = false;
        let active_keys = self.state.active_destination_keys();
        for destination_key in active_keys {
            let Some(destination_state) = self.state.destinations.get_mut(&destination_key) else {
                continue;
            };
            let destination = DestinationId::from(&destination_key);
            let destination_active_routes = active_routes
                .iter()
                .filter(|active| active.destination == destination_key)
                .collect::<Vec<_>>();
            let direct_evidence = direct_evidence_for_destination(
                topology,
                self.local_node_id,
                &destination,
                now_tick,
            );
            let mut forward_input = forward_evidence_for_observer(destination_state, now_tick);
            if forward_input.evidence.is_empty() {
                let carry_forward = synthesized_node_forward_evidence_from_active_routes(
                    destination_state,
                    &destination_active_routes,
                    &self.state.neighbor_endpoints,
                    now_tick,
                    &search_config,
                );
                if !carry_forward.is_empty() {
                    forward_input = ForwardEvidenceInput {
                        evidence: carry_forward,
                        synthesized: true,
                        service_carry_forward: false,
                    };
                }
            }
            let forward_evidence = forward_input.evidence.clone();
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
            let _pending_forward_evidence =
                std::mem::take(&mut destination_state.pending_forward_evidence);
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
                    service_freshness_weight,
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
            if forward_input.synthesized && forward_input.service_carry_forward {
                for active in self.active_routes.values_mut() {
                    if active.destination == destination_key {
                        active.recovery.note_service_retention_carry_forward();
                    }
                }
            }
        }
        changed
    }
}
