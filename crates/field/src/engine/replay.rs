//! Replay, reduced, and exported field-engine surfaces.

use jacquard_core::{
    DestinationId, NodeId, RouteCommitment, RouteEpoch, RouteId, RouteShapeVisibility, Tick,
};
use serde::{Deserialize, Serialize};
use telltale_search::{SearchEffortProfile, SearchQuery, SearchSchedulerProfile};

use crate::{
    choreography::{
        BlockedReceiveMarker, FieldExecutionPolicyClass, FieldHostWaitStatus, FieldProtocolKind,
        FieldProtocolReconfiguration, FieldProtocolReconfigurationCause, FieldRoundDisposition,
    },
    recovery::FieldRouteRecoveryState,
    route::{FieldBootstrapClass, FieldContinuityBand},
    search::{
        FieldPlannerSearchRecord, FieldSearchConfig, FieldSearchEpoch, FieldSearchPlanningFailure,
        FieldSearchReconfiguration,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReplaySurfaceClass {
    Semantic,
    Reduced,
    Observational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSearchReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub record: Option<FieldPlannerSearchRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProtocolReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub artifacts: Vec<crate::choreography::FieldProtocolArtifact>,
    pub reconfigurations: Vec<crate::choreography::FieldProtocolReconfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub artifacts: Vec<FieldRuntimeRoundArtifact>,
    pub policy_events: Vec<FieldPolicyEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRecoveryReplayEntry {
    pub route_id: RouteId,
    pub state: FieldRouteRecoveryState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRecoveryReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub entries: Vec<FieldRecoveryReplayEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldCommitmentReplayEntry {
    pub route_id: RouteId,
    pub commitments: Vec<RouteCommitment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldCommitmentReplaySurface {
    pub schema_version: u16,
    pub surface_class: FieldReplaySurfaceClass,
    pub entries: Vec<FieldCommitmentReplayEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedReplayBundle {
    pub schema_version: u16,
    pub runtime_search: FieldExportedRuntimeSearchReplay,
    pub protocol: FieldExportedProtocolReplay,
    pub recovery: FieldExportedRecoveryReplay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRuntimeSearchReplay {
    pub schema_version: u16,
    pub search: Option<FieldExportedSearchProjection>,
    pub runtime_artifacts: Vec<FieldExportedRuntimeRoundArtifact>,
    pub policy_events: Vec<FieldExportedPolicyEvent>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchProjection {
    pub objective_class: String,
    pub query: Option<FieldExportedSearchQuery>,
    pub execution_policy: FieldExportedSearchExecutionPolicy,
    pub selected_result: Option<FieldExportedSelectedResult>,
    pub snapshot_epoch: Option<FieldExportedSearchEpoch>,
    pub reconfiguration: Option<FieldExportedSearchReconfiguration>,
    pub planning_failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchQuery {
    pub start: NodeId,
    pub kind: String,
    pub accepted_goals: Vec<NodeId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchExecutionPolicy {
    pub scheduler_profile: String,
    pub batch_width: u64,
    pub exact: bool,
    pub run_to_completion: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSelectedResult {
    pub witness: Vec<NodeId>,
    pub selected_neighbor: Option<NodeId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchEpoch {
    pub route_epoch: u64,
    pub snapshot_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedSearchReconfiguration {
    pub from: FieldExportedSearchEpoch,
    pub to: FieldExportedSearchEpoch,
    pub reseeding_policy: String,
    pub transition_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRuntimeRoundArtifact {
    pub protocol: String,
    pub destination: Option<DestinationId>,
    pub destination_class: Option<String>,
    pub blocked_receive: Option<String>,
    pub disposition: String,
    pub host_wait_status: String,
    pub emitted_count: usize,
    pub step_budget_remaining: u8,
    pub execution_policy: String,
    pub search_snapshot_epoch: Option<FieldExportedSearchEpoch>,
    pub search_selected_result_present: bool,
    pub search_reconfiguration_present: bool,
    pub router_artifact: Option<FieldExportedRuntimeRouteArtifact>,
    pub observed_at_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRuntimeRouteArtifact {
    pub destination: DestinationId,
    pub route_shape: String,
    pub bootstrap_class: String,
    pub continuity_band: String,
    pub route_support: u16,
    pub topology_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedPolicyEvent {
    pub gate: String,
    pub reason: String,
    pub destination: Option<DestinationId>,
    pub route_id: Option<RouteId>,
    pub observed_at_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedProtocolReplay {
    pub schema_version: u16,
    pub artifacts: Vec<FieldExportedProtocolArtifact>,
    pub reconfigurations: Vec<FieldExportedProtocolReconfiguration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedProtocolArtifact {
    pub protocol: String,
    pub route_id: Option<RouteId>,
    pub topology_epoch: u64,
    pub destination: Option<DestinationId>,
    pub detail: String,
    pub last_updated_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedProtocolReconfiguration {
    pub protocol: String,
    pub route_id: Option<RouteId>,
    pub destination: Option<DestinationId>,
    pub prior_owner_tag: u64,
    pub next_owner_tag: u64,
    pub prior_generation: u32,
    pub next_generation: u32,
    pub cause: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRecoveryReplay {
    pub schema_version: u16,
    pub entries: Vec<FieldExportedRecoveryEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldExportedRecoveryEntry {
    pub route_id: RouteId,
    pub checkpoint_available: bool,
    pub last_trigger: Option<String>,
    pub last_outcome: Option<String>,
    pub bootstrap_active: bool,
    pub continuity_band: Option<String>,
    pub last_continuity_transition: Option<String>,
    pub last_bootstrap_transition: Option<String>,
    pub last_promotion_decision: Option<String>,
    pub last_promotion_blocker: Option<String>,
    pub bootstrap_activation_count: u32,
    pub bootstrap_hold_count: u32,
    pub bootstrap_narrow_count: u32,
    pub bootstrap_upgrade_count: u32,
    pub bootstrap_withdraw_count: u32,
    pub degraded_steady_entry_count: u32,
    pub degraded_steady_recovery_count: u32,
    pub degraded_to_bootstrap_count: u32,
    pub degraded_steady_round_count: u32,
    pub service_retention_carry_forward_count: u32,
    pub asymmetric_shift_success_count: u32,
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub continuation_shift_count: u32,
    pub corridor_narrow_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanReplayFixture {
    pub scenario: String,
    pub search: Option<FieldLeanSearchFixture>,
    pub protocol: FieldLeanProtocolFixture,
    pub runtime: FieldLeanRuntimeLinkageFixture,
    pub recovery: Option<FieldLeanRecoveryFixture>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanSearchFixture {
    pub objective_class: String,
    pub query_kind: String,
    pub selected_neighbor_present: bool,
    pub snapshot_epoch_present: bool,
    pub planning_failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanProtocolFixture {
    pub reconfiguration_causes: Vec<String>,
    pub route_bound_reconfiguration_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanRuntimeLinkageFixture {
    pub artifact_count: usize,
    pub search_linked_artifact_count: usize,
    pub route_artifact_count: usize,
    pub bootstrap_route_artifact_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldLeanRecoveryFixture {
    pub last_trigger: Option<String>,
    pub last_outcome: Option<String>,
    pub bootstrap_active: bool,
    pub continuity_band: Option<String>,
    pub last_continuity_transition: Option<String>,
    pub last_bootstrap_transition: Option<String>,
    pub last_promotion_decision: Option<String>,
    pub last_promotion_blocker: Option<String>,
    pub bootstrap_activation_count: u32,
    pub bootstrap_hold_count: u32,
    pub bootstrap_narrow_count: u32,
    pub bootstrap_upgrade_count: u32,
    pub bootstrap_withdraw_count: u32,
    pub degraded_steady_entry_count: u32,
    pub degraded_steady_recovery_count: u32,
    pub degraded_to_bootstrap_count: u32,
    pub degraded_steady_round_count: u32,
    pub service_retention_carry_forward_count: u32,
    pub asymmetric_shift_success_count: u32,
    pub checkpoint_capture_count: u32,
    pub checkpoint_restore_count: u32,
    pub continuation_shift_count: u32,
    pub corridor_narrow_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReplaySnapshot {
    pub schema_version: u16,
    pub search: FieldSearchReplaySurface,
    pub protocol: FieldProtocolReplaySurface,
    pub runtime: FieldRuntimeReplaySurface,
    pub recovery: FieldRecoveryReplaySurface,
    pub commitments: FieldCommitmentReplaySurface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReducedObjectiveClass {
    Node,
    Gateway,
    Service,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldReducedQueryKind {
    SingleGoal,
    MultiGoal,
    CandidateSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedSearchQuery {
    pub start: NodeId,
    pub kind: FieldReducedQueryKind,
    pub accepted_goals: Vec<NodeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldReducedSearchExecutionPolicy {
    pub scheduler_profile: SearchSchedulerProfile,
    pub batch_width: u64,
    pub exact: bool,
    pub run_to_completion: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedSelectedResult {
    pub witness: Vec<NodeId>,
    pub selected_neighbor: Option<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedSearchProjection {
    pub objective_class: FieldReducedObjectiveClass,
    pub query: Option<FieldReducedSearchQuery>,
    pub execution_policy: FieldReducedSearchExecutionPolicy,
    pub selected_result: Option<FieldReducedSelectedResult>,
    pub snapshot_epoch: Option<FieldSearchEpoch>,
    pub reconfiguration: Option<FieldSearchReconfiguration>,
    pub planning_failure: Option<FieldSearchPlanningFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedRuntimeSearchReplay {
    pub schema_version: u16,
    pub search: Option<FieldReducedSearchProjection>,
    pub runtime_artifacts: Vec<FieldRuntimeRoundArtifact>,
    pub policy_events: Vec<FieldPolicyEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolSession {
    pub protocol: FieldProtocolKind,
    pub route_id: Option<RouteId>,
    pub topology_epoch: RouteEpoch,
    pub destination: Option<DestinationId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolArtifact {
    pub session: FieldReducedProtocolSession,
    pub detail: String,
    pub last_updated_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolReconfiguration {
    pub prior_session: FieldReducedProtocolSession,
    pub next_session: FieldReducedProtocolSession,
    pub prior_owner_tag: u64,
    pub next_owner_tag: u64,
    pub prior_generation: u32,
    pub next_generation: u32,
    pub cause: FieldProtocolReconfigurationCause,
    pub recorded_at: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReducedProtocolReplay {
    pub schema_version: u16,
    pub artifacts: Vec<FieldReducedProtocolArtifact>,
    pub reconfigurations: Vec<FieldReducedProtocolReconfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeRouteArtifact {
    pub destination: DestinationId,
    pub route_shape: RouteShapeVisibility,
    pub bootstrap_class: FieldBootstrapClass,
    pub continuity_band: FieldContinuityBand,
    pub route_support: u16,
    pub topology_epoch: RouteEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRuntimeRoundArtifact {
    pub protocol: FieldProtocolKind,
    pub destination: Option<DestinationId>,
    pub destination_class: Option<FieldReducedObjectiveClass>,
    pub blocked_receive: Option<BlockedReceiveMarker>,
    pub disposition: FieldRoundDisposition,
    pub host_wait_status: FieldHostWaitStatus,
    pub emitted_count: usize,
    pub step_budget_remaining: u8,
    pub execution_policy: FieldExecutionPolicyClass,
    pub search_snapshot_epoch: Option<FieldSearchEpoch>,
    pub search_selected_result_present: bool,
    pub search_reconfiguration_present: bool,
    pub router_artifact: Option<FieldRuntimeRouteArtifact>,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldPolicyGate {
    Posture,
    Promotion,
    Continuity,
    CarryForward,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldPolicyReason {
    BlockedByDwell,
    BlockedBySupportTrend,
    BlockedByUncertainty,
    BlockedByAntiEntropyConfirmation,
    BlockedByContinuationCoherence,
    BlockedByFreshness,
    SoftenedBySupport,
    SoftenedByEntropy,
    EmittedByContinuityGate,
    EmittedByEvidenceGate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldPolicyEvent {
    pub gate: FieldPolicyGate,
    pub reason: FieldPolicyReason,
    pub destination: Option<DestinationId>,
    pub route_id: Option<RouteId>,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldForwardSummaryObservation {
    pub topology_epoch: RouteEpoch,
    pub observed_at_tick: Tick,
    pub delivery_support: u16,
    pub min_hops: u8,
    pub max_hops: u8,
}

impl FieldForwardSummaryObservation {
    #[must_use]
    pub fn new(
        topology_epoch: RouteEpoch,
        observed_at_tick: Tick,
        delivery_support: u16,
        min_hops: u8,
        max_hops: u8,
    ) -> Self {
        Self {
            topology_epoch,
            observed_at_tick,
            delivery_support,
            min_hops,
            max_hops,
        }
    }
}

fn reduced_objective_class(
    objective: &jacquard_core::RoutingObjective,
) -> FieldReducedObjectiveClass {
    reduced_objective_class_for_destination(&objective.destination)
}

fn reduced_objective_class_for_destination(
    destination: &DestinationId,
) -> FieldReducedObjectiveClass {
    match destination {
        DestinationId::Node(_) => FieldReducedObjectiveClass::Node,
        DestinationId::Gateway(_) => FieldReducedObjectiveClass::Gateway,
        DestinationId::Service(_) => FieldReducedObjectiveClass::Service,
    }
}

fn reduced_query(query: &SearchQuery<NodeId>) -> FieldReducedSearchQuery {
    match query {
        SearchQuery::SingleGoal { start, goal } => FieldReducedSearchQuery {
            start: *start,
            kind: FieldReducedQueryKind::SingleGoal,
            accepted_goals: vec![*goal],
        },
        SearchQuery::MultiGoal { start, goals } => FieldReducedSearchQuery {
            start: *start,
            kind: FieldReducedQueryKind::MultiGoal,
            accepted_goals: goals.clone(),
        },
        SearchQuery::CandidateSet {
            start, candidates, ..
        } => FieldReducedSearchQuery {
            start: *start,
            kind: FieldReducedQueryKind::CandidateSet,
            accepted_goals: candidates.clone(),
        },
    }
}

fn reduced_execution_policy(config: &FieldSearchConfig) -> FieldReducedSearchExecutionPolicy {
    let policy = config.execution_policy();
    FieldReducedSearchExecutionPolicy {
        scheduler_profile: policy.scheduler_profile,
        batch_width: policy.batch_width,
        exact: matches!(
            (policy.scheduler_profile, policy.effort_profile),
            (
                SearchSchedulerProfile::CanonicalSerial
                    | SearchSchedulerProfile::ThreadedExactSingleLane
                    | SearchSchedulerProfile::BatchedParallelExact,
                SearchEffortProfile::RunToCompletion
            )
        ),
        run_to_completion: matches!(policy.effort_profile, SearchEffortProfile::RunToCompletion),
    }
}

fn reduced_selected_result(
    record: &FieldPlannerSearchRecord,
) -> Option<FieldReducedSelectedResult> {
    let witness = record
        .selected_continuation
        .as_ref()
        .map(|continuation| continuation.selected_private_witness.clone())
        .or_else(|| {
            record
                .run
                .as_ref()
                .and_then(|run| run.selected_node_path.clone())
        })
        .or_else(|| {
            record
                .run
                .as_ref()
                .and_then(|run| run.report.observation.selected_result_witness.clone())
        })?;
    let selected_neighbor = record
        .selected_continuation
        .as_ref()
        .map(|continuation| continuation.chosen_neighbor)
        .or_else(|| witness.get(1).copied());
    Some(FieldReducedSelectedResult {
        witness,
        selected_neighbor,
    })
}

fn reduced_search_projection(record: &FieldPlannerSearchRecord) -> FieldReducedSearchProjection {
    FieldReducedSearchProjection {
        objective_class: reduced_objective_class(&record.objective),
        query: record.query.as_ref().map(reduced_query),
        execution_policy: reduced_execution_policy(&record.effective_config),
        selected_result: reduced_selected_result(record),
        snapshot_epoch: record
            .run
            .as_ref()
            .map(|run| run.report.final_state.epoch.clone()),
        reconfiguration: record
            .run
            .as_ref()
            .and_then(|run| run.reconfiguration.clone()),
        planning_failure: record.planning_failure,
    }
}

fn reduced_protocol_session(
    session: &crate::choreography::FieldProtocolSessionKey,
) -> FieldReducedProtocolSession {
    FieldReducedProtocolSession {
        protocol: session.protocol(),
        route_id: session.route_id(),
        topology_epoch: session.topology_epoch(),
        destination: session.destination(),
    }
}

fn reduced_protocol_artifact(
    artifact: &crate::choreography::FieldProtocolArtifact,
) -> FieldReducedProtocolArtifact {
    FieldReducedProtocolArtifact {
        session: reduced_protocol_session(artifact.session()),
        detail: artifact.detail.as_str().to_owned(),
        last_updated_at: artifact.last_updated_at,
    }
}

fn reduced_protocol_reconfiguration(
    reconfiguration: &FieldProtocolReconfiguration,
) -> FieldReducedProtocolReconfiguration {
    FieldReducedProtocolReconfiguration {
        prior_session: reduced_protocol_session(&reconfiguration.prior_session),
        next_session: reduced_protocol_session(&reconfiguration.next_session),
        prior_owner_tag: reconfiguration.prior_owner_tag,
        next_owner_tag: reconfiguration.next_owner_tag,
        prior_generation: reconfiguration.prior_generation,
        next_generation: reconfiguration.next_generation,
        cause: reconfiguration.cause,
        recorded_at: reconfiguration.recorded_at,
    }
}

impl FieldReplaySnapshot {
    #[must_use]
    pub fn reduced_runtime_search_replay(&self) -> FieldReducedRuntimeSearchReplay {
        FieldReducedRuntimeSearchReplay {
            schema_version: self.schema_version,
            search: self.search.record.as_ref().map(reduced_search_projection),
            runtime_artifacts: self.runtime.artifacts.clone(),
            policy_events: self.runtime.policy_events.clone(),
        }
    }

    #[must_use]
    pub fn reduced_protocol_replay(&self) -> FieldReducedProtocolReplay {
        FieldReducedProtocolReplay {
            schema_version: self.schema_version,
            artifacts: self
                .protocol
                .artifacts
                .iter()
                .map(reduced_protocol_artifact)
                .collect(),
            reconfigurations: self
                .protocol
                .reconfigurations
                .iter()
                .map(reduced_protocol_reconfiguration)
                .collect(),
        }
    }

    #[must_use]
    pub fn exported_bundle(&self) -> FieldExportedReplayBundle {
        FieldExportedReplayBundle {
            schema_version: self.schema_version,
            runtime_search: exported_runtime_search_replay(&self.reduced_runtime_search_replay()),
            protocol: exported_protocol_replay(&self.reduced_protocol_replay()),
            recovery: exported_recovery_replay(&self.recovery),
        }
    }

    pub fn exported_bundle_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.exported_bundle())
    }
}

impl FieldExportedReplayBundle {
    // long-block-exception: this helper intentionally projects the full reduced replay bundle into one proof-facing fixture object in one auditable place.
    #[must_use]
    pub fn lean_replay_fixture(&self, scenario: impl Into<String>) -> FieldLeanReplayFixture {
        let recovery = self
            .recovery
            .entries
            .first()
            .map(|entry| FieldLeanRecoveryFixture {
                last_trigger: entry.last_trigger.clone(),
                last_outcome: entry.last_outcome.clone(),
                bootstrap_active: entry.bootstrap_active,
                continuity_band: entry.continuity_band.clone(),
                last_continuity_transition: entry.last_continuity_transition.clone(),
                last_bootstrap_transition: entry.last_bootstrap_transition.clone(),
                last_promotion_decision: entry.last_promotion_decision.clone(),
                last_promotion_blocker: entry.last_promotion_blocker.clone(),
                bootstrap_activation_count: entry.bootstrap_activation_count,
                bootstrap_hold_count: entry.bootstrap_hold_count,
                bootstrap_narrow_count: entry.bootstrap_narrow_count,
                bootstrap_upgrade_count: entry.bootstrap_upgrade_count,
                bootstrap_withdraw_count: entry.bootstrap_withdraw_count,
                degraded_steady_entry_count: entry.degraded_steady_entry_count,
                degraded_steady_recovery_count: entry.degraded_steady_recovery_count,
                degraded_to_bootstrap_count: entry.degraded_to_bootstrap_count,
                degraded_steady_round_count: entry.degraded_steady_round_count,
                service_retention_carry_forward_count: entry.service_retention_carry_forward_count,
                asymmetric_shift_success_count: entry.asymmetric_shift_success_count,
                checkpoint_capture_count: entry.checkpoint_capture_count,
                checkpoint_restore_count: entry.checkpoint_restore_count,
                continuation_shift_count: entry.continuation_shift_count,
                corridor_narrow_count: entry.corridor_narrow_count,
            });
        FieldLeanReplayFixture {
            scenario: scenario.into(),
            search: self
                .runtime_search
                .search
                .as_ref()
                .map(|search| FieldLeanSearchFixture {
                    objective_class: search.objective_class.clone(),
                    query_kind: search
                        .query
                        .as_ref()
                        .map_or_else(|| "None".to_string(), |query| query.kind.clone()),
                    selected_neighbor_present: search
                        .selected_result
                        .as_ref()
                        .is_some_and(|selected| selected.selected_neighbor.is_some()),
                    snapshot_epoch_present: search.snapshot_epoch.is_some(),
                    planning_failure: search.planning_failure.clone(),
                }),
            protocol: FieldLeanProtocolFixture {
                reconfiguration_causes: self
                    .protocol
                    .reconfigurations
                    .iter()
                    .map(|step| step.cause.clone())
                    .collect(),
                route_bound_reconfiguration_count: self
                    .protocol
                    .reconfigurations
                    .iter()
                    .filter(|step| step.route_id.is_some())
                    .count(),
            },
            runtime: FieldLeanRuntimeLinkageFixture {
                artifact_count: self.runtime_search.runtime_artifacts.len(),
                search_linked_artifact_count: self
                    .runtime_search
                    .runtime_artifacts
                    .iter()
                    .filter(|artifact| artifact.search_snapshot_epoch.is_some())
                    .count(),
                route_artifact_count: self
                    .runtime_search
                    .runtime_artifacts
                    .iter()
                    .filter(|artifact| artifact.router_artifact.is_some())
                    .count(),
                bootstrap_route_artifact_count: self
                    .runtime_search
                    .runtime_artifacts
                    .iter()
                    .filter(|artifact| {
                        artifact
                            .router_artifact
                            .as_ref()
                            .is_some_and(|route| route.bootstrap_class == "Bootstrap")
                    })
                    .count(),
            },
            recovery,
        }
    }
}

fn exported_runtime_search_replay(
    replay: &FieldReducedRuntimeSearchReplay,
) -> FieldExportedRuntimeSearchReplay {
    FieldExportedRuntimeSearchReplay {
        schema_version: replay.schema_version,
        search: replay.search.as_ref().map(exported_search_projection),
        runtime_artifacts: replay
            .runtime_artifacts
            .iter()
            .map(exported_runtime_round_artifact)
            .collect(),
        policy_events: replay
            .policy_events
            .iter()
            .map(exported_policy_event)
            .collect(),
    }
}

fn exported_search_projection(
    projection: &FieldReducedSearchProjection,
) -> FieldExportedSearchProjection {
    FieldExportedSearchProjection {
        objective_class: format!("{:?}", projection.objective_class),
        query: projection.query.as_ref().map(exported_search_query),
        execution_policy: FieldExportedSearchExecutionPolicy {
            scheduler_profile: format!("{:?}", projection.execution_policy.scheduler_profile),
            batch_width: projection.execution_policy.batch_width,
            exact: projection.execution_policy.exact,
            run_to_completion: projection.execution_policy.run_to_completion,
        },
        selected_result: projection.selected_result.as_ref().map(|selected| {
            FieldExportedSelectedResult {
                witness: selected.witness.clone(),
                selected_neighbor: selected.selected_neighbor,
            }
        }),
        snapshot_epoch: projection
            .snapshot_epoch
            .as_ref()
            .map(exported_search_epoch),
        reconfiguration: projection
            .reconfiguration
            .as_ref()
            .map(exported_search_reconfiguration),
        planning_failure: projection
            .planning_failure
            .map(|failure| format!("{failure:?}")),
    }
}

fn exported_search_query(query: &FieldReducedSearchQuery) -> FieldExportedSearchQuery {
    FieldExportedSearchQuery {
        start: query.start,
        kind: format!("{:?}", query.kind),
        accepted_goals: query.accepted_goals.clone(),
    }
}

fn exported_search_epoch(epoch: &FieldSearchEpoch) -> FieldExportedSearchEpoch {
    FieldExportedSearchEpoch {
        route_epoch: epoch.route_epoch.0,
        snapshot_id: format!("{:?}", epoch.snapshot_id.0),
    }
}

fn exported_search_reconfiguration(
    reconfiguration: &FieldSearchReconfiguration,
) -> FieldExportedSearchReconfiguration {
    FieldExportedSearchReconfiguration {
        from: exported_search_epoch(&reconfiguration.from),
        to: exported_search_epoch(&reconfiguration.to),
        reseeding_policy: format!("{:?}", reconfiguration.reseeding_policy),
        transition_class: format!("{:?}", reconfiguration.transition_class),
    }
}

fn exported_runtime_round_artifact(
    artifact: &FieldRuntimeRoundArtifact,
) -> FieldExportedRuntimeRoundArtifact {
    FieldExportedRuntimeRoundArtifact {
        protocol: format!("{:?}", artifact.protocol),
        destination: artifact.destination.clone(),
        destination_class: artifact.destination_class.map(|class| format!("{class:?}")),
        blocked_receive: artifact.blocked_receive.map(|marker| format!("{marker:?}")),
        disposition: format!("{:?}", artifact.disposition),
        host_wait_status: format!("{:?}", artifact.host_wait_status),
        emitted_count: artifact.emitted_count,
        step_budget_remaining: artifact.step_budget_remaining,
        execution_policy: format!("{:?}", artifact.execution_policy),
        search_snapshot_epoch: artifact
            .search_snapshot_epoch
            .as_ref()
            .map(exported_search_epoch),
        search_selected_result_present: artifact.search_selected_result_present,
        search_reconfiguration_present: artifact.search_reconfiguration_present,
        router_artifact: artifact.router_artifact.as_ref().map(|route| {
            FieldExportedRuntimeRouteArtifact {
                destination: route.destination.clone(),
                route_shape: format!("{:?}", route.route_shape),
                bootstrap_class: format!("{:?}", route.bootstrap_class),
                continuity_band: format!("{:?}", route.continuity_band),
                route_support: route.route_support,
                topology_epoch: route.topology_epoch.0,
            }
        }),
        observed_at_tick: artifact.observed_at_tick.0,
    }
}

fn exported_policy_event(event: &FieldPolicyEvent) -> FieldExportedPolicyEvent {
    FieldExportedPolicyEvent {
        gate: format!("{:?}", event.gate),
        reason: format!("{:?}", event.reason),
        destination: event.destination.clone(),
        route_id: event.route_id,
        observed_at_tick: event.observed_at_tick.0,
    }
}

fn exported_protocol_replay(replay: &FieldReducedProtocolReplay) -> FieldExportedProtocolReplay {
    FieldExportedProtocolReplay {
        schema_version: replay.schema_version,
        artifacts: replay
            .artifacts
            .iter()
            .map(|artifact| FieldExportedProtocolArtifact {
                protocol: format!("{:?}", artifact.session.protocol),
                route_id: artifact.session.route_id,
                topology_epoch: artifact.session.topology_epoch.0,
                destination: artifact.session.destination.clone(),
                detail: artifact.detail.clone(),
                last_updated_at: artifact.last_updated_at.0,
            })
            .collect(),
        reconfigurations: replay
            .reconfigurations
            .iter()
            .map(|step| FieldExportedProtocolReconfiguration {
                protocol: format!("{:?}", step.prior_session.protocol),
                route_id: step.prior_session.route_id,
                destination: step.prior_session.destination.clone(),
                prior_owner_tag: step.prior_owner_tag,
                next_owner_tag: step.next_owner_tag,
                prior_generation: step.prior_generation,
                next_generation: step.next_generation,
                cause: format!("{:?}", step.cause),
                recorded_at: step.recorded_at.0,
            })
            .collect(),
    }
}

fn exported_recovery_replay(replay: &FieldRecoveryReplaySurface) -> FieldExportedRecoveryReplay {
    FieldExportedRecoveryReplay {
        schema_version: replay.schema_version,
        entries: replay
            .entries
            .iter()
            .map(|entry| FieldExportedRecoveryEntry {
                route_id: entry.route_id,
                checkpoint_available: entry.state.checkpoint_available,
                last_trigger: entry
                    .state
                    .last_trigger
                    .map(|trigger| format!("{trigger:?}")),
                last_outcome: entry
                    .state
                    .last_outcome
                    .map(|outcome| format!("{outcome:?}")),
                bootstrap_active: entry.state.bootstrap_active,
                continuity_band: entry.state.continuity_band.map(|band| format!("{band:?}")),
                last_continuity_transition: entry
                    .state
                    .last_continuity_transition
                    .map(|transition| format!("{transition:?}")),
                last_bootstrap_transition: entry
                    .state
                    .last_bootstrap_transition
                    .map(|transition| format!("{transition:?}")),
                last_promotion_decision: entry
                    .state
                    .last_promotion_decision
                    .map(|decision| format!("{decision:?}")),
                last_promotion_blocker: entry
                    .state
                    .last_promotion_blocker
                    .map(|blocker| format!("{blocker:?}")),
                bootstrap_activation_count: entry.state.bootstrap_activation_count,
                bootstrap_hold_count: entry.state.bootstrap_hold_count,
                bootstrap_narrow_count: entry.state.bootstrap_narrow_count,
                bootstrap_upgrade_count: entry.state.bootstrap_upgrade_count,
                bootstrap_withdraw_count: entry.state.bootstrap_withdraw_count,
                degraded_steady_entry_count: entry.state.degraded_steady_entry_count,
                degraded_steady_recovery_count: entry.state.degraded_steady_recovery_count,
                degraded_to_bootstrap_count: entry.state.degraded_to_bootstrap_count,
                degraded_steady_round_count: entry.state.degraded_steady_round_count,
                service_retention_carry_forward_count: entry
                    .state
                    .service_retention_carry_forward_count,
                asymmetric_shift_success_count: entry.state.asymmetric_shift_success_count,
                checkpoint_capture_count: entry.state.checkpoint_capture_count,
                checkpoint_restore_count: entry.state.checkpoint_restore_count,
                continuation_shift_count: entry.state.continuation_shift_count,
                corridor_narrow_count: entry.state.corridor_narrow_count,
            })
            .collect(),
    }
}
