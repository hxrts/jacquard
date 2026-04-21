use std::collections::{BTreeSet, VecDeque};

use jacquard_core::{
    DestinationId, MaterializedRoute, NodeId, RouteEpoch, RouteShapeVisibility, Tick,
};
use jacquard_traits::RoutingEngine;

use super::{
    FieldCommitmentReplayEntry, FieldCommitmentReplaySurface, FieldEngine,
    FieldExportedReplayBundle, FieldForwardSummaryObservation, FieldPolicyEvent,
    FieldProtocolReplaySurface, FieldRecoveryReplayEntry, FieldRecoveryReplaySurface,
    FieldReplaySnapshot, FieldReplaySurfaceClass, FieldRouterAnalysisRouteSummary,
    FieldRouterAnalysisSnapshot, FieldRuntimeReplaySurface, FieldRuntimeRoundArtifact,
    FieldRuntimeRouteArtifact, FieldSearchReplaySurface, FIELD_POLICY_EVENT_RETENTION_MAX,
    FIELD_REPLAY_SURFACE_VERSION, FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX,
};
use crate::{
    planner::admission::{bootstrap_class_for_state, continuity_band_for_state},
    search::FieldPlannerSearchRecord,
    state::{
        DestinationFieldState, DestinationInterestClass, DestinationKey, EntropyBucket, HopBand,
        OperatingRegime, RoutingPosture, SupportBucket,
    },
    summary::{
        EvidenceContributionClass, FieldSummary, ForwardPropagatedEvidence,
        ReverseFeedbackEvidence, SummaryDestinationKey, SummaryUncertaintyClass,
        FIELD_SUMMARY_ENCODING_BYTES,
    },
};

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    #[must_use]
    pub(crate) fn effective_search_config(&self) -> crate::FieldSearchConfig {
        let desired_scheduler_profile =
            match (self.state.regime.current, self.state.posture.current) {
                (OperatingRegime::Congested, _)
                | (OperatingRegime::Unstable, RoutingPosture::RiskSuppressed)
                | (_, RoutingPosture::RiskSuppressed) => {
                    telltale_search::SearchSchedulerProfile::ThreadedExactSingleLane
                }
                _ => telltale_search::SearchSchedulerProfile::CanonicalSerial,
            };
        self.search_config
            .clone()
            .with_scheduler_profile(desired_scheduler_profile)
            .unwrap_or_else(|_| self.search_config.clone())
    }

    #[must_use]
    pub fn last_search_record(&self) -> Option<FieldPlannerSearchRecord> {
        self.last_search_record.borrow().clone()
    }

    #[must_use]
    pub fn runtime_round_artifacts(&self) -> Vec<FieldRuntimeRoundArtifact> {
        self.runtime_round_artifacts
            .borrow()
            .iter()
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn policy_events(&self) -> Vec<FieldPolicyEvent> {
        self.policy_events.borrow().iter().cloned().collect()
    }

    #[must_use]
    pub fn route_recovery_entries(&self) -> Vec<FieldRecoveryReplayEntry> {
        self.active_routes
            .iter()
            .map(|(route_id, active)| FieldRecoveryReplayEntry {
                route_id: *route_id,
                state: active.recovery.state.clone(),
            })
            .collect()
    }

    pub fn ingest_forward_summary(
        &mut self,
        from_neighbor: NodeId,
        payload: [u8; FIELD_SUMMARY_ENCODING_BYTES],
        observed_at_tick: Tick,
    ) -> Result<(), &'static str> {
        let summary = FieldSummary::decode(payload)?;
        let destination = DestinationId::from(&DestinationKey::from(&summary.destination));
        let state = self.state.upsert_destination_interest(
            &destination,
            DestinationInterestClass::Propagated,
            observed_at_tick,
        );
        state
            .pending_forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor,
                summary,
                observed_at_tick,
            });
        Ok(())
    }

    // long-block-exception: forward summary recording keeps destination
    // upsert and evidence normalization in one ingestion path.
    pub fn record_forward_summary(
        &mut self,
        destination: &DestinationId,
        from_neighbor: NodeId,
        observation: FieldForwardSummaryObservation,
    ) {
        let service_bias = matches!(destination, DestinationId::Service(_));
        let state = self.state.upsert_destination_interest(
            destination,
            DestinationInterestClass::Propagated,
            observation.observed_at_tick,
        );
        state
            .pending_forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor,
                summary: FieldSummary {
                    destination: SummaryDestinationKey::from(destination),
                    topology_epoch: observation.topology_epoch,
                    freshness_tick: observation.observed_at_tick,
                    hop_band: HopBand::new(observation.min_hops, observation.max_hops),
                    delivery_support: SupportBucket::new(observation.delivery_support),
                    congestion_penalty: EntropyBucket::default(),
                    retention_support: SupportBucket::new(if service_bias {
                        observation.delivery_support.saturating_sub(40)
                    } else {
                        0
                    }),
                    uncertainty_penalty: EntropyBucket::default(),
                    evidence_class: EvidenceContributionClass::ForwardPropagated,
                    uncertainty_class: SummaryUncertaintyClass::Low,
                },
                observed_at_tick: observation.observed_at_tick,
            });
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.posterior.top_corridor_mass = SupportBucket::new(
            state
                .posterior
                .top_corridor_mass
                .value()
                .max(observation.delivery_support.saturating_sub(40)),
        );
        state.corridor_belief.delivery_support = SupportBucket::new(
            state
                .corridor_belief
                .delivery_support
                .value()
                .max(observation.delivery_support.saturating_sub(60)),
        );
        state.corridor_belief.retention_affinity = SupportBucket::new(
            state
                .corridor_belief
                .retention_affinity
                .value()
                .max(observation.delivery_support.saturating_sub(80)),
        );
        state.corridor_belief.expected_hop_band = HopBand::new(
            observation.min_hops.saturating_add(1),
            observation.max_hops.saturating_add(1),
        );
        state.frontier = state
            .frontier
            .clone()
            .insert(crate::state::NeighborContinuation {
                neighbor_id: from_neighbor,
                net_value: SupportBucket::new(observation.delivery_support),
                downstream_support: SupportBucket::new(observation.delivery_support),
                expected_hop_band: HopBand::new(
                    observation.min_hops.saturating_add(1),
                    observation.max_hops.saturating_add(1),
                ),
                freshness: observation.observed_at_tick,
            });
        if service_bias {
            reinforce_service_bootstrap_fanout(state, observation.delivery_support);
        }
    }

    pub fn record_reverse_feedback(
        &mut self,
        destination: &DestinationId,
        from_neighbor: NodeId,
        delivery_feedback: u16,
        observed_at_tick: Tick,
    ) {
        let state = self.state.upsert_destination_interest(
            destination,
            DestinationInterestClass::Transit,
            observed_at_tick,
        );
        state
            .pending_reverse_feedback
            .push(ReverseFeedbackEvidence {
                from_neighbor,
                delivery_feedback: SupportBucket::new(delivery_feedback),
                observed_at_tick,
            });
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ReverseValidated;
        state.posterior.usability_entropy = EntropyBucket::new(
            state
                .posterior
                .usability_entropy
                .value()
                .saturating_sub(120),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(
            state
                .posterior
                .top_corridor_mass
                .value()
                .max(delivery_feedback.saturating_sub(20)),
        );
        state.corridor_belief.delivery_support = SupportBucket::new(
            state
                .corridor_belief
                .delivery_support
                .value()
                .max(delivery_feedback.saturating_sub(40)),
        );
        state.corridor_belief.retention_affinity = SupportBucket::new(
            state
                .corridor_belief
                .retention_affinity
                .value()
                .max(delivery_feedback.saturating_sub(60)),
        );
    }

    #[must_use]
    pub fn protocol_artifacts(&self) -> Vec<crate::choreography::FieldProtocolArtifact> {
        self.protocol_runtime.artifacts()
    }

    #[must_use]
    pub fn replay_snapshot(&self, routes: &[MaterializedRoute]) -> FieldReplaySnapshot
    where
        Self: RoutingEngine,
    {
        FieldReplaySnapshot {
            schema_version: FIELD_REPLAY_SURFACE_VERSION,
            search: FieldSearchReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Observational,
                record: self.last_search_record(),
            },
            protocol: FieldProtocolReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Observational,
                artifacts: self.protocol_artifacts(),
                reconfigurations: self.protocol_runtime.reconfigurations(),
            },
            runtime: FieldRuntimeReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Reduced,
                artifacts: self.runtime_round_artifacts(),
                policy_events: self.policy_events(),
            },
            recovery: FieldRecoveryReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Reduced,
                entries: self.route_recovery_entries(),
            },
            commitments: FieldCommitmentReplaySurface {
                schema_version: FIELD_REPLAY_SURFACE_VERSION,
                surface_class: FieldReplaySurfaceClass::Observational,
                entries: routes
                    .iter()
                    .map(|route| FieldCommitmentReplayEntry {
                        route_id: *route.identity.route_id(),
                        commitments: self.route_commitments(route),
                    })
                    .collect(),
            },
        }
    }

    // long-block-exception: router analysis export intentionally assembles one audited snapshot surface.
    #[must_use]
    pub fn router_analysis_snapshot(
        &self,
        routes: &[MaterializedRoute],
    ) -> FieldRouterAnalysisSnapshot
    where
        Self: RoutingEngine,
    {
        let search = self.last_search_record();
        let current_search = search
            .as_ref()
            .filter(|record| record.observed_at_tick == self.state.last_tick_processed);
        let recovery_entries = self.route_recovery_entries();
        let reconfigurations = self.protocol_runtime.reconfigurations();
        FieldRouterAnalysisSnapshot {
            selected_result_present: current_search
                .is_some_and(|record| record.selected_continuation.is_some()),
            search_reconfiguration_present: current_search.is_some_and(|record| {
                record
                    .run
                    .as_ref()
                    .and_then(|run| run.reconfiguration.clone())
                    .is_some()
            }),
            execution_policy: current_search
                .map(|record| format!("{:?}", record.effective_config.scheduler_profile())),
            bootstrap_active: recovery_entries
                .iter()
                .any(|entry| entry.state.bootstrap_active),
            continuity_band: recovery_entries
                .iter()
                .find_map(|entry| entry.state.continuity_band.map(|band| format!("{band:?}"))),
            last_continuity_transition: recovery_entries.iter().find_map(|entry| {
                entry
                    .state
                    .last_continuity_transition
                    .map(|transition| format!("{transition:?}"))
            }),
            last_promotion_decision: recovery_entries.iter().find_map(|entry| {
                entry
                    .state
                    .last_promotion_decision
                    .map(|decision| format!("{decision:?}"))
            }),
            last_promotion_blocker: recovery_entries.iter().find_map(|entry| {
                entry
                    .state
                    .last_promotion_blocker
                    .map(|blocker| format!("{blocker:?}"))
            }),
            bootstrap_activation_count: recovery_entries
                .iter()
                .map(|entry| entry.state.bootstrap_activation_count)
                .max()
                .unwrap_or(0),
            bootstrap_hold_count: recovery_entries
                .iter()
                .map(|entry| entry.state.bootstrap_hold_count)
                .max()
                .unwrap_or(0),
            bootstrap_narrow_count: recovery_entries
                .iter()
                .map(|entry| entry.state.bootstrap_narrow_count)
                .max()
                .unwrap_or(0),
            bootstrap_upgrade_count: recovery_entries
                .iter()
                .map(|entry| entry.state.bootstrap_upgrade_count)
                .max()
                .unwrap_or(0),
            bootstrap_withdraw_count: recovery_entries
                .iter()
                .map(|entry| entry.state.bootstrap_withdraw_count)
                .max()
                .unwrap_or(0),
            degraded_steady_entry_count: recovery_entries
                .iter()
                .map(|entry| entry.state.degraded_steady_entry_count)
                .max()
                .unwrap_or(0),
            degraded_steady_recovery_count: recovery_entries
                .iter()
                .map(|entry| entry.state.degraded_steady_recovery_count)
                .max()
                .unwrap_or(0),
            degraded_to_bootstrap_count: recovery_entries
                .iter()
                .map(|entry| entry.state.degraded_to_bootstrap_count)
                .max()
                .unwrap_or(0),
            degraded_steady_round_count: recovery_entries
                .iter()
                .map(|entry| entry.state.degraded_steady_round_count)
                .max()
                .unwrap_or(0),
            service_retention_carry_forward_count: recovery_entries
                .iter()
                .map(|entry| entry.state.service_retention_carry_forward_count)
                .max()
                .unwrap_or(0),
            asymmetric_shift_success_count: recovery_entries
                .iter()
                .map(|entry| entry.state.asymmetric_shift_success_count)
                .max()
                .unwrap_or(0),
            protocol_reconfiguration_count: reconfigurations.len(),
            route_bound_reconfiguration_count: reconfigurations
                .iter()
                .filter(|step| {
                    step.prior_session.route_id.is_some() || step.next_session.route_id.is_some()
                })
                .count(),
            continuation_shift_count: recovery_entries
                .iter()
                .map(|entry| entry.state.continuation_shift_count)
                .max()
                .unwrap_or(0),
            corridor_narrow_count: recovery_entries
                .iter()
                .map(|entry| entry.state.corridor_narrow_count)
                .max()
                .unwrap_or(0),
            checkpoint_capture_count: recovery_entries
                .iter()
                .map(|entry| entry.state.checkpoint_capture_count)
                .max()
                .unwrap_or(0),
            checkpoint_restore_count: recovery_entries
                .iter()
                .map(|entry| entry.state.checkpoint_restore_count)
                .max()
                .unwrap_or(0),
            reconfiguration_causes: reconfigurations
                .iter()
                .map(|entry| format!("{:?}", entry.cause))
                .collect(),
            route_summaries: routes
                .iter()
                .map(|route| {
                    let recovery = recovery_entries
                        .iter()
                        .find(|entry| entry.route_id == *route.identity.route_id());
                    FieldRouterAnalysisRouteSummary {
                        route_id: *route.identity.route_id(),
                        continuity_band: recovery.and_then(|entry| {
                            entry.state.continuity_band.map(|band| format!("{band:?}"))
                        }),
                        last_outcome: recovery.and_then(|entry| {
                            entry
                                .state
                                .last_outcome
                                .map(|outcome| format!("{outcome:?}"))
                        }),
                        last_promotion_decision: recovery.and_then(|entry| {
                            entry
                                .state
                                .last_promotion_decision
                                .map(|decision| format!("{decision:?}"))
                        }),
                        last_promotion_blocker: recovery.and_then(|entry| {
                            entry
                                .state
                                .last_promotion_blocker
                                .map(|blocker| format!("{blocker:?}"))
                        }),
                        continuation_shift_count: recovery
                            .map(|entry| entry.state.continuation_shift_count)
                            .unwrap_or(0),
                    }
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn exported_replay_bundle(&self, routes: &[MaterializedRoute]) -> FieldExportedReplayBundle
    where
        Self: RoutingEngine,
    {
        let snapshot = self.replay_snapshot(routes);
        snapshot.exported_bundle()
    }

    pub fn exported_replay_bundle_json(
        &self,
        routes: &[MaterializedRoute],
    ) -> Result<String, serde_json::Error>
    where
        Self: RoutingEngine,
    {
        serde_json::to_string_pretty(&self.exported_replay_bundle(routes))
    }

    pub(crate) fn runtime_route_artifact_for_destination(
        &self,
        destination: &DestinationId,
        destination_state: &DestinationFieldState,
        topology_epoch: RouteEpoch,
    ) -> FieldRuntimeRouteArtifact {
        let route_shape = if destination_state.frontier.as_slice().is_empty()
            || destination_state.corridor_belief.delivery_support.value() == 0
        {
            RouteShapeVisibility::Opaque
        } else {
            RouteShapeVisibility::CorridorEnvelope
        };
        FieldRuntimeRouteArtifact {
            destination: destination.clone(),
            route_shape,
            bootstrap_class: bootstrap_class_for_state(destination_state),
            continuity_band: continuity_band_for_state(destination_state),
            route_support: destination_state.corridor_belief.delivery_support.value(),
            topology_epoch,
        }
    }

    pub(crate) fn record_runtime_round_artifact(&self, artifact: FieldRuntimeRoundArtifact) {
        let mut retained = self.runtime_round_artifacts.borrow_mut();
        retain_bounded(
            &mut retained,
            FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX,
            artifact,
        );
    }

    pub(crate) fn record_policy_event(&self, event: FieldPolicyEvent) {
        let mut retained = self.policy_events.borrow_mut();
        retain_bounded(&mut retained, FIELD_POLICY_EVENT_RETENTION_MAX, event);
    }
}

fn retain_bounded<T>(retained: &mut VecDeque<T>, max_len: usize, item: T) {
    if retained.len() >= max_len {
        retained.pop_front();
    }
    retained.push_back(item);
}

fn reinforce_service_bootstrap_fanout(
    state: &mut crate::state::DestinationFieldState,
    delivery_support: u16,
) {
    let coherent_branch_count = service_bootstrap_branch_count(state);
    if coherent_branch_count < 2 {
        return;
    }
    let corroboration_bonus = u16::try_from(
        coherent_branch_count
            .saturating_sub(1)
            .saturating_mul(70)
            .min(220),
    )
    .expect("bounded corroboration bonus fits u16");
    state.posterior.top_corridor_mass = SupportBucket::new(
        state.posterior.top_corridor_mass.value().max(
            delivery_support
                .saturating_sub(10)
                .saturating_add(corroboration_bonus),
        ),
    );
    state.corridor_belief.delivery_support = SupportBucket::new(
        state.corridor_belief.delivery_support.value().max(
            delivery_support
                .saturating_sub(35)
                .saturating_add(corroboration_bonus / 2),
        ),
    );
    state.corridor_belief.retention_affinity = SupportBucket::new(
        state.corridor_belief.retention_affinity.value().max(
            delivery_support
                .saturating_sub(20)
                .saturating_add(corroboration_bonus),
        ),
    );
}

fn service_bootstrap_branch_count(state: &crate::state::DestinationFieldState) -> usize {
    let mut neighbors = BTreeSet::new();
    for entry in state.frontier.as_slice() {
        if entry.downstream_support.value() >= 140 && entry.net_value.value() >= 180 {
            neighbors.insert(entry.neighbor_id);
        }
    }
    for evidence in &state.pending_forward_evidence {
        if evidence.summary.retention_support.value() >= 140
            && evidence.summary.delivery_support.value() >= 120
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            neighbors.insert(evidence.from_neighbor);
        }
    }
    neighbors.len()
}

#[cfg(test)]
mod tests {
    use jacquard_core::{DestinationId, LinkEndpoint, RouteEpoch, ServiceId, Tick, TransportError};
    use jacquard_traits::{effect_handler, TransportSenderEffects};
    use telltale_search::SearchSchedulerProfile;

    use super::*;
    use crate::{FieldPolicyGate, FieldPolicyReason};

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

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    #[test]
    fn effective_search_config_tracks_posture_without_changing_field_defaults() {
        let mut engine = FieldEngine::new(node(1), NoopTransport, ());
        assert_eq!(
            engine.effective_search_config().scheduler_profile(),
            SearchSchedulerProfile::CanonicalSerial,
        );

        engine.state.posture.current = RoutingPosture::RiskSuppressed;
        let expected = if cfg!(target_arch = "wasm32") {
            SearchSchedulerProfile::CanonicalSerial
        } else {
            SearchSchedulerProfile::ThreadedExactSingleLane
        };
        assert_eq!(
            engine.effective_search_config().scheduler_profile(),
            expected
        );
        assert_eq!(
            engine
                .effective_search_config()
                .per_objective_query_budget(),
            engine.search_config.per_objective_query_budget(),
        );
    }

    #[test]
    fn replay_snapshot_is_versioned_and_surface_typed() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(snapshot.schema_version, FIELD_REPLAY_SURFACE_VERSION);
        assert_eq!(
            snapshot.search.surface_class,
            FieldReplaySurfaceClass::Observational
        );
        assert_eq!(
            snapshot.protocol.surface_class,
            FieldReplaySurfaceClass::Observational
        );
        assert_eq!(
            snapshot.runtime.surface_class,
            FieldReplaySurfaceClass::Reduced
        );
        assert_eq!(
            snapshot.recovery.surface_class,
            FieldReplaySurfaceClass::Reduced
        );
        assert_eq!(
            snapshot.commitments.surface_class,
            FieldReplaySurfaceClass::Observational
        );
    }

    #[test]
    fn replay_snapshot_matches_direct_public_surfaces() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(snapshot.search.record, engine.last_search_record());
        assert_eq!(snapshot.protocol.artifacts, engine.protocol_artifacts());
        assert_eq!(
            snapshot.protocol.reconfigurations,
            engine.protocol_runtime.reconfigurations()
        );
        assert_eq!(snapshot.runtime.artifacts, engine.runtime_round_artifacts());
        assert_eq!(snapshot.runtime.policy_events, engine.policy_events());
        assert_eq!(snapshot.recovery.entries, engine.route_recovery_entries());
        assert!(snapshot.commitments.entries.is_empty());
    }

    #[test]
    fn replay_snapshot_runtime_surface_stays_bounded() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        for index in 0..(FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX + 4) {
            engine.record_runtime_round_artifact(FieldRuntimeRoundArtifact {
                protocol: crate::choreography::FieldProtocolKind::SummaryDissemination,
                destination: None,
                destination_class: None,
                blocked_receive: None,
                disposition: crate::choreography::FieldRoundDisposition::Continue,
                host_wait_status: crate::choreography::FieldHostWaitStatus::Idle,
                emitted_count: index,
                step_budget_remaining: 1,
                execution_policy: crate::choreography::FieldExecutionPolicyClass::Cheap,
                search_snapshot_epoch: None,
                search_selected_result_present: false,
                search_reconfiguration_present: false,
                router_artifact: None,
                observed_at_tick: Tick(u64::try_from(index).expect("test index fits")),
            });
        }

        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(
            snapshot.runtime.artifacts.len(),
            FIELD_RUNTIME_ROUND_ARTIFACT_RETENTION_MAX
        );
    }

    #[test]
    fn replay_snapshot_policy_event_surface_stays_bounded() {
        let engine = FieldEngine::new(node(1), NoopTransport, ());
        for index in 0..(FIELD_POLICY_EVENT_RETENTION_MAX + 4) {
            engine.record_policy_event(FieldPolicyEvent {
                gate: FieldPolicyGate::Promotion,
                reason: FieldPolicyReason::BlockedBySupportTrend,
                destination: None,
                route_id: None,
                observed_at_tick: Tick(u64::try_from(index).expect("test index fits")),
            });
        }

        let snapshot = engine.replay_snapshot(&[]);
        assert_eq!(
            snapshot.runtime.policy_events.len(),
            FIELD_POLICY_EVENT_RETENTION_MAX
        );
    }

    #[test]
    fn record_forward_summary_reinforces_service_fanout_before_refresh() {
        let mut engine = FieldEngine::new(node(1), NoopTransport, ());
        let destination = DestinationId::Service(ServiceId(vec![9; 16]));
        for (neighbor, support) in [(node(2), 910), (node(3), 840), (node(4), 780)] {
            engine.record_forward_summary(
                &destination,
                neighbor,
                FieldForwardSummaryObservation::new(RouteEpoch(1), Tick(1), support, 1, 2),
            );
        }

        let state = engine
            .state
            .destinations
            .get(&crate::state::DestinationKey::Service(vec![9; 16]))
            .expect("tracked service destination");
        assert_eq!(state.frontier.len(), 3);
        assert!(
            state.posterior.top_corridor_mass.value() >= 980,
            "service fanout should corroborate corridor mass early: {}",
            state.posterior.top_corridor_mass.value()
        );
        assert!(
            state.corridor_belief.retention_affinity.value() >= 900,
            "service fanout should seed strong retention before refresh: {}",
            state.corridor_belief.retention_affinity.value()
        );
    }
}
