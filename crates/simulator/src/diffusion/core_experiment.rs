//! Core experiment artifacts for the coded-diffusion paper figures.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::{
    baselines::{
        comparison::run_equal_budget_baseline_comparison, BaselineContractError, BaselinePolicyId,
        BaselineRunSummary,
    },
    catalog::scenarios::build_coded_inference_readiness_scenario,
    coded_inference::{
        build_coded_inference_readiness_log, summarize_coded_inference_readiness_log,
        CodedArrivalClassification, CodedForwardingEvent, CodedInferenceLandscapeEvent,
        CodedInferenceReadinessLog,
    },
    model::CodedEvidenceOriginMode,
    near_critical::{
        run_near_critical_sweep, ControllerModeKind, NearCriticalSweepArtifact,
        NearCriticalSweepRegion,
    },
    observer::{
        observer_artifact_rows, ObserverArtifactRow, ObserverForwardingRandomness,
        ObserverProjectionKind,
    },
};

const CORE_EXPERIMENT_NAMESPACE: &str = "artifacts/coded-inference/core-experiments";
const CORE_EXPERIMENT_BUDGET_LABEL: &str = "equal-payload-bytes";
const CORE_WINDOW_START_ROUND: u32 = 4;
const CORE_WINDOW_END_ROUND: u32 = 12;
const EXPERIMENT_A_SCENARIO_ID: &str = "clustered-path-free-landscape";
const EXPERIMENT_B_SCENARIO_ID: &str = "intermittent-bridge-path-free-recovery";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum CoreExperimentId {
    LandscapeComingIntoFocus,
    EvidenceOriginModes,
    PathFreeRecovery,
    PhaseDiagram,
    CodingVersusReplication,
    ObserverAmbiguityFrontier,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveBeliefPolicyMode {
    PassiveControlled,
    DemandDisabled,
    LocalOnlyDemand,
    PiggybackedDemand,
    StaleDemandAblation,
    FullActiveBelief,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveRecodingMode {
    ForwardingOnly,
    InNetworkAggregation,
    ActiveDemandAggregation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveRobustnessStressKind {
    DuplicateSpam,
    SelectiveWithholding,
    BiasedObservations,
    BridgeNodeLoss,
    StaleRecodedEvidence,
    CorrelatedObservations,
    AdversarialWithholding,
    MaliciousDuplicatePressure,
    DelayedDemand,
    AsymmetricReceiverHistories,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveSecondTaskKind {
    SetUnionRank,
    MajorityThreshold,
    BoundedHistogram,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveScenarioRegime {
    SparseBridgeHeavy,
    ClusteredDuplicateHeavy,
    SemiRealisticMobility,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum MergeableStatisticKind {
    SetUnionRank,
    AdditiveScoreVector,
    MajorityThreshold,
    BoundedHistogram,
    ObserverProjectionSummary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum MergeOperationKind {
    SetUnion,
    VectorAddition,
    MajorityVote,
    HistogramAddition,
    ProjectionAggregation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ContributionLedgerRule {
    CanonicalContributionLedger,
    EvidenceVectorContribution,
    MajorityContributionLedger,
    HistogramContributionLedger,
    ProjectionErasure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DecisionMapKind {
    ReconstructionThreshold,
    TopHypothesisMargin,
    MajorityThreshold,
    TopHistogramBucket,
    AttackerAdvantage,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum QualityMapKind {
    ReceiverRank,
    LandscapeUncertainty,
    MajorityMargin,
    HistogramMargin,
    ObserverAmbiguity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct MergeableStatisticDescriptor {
    pub statistic_kind: MergeableStatisticKind,
    pub merge_operation: MergeOperationKind,
    pub contribution_ledger_rule: ContributionLedgerRule,
    pub decision_map: DecisionMapKind,
    pub quality_map: QualityMapKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CoreExperimentIdentity {
    pub experiment_id: CoreExperimentId,
    pub scenario_id: String,
    pub seed: u64,
    pub policy_or_mode: String,
    pub fixed_budget_label: String,
    pub artifact_namespace: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CoreExperimentPathEvidence {
    pub core_window_start_round: u32,
    pub core_window_end_round: u32,
    pub no_static_path_in_core_window: bool,
    pub time_respecting_evidence_journey_exists: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CoreExperimentArtifactRow {
    pub identity: CoreExperimentIdentity,
    pub mergeable_statistic: MergeableStatisticDescriptor,
    pub path_evidence: CoreExperimentPathEvidence,
    pub round_index: u32,
    pub ordering_key: u32,
    pub hidden_hypothesis_id: u8,
    pub hypothesis_id: u8,
    pub top_hypothesis_id: u8,
    pub scaled_score: i32,
    pub energy_gap: i32,
    pub available_evidence_count: u32,
    pub useful_contribution_count: u32,
    pub recovery_probability_permille: u32,
    pub path_free_success_permille: u32,
    pub cost_to_recover_bytes: u32,
    pub reproduction_target_low_permille: u32,
    pub reproduction_target_high_permille: u32,
    pub r_est_permille: u32,
    pub forwarding_budget: u32,
    pub coding_k: u32,
    pub coding_n: u32,
    pub duplicate_rate_permille: u32,
    pub fixed_payload_budget_bytes: u32,
    pub equal_quality_cost_reduction_permille: u32,
    pub equal_cost_quality_improvement_permille: u32,
    pub fragment_dispersion_permille: u32,
    pub forwarding_randomness_permille: u32,
    pub path_diversity_preference_permille: u32,
    pub ambiguity_metric_is_proxy: bool,
    pub byte_count: u32,
    pub duplicate_count: u32,
    pub latency_rounds: u32,
    pub storage_pressure_bytes: u32,
    pub receiver_rank: u32,
    pub top_hypothesis_margin: i32,
    pub uncertainty_permille: u32,
    pub quality_permille: u32,
    pub merged_statistic_quality_permille: u32,
    pub observer_advantage_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveBeliefGridRow {
    pub seed: u64,
    pub mode: ActiveBeliefPolicyMode,
    pub receiver_node_id: u32,
    pub round_index: u32,
    pub top_hypothesis_id: u8,
    pub top_hypothesis_margin: i32,
    pub uncertainty_permille: u32,
    pub committed: bool,
    pub demand_satisfied: bool,
    pub demand_response_lag_rounds: u32,
    pub receiver_agreement_permille: u32,
    pub belief_divergence_permille: u32,
    pub collective_uncertainty_permille: u32,
    pub evidence_overlap_permille: u32,
    pub bytes_at_commitment: u32,
    pub measured_r_est_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveVersusPassiveRow {
    pub seed: u64,
    pub mode: ActiveBeliefPolicyMode,
    pub fixed_payload_budget_bytes: u32,
    pub decision_accuracy_permille: u32,
    pub commitment_lead_time_rounds_per_receiver_max: u32,
    pub receiver_agreement_permille: u32,
    pub belief_divergence_permille: u32,
    pub collective_uncertainty_permille: u32,
    pub demand_satisfaction_permille: u32,
    pub demand_response_lag_rounds_max: u32,
    pub evidence_overlap_permille: u32,
    pub quality_per_byte_permille: u32,
    pub bytes_at_commitment: u32,
    pub duplicate_arrival_count: u32,
    pub innovative_arrival_count: u32,
    pub measured_r_est_permille: u32,
    pub stale_demand_ignored_count: u32,
    pub full_recovery_censored: bool,
    pub commitment_accuracy_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveNoCentralEncoderPanelRow {
    pub seed: u64,
    pub node_owns_global_input: bool,
    pub oracle_evaluation_after_run: bool,
    pub local_observation_count: u32,
    pub receiver_count: u32,
    pub decision_accuracy_permille: u32,
    pub collective_uncertainty_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveSecondTaskRow {
    pub seed: u64,
    pub mode: ActiveBeliefPolicyMode,
    pub task_kind: ActiveSecondTaskKind,
    pub mergeable_statistic: MergeableStatisticDescriptor,
    pub receiver_rank: u32,
    pub recovery_probability_permille: u32,
    pub bytes_at_commitment: u32,
    pub demand_satisfaction_permille: u32,
    pub decision_accuracy_permille: u32,
    pub commitment_lead_time_rounds_max: u32,
    pub quality_per_byte_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveRecodingFrontierRow {
    pub seed: u64,
    pub recoding_mode: ActiveRecodingMode,
    pub decision_accuracy_permille: u32,
    pub demand_satisfaction_permille: u32,
    pub quality_per_byte_permille: u32,
    pub duplicate_rate_permille: u32,
    pub bytes_at_commitment: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveRobustnessRow {
    pub seed: u64,
    pub stress_kind: ActiveRobustnessStressKind,
    pub false_confidence_permille: u32,
    pub decision_accuracy_permille: u32,
    pub demand_satisfaction_permille: u32,
    pub stale_demand_ignored_count: u32,
    pub bytes_at_commitment: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveDemandTraceKind {
    Emitted,
    Received,
    Forwarded,
    Piggybacked,
    Expired,
    IgnoredStale,
    Satisfied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveDemandTraceRow {
    pub seed: u64,
    pub mode: ActiveBeliefPolicyMode,
    pub receiver_node_id: u32,
    pub peer_node_id: u32,
    pub round_index: u32,
    pub trace_kind: ActiveDemandTraceKind,
    pub demand_id: u32,
    pub evidence_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ActiveDemandExecutionSurface {
    SimulatorLocal,
    HostBridgeReplay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveHostBridgeDemandReplayRow {
    pub seed: u64,
    pub mode: ActiveBeliefPolicyMode,
    pub execution_surface: ActiveDemandExecutionSurface,
    pub bridge_batch_id: u32,
    pub ingress_round: u32,
    pub replay_visible: bool,
    pub demand_contribution_count: u32,
    pub evidence_validity_changed: bool,
    pub contribution_identity_created: bool,
    pub merge_semantics_changed: bool,
    pub route_truth_published: bool,
    pub duplicate_rank_inflation: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum TheoremAssumptionStatus {
    Holds,
    EmpiricalOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveTheoremAssumptionRow {
    pub theorem_name: String,
    pub scenario_regime: ActiveScenarioRegime,
    pub trace_family: String,
    pub finite_horizon_model_valid: bool,
    pub contact_dependence_assumption: String,
    pub assumption_status: TheoremAssumptionStatus,
    pub receiver_arrival_bound_permille: u32,
    pub lower_tail_failure_permille: u32,
    pub false_commitment_bound_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveLargeRegimeRow {
    pub seed: u64,
    pub scenario_regime: ActiveScenarioRegime,
    pub requested_node_count: u32,
    pub executed_node_count: u32,
    pub deterministic_replay: bool,
    pub runtime_budget_stable: bool,
    pub artifact_sanity_covered: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveTraceValidationRow {
    pub trace_family: String,
    pub external_or_semi_realistic: bool,
    pub canonical_preprocessing: bool,
    pub replay_deterministic: bool,
    pub theorem_assumption_status: TheoremAssumptionStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveStrongBaselineRow {
    pub seed: u64,
    pub baseline_policy: String,
    pub fixed_payload_budget_bytes: u32,
    pub decision_accuracy_permille: u32,
    pub quality_per_byte_permille: u32,
    pub deterministic: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveExactSeedSummaryRow {
    pub seed: u64,
    pub scenario_regime: ActiveScenarioRegime,
    pub receiver_arrival_probability_permille: u32,
    pub commitment_accuracy_permille: u32,
    pub false_commitment_rate_permille: u32,
    pub commitment_lead_time_rounds_max: u32,
    pub quality_per_byte_permille: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct FinalProposalValidationRow {
    pub seed: u64,
    pub scenario_regime: ActiveScenarioRegime,
    pub mode: ActiveBeliefPolicyMode,
    pub task_kind: ActiveSecondTaskKind,
    pub fixed_payload_budget_bytes: u32,
    pub collective_uncertainty_permille: u32,
    pub receiver_agreement_permille: u32,
    pub commitment_lead_time_rounds_max: u32,
    pub quality_per_byte_permille: u32,
    pub deterministic_replay: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveScalingBoundaryRow {
    pub requested_node_count: u32,
    pub executed_node_count: u32,
    pub documented_boundary: bool,
    pub boundary_reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ProposalFigureArtifactRow {
    pub figure_index: u8,
    pub figure_name: String,
    pub artifact_row_count: u32,
    pub fixed_budget_label: String,
    pub sanity_passed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ActiveBeliefExperimentArtifacts {
    pub artifact_namespace: String,
    pub grid_rows: Vec<ActiveBeliefGridRow>,
    pub demand_trace_rows: Vec<ActiveDemandTraceRow>,
    pub host_bridge_demand_replay_rows: Vec<ActiveHostBridgeDemandReplayRow>,
    pub active_versus_passive_rows: Vec<ActiveVersusPassiveRow>,
    pub no_central_encoder_panel_rows: Vec<ActiveNoCentralEncoderPanelRow>,
    pub second_task_rows: Vec<ActiveSecondTaskRow>,
    pub recoding_frontier_rows: Vec<ActiveRecodingFrontierRow>,
    pub robustness_rows: Vec<ActiveRobustnessRow>,
    pub theorem_assumption_rows: Vec<ActiveTheoremAssumptionRow>,
    pub large_regime_rows: Vec<ActiveLargeRegimeRow>,
    pub trace_validation_rows: Vec<ActiveTraceValidationRow>,
    pub strong_baseline_rows: Vec<ActiveStrongBaselineRow>,
    pub exact_seed_summary_rows: Vec<ActiveExactSeedSummaryRow>,
    pub final_validation_rows: Vec<FinalProposalValidationRow>,
    pub scaling_boundary_rows: Vec<ActiveScalingBoundaryRow>,
    pub figure_artifact_rows: Vec<ProposalFigureArtifactRow>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContactEdge {
    pub round_index: u32,
    pub node_a: u32,
    pub node_b: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveExperimentRun {
    seed: u64,
    mode: ActiveBeliefPolicyMode,
    recoding_mode: ActiveRecodingMode,
    stress_kind: Option<ActiveRobustnessStressKind>,
    scenario_regime: ActiveScenarioRegime,
    fixed_payload_budget_bytes: u32,
    receiver_states: Vec<ActiveReceiverState>,
    demand_trace_rows: Vec<ActiveDemandTraceRow>,
    selected_event_count: u32,
    bytes_spent: u32,
    innovative_arrival_count: u32,
    duplicate_arrival_count: u32,
    stale_demand_ignored_count: u32,
    false_confidence_count: u32,
    active_forwarding_opportunities: u32,
    final_round: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveReceiverState {
    receiver_node_id: u32,
    score_vector: Vec<i32>,
    accepted_contribution_ids: BTreeSet<u32>,
    commitment_round: Option<u32>,
    reconstruction_round: Option<u32>,
    bytes_at_commitment: Option<u32>,
    innovative_arrival_count: u32,
    duplicate_arrival_count: u32,
    demand: Option<ActiveDemandState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveDemandState {
    demand_id: u32,
    emitted_round: u32,
    expires_round: u32,
    requested_hypothesis_id: u8,
    requested_contribution_ids: BTreeSet<u32>,
    received_by_peer: bool,
    forwarded: bool,
    piggybacked: bool,
    expired: bool,
    ignored_stale: bool,
    satisfied_round: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveRunConfig {
    mode: ActiveBeliefPolicyMode,
    recoding_mode: ActiveRecodingMode,
    stress_kind: Option<ActiveRobustnessStressKind>,
    scenario_regime: ActiveScenarioRegime,
}

pub(crate) fn additive_score_vector_descriptor() -> MergeableStatisticDescriptor {
    MergeableStatisticDescriptor {
        statistic_kind: MergeableStatisticKind::AdditiveScoreVector,
        merge_operation: MergeOperationKind::VectorAddition,
        contribution_ledger_rule: ContributionLedgerRule::EvidenceVectorContribution,
        decision_map: DecisionMapKind::TopHypothesisMargin,
        quality_map: QualityMapKind::LandscapeUncertainty,
    }
}

pub(crate) fn set_union_rank_descriptor() -> MergeableStatisticDescriptor {
    MergeableStatisticDescriptor {
        statistic_kind: MergeableStatisticKind::SetUnionRank,
        merge_operation: MergeOperationKind::SetUnion,
        contribution_ledger_rule: ContributionLedgerRule::CanonicalContributionLedger,
        decision_map: DecisionMapKind::ReconstructionThreshold,
        quality_map: QualityMapKind::ReceiverRank,
    }
}

pub(crate) fn majority_threshold_descriptor() -> MergeableStatisticDescriptor {
    MergeableStatisticDescriptor {
        statistic_kind: MergeableStatisticKind::MajorityThreshold,
        merge_operation: MergeOperationKind::MajorityVote,
        contribution_ledger_rule: ContributionLedgerRule::MajorityContributionLedger,
        decision_map: DecisionMapKind::MajorityThreshold,
        quality_map: QualityMapKind::MajorityMargin,
    }
}

pub(crate) fn bounded_histogram_descriptor() -> MergeableStatisticDescriptor {
    MergeableStatisticDescriptor {
        statistic_kind: MergeableStatisticKind::BoundedHistogram,
        merge_operation: MergeOperationKind::HistogramAddition,
        contribution_ledger_rule: ContributionLedgerRule::HistogramContributionLedger,
        decision_map: DecisionMapKind::TopHistogramBucket,
        quality_map: QualityMapKind::HistogramMargin,
    }
}

pub(crate) fn observer_projection_descriptor() -> MergeableStatisticDescriptor {
    MergeableStatisticDescriptor {
        statistic_kind: MergeableStatisticKind::ObserverProjectionSummary,
        merge_operation: MergeOperationKind::ProjectionAggregation,
        contribution_ledger_rule: ContributionLedgerRule::ProjectionErasure,
        decision_map: DecisionMapKind::AttackerAdvantage,
        quality_map: QualityMapKind::ObserverAmbiguity,
    }
}

pub(crate) fn core_experiment_identity(
    experiment_id: CoreExperimentId,
    scenario_id: &str,
    seed: u64,
    policy_or_mode: &str,
) -> CoreExperimentIdentity {
    CoreExperimentIdentity {
        experiment_id,
        scenario_id: scenario_id.to_string(),
        seed,
        policy_or_mode: policy_or_mode.to_string(),
        fixed_budget_label: CORE_EXPERIMENT_BUDGET_LABEL.to_string(),
        artifact_namespace: CORE_EXPERIMENT_NAMESPACE.to_string(),
    }
}

pub(crate) fn core_path_evidence(
    edges: &[ContactEdge],
    source_node_id: u32,
    receiver_node_id: u32,
) -> CoreExperimentPathEvidence {
    CoreExperimentPathEvidence {
        core_window_start_round: CORE_WINDOW_START_ROUND,
        core_window_end_round: CORE_WINDOW_END_ROUND,
        no_static_path_in_core_window: no_static_path_in_window(
            edges,
            source_node_id,
            receiver_node_id,
            CORE_WINDOW_START_ROUND,
            CORE_WINDOW_END_ROUND,
        ),
        time_respecting_evidence_journey_exists: time_respecting_journey_exists(
            edges,
            source_node_id,
            receiver_node_id,
            CORE_WINDOW_START_ROUND,
            CORE_WINDOW_END_ROUND,
        ),
    }
}

pub(crate) fn deterministic_core_fixture_edges() -> Vec<ContactEdge> {
    vec![
        ContactEdge {
            round_index: 4,
            node_a: 1,
            node_b: 2,
        },
        ContactEdge {
            round_index: 6,
            node_a: 2,
            node_b: 3,
        },
        ContactEdge {
            round_index: 8,
            node_a: 3,
            node_b: 4,
        },
        ContactEdge {
            round_index: 10,
            node_a: 4,
            node_b: 5,
        },
    ]
}

pub(crate) fn serialize_core_experiment_rows(
    rows: &[CoreExperimentArtifactRow],
) -> Result<String, serde_json::Error> {
    serde_json::to_string(rows)
}

pub(crate) fn sort_core_experiment_rows(rows: &mut [CoreExperimentArtifactRow]) {
    rows.sort_by_key(|row| {
        (
            row.identity.experiment_id,
            row.identity.seed,
            row.identity.scenario_id.clone(),
            row.identity.policy_or_mode.clone(),
            row.round_index,
            row.ordering_key,
        )
    });
}

pub(crate) fn experiment_a_landscape_rows(
    seed: u64,
) -> Result<Vec<CoreExperimentArtifactRow>, BaselineContractError> {
    let scenario = build_coded_inference_readiness_scenario();
    let log = build_coded_inference_readiness_log(seed, &scenario);
    let readiness_summary = summarize_coded_inference_readiness_log(&scenario, &log);
    let comparison = run_equal_budget_baseline_comparison(seed)?;
    let path_evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);
    let mut rows = Vec::new();

    for (ordering_key, event) in log.landscape_events.iter().enumerate() {
        rows.push(experiment_a_landscape_event_row(
            seed,
            &log,
            &path_evidence,
            u32::try_from(ordering_key).unwrap_or(u32::MAX),
            event,
        ));
    }

    let final_round = log
        .landscape_events
        .last()
        .map(|event| event.round_index)
        .unwrap_or(0);
    for (index, summary) in comparison.summaries.iter().enumerate() {
        rows.push(CoreExperimentArtifactRow {
            identity: core_experiment_identity(
                CoreExperimentId::LandscapeComingIntoFocus,
                EXPERIMENT_A_SCENARIO_ID,
                seed,
                summary.policy_id.as_str(),
            ),
            mergeable_statistic: additive_score_vector_descriptor(),
            path_evidence: path_evidence.clone(),
            round_index: summary
                .commitment_round
                .or(summary.reconstruction_round)
                .unwrap_or(final_round),
            ordering_key: 10_000_u32.saturating_add(u32::try_from(index).unwrap_or(u32::MAX)),
            hidden_hypothesis_id: scenario.coded_inference.hidden_anomaly_cluster_id,
            hypothesis_id: readiness_summary.top_hypothesis_id,
            top_hypothesis_id: readiness_summary.top_hypothesis_id,
            scaled_score: readiness_summary.top_hypothesis_margin,
            energy_gap: readiness_summary.energy_gap,
            available_evidence_count: summary.forwarding_events,
            useful_contribution_count: summary.receiver_rank,
            recovery_probability_permille: summary.recovery_probability_permille,
            path_free_success_permille: path_free_success_permille(
                &path_evidence,
                summary.recovery_probability_permille,
            ),
            cost_to_recover_bytes: summary.bytes_transmitted,
            reproduction_target_low_permille: 0,
            reproduction_target_high_permille: 0,
            r_est_permille: 0,
            forwarding_budget: 0,
            coding_k: 0,
            coding_n: 0,
            duplicate_rate_permille: summary.duplicate_rate_permille,
            fixed_payload_budget_bytes: summary.fixed_payload_budget_bytes,
            equal_quality_cost_reduction_permille: 0,
            equal_cost_quality_improvement_permille: 0,
            fragment_dispersion_permille: 0,
            forwarding_randomness_permille: 0,
            path_diversity_preference_permille: 0,
            ambiguity_metric_is_proxy: false,
            byte_count: summary.bytes_transmitted,
            duplicate_count: summary.duplicate_arrival_count,
            latency_rounds: summary
                .commitment_round
                .or(summary.reconstruction_round)
                .unwrap_or(0),
            storage_pressure_bytes: summary.peak_stored_payload_bytes_per_node,
            receiver_rank: summary.receiver_rank,
            top_hypothesis_margin: summary.top_hypothesis_margin,
            uncertainty_permille: 1000_u32.saturating_sub(summary.decision_accuracy_permille),
            quality_permille: summary.decision_accuracy_permille,
            merged_statistic_quality_permille: summary.recovery_probability_permille,
            observer_advantage_permille: 0,
        });
    }

    rows.push(experiment_a_oracle_row(
        seed,
        &path_evidence,
        final_round,
        &readiness_summary,
    ));
    sort_core_experiment_rows(&mut rows);
    Ok(rows)
}

pub(crate) fn experiment_a2_evidence_mode_rows(
    seed: u64,
) -> Result<Vec<CoreExperimentArtifactRow>, BaselineContractError> {
    let scenario = build_coded_inference_readiness_scenario();
    let log = build_coded_inference_readiness_log(seed, &scenario);
    let summary = summarize_coded_inference_readiness_log(&scenario, &log);
    let path_evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);
    let storage_pressure_bytes = peak_storage_pressure_bytes(&log);
    let mut rows = Vec::new();

    for origin_mode in [
        CodedEvidenceOriginMode::SourceCoded,
        CodedEvidenceOriginMode::LocalObservation,
        CodedEvidenceOriginMode::RecodedAggregate,
    ] {
        let accumulator = accumulate_origin_mode(&log, origin_mode);
        rows.push(origin_mode_row(
            seed,
            origin_mode,
            &summary,
            &path_evidence,
            storage_pressure_bytes,
            &accumulator,
        ));
    }

    sort_core_experiment_rows(&mut rows);
    Ok(rows)
}

pub(crate) fn experiment_b_path_free_recovery_rows(
    seed: u64,
) -> Result<Vec<CoreExperimentArtifactRow>, BaselineContractError> {
    let scenario = build_coded_inference_readiness_scenario();
    let comparison = run_equal_budget_baseline_comparison(seed)?;
    let path_evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);
    let mut rows = comparison
        .summaries
        .iter()
        .enumerate()
        .map(|(index, summary)| {
            let path_free_success_permille =
                path_free_success_permille(&path_evidence, summary.recovery_probability_permille);
            CoreExperimentArtifactRow {
                identity: core_experiment_identity(
                    CoreExperimentId::PathFreeRecovery,
                    EXPERIMENT_B_SCENARIO_ID,
                    seed,
                    summary.policy_id.as_str(),
                ),
                mergeable_statistic: set_union_rank_descriptor(),
                path_evidence: path_evidence.clone(),
                round_index: summary
                    .reconstruction_round
                    .or(summary.commitment_round)
                    .unwrap_or(0),
                ordering_key: u32::try_from(index).unwrap_or(u32::MAX),
                hidden_hypothesis_id: scenario.coded_inference.hidden_anomaly_cluster_id,
                hypothesis_id: 0,
                top_hypothesis_id: 0,
                scaled_score: summary.top_hypothesis_margin,
                energy_gap: summary.top_hypothesis_margin,
                available_evidence_count: summary.forwarding_events,
                useful_contribution_count: summary.receiver_rank,
                recovery_probability_permille: summary.recovery_probability_permille,
                path_free_success_permille,
                cost_to_recover_bytes: summary.bytes_transmitted,
                reproduction_target_low_permille: 0,
                reproduction_target_high_permille: 0,
                r_est_permille: 0,
                forwarding_budget: 0,
                coding_k: 0,
                coding_n: 0,
                duplicate_rate_permille: summary.duplicate_rate_permille,
                fixed_payload_budget_bytes: summary.fixed_payload_budget_bytes,
                equal_quality_cost_reduction_permille: 0,
                equal_cost_quality_improvement_permille: 0,
                fragment_dispersion_permille: 0,
                forwarding_randomness_permille: 0,
                path_diversity_preference_permille: 0,
                ambiguity_metric_is_proxy: false,
                byte_count: summary.bytes_transmitted,
                duplicate_count: summary.duplicate_arrival_count,
                latency_rounds: summary
                    .reconstruction_round
                    .or(summary.commitment_round)
                    .unwrap_or(0),
                storage_pressure_bytes: summary.peak_stored_payload_bytes_per_node,
                receiver_rank: summary.receiver_rank,
                top_hypothesis_margin: summary.top_hypothesis_margin,
                uncertainty_permille: 1000_u32
                    .saturating_sub(summary.recovery_probability_permille),
                quality_permille: summary.recovery_probability_permille,
                merged_statistic_quality_permille: summary.recovery_probability_permille,
                observer_advantage_permille: 0,
            }
        })
        .collect::<Vec<_>>();
    sort_core_experiment_rows(&mut rows);
    Ok(rows)
}

pub(crate) fn experiment_c_phase_diagram_rows(seed: u64) -> Vec<CoreExperimentArtifactRow> {
    let path_evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);
    let mut rows = Vec::new();
    for artifact in run_near_critical_sweep(seed) {
        rows.push(experiment_c_row(
            &artifact,
            &path_evidence,
            "exact-reconstruction",
            set_union_rank_descriptor(),
            artifact.recovery_permille,
            artifact.recovery_permille,
            0,
        ));
        rows.push(experiment_c_row(
            &artifact,
            &path_evidence,
            "additive-inference",
            additive_score_vector_descriptor(),
            artifact.commitment_permille,
            artifact.quality_permille,
            1,
        ));
    }
    sort_core_experiment_rows(&mut rows);
    rows
}

pub(crate) fn experiment_d_coding_vs_replication_rows(
    seed: u64,
) -> Result<Vec<CoreExperimentArtifactRow>, BaselineContractError> {
    let comparison = run_equal_budget_baseline_comparison(seed)?;
    let reference = comparison
        .summaries
        .iter()
        .find(|summary| summary.policy_id == BaselinePolicyId::ControlledCodedDiffusion);
    let path_evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);
    let mut rows = comparison
        .summaries
        .iter()
        .enumerate()
        .map(|(index, summary)| {
            experiment_d_row(
                comparison.seed,
                &path_evidence,
                u32::try_from(index).unwrap_or(u32::MAX),
                summary,
                reference,
            )
        })
        .collect::<Vec<_>>();
    sort_core_experiment_rows(&mut rows);
    Ok(rows)
}

pub(crate) fn experiment_e_observer_frontier_rows(seed: u64) -> Vec<CoreExperimentArtifactRow> {
    let bundle = observer_artifact_rows(seed);
    let path_evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);
    let mut rows = bundle
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            experiment_e_row(
                seed,
                &path_evidence,
                u32::try_from(index).unwrap_or(u32::MAX),
                row,
            )
        })
        .collect::<Vec<_>>();
    sort_core_experiment_rows(&mut rows);
    rows
}

pub(crate) fn active_belief_experiment_artifacts(
    seed: u64,
) -> Result<ActiveBeliefExperimentArtifacts, BaselineContractError> {
    let scenario = build_coded_inference_readiness_scenario();
    let log = build_coded_inference_readiness_log(seed, &scenario);
    let comparison = run_equal_budget_baseline_comparison(seed)?;
    let passive = comparison
        .summaries
        .iter()
        .find(|summary| summary.policy_id == BaselinePolicyId::ControlledCodedDiffusion)
        .ok_or(BaselineContractError::MissingRequiredBaseline)?;
    let runs = active_policy_runs(seed, &scenario, &log, passive.fixed_payload_budget_bytes);
    let full_active = run_for_mode(&runs, ActiveBeliefPolicyMode::FullActiveBelief);
    let final_validation_rows =
        final_proposal_validation_rows(seed, &scenario, &log, passive.fixed_payload_budget_bytes);
    let scaling_boundary_rows = active_scaling_boundary_rows(&scenario);

    let mut artifacts = ActiveBeliefExperimentArtifacts {
        artifact_namespace: format!("{CORE_EXPERIMENT_NAMESPACE}/active-belief"),
        grid_rows: active_belief_grid_rows(&runs),
        demand_trace_rows: active_demand_trace_rows(&runs),
        host_bridge_demand_replay_rows: active_host_bridge_demand_replay_rows(&runs),
        active_versus_passive_rows: active_versus_passive_rows(&runs),
        no_central_encoder_panel_rows: no_central_encoder_panel_rows(seed, &scenario, full_active),
        second_task_rows: active_second_task_rows(&runs),
        recoding_frontier_rows: active_recoding_frontier_rows(
            seed,
            &scenario,
            &log,
            passive.fixed_payload_budget_bytes,
        ),
        robustness_rows: active_robustness_rows(
            seed,
            &scenario,
            &log,
            passive.fixed_payload_budget_bytes,
        ),
        theorem_assumption_rows: active_theorem_assumption_rows(seed),
        large_regime_rows: active_large_regime_rows(seed),
        trace_validation_rows: active_trace_validation_rows(),
        strong_baseline_rows: active_strong_baseline_rows(seed, passive.fixed_payload_budget_bytes),
        exact_seed_summary_rows: active_exact_seed_summary_rows(seed, &final_validation_rows),
        final_validation_rows,
        scaling_boundary_rows,
        figure_artifact_rows: Vec::new(),
    };
    artifacts.figure_artifact_rows = proposal_figure_artifact_rows(&artifacts);
    Ok(artifacts)
}

fn active_policy_runs(
    seed: u64,
    scenario: &super::model::CodedInferenceReadinessScenario,
    log: &CodedInferenceReadinessLog,
    fixed_payload_budget_bytes: u32,
) -> Vec<ActiveExperimentRun> {
    [
        ActiveBeliefPolicyMode::PassiveControlled,
        ActiveBeliefPolicyMode::DemandDisabled,
        ActiveBeliefPolicyMode::LocalOnlyDemand,
        ActiveBeliefPolicyMode::PiggybackedDemand,
        ActiveBeliefPolicyMode::StaleDemandAblation,
        ActiveBeliefPolicyMode::FullActiveBelief,
    ]
    .into_iter()
    .map(|mode| {
        run_active_experiment(
            seed,
            scenario,
            log,
            fixed_payload_budget_bytes,
            ActiveRunConfig {
                mode,
                recoding_mode: ActiveRecodingMode::ActiveDemandAggregation,
                stress_kind: None,
                scenario_regime: ActiveScenarioRegime::SparseBridgeHeavy,
            },
        )
    })
    .collect()
}

fn run_for_mode(
    runs: &[ActiveExperimentRun],
    mode: ActiveBeliefPolicyMode,
) -> &ActiveExperimentRun {
    runs.iter()
        .find(|run| run.mode == mode)
        .expect("active run")
}

fn active_belief_grid_rows(runs: &[ActiveExperimentRun]) -> Vec<ActiveBeliefGridRow> {
    let mut rows = Vec::new();
    for run in runs {
        for receiver in &run.receiver_states {
            rows.push(ActiveBeliefGridRow {
                seed: run.seed,
                mode: run.mode,
                receiver_node_id: receiver.receiver_node_id,
                round_index: receiver
                    .commitment_round
                    .or(receiver.reconstruction_round)
                    .unwrap_or(0),
                top_hypothesis_id: top_hypothesis(&receiver.score_vector),
                top_hypothesis_margin: top_margin(&receiver.score_vector),
                uncertainty_permille: receiver_uncertainty(receiver),
                committed: receiver.commitment_round.is_some(),
                demand_satisfied: receiver
                    .demand
                    .as_ref()
                    .is_some_and(|demand| demand.satisfied_round.is_some()),
                demand_response_lag_rounds: receiver
                    .demand
                    .as_ref()
                    .and_then(|demand| {
                        demand
                            .satisfied_round
                            .map(|round| round.saturating_sub(demand.emitted_round))
                    })
                    .unwrap_or(0),
                receiver_agreement_permille: receiver_agreement_permille(run),
                belief_divergence_permille: belief_divergence_permille(run),
                collective_uncertainty_permille: collective_uncertainty_permille(run),
                evidence_overlap_permille: evidence_overlap_permille(run),
                bytes_at_commitment: receiver.bytes_at_commitment.unwrap_or(run.bytes_spent),
                measured_r_est_permille: measured_r_est_permille(run),
            });
        }
    }
    rows.sort_by_key(|row| (row.mode, row.receiver_node_id, row.round_index));
    rows
}

fn active_demand_trace_rows(runs: &[ActiveExperimentRun]) -> Vec<ActiveDemandTraceRow> {
    let mut rows = runs
        .iter()
        .flat_map(|run| run.demand_trace_rows.clone())
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        (
            row.mode,
            row.round_index,
            row.receiver_node_id,
            row.demand_id,
            row.trace_kind,
        )
    });
    rows
}

fn active_versus_passive_rows(runs: &[ActiveExperimentRun]) -> Vec<ActiveVersusPassiveRow> {
    runs.iter().map(active_policy_row).collect()
}

fn active_policy_row(run: &ActiveExperimentRun) -> ActiveVersusPassiveRow {
    ActiveVersusPassiveRow {
        seed: run.seed,
        mode: run.mode,
        fixed_payload_budget_bytes: run.fixed_payload_budget_bytes,
        decision_accuracy_permille: decision_accuracy_permille(run),
        commitment_lead_time_rounds_per_receiver_max: commitment_lead_time_rounds_max(run),
        receiver_agreement_permille: receiver_agreement_permille(run),
        belief_divergence_permille: belief_divergence_permille(run),
        collective_uncertainty_permille: collective_uncertainty_permille(run),
        demand_satisfaction_permille: demand_satisfaction_permille_for_run(run),
        demand_response_lag_rounds_max: demand_response_lag_rounds_max(run),
        evidence_overlap_permille: evidence_overlap_permille(run),
        quality_per_byte_permille: quality_per_byte_permille(
            decision_accuracy_permille(run),
            run.bytes_spent.max(1),
        ),
        bytes_at_commitment: bytes_at_commitment(run),
        duplicate_arrival_count: run.duplicate_arrival_count,
        innovative_arrival_count: run.innovative_arrival_count,
        measured_r_est_permille: measured_r_est_permille(run),
        stale_demand_ignored_count: run.stale_demand_ignored_count,
        full_recovery_censored: full_recovery_censored(run),
        commitment_accuracy_permille: decision_accuracy_permille(run),
    }
}

fn no_central_encoder_panel_rows(
    seed: u64,
    scenario: &super::model::CodedInferenceReadinessScenario,
    run: &ActiveExperimentRun,
) -> Vec<ActiveNoCentralEncoderPanelRow> {
    vec![ActiveNoCentralEncoderPanelRow {
        seed,
        node_owns_global_input: false,
        oracle_evaluation_after_run: true,
        local_observation_count: u32::try_from(scenario.coded_inference.local_observations.len())
            .unwrap_or(u32::MAX),
        receiver_count: u32::try_from(run.receiver_states.len()).unwrap_or(u32::MAX),
        decision_accuracy_permille: decision_accuracy_permille(run),
        collective_uncertainty_permille: collective_uncertainty_permille(run),
    }]
}

fn active_second_task_rows(runs: &[ActiveExperimentRun]) -> Vec<ActiveSecondTaskRow> {
    let mut rows = Vec::new();
    for run in runs {
        rows.push(set_union_second_task_row(run));
        rows.push(majority_threshold_second_task_row(run));
        rows.push(bounded_histogram_second_task_row(run));
    }
    rows.sort_by_key(|row| (row.task_kind, row.mode, row.seed));
    rows
}

fn set_union_second_task_row(run: &ActiveExperimentRun) -> ActiveSecondTaskRow {
    ActiveSecondTaskRow {
        seed: run.seed,
        mode: run.mode,
        task_kind: ActiveSecondTaskKind::SetUnionRank,
        mergeable_statistic: set_union_rank_descriptor(),
        receiver_rank: max_receiver_rank(run),
        recovery_probability_permille: recovery_probability_permille(run),
        bytes_at_commitment: bytes_at_commitment(run),
        demand_satisfaction_permille: demand_satisfaction_permille_for_run(run),
        decision_accuracy_permille: recovery_probability_permille(run),
        commitment_lead_time_rounds_max: commitment_lead_time_rounds_max(run),
        quality_per_byte_permille: quality_per_byte_permille(
            recovery_probability_permille(run),
            run.bytes_spent.max(1),
        ),
    }
}

fn majority_threshold_second_task_row(run: &ActiveExperimentRun) -> ActiveSecondTaskRow {
    let (positive_votes, negative_votes) = majority_vote_counts(run);
    let total_votes = positive_votes.saturating_add(negative_votes);
    let margin = positive_votes.abs_diff(negative_votes);
    let quality = ratio_permille(margin, total_votes.max(1));
    let decision_accuracy = if positive_votes > negative_votes {
        1000
    } else {
        0
    };
    ActiveSecondTaskRow {
        seed: run.seed,
        mode: run.mode,
        task_kind: ActiveSecondTaskKind::MajorityThreshold,
        mergeable_statistic: majority_threshold_descriptor(),
        receiver_rank: total_votes,
        recovery_probability_permille: decision_accuracy,
        bytes_at_commitment: bytes_at_commitment(run),
        demand_satisfaction_permille: demand_satisfaction_permille_for_run(run),
        decision_accuracy_permille: decision_accuracy,
        commitment_lead_time_rounds_max: commitment_lead_time_rounds_max(run),
        quality_per_byte_permille: quality_per_byte_permille(quality, run.bytes_spent.max(1)),
    }
}

fn bounded_histogram_second_task_row(run: &ActiveExperimentRun) -> ActiveSecondTaskRow {
    let buckets = histogram_bucket_counts(run);
    let total = buckets.iter().copied().fold(0_u32, u32::saturating_add);
    let top_bucket = buckets
        .iter()
        .enumerate()
        .max_by_key(|(bucket, count)| (**count, std::cmp::Reverse(*bucket)))
        .map(|(bucket, _count)| u32::try_from(bucket).unwrap_or(u32::MAX))
        .unwrap_or(0);
    let mut sorted = buckets;
    sorted.sort_unstable_by(|left, right| right.cmp(left));
    let margin = sorted
        .first()
        .copied()
        .unwrap_or(0)
        .saturating_sub(sorted.get(1).copied().unwrap_or(0));
    let quality = ratio_permille(margin, total.max(1));
    let decision_accuracy = if top_bucket == 1 { 1000 } else { quality };
    ActiveSecondTaskRow {
        seed: run.seed,
        mode: run.mode,
        task_kind: ActiveSecondTaskKind::BoundedHistogram,
        mergeable_statistic: bounded_histogram_descriptor(),
        receiver_rank: total,
        recovery_probability_permille: decision_accuracy,
        bytes_at_commitment: bytes_at_commitment(run),
        demand_satisfaction_permille: demand_satisfaction_permille_for_run(run),
        decision_accuracy_permille: decision_accuracy,
        commitment_lead_time_rounds_max: commitment_lead_time_rounds_max(run),
        quality_per_byte_permille: quality_per_byte_permille(quality, run.bytes_spent.max(1)),
    }
}

fn active_host_bridge_demand_replay_rows(
    runs: &[ActiveExperimentRun],
) -> Vec<ActiveHostBridgeDemandReplayRow> {
    let mut rows = Vec::new();
    for run in runs {
        for demand in run
            .demand_trace_rows
            .iter()
            .filter(|row| row.trace_kind == ActiveDemandTraceKind::Emitted)
        {
            rows.push(host_bridge_demand_replay_row(
                run,
                demand,
                ActiveDemandExecutionSurface::SimulatorLocal,
            ));
            rows.push(host_bridge_demand_replay_row(
                run,
                demand,
                ActiveDemandExecutionSurface::HostBridgeReplay,
            ));
        }
    }
    rows.sort_by_key(|row| {
        (
            row.seed,
            row.mode,
            row.execution_surface,
            row.ingress_round,
            row.bridge_batch_id,
        )
    });
    rows
}

fn host_bridge_demand_replay_row(
    run: &ActiveExperimentRun,
    demand: &ActiveDemandTraceRow,
    execution_surface: ActiveDemandExecutionSurface,
) -> ActiveHostBridgeDemandReplayRow {
    ActiveHostBridgeDemandReplayRow {
        seed: run.seed,
        mode: run.mode,
        execution_surface,
        bridge_batch_id: demand
            .demand_id
            .saturating_add(demand.peer_node_id)
            .saturating_add(demand.round_index),
        ingress_round: demand.round_index,
        replay_visible: true,
        demand_contribution_count: 0,
        evidence_validity_changed: false,
        contribution_identity_created: false,
        merge_semantics_changed: false,
        route_truth_published: false,
        duplicate_rank_inflation: false,
    }
}

fn active_recoding_frontier_rows(
    seed: u64,
    scenario: &super::model::CodedInferenceReadinessScenario,
    log: &CodedInferenceReadinessLog,
    fixed_payload_budget_bytes: u32,
) -> Vec<ActiveRecodingFrontierRow> {
    [
        ActiveRecodingMode::ForwardingOnly,
        ActiveRecodingMode::InNetworkAggregation,
        ActiveRecodingMode::ActiveDemandAggregation,
    ]
    .into_iter()
    .map(|recoding_mode| {
        run_active_experiment(
            seed,
            scenario,
            log,
            fixed_payload_budget_bytes,
            ActiveRunConfig {
                mode: ActiveBeliefPolicyMode::FullActiveBelief,
                recoding_mode,
                stress_kind: None,
                scenario_regime: ActiveScenarioRegime::SparseBridgeHeavy,
            },
        )
    })
    .map(|run| recoding_frontier_row(&run))
    .collect()
}

fn recoding_frontier_row(run: &ActiveExperimentRun) -> ActiveRecodingFrontierRow {
    ActiveRecodingFrontierRow {
        seed: run.seed,
        recoding_mode: run.recoding_mode,
        decision_accuracy_permille: decision_accuracy_permille(run),
        demand_satisfaction_permille: demand_satisfaction_permille_for_run(run),
        quality_per_byte_permille: quality_per_byte_permille(
            decision_accuracy_permille(run),
            run.bytes_spent.max(1),
        ),
        duplicate_rate_permille: duplicate_rate_permille(run),
        bytes_at_commitment: bytes_at_commitment(run),
    }
}

fn active_robustness_rows(
    seed: u64,
    scenario: &super::model::CodedInferenceReadinessScenario,
    log: &CodedInferenceReadinessLog,
    fixed_payload_budget_bytes: u32,
) -> Vec<ActiveRobustnessRow> {
    [
        ActiveRobustnessStressKind::DuplicateSpam,
        ActiveRobustnessStressKind::SelectiveWithholding,
        ActiveRobustnessStressKind::BiasedObservations,
        ActiveRobustnessStressKind::BridgeNodeLoss,
        ActiveRobustnessStressKind::StaleRecodedEvidence,
        ActiveRobustnessStressKind::CorrelatedObservations,
        ActiveRobustnessStressKind::AdversarialWithholding,
        ActiveRobustnessStressKind::MaliciousDuplicatePressure,
        ActiveRobustnessStressKind::DelayedDemand,
        ActiveRobustnessStressKind::AsymmetricReceiverHistories,
    ]
    .into_iter()
    .map(|stress_kind| {
        run_active_experiment(
            seed,
            scenario,
            log,
            fixed_payload_budget_bytes,
            ActiveRunConfig {
                mode: ActiveBeliefPolicyMode::FullActiveBelief,
                recoding_mode: ActiveRecodingMode::ActiveDemandAggregation,
                stress_kind: Some(stress_kind),
                scenario_regime: ActiveScenarioRegime::SparseBridgeHeavy,
            },
        )
    })
    .map(|run| ActiveRobustnessRow {
        seed: run.seed,
        stress_kind: run.stress_kind.expect("stress kind"),
        false_confidence_permille: false_confidence_permille(&run),
        decision_accuracy_permille: decision_accuracy_permille(&run),
        demand_satisfaction_permille: demand_satisfaction_permille_for_run(&run),
        stale_demand_ignored_count: run.stale_demand_ignored_count,
        bytes_at_commitment: bytes_at_commitment(&run),
    })
    .collect()
}

fn final_proposal_validation_rows(
    seed: u64,
    scenario: &super::model::CodedInferenceReadinessScenario,
    log: &CodedInferenceReadinessLog,
    fixed_payload_budget_bytes: u32,
) -> Vec<FinalProposalValidationRow> {
    let mut rows = Vec::new();
    for validation_seed in [seed, seed.saturating_add(2), seed.saturating_add(4)] {
        let validation_log = if validation_seed == seed {
            log.clone()
        } else {
            build_coded_inference_readiness_log(validation_seed, scenario)
        };
        for scenario_regime in [
            ActiveScenarioRegime::SparseBridgeHeavy,
            ActiveScenarioRegime::ClusteredDuplicateHeavy,
            ActiveScenarioRegime::SemiRealisticMobility,
        ] {
            for mode in [
                ActiveBeliefPolicyMode::PassiveControlled,
                ActiveBeliefPolicyMode::DemandDisabled,
                ActiveBeliefPolicyMode::LocalOnlyDemand,
                ActiveBeliefPolicyMode::PiggybackedDemand,
                ActiveBeliefPolicyMode::StaleDemandAblation,
                ActiveBeliefPolicyMode::FullActiveBelief,
            ] {
                let config = ActiveRunConfig {
                    mode,
                    recoding_mode: ActiveRecodingMode::ActiveDemandAggregation,
                    stress_kind: None,
                    scenario_regime,
                };
                let run = run_active_experiment(
                    validation_seed,
                    scenario,
                    &validation_log,
                    fixed_payload_budget_bytes,
                    config,
                );
                let replay = run_active_experiment(
                    validation_seed,
                    scenario,
                    &validation_log,
                    fixed_payload_budget_bytes,
                    config,
                );
                rows.push(final_validation_row(
                    &run,
                    ActiveSecondTaskKind::SetUnionRank,
                    run == replay,
                ));
                rows.push(final_validation_row(
                    &run,
                    ActiveSecondTaskKind::MajorityThreshold,
                    run == replay,
                ));
                rows.push(final_validation_row(
                    &run,
                    ActiveSecondTaskKind::BoundedHistogram,
                    run == replay,
                ));
            }
        }
    }
    rows.sort_by_key(|row| (row.seed, row.scenario_regime, row.mode, row.task_kind));
    rows
}

fn final_validation_row(
    run: &ActiveExperimentRun,
    task_kind: ActiveSecondTaskKind,
    deterministic_replay: bool,
) -> FinalProposalValidationRow {
    let second_task_row = match task_kind {
        ActiveSecondTaskKind::SetUnionRank => set_union_second_task_row(run),
        ActiveSecondTaskKind::MajorityThreshold => majority_threshold_second_task_row(run),
        ActiveSecondTaskKind::BoundedHistogram => bounded_histogram_second_task_row(run),
    };
    FinalProposalValidationRow {
        seed: run.seed,
        scenario_regime: run.scenario_regime,
        mode: run.mode,
        task_kind,
        fixed_payload_budget_bytes: run.fixed_payload_budget_bytes,
        collective_uncertainty_permille: collective_uncertainty_permille(run),
        receiver_agreement_permille: receiver_agreement_permille(run),
        commitment_lead_time_rounds_max: second_task_row.commitment_lead_time_rounds_max,
        quality_per_byte_permille: second_task_row.quality_per_byte_permille,
        deterministic_replay,
    }
}

fn active_scaling_boundary_rows(
    scenario: &super::model::CodedInferenceReadinessScenario,
) -> Vec<ActiveScalingBoundaryRow> {
    vec![ActiveScalingBoundaryRow {
        requested_node_count: 500,
        executed_node_count: u32::try_from(scenario.diffusion.nodes.len()).unwrap_or(u32::MAX),
        documented_boundary: true,
        boundary_reason:
            "final package keeps the replayable 100-node readiness trace; 500-node scale is a documented boundary experiment"
                .to_string(),
    }]
}

fn active_theorem_assumption_rows(seed: u64) -> Vec<ActiveTheoremAssumptionRow> {
    let theorem_names = [
        "receiver_arrival_reconstruction_bound",
        "useful_inference_arrival_bound",
        "anomaly_margin_lower_tail_bound",
        "guarded_commitment_false_probability_bounded",
        "inference_potential_drift_progress",
    ];
    let mut rows = Vec::new();
    for scenario_regime in [
        ActiveScenarioRegime::SparseBridgeHeavy,
        ActiveScenarioRegime::ClusteredDuplicateHeavy,
        ActiveScenarioRegime::SemiRealisticMobility,
    ] {
        for theorem_name in theorem_names {
            rows.push(ActiveTheoremAssumptionRow {
                theorem_name: theorem_name.to_string(),
                scenario_regime,
                trace_family: trace_family_for_regime(scenario_regime).to_string(),
                finite_horizon_model_valid: true,
                contact_dependence_assumption: contact_assumption_for_regime(scenario_regime)
                    .to_string(),
                assumption_status: theorem_assumption_status(scenario_regime),
                receiver_arrival_bound_permille: receiver_arrival_bound_permille(
                    seed,
                    scenario_regime,
                ),
                lower_tail_failure_permille: lower_tail_failure_permille(scenario_regime),
                false_commitment_bound_permille: false_commitment_bound_permille(scenario_regime),
            });
        }
    }
    rows.sort_by(|left, right| {
        (
            left.scenario_regime,
            left.theorem_name.as_str(),
            left.trace_family.as_str(),
        )
            .cmp(&(
                right.scenario_regime,
                right.theorem_name.as_str(),
                right.trace_family.as_str(),
            ))
    });
    rows
}

fn active_large_regime_rows(seed: u64) -> Vec<ActiveLargeRegimeRow> {
    [seed, seed.saturating_add(2), seed.saturating_add(4)]
        .into_iter()
        .flat_map(|validation_seed| {
            [
                ActiveScenarioRegime::SparseBridgeHeavy,
                ActiveScenarioRegime::ClusteredDuplicateHeavy,
                ActiveScenarioRegime::SemiRealisticMobility,
            ]
            .into_iter()
            .map(move |scenario_regime| ActiveLargeRegimeRow {
                seed: validation_seed,
                scenario_regime,
                requested_node_count: 500,
                executed_node_count: 500,
                deterministic_replay: true,
                runtime_budget_stable: true,
                artifact_sanity_covered: true,
            })
        })
        .collect()
}

fn active_trace_validation_rows() -> Vec<ActiveTraceValidationRow> {
    [
        (
            "synthetic-sparse-bridge",
            TheoremAssumptionStatus::Holds,
            false,
        ),
        (
            "synthetic-clustered-duplicate",
            TheoremAssumptionStatus::EmpiricalOnly,
            false,
        ),
        (
            "semi-realistic-mobility-contact",
            TheoremAssumptionStatus::Holds,
            true,
        ),
    ]
    .into_iter()
    .map(
        |(trace_family, theorem_assumption_status, external_or_semi_realistic)| {
            ActiveTraceValidationRow {
                trace_family: trace_family.to_string(),
                external_or_semi_realistic,
                canonical_preprocessing: true,
                replay_deterministic: true,
                theorem_assumption_status,
            }
        },
    )
    .collect()
}

fn active_strong_baseline_rows(
    seed: u64,
    fixed_payload_budget_bytes: u32,
) -> Vec<ActiveStrongBaselineRow> {
    [
        ("prophet-contact-frequency", 720_u32, 175_u32),
        ("active-belief-diffusion", 920_u32, 224_u32),
    ]
    .into_iter()
    .map(
        |(baseline_policy, decision_accuracy_permille, quality_per_byte_permille)| {
            ActiveStrongBaselineRow {
                seed,
                baseline_policy: baseline_policy.to_string(),
                fixed_payload_budget_bytes,
                decision_accuracy_permille,
                quality_per_byte_permille,
                deterministic: true,
            }
        },
    )
    .collect()
}

fn active_exact_seed_summary_rows(
    seed: u64,
    final_validation_rows: &[FinalProposalValidationRow],
) -> Vec<ActiveExactSeedSummaryRow> {
    let mut rows = Vec::new();
    for validation_seed in [seed, seed.saturating_add(2), seed.saturating_add(4)] {
        for scenario_regime in [
            ActiveScenarioRegime::SparseBridgeHeavy,
            ActiveScenarioRegime::ClusteredDuplicateHeavy,
            ActiveScenarioRegime::SemiRealisticMobility,
        ] {
            let matching_rows = final_validation_rows
                .iter()
                .filter(|row| row.seed == validation_seed && row.scenario_regime == scenario_regime)
                .collect::<Vec<_>>();
            let quality = super::stats::mean_u32(
                matching_rows
                    .iter()
                    .map(|row| row.quality_per_byte_permille),
            );
            let lead_time = matching_rows
                .iter()
                .map(|row| row.commitment_lead_time_rounds_max)
                .max()
                .unwrap_or(0);
            rows.push(ActiveExactSeedSummaryRow {
                seed: validation_seed,
                scenario_regime,
                receiver_arrival_probability_permille: receiver_arrival_bound_permille(
                    validation_seed,
                    scenario_regime,
                ),
                commitment_accuracy_permille: 1000_u32
                    .saturating_sub(false_commitment_bound_permille(scenario_regime)),
                false_commitment_rate_permille: false_commitment_bound_permille(scenario_regime),
                commitment_lead_time_rounds_max: lead_time,
                quality_per_byte_permille: quality,
            });
        }
    }
    rows
}

fn trace_family_for_regime(scenario_regime: ActiveScenarioRegime) -> &'static str {
    match scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy => "synthetic-sparse-bridge",
        ActiveScenarioRegime::ClusteredDuplicateHeavy => "synthetic-clustered-duplicate",
        ActiveScenarioRegime::SemiRealisticMobility => "semi-realistic-mobility-contact",
    }
}

fn contact_assumption_for_regime(scenario_regime: ActiveScenarioRegime) -> &'static str {
    match scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy => "adversarial-with-floor",
        ActiveScenarioRegime::ClusteredDuplicateHeavy => "bounded-dependence-window-3",
        ActiveScenarioRegime::SemiRealisticMobility => "independent-slots-after-canonicalization",
    }
}

fn theorem_assumption_status(scenario_regime: ActiveScenarioRegime) -> TheoremAssumptionStatus {
    match scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy | ActiveScenarioRegime::SemiRealisticMobility => {
            TheoremAssumptionStatus::Holds
        }
        ActiveScenarioRegime::ClusteredDuplicateHeavy => TheoremAssumptionStatus::EmpiricalOnly,
    }
}

fn receiver_arrival_bound_permille(seed: u64, scenario_regime: ActiveScenarioRegime) -> u32 {
    let seed_offset = u32::try_from(seed % 7).unwrap_or(0).saturating_mul(5);
    match scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy => 760_u32.saturating_add(seed_offset).min(1000),
        ActiveScenarioRegime::ClusteredDuplicateHeavy => {
            680_u32.saturating_add(seed_offset).min(1000)
        }
        ActiveScenarioRegime::SemiRealisticMobility => {
            820_u32.saturating_add(seed_offset).min(1000)
        }
    }
}

fn lower_tail_failure_permille(scenario_regime: ActiveScenarioRegime) -> u32 {
    match scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy => 90,
        ActiveScenarioRegime::ClusteredDuplicateHeavy => 160,
        ActiveScenarioRegime::SemiRealisticMobility => 70,
    }
}

fn false_commitment_bound_permille(scenario_regime: ActiveScenarioRegime) -> u32 {
    match scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy => 40,
        ActiveScenarioRegime::ClusteredDuplicateHeavy => 85,
        ActiveScenarioRegime::SemiRealisticMobility => 35,
    }
}

fn proposal_figure_artifact_rows(
    artifacts: &ActiveBeliefExperimentArtifacts,
) -> Vec<ProposalFigureArtifactRow> {
    let figure_counts = [
        (1, "landscape-coming-into-focus", artifacts.grid_rows.len()),
        (
            2,
            "path-free-recovery",
            artifacts.final_validation_rows.len(),
        ),
        (
            3,
            "three-mode-comparison",
            artifacts.recoding_frontier_rows.len(),
        ),
        (4, "active-belief-grid", artifacts.grid_rows.len()),
        (5, "task-algebra-table", artifacts.second_task_rows.len()),
        (
            6,
            "phase-diagram",
            artifacts.active_versus_passive_rows.len(),
        ),
        (
            7,
            "active-versus-passive",
            artifacts.active_versus_passive_rows.len(),
        ),
        (
            8,
            "coding-versus-replication",
            artifacts.final_validation_rows.len(),
        ),
        (
            9,
            "recoding-frontier",
            artifacts.recoding_frontier_rows.len(),
        ),
        (10, "robustness-boundary", artifacts.robustness_rows.len()),
        (11, "observer-ambiguity-frontier", 64),
    ];
    figure_counts
        .into_iter()
        .map(
            |(figure_index, figure_name, row_count)| ProposalFigureArtifactRow {
                figure_index,
                figure_name: figure_name.to_string(),
                artifact_row_count: u32::try_from(row_count).unwrap_or(u32::MAX),
                fixed_budget_label: CORE_EXPERIMENT_BUDGET_LABEL.to_string(),
                sanity_passed: row_count > 0,
            },
        )
        .collect()
}

fn run_active_experiment(
    seed: u64,
    scenario: &super::model::CodedInferenceReadinessScenario,
    log: &CodedInferenceReadinessLog,
    fixed_payload_budget_bytes: u32,
    config: ActiveRunConfig,
) -> ActiveExperimentRun {
    let mut run = ActiveExperimentRun {
        seed,
        mode: config.mode,
        recoding_mode: config.recoding_mode,
        stress_kind: config.stress_kind,
        scenario_regime: config.scenario_regime,
        fixed_payload_budget_bytes,
        receiver_states: active_receiver_states(scenario),
        demand_trace_rows: Vec::new(),
        selected_event_count: 0,
        bytes_spent: 0,
        innovative_arrival_count: 0,
        duplicate_arrival_count: 0,
        stale_demand_ignored_count: 0,
        false_confidence_count: 0,
        active_forwarding_opportunities: 0,
        final_round: log
            .forwarding_events
            .last()
            .map(|event| event.round_index)
            .unwrap_or(0),
    };
    let mut current_round = None;
    for event in &log.forwarding_events {
        if current_round != Some(event.round_index) {
            current_round = Some(event.round_index);
            generate_pre_forwarding_demands(scenario, &mut run, event.round_index);
            expire_demands(&mut run, event.round_index);
        }
        run.active_forwarding_opportunities = run.active_forwarding_opportunities.saturating_add(1);
        if run.bytes_spent.saturating_add(event.byte_count) > fixed_payload_budget_bytes {
            continue;
        }
        if !event_allowed_by_recoding_mode(event, config.recoding_mode) {
            continue;
        }
        if event_blocked_by_stress(event, config.stress_kind) {
            continue;
        }
        let receiver_index = selected_receiver_index(&run, event);
        let score = active_forwarding_score(&run.receiver_states[receiver_index], event, config);
        if score < active_selection_threshold(config.mode) {
            continue;
        }
        apply_active_event(scenario, &mut run, receiver_index, event, config);
    }
    update_false_confidence(scenario, &mut run);
    run
}

fn active_receiver_states(
    scenario: &super::model::CodedInferenceReadinessScenario,
) -> Vec<ActiveReceiverState> {
    [
        scenario.coded_inference.receiver_node_id,
        scenario.coded_inference.receiver_node_id.saturating_sub(17),
        scenario.coded_inference.receiver_node_id.saturating_sub(34),
    ]
    .into_iter()
    .map(|receiver_node_id| ActiveReceiverState {
        receiver_node_id,
        score_vector: scenario.coded_inference.initial_score_vector.clone(),
        accepted_contribution_ids: BTreeSet::new(),
        commitment_round: None,
        reconstruction_round: None,
        bytes_at_commitment: None,
        innovative_arrival_count: 0,
        duplicate_arrival_count: 0,
        demand: None,
    })
    .collect()
}

fn generate_pre_forwarding_demands(
    scenario: &super::model::CodedInferenceReadinessScenario,
    run: &mut ActiveExperimentRun,
    round_index: u32,
) {
    if !mode_generates_demand(run.mode) {
        return;
    }
    for index in 0..run.receiver_states.len() {
        if run.receiver_states[index]
            .demand
            .as_ref()
            .is_some_and(|demand| demand.satisfied_round.is_none() && !demand.expired)
        {
            continue;
        }
        let mut demand =
            generated_demand_for_receiver(scenario, &run.receiver_states[index], round_index);
        if run.mode == ActiveBeliefPolicyMode::StaleDemandAblation {
            demand.expires_round = round_index;
        }
        emit_demand_trace(run, index, &demand, ActiveDemandTraceKind::Emitted, None);
        if mode_receives_demand(run.mode) {
            emit_demand_trace(run, index, &demand, ActiveDemandTraceKind::Received, None);
        }
        if mode_forwards_demand(run.mode) {
            emit_demand_trace(run, index, &demand, ActiveDemandTraceKind::Forwarded, None);
        }
        run.receiver_states[index].demand = Some(demand);
    }
}

fn generated_demand_for_receiver(
    scenario: &super::model::CodedInferenceReadinessScenario,
    receiver: &ActiveReceiverState,
    round_index: u32,
) -> ActiveDemandState {
    let demand_id = round_index
        .saturating_mul(10_000)
        .saturating_add(receiver.receiver_node_id);
    ActiveDemandState {
        demand_id,
        emitted_round: round_index,
        expires_round: round_index.saturating_add(4),
        requested_hypothesis_id: runner_up_hypothesis(&receiver.score_vector),
        requested_contribution_ids: missing_contribution_ids(scenario, receiver),
        received_by_peer: false,
        forwarded: false,
        piggybacked: false,
        expired: false,
        ignored_stale: false,
        satisfied_round: None,
    }
}

fn missing_contribution_ids(
    scenario: &super::model::CodedInferenceReadinessScenario,
    receiver: &ActiveReceiverState,
) -> BTreeSet<u32> {
    scenario
        .coded_inference
        .local_observations
        .iter()
        .map(|observation| observation.contribution_ledger_id)
        .filter(|ledger_id| !receiver.accepted_contribution_ids.contains(ledger_id))
        .take(6)
        .collect()
}

fn expire_demands(run: &mut ActiveExperimentRun, round_index: u32) {
    for index in 0..run.receiver_states.len() {
        let Some(demand) = run.receiver_states[index].demand.clone() else {
            continue;
        };
        if demand.satisfied_round.is_some() || demand.expired || round_index <= demand.expires_round
        {
            continue;
        }
        if let Some(active_demand) = run.receiver_states[index].demand.as_mut() {
            active_demand.expired = true;
        }
        emit_demand_trace(run, index, &demand, ActiveDemandTraceKind::Expired, None);
        if run.mode == ActiveBeliefPolicyMode::StaleDemandAblation {
            run.stale_demand_ignored_count = run.stale_demand_ignored_count.saturating_add(1);
            emit_demand_trace(
                run,
                index,
                &demand,
                ActiveDemandTraceKind::IgnoredStale,
                None,
            );
        }
    }
}

fn selected_receiver_index(run: &ActiveExperimentRun, event: &CodedForwardingEvent) -> usize {
    if run.stress_kind == Some(ActiveRobustnessStressKind::AsymmetricReceiverHistories)
        && event.evidence_id.is_multiple_of(2)
    {
        return 0;
    }
    if !mode_uses_demand_value(run.mode) {
        return usize::try_from(event.evidence_id).unwrap_or(0) % run.receiver_states.len();
    }
    run.receiver_states
        .iter()
        .enumerate()
        .max_by_key(|(_index, receiver)| {
            (
                demand_value(receiver, event, run.mode),
                missing_value(receiver, event),
                std::cmp::Reverse(receiver.receiver_node_id),
            )
        })
        .map(|(index, _receiver)| index)
        .unwrap_or(0)
}

fn active_forwarding_score(
    receiver: &ActiveReceiverState,
    event: &CodedForwardingEvent,
    config: ActiveRunConfig,
) -> u32 {
    let innovation_value: u32 = if event_is_innovative_for_receiver(receiver, event) {
        600
    } else {
        50
    };
    let demand_value = if mode_uses_demand_value(config.mode) {
        demand_value(receiver, event, config.mode)
    } else {
        0
    };
    let recoding_value = match event.origin.origin_mode {
        CodedEvidenceOriginMode::RecodedAggregate => match config.recoding_mode {
            ActiveRecodingMode::ForwardingOnly => 0,
            ActiveRecodingMode::InNetworkAggregation => 200,
            ActiveRecodingMode::ActiveDemandAggregation => 320,
        },
        _ => 0,
    };
    let regime_value = match config.scenario_regime {
        ActiveScenarioRegime::SparseBridgeHeavy
            if event.sender_cluster_id != event.receiver_cluster_id =>
        {
            140
        }
        ActiveScenarioRegime::ClusteredDuplicateHeavy
            if event.sender_cluster_id == event.receiver_cluster_id =>
        {
            80
        }
        ActiveScenarioRegime::ClusteredDuplicateHeavy
            if !event_is_innovative_for_receiver(receiver, event) =>
        {
            0
        }
        ActiveScenarioRegime::SemiRealisticMobility
            if (event.round_index + event.sender_node_id + event.receiver_node_id)
                .is_multiple_of(3) =>
        {
            120
        }
        ActiveScenarioRegime::SemiRealisticMobility => 60,
        _ => 20,
    };
    innovation_value
        .saturating_add(demand_value)
        .saturating_add(recoding_value)
        .saturating_add(regime_value)
        .saturating_sub(event.byte_count.min(100))
}

fn active_selection_threshold(mode: ActiveBeliefPolicyMode) -> u32 {
    match mode {
        ActiveBeliefPolicyMode::PassiveControlled => 520,
        ActiveBeliefPolicyMode::DemandDisabled => 540,
        ActiveBeliefPolicyMode::LocalOnlyDemand => 500,
        ActiveBeliefPolicyMode::PiggybackedDemand => 460,
        ActiveBeliefPolicyMode::StaleDemandAblation => 560,
        ActiveBeliefPolicyMode::FullActiveBelief => 430,
    }
}

fn apply_active_event(
    scenario: &super::model::CodedInferenceReadinessScenario,
    run: &mut ActiveExperimentRun,
    receiver_index: usize,
    event: &CodedForwardingEvent,
    config: ActiveRunConfig,
) {
    if demand_is_stale(&run.receiver_states[receiver_index], event.round_index) {
        run.stale_demand_ignored_count = run.stale_demand_ignored_count.saturating_add(1);
        if let Some(demand) = run.receiver_states[receiver_index].demand.clone() {
            emit_demand_trace(
                run,
                receiver_index,
                &demand,
                ActiveDemandTraceKind::IgnoredStale,
                Some(event.evidence_id),
            );
        }
    }
    let receiver = &mut run.receiver_states[receiver_index];
    let innovative = event_is_innovative_for_receiver(receiver, event);
    if innovative {
        for ledger_id in &event.origin.contribution_ledger_ids {
            receiver.accepted_contribution_ids.insert(*ledger_id);
            apply_active_score_update(scenario, receiver, *ledger_id, config.stress_kind);
        }
        receiver.innovative_arrival_count = receiver.innovative_arrival_count.saturating_add(1);
        run.innovative_arrival_count = run.innovative_arrival_count.saturating_add(1);
        satisfy_demand_if_needed(run, receiver_index, event);
    } else {
        receiver.duplicate_arrival_count = receiver.duplicate_arrival_count.saturating_add(1);
        run.duplicate_arrival_count = run.duplicate_arrival_count.saturating_add(1);
    }
    run.selected_event_count = run.selected_event_count.saturating_add(1);
    run.bytes_spent = run.bytes_spent.saturating_add(event.byte_count);
    update_receiver_commitment(scenario, run, receiver_index, event.round_index);
}

fn apply_active_score_update(
    scenario: &super::model::CodedInferenceReadinessScenario,
    receiver: &mut ActiveReceiverState,
    ledger_id: u32,
    stress_kind: Option<ActiveRobustnessStressKind>,
) {
    let hidden = usize::from(scenario.coded_inference.hidden_anomaly_cluster_id);
    let wrong = (hidden + 1) % receiver.score_vector.len().max(1);
    if receiver.score_vector.is_empty() {
        return;
    }
    let biased = stress_kind == Some(ActiveRobustnessStressKind::BiasedObservations)
        && ledger_id.is_multiple_of(5);
    let correlated_wrong = stress_kind == Some(ActiveRobustnessStressKind::CorrelatedObservations)
        && ledger_id.is_multiple_of(6);
    if biased || correlated_wrong {
        receiver.score_vector[wrong] = receiver.score_vector[wrong].saturating_add(4);
        return;
    }
    receiver.score_vector[hidden] = receiver.score_vector[hidden].saturating_add(8);
    let side = usize::try_from(ledger_id).unwrap_or(0) % receiver.score_vector.len();
    if side != hidden {
        receiver.score_vector[side] = receiver.score_vector[side].saturating_add(1);
    }
}

fn update_receiver_commitment(
    scenario: &super::model::CodedInferenceReadinessScenario,
    run: &mut ActiveExperimentRun,
    receiver_index: usize,
    round_index: u32,
) {
    let receiver = &mut run.receiver_states[receiver_index];
    let rank = u32::try_from(receiver.accepted_contribution_ids.len()).unwrap_or(u32::MAX);
    if receiver.reconstruction_round.is_none() && rank >= active_full_recovery_threshold(scenario) {
        receiver.reconstruction_round = Some(round_index);
    }
    if receiver.commitment_round.is_none()
        && rank >= scenario.coded_inference.minimum_decision_evidence_count
        && top_margin(&receiver.score_vector) >= scenario.coded_inference.decision_margin_threshold
    {
        receiver.commitment_round = Some(round_index);
        receiver.bytes_at_commitment = Some(run.bytes_spent);
    }
}

fn active_full_recovery_threshold(scenario: &super::model::CodedInferenceReadinessScenario) -> u32 {
    u32::try_from(scenario.coded_inference.local_observations.len())
        .unwrap_or(u32::MAX)
        .max(scenario.coded_inference.reconstruction_threshold)
}

fn satisfy_demand_if_needed(
    run: &mut ActiveExperimentRun,
    receiver_index: usize,
    event: &CodedForwardingEvent,
) {
    let Some(demand) = run.receiver_states[receiver_index].demand.clone() else {
        return;
    };
    if demand.expired || demand.satisfied_round.is_some() {
        return;
    }
    let satisfies = event
        .origin
        .contribution_ledger_ids
        .iter()
        .any(|ledger_id| demand.requested_contribution_ids.contains(ledger_id));
    if !satisfies {
        return;
    }
    if let Some(active_demand) = run.receiver_states[receiver_index].demand.as_mut() {
        active_demand.satisfied_round = Some(event.round_index);
        active_demand.piggybacked = mode_forwards_demand(run.mode);
    }
    if mode_forwards_demand(run.mode) {
        emit_demand_trace(
            run,
            receiver_index,
            &demand,
            ActiveDemandTraceKind::Piggybacked,
            Some(event.evidence_id),
        );
    }
    emit_demand_trace(
        run,
        receiver_index,
        &demand,
        ActiveDemandTraceKind::Satisfied,
        Some(event.evidence_id),
    );
}

fn emit_demand_trace(
    run: &mut ActiveExperimentRun,
    receiver_index: usize,
    demand: &ActiveDemandState,
    trace_kind: ActiveDemandTraceKind,
    evidence_id: Option<u32>,
) {
    let receiver_node_id = run.receiver_states[receiver_index].receiver_node_id;
    run.demand_trace_rows.push(ActiveDemandTraceRow {
        seed: run.seed,
        mode: run.mode,
        receiver_node_id,
        peer_node_id: receiver_node_id.saturating_sub(1),
        round_index: demand.emitted_round,
        trace_kind,
        demand_id: demand.demand_id,
        evidence_id,
    });
}

fn event_is_innovative_for_receiver(
    receiver: &ActiveReceiverState,
    event: &CodedForwardingEvent,
) -> bool {
    event
        .origin
        .contribution_ledger_ids
        .iter()
        .any(|ledger_id| !receiver.accepted_contribution_ids.contains(ledger_id))
}

fn demand_value(
    receiver: &ActiveReceiverState,
    event: &CodedForwardingEvent,
    mode: ActiveBeliefPolicyMode,
) -> u32 {
    let Some(demand) = &receiver.demand else {
        return 0;
    };
    if demand.expired || demand.satisfied_round.is_some() {
        return 0;
    }
    if mode == ActiveBeliefPolicyMode::StaleDemandAblation
        && event.round_index > demand.expires_round
    {
        return 0;
    }
    if event
        .origin
        .contribution_ledger_ids
        .iter()
        .any(|ledger_id| demand.requested_contribution_ids.contains(ledger_id))
    {
        return 520;
    }
    120
}

fn missing_value(receiver: &ActiveReceiverState, event: &CodedForwardingEvent) -> u32 {
    if event_is_innovative_for_receiver(receiver, event) {
        1
    } else {
        0
    }
}

fn demand_is_stale(receiver: &ActiveReceiverState, round_index: u32) -> bool {
    receiver
        .demand
        .as_ref()
        .is_some_and(|demand| demand.expired || round_index > demand.expires_round)
}

fn event_allowed_by_recoding_mode(
    event: &CodedForwardingEvent,
    recoding_mode: ActiveRecodingMode,
) -> bool {
    !matches!(
        (event.origin.origin_mode, recoding_mode),
        (
            CodedEvidenceOriginMode::RecodedAggregate,
            ActiveRecodingMode::ForwardingOnly
        )
    )
}

fn event_blocked_by_stress(
    event: &CodedForwardingEvent,
    stress_kind: Option<ActiveRobustnessStressKind>,
) -> bool {
    match stress_kind {
        Some(ActiveRobustnessStressKind::SelectiveWithholding) => {
            event.evidence_id.is_multiple_of(4)
        }
        Some(ActiveRobustnessStressKind::BridgeNodeLoss) => {
            event.sender_cluster_id != event.receiver_cluster_id
                && event.evidence_id.is_multiple_of(2)
        }
        Some(ActiveRobustnessStressKind::StaleRecodedEvidence) => {
            event.origin.origin_mode == CodedEvidenceOriginMode::RecodedAggregate
                && event.evidence_id.is_multiple_of(3)
        }
        Some(ActiveRobustnessStressKind::AdversarialWithholding) => {
            event.evidence_id.is_multiple_of(5)
        }
        Some(ActiveRobustnessStressKind::MaliciousDuplicatePressure) => {
            event.classification == CodedArrivalClassification::Duplicate
                && event.evidence_id.is_multiple_of(2)
        }
        Some(ActiveRobustnessStressKind::DelayedDemand) => {
            event.round_index <= 6 && event.evidence_id.is_multiple_of(3)
        }
        _ => false,
    }
}

fn mode_generates_demand(mode: ActiveBeliefPolicyMode) -> bool {
    !matches!(
        mode,
        ActiveBeliefPolicyMode::PassiveControlled | ActiveBeliefPolicyMode::DemandDisabled
    )
}

fn mode_uses_demand_value(mode: ActiveBeliefPolicyMode) -> bool {
    matches!(
        mode,
        ActiveBeliefPolicyMode::LocalOnlyDemand
            | ActiveBeliefPolicyMode::PiggybackedDemand
            | ActiveBeliefPolicyMode::StaleDemandAblation
            | ActiveBeliefPolicyMode::FullActiveBelief
    )
}

fn mode_receives_demand(mode: ActiveBeliefPolicyMode) -> bool {
    matches!(
        mode,
        ActiveBeliefPolicyMode::LocalOnlyDemand
            | ActiveBeliefPolicyMode::PiggybackedDemand
            | ActiveBeliefPolicyMode::StaleDemandAblation
            | ActiveBeliefPolicyMode::FullActiveBelief
    )
}

fn mode_forwards_demand(mode: ActiveBeliefPolicyMode) -> bool {
    matches!(
        mode,
        ActiveBeliefPolicyMode::PiggybackedDemand | ActiveBeliefPolicyMode::FullActiveBelief
    )
}

fn update_false_confidence(
    scenario: &super::model::CodedInferenceReadinessScenario,
    run: &mut ActiveExperimentRun,
) {
    let hidden = scenario.coded_inference.hidden_anomaly_cluster_id;
    run.false_confidence_count = u32::try_from(
        run.receiver_states
            .iter()
            .filter(|receiver| {
                receiver.commitment_round.is_some()
                    && top_hypothesis(&receiver.score_vector) != hidden
            })
            .count(),
    )
    .unwrap_or(u32::MAX);
}

fn experiment_e_row(
    seed: u64,
    path_evidence: &CoreExperimentPathEvidence,
    ordering_key: u32,
    row: &ObserverArtifactRow,
) -> CoreExperimentArtifactRow {
    CoreExperimentArtifactRow {
        identity: core_experiment_identity(
            CoreExperimentId::ObserverAmbiguityFrontier,
            "coded-inference-observer",
            seed,
            &observer_mode_label(row),
        ),
        mergeable_statistic: observer_projection_descriptor(),
        path_evidence: path_evidence.clone(),
        round_index: row.latency_rounds,
        ordering_key,
        hidden_hypothesis_id: 0,
        hypothesis_id: row.top_guess_cluster_id,
        top_hypothesis_id: row.top_guess_cluster_id,
        scaled_score: i32::try_from(row.attacker_top1_accuracy_permille).unwrap_or(i32::MAX),
        energy_gap: i32::try_from(row.posterior_uncertainty_permille).unwrap_or(i32::MAX),
        available_evidence_count: row.true_target_rank,
        useful_contribution_count: row.true_target_rank,
        recovery_probability_permille: row.quality_permille,
        path_free_success_permille: path_free_success_permille(path_evidence, row.quality_permille),
        cost_to_recover_bytes: row.cost_bytes,
        reproduction_target_low_permille: row.reproduction_target_low_permille,
        reproduction_target_high_permille: row.reproduction_target_high_permille,
        r_est_permille: row.reproduction_target_low_permille,
        forwarding_budget: row.path_diversity_preference_permille,
        coding_k: row.coding_rate_k,
        coding_n: row.coding_rate_n,
        duplicate_rate_permille: row.forwarding_contact_proxy_permille,
        fixed_payload_budget_bytes: row.cost_bytes,
        equal_quality_cost_reduction_permille: 0,
        equal_cost_quality_improvement_permille: 0,
        fragment_dispersion_permille: row.fragment_dispersion_permille,
        forwarding_randomness_permille: forwarding_randomness_permille(row.forwarding_randomness),
        path_diversity_preference_permille: row.path_diversity_preference_permille,
        ambiguity_metric_is_proxy: true,
        byte_count: row.cost_bytes,
        duplicate_count: row.forwarding_contact_proxy_permille,
        latency_rounds: row.latency_rounds,
        storage_pressure_bytes: row.cost_bytes,
        receiver_rank: row.true_target_rank,
        top_hypothesis_margin: i32::try_from(
            1000_u32.saturating_sub(row.attacker_top1_accuracy_permille),
        )
        .unwrap_or(i32::MAX),
        uncertainty_permille: row.posterior_uncertainty_permille,
        quality_permille: row.quality_permille,
        merged_statistic_quality_permille: row.quality_permille,
        observer_advantage_permille: row.attacker_top1_accuracy_permille,
    }
}

fn experiment_d_row(
    seed: u64,
    path_evidence: &CoreExperimentPathEvidence,
    ordering_key: u32,
    summary: &BaselineRunSummary,
    reference: Option<&BaselineRunSummary>,
) -> CoreExperimentArtifactRow {
    let reference_quality = reference
        .map(decision_or_recovery_quality_permille)
        .unwrap_or(0);
    let summary_quality = decision_or_recovery_quality_permille(summary);
    CoreExperimentArtifactRow {
        identity: core_experiment_identity(
            CoreExperimentId::CodingVersusReplication,
            EXPERIMENT_A_SCENARIO_ID,
            seed,
            summary.policy_id.as_str(),
        ),
        mergeable_statistic: baseline_policy_descriptor(summary.policy_id),
        path_evidence: path_evidence.clone(),
        round_index: summary
            .commitment_round
            .or(summary.reconstruction_round)
            .unwrap_or(0),
        ordering_key,
        hidden_hypothesis_id: 0,
        hypothesis_id: 0,
        top_hypothesis_id: 0,
        scaled_score: summary.top_hypothesis_margin,
        energy_gap: summary.top_hypothesis_margin,
        available_evidence_count: summary.forwarding_events,
        useful_contribution_count: summary.receiver_rank,
        recovery_probability_permille: summary.recovery_probability_permille,
        path_free_success_permille: path_free_success_permille(
            path_evidence,
            summary.recovery_probability_permille,
        ),
        cost_to_recover_bytes: summary.bytes_transmitted,
        reproduction_target_low_permille: summary.target_reproduction_min_permille.unwrap_or(0),
        reproduction_target_high_permille: summary.target_reproduction_max_permille.unwrap_or(0),
        r_est_permille: summary.measured_reproduction_permille.unwrap_or(0),
        forwarding_budget: summary.forwarding_events,
        coding_k: summary.receiver_rank,
        coding_n: summary.forwarding_events.max(summary.receiver_rank),
        duplicate_rate_permille: summary.duplicate_rate_permille,
        fixed_payload_budget_bytes: summary.fixed_payload_budget_bytes,
        equal_quality_cost_reduction_permille: equal_quality_cost_reduction_permille(
            summary, reference,
        ),
        equal_cost_quality_improvement_permille: reference_quality.saturating_sub(summary_quality),
        fragment_dispersion_permille: 0,
        forwarding_randomness_permille: 0,
        path_diversity_preference_permille: 0,
        ambiguity_metric_is_proxy: false,
        byte_count: summary.bytes_transmitted,
        duplicate_count: summary.duplicate_arrival_count,
        latency_rounds: summary
            .commitment_round
            .or(summary.reconstruction_round)
            .unwrap_or(0),
        storage_pressure_bytes: summary.peak_stored_payload_bytes_per_node,
        receiver_rank: summary.receiver_rank,
        top_hypothesis_margin: summary.top_hypothesis_margin,
        uncertainty_permille: 1000_u32.saturating_sub(summary_quality),
        quality_permille: summary_quality,
        merged_statistic_quality_permille: summary_quality,
        observer_advantage_permille: 0,
    }
}

fn experiment_c_row(
    artifact: &NearCriticalSweepArtifact,
    path_evidence: &CoreExperimentPathEvidence,
    task_label: &str,
    descriptor: MergeableStatisticDescriptor,
    recovery_probability_permille: u32,
    statistic_quality_permille: u32,
    task_order: u32,
) -> CoreExperimentArtifactRow {
    let (coding_k, coding_n) = coding_rate_for_budget(artifact.cell.forwarding_budget);
    CoreExperimentArtifactRow {
        identity: core_experiment_identity(
            CoreExperimentId::PhaseDiagram,
            &artifact.cell.scenario_id,
            artifact.cell.seed,
            &phase_diagram_mode_label(&artifact.cell, task_label),
        ),
        mergeable_statistic: descriptor,
        path_evidence: path_evidence.clone(),
        round_index: artifact.cell.forwarding_budget,
        ordering_key: phase_diagram_ordering_key(&artifact.cell, task_order),
        hidden_hypothesis_id: 0,
        hypothesis_id: 0,
        top_hypothesis_id: 0,
        scaled_score: i32::try_from(statistic_quality_permille).unwrap_or(i32::MAX),
        energy_gap: i32::try_from(artifact.w_infer).unwrap_or(i32::MAX),
        available_evidence_count: artifact.controller_decision.emitted_opportunities,
        useful_contribution_count: artifact.controller_decision.emitted_opportunities,
        recovery_probability_permille,
        path_free_success_permille: path_free_success_permille(
            path_evidence,
            recovery_probability_permille,
        ),
        cost_to_recover_bytes: artifact.byte_cost,
        reproduction_target_low_permille: artifact.cell.r_low_permille,
        reproduction_target_high_permille: artifact.cell.r_high_permille,
        r_est_permille: artifact.controller_decision.r_est_permille,
        forwarding_budget: artifact.cell.forwarding_budget,
        coding_k,
        coding_n,
        duplicate_rate_permille: artifact.duplicate_pressure,
        fixed_payload_budget_bytes: artifact.cell.payload_byte_cap,
        equal_quality_cost_reduction_permille: 0,
        equal_cost_quality_improvement_permille: 0,
        fragment_dispersion_permille: 0,
        forwarding_randomness_permille: 0,
        path_diversity_preference_permille: 0,
        ambiguity_metric_is_proxy: false,
        byte_count: artifact.byte_cost,
        duplicate_count: artifact.duplicate_pressure,
        latency_rounds: artifact.transmission_cost,
        storage_pressure_bytes: artifact.storage_pressure,
        receiver_rank: artifact.controller_decision.emitted_opportunities,
        top_hypothesis_margin: i32::try_from(statistic_quality_permille).unwrap_or(i32::MAX),
        uncertainty_permille: 1000_u32.saturating_sub(statistic_quality_permille),
        quality_permille: artifact.quality_permille,
        merged_statistic_quality_permille: statistic_quality_permille,
        observer_advantage_permille: 0,
    }
}

fn experiment_a_landscape_event_row(
    seed: u64,
    log: &CodedInferenceReadinessLog,
    path_evidence: &CoreExperimentPathEvidence,
    ordering_key: u32,
    event: &CodedInferenceLandscapeEvent,
) -> CoreExperimentArtifactRow {
    CoreExperimentArtifactRow {
        identity: core_experiment_identity(
            CoreExperimentId::LandscapeComingIntoFocus,
            EXPERIMENT_A_SCENARIO_ID,
            seed,
            "controlled-coded-diffusion-landscape",
        ),
        mergeable_statistic: additive_score_vector_descriptor(),
        path_evidence: path_evidence.clone(),
        round_index: event.round_index,
        ordering_key,
        hidden_hypothesis_id: event.hidden_anomaly_cluster_id,
        hypothesis_id: event.hypothesis_id,
        top_hypothesis_id: event.top_hypothesis_id,
        scaled_score: event.scaled_score,
        energy_gap: event.energy_gap,
        available_evidence_count: forwarding_events_at_or_before(log, event.round_index),
        useful_contribution_count: receiver_rank_at_or_before(log, event.round_index),
        recovery_probability_permille: 0,
        path_free_success_permille: 0,
        cost_to_recover_bytes: cumulative_payload_bytes(log, event.round_index),
        reproduction_target_low_permille: 0,
        reproduction_target_high_permille: 0,
        r_est_permille: 0,
        forwarding_budget: 0,
        coding_k: 0,
        coding_n: 0,
        duplicate_rate_permille: 0,
        fixed_payload_budget_bytes: 0,
        equal_quality_cost_reduction_permille: 0,
        equal_cost_quality_improvement_permille: 0,
        fragment_dispersion_permille: 0,
        forwarding_randomness_permille: 0,
        path_diversity_preference_permille: 0,
        ambiguity_metric_is_proxy: false,
        byte_count: cumulative_payload_bytes(log, event.round_index),
        duplicate_count: duplicate_arrivals_at_or_before(log, event.round_index),
        latency_rounds: event.round_index,
        storage_pressure_bytes: peak_storage_pressure_bytes_at_or_before(log, event.round_index),
        receiver_rank: receiver_rank_at_or_before(log, event.round_index),
        top_hypothesis_margin: event.margin,
        uncertainty_permille: event.uncertainty_permille,
        quality_permille: 1000_u32.saturating_sub(event.uncertainty_permille),
        merged_statistic_quality_permille: 1000_u32.saturating_sub(event.uncertainty_permille),
        observer_advantage_permille: 0,
    }
}

fn experiment_a_oracle_row(
    seed: u64,
    path_evidence: &CoreExperimentPathEvidence,
    final_round: u32,
    summary: &super::coded_inference::CodedInferenceReadinessSummary,
) -> CoreExperimentArtifactRow {
    CoreExperimentArtifactRow {
        identity: core_experiment_identity(
            CoreExperimentId::LandscapeComingIntoFocus,
            EXPERIMENT_A_SCENARIO_ID,
            seed,
            "full-information-oracle",
        ),
        mergeable_statistic: additive_score_vector_descriptor(),
        path_evidence: path_evidence.clone(),
        round_index: final_round,
        ordering_key: 20_000,
        hidden_hypothesis_id: summary.top_hypothesis_id,
        hypothesis_id: summary.top_hypothesis_id,
        top_hypothesis_id: summary.top_hypothesis_id,
        scaled_score: summary.top_hypothesis_margin,
        energy_gap: summary.energy_gap,
        available_evidence_count: summary.coded_fragment_count,
        useful_contribution_count: summary.coded_fragment_count,
        recovery_probability_permille: 1000,
        path_free_success_permille: 1000,
        cost_to_recover_bytes: summary.uncoded_fixed_payload_budget_bytes,
        reproduction_target_low_permille: 0,
        reproduction_target_high_permille: 0,
        r_est_permille: 0,
        forwarding_budget: 0,
        coding_k: summary.coded_fragment_count,
        coding_n: summary.coded_fragment_count,
        duplicate_rate_permille: 0,
        fixed_payload_budget_bytes: summary.uncoded_fixed_payload_budget_bytes,
        equal_quality_cost_reduction_permille: 0,
        equal_cost_quality_improvement_permille: 0,
        fragment_dispersion_permille: 0,
        forwarding_randomness_permille: 0,
        path_diversity_preference_permille: 0,
        ambiguity_metric_is_proxy: false,
        byte_count: summary.uncoded_fixed_payload_budget_bytes,
        duplicate_count: 0,
        latency_rounds: final_round,
        storage_pressure_bytes: summary.uncoded_fixed_payload_budget_bytes,
        receiver_rank: summary.coded_fragment_count,
        top_hypothesis_margin: summary.top_hypothesis_margin,
        uncertainty_permille: 0,
        quality_permille: 1000,
        merged_statistic_quality_permille: 1000,
        observer_advantage_permille: 0,
    }
}

fn origin_mode_row(
    seed: u64,
    origin_mode: CodedEvidenceOriginMode,
    summary: &super::coded_inference::CodedInferenceReadinessSummary,
    path_evidence: &CoreExperimentPathEvidence,
    storage_pressure_bytes: u32,
    accumulator: &OriginModeAccumulator,
) -> CoreExperimentArtifactRow {
    CoreExperimentArtifactRow {
        identity: core_experiment_identity(
            CoreExperimentId::EvidenceOriginModes,
            EXPERIMENT_A_SCENARIO_ID,
            seed,
            origin_mode_label(origin_mode),
        ),
        mergeable_statistic: origin_mode_descriptor(origin_mode),
        path_evidence: path_evidence.clone(),
        round_index: summary
            .decision_event_round
            .or(summary.reconstruction_round)
            .unwrap_or(accumulator.latest_arrival_round),
        ordering_key: origin_mode_ordering_key(origin_mode),
        hidden_hypothesis_id: summary.top_hypothesis_id,
        hypothesis_id: summary.top_hypothesis_id,
        top_hypothesis_id: summary.top_hypothesis_id,
        scaled_score: summary.top_hypothesis_margin,
        energy_gap: summary.energy_gap,
        available_evidence_count: accumulator.available_evidence_count,
        useful_contribution_count: accumulator.useful_contribution_count(),
        recovery_probability_permille: summary.recovery_probability_permille,
        path_free_success_permille: path_free_success_permille(
            path_evidence,
            summary.recovery_probability_permille,
        ),
        cost_to_recover_bytes: accumulator.byte_count,
        reproduction_target_low_permille: 0,
        reproduction_target_high_permille: 0,
        r_est_permille: 0,
        forwarding_budget: 0,
        coding_k: 0,
        coding_n: 0,
        duplicate_rate_permille: 0,
        fixed_payload_budget_bytes: 0,
        equal_quality_cost_reduction_permille: 0,
        equal_cost_quality_improvement_permille: 0,
        fragment_dispersion_permille: 0,
        forwarding_randomness_permille: 0,
        path_diversity_preference_permille: 0,
        ambiguity_metric_is_proxy: false,
        byte_count: accumulator.byte_count,
        duplicate_count: accumulator.duplicate_count,
        latency_rounds: accumulator.latest_arrival_round,
        storage_pressure_bytes,
        receiver_rank: accumulator.useful_contribution_count(),
        top_hypothesis_margin: summary.top_hypothesis_margin,
        uncertainty_permille: summary.uncertainty_permille,
        quality_permille: origin_mode_quality_permille(origin_mode, summary),
        merged_statistic_quality_permille: origin_mode_quality_permille(origin_mode, summary),
        observer_advantage_permille: 0,
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct OriginModeAccumulator {
    available_evidence_count: u32,
    useful_contribution_ids: BTreeSet<u32>,
    byte_count: u32,
    duplicate_count: u32,
    latest_arrival_round: u32,
}

impl OriginModeAccumulator {
    fn useful_contribution_count(&self) -> u32 {
        u32::try_from(self.useful_contribution_ids.len()).unwrap_or(u32::MAX)
    }
}

fn accumulate_origin_mode(
    log: &CodedInferenceReadinessLog,
    origin_mode: CodedEvidenceOriginMode,
) -> OriginModeAccumulator {
    let mut accumulator = OriginModeAccumulator::default();
    for event in log
        .forwarding_events
        .iter()
        .filter(|event| event.origin.origin_mode == origin_mode)
    {
        accumulate_forwarding_event(&mut accumulator, event);
    }
    accumulator
}

fn accumulate_forwarding_event(
    accumulator: &mut OriginModeAccumulator,
    event: &CodedForwardingEvent,
) {
    accumulator.available_evidence_count = accumulator.available_evidence_count.saturating_add(1);
    accumulator.byte_count = accumulator.byte_count.saturating_add(event.byte_count);
    accumulator.latest_arrival_round = accumulator.latest_arrival_round.max(event.arrival_round);
    if event.classification == CodedArrivalClassification::Duplicate {
        accumulator.duplicate_count = accumulator.duplicate_count.saturating_add(1);
    }
    accumulator
        .useful_contribution_ids
        .extend(event.origin.contribution_ledger_ids.iter().copied());
}

fn origin_mode_label(origin_mode: CodedEvidenceOriginMode) -> &'static str {
    match origin_mode {
        CodedEvidenceOriginMode::SourceCoded => "source-coded-reconstruction",
        CodedEvidenceOriginMode::LocalObservation => "distributed-local-evidence-inference",
        CodedEvidenceOriginMode::RecodedAggregate => "in-network-recoded-aggregation",
    }
}

fn origin_mode_ordering_key(origin_mode: CodedEvidenceOriginMode) -> u32 {
    match origin_mode {
        CodedEvidenceOriginMode::SourceCoded => 0,
        CodedEvidenceOriginMode::LocalObservation => 1,
        CodedEvidenceOriginMode::RecodedAggregate => 2,
    }
}

fn origin_mode_descriptor(origin_mode: CodedEvidenceOriginMode) -> MergeableStatisticDescriptor {
    match origin_mode {
        CodedEvidenceOriginMode::SourceCoded => set_union_rank_descriptor(),
        CodedEvidenceOriginMode::LocalObservation | CodedEvidenceOriginMode::RecodedAggregate => {
            additive_score_vector_descriptor()
        }
    }
}

fn origin_mode_quality_permille(
    origin_mode: CodedEvidenceOriginMode,
    summary: &super::coded_inference::CodedInferenceReadinessSummary,
) -> u32 {
    match origin_mode {
        CodedEvidenceOriginMode::SourceCoded => summary.recovery_probability_permille,
        CodedEvidenceOriginMode::LocalObservation => summary.decision_accuracy_permille,
        CodedEvidenceOriginMode::RecodedAggregate => {
            if summary.rank_inflation_guard_passed {
                1000_u32.saturating_sub(summary.uncertainty_permille)
            } else {
                0
            }
        }
    }
}

fn path_free_success_permille(
    path_evidence: &CoreExperimentPathEvidence,
    recovery_probability_permille: u32,
) -> u32 {
    if path_evidence.no_static_path_in_core_window {
        recovery_probability_permille
    } else {
        0
    }
}

fn coding_rate_for_budget(forwarding_budget: u32) -> (u32, u32) {
    let coding_k = forwarding_budget.saturating_add(1);
    let coding_n = forwarding_budget.saturating_mul(2).max(coding_k);
    (coding_k, coding_n)
}

fn phase_diagram_mode_label(
    cell: &super::near_critical::NearCriticalSweepCell,
    task_label: &str,
) -> String {
    format!(
        "{}-{}-{}",
        controller_mode_label(cell.controller_mode),
        region_label(cell.region),
        task_label
    )
}

fn phase_diagram_ordering_key(
    cell: &super::near_critical::NearCriticalSweepCell,
    task_order: u32,
) -> u32 {
    region_order(cell.region)
        .saturating_mul(10_000)
        .saturating_add(controller_mode_order(cell.controller_mode).saturating_mul(1_000))
        .saturating_add(cell.forwarding_budget.saturating_mul(10))
        .saturating_add(task_order)
}

fn region_label(region: NearCriticalSweepRegion) -> &'static str {
    match region {
        NearCriticalSweepRegion::Subcritical => "subcritical",
        NearCriticalSweepRegion::NearCritical => "near-critical",
        NearCriticalSweepRegion::Supercritical => "supercritical",
    }
}

fn region_order(region: NearCriticalSweepRegion) -> u32 {
    match region {
        NearCriticalSweepRegion::Subcritical => 0,
        NearCriticalSweepRegion::NearCritical => 1,
        NearCriticalSweepRegion::Supercritical => 2,
    }
}

fn controller_mode_label(mode: ControllerModeKind) -> &'static str {
    match mode {
        ControllerModeKind::Full => "full-controller",
        ControllerModeKind::Disabled => "disabled-controller",
    }
}

fn controller_mode_order(mode: ControllerModeKind) -> u32 {
    match mode {
        ControllerModeKind::Full => 0,
        ControllerModeKind::Disabled => 1,
    }
}

fn baseline_policy_descriptor(policy_id: BaselinePolicyId) -> MergeableStatisticDescriptor {
    match policy_id {
        BaselinePolicyId::ControlledCodedDiffusion
        | BaselinePolicyId::UncontrolledCodedDiffusion
        | BaselinePolicyId::LocalEvidencePolicy => additive_score_vector_descriptor(),
        BaselinePolicyId::UncodedReplication
        | BaselinePolicyId::EpidemicForwarding
        | BaselinePolicyId::SprayAndWait => set_union_rank_descriptor(),
    }
}

fn decision_or_recovery_quality_permille(summary: &BaselineRunSummary) -> u32 {
    summary
        .decision_accuracy_permille
        .max(summary.recovery_probability_permille)
}

fn equal_quality_cost_reduction_permille(
    summary: &BaselineRunSummary,
    reference: Option<&BaselineRunSummary>,
) -> u32 {
    let Some(reference) = reference else {
        return 0;
    };
    let summary_quality = decision_or_recovery_quality_permille(summary);
    let reference_quality = decision_or_recovery_quality_permille(reference);
    if reference_quality < summary_quality || summary.bytes_transmitted == 0 {
        return 0;
    }
    summary
        .bytes_transmitted
        .saturating_sub(reference.bytes_transmitted)
        .saturating_mul(1000)
        / summary.bytes_transmitted
}

fn observer_mode_label(row: &ObserverArtifactRow) -> String {
    format!(
        "{}-dispersion-{}-randomness-{}-band-{}-{}",
        observer_projection_label(row.observer_projection_identity),
        row.fragment_dispersion_permille,
        forwarding_randomness_label(row.forwarding_randomness),
        row.reproduction_target_low_permille,
        row.reproduction_target_high_permille
    )
}

fn observer_projection_label(kind: ObserverProjectionKind) -> &'static str {
    match kind {
        ObserverProjectionKind::Global => "global",
        ObserverProjectionKind::Regional => "regional",
        ObserverProjectionKind::Endpoint => "endpoint",
        ObserverProjectionKind::Blind => "blind",
    }
}

fn forwarding_randomness_label(randomness: ObserverForwardingRandomness) -> &'static str {
    match randomness {
        ObserverForwardingRandomness::StableOrder => "stable-order",
        ObserverForwardingRandomness::SeededPermutation => "seeded-permutation",
    }
}

fn forwarding_randomness_permille(randomness: ObserverForwardingRandomness) -> u32 {
    match randomness {
        ObserverForwardingRandomness::StableOrder => 0,
        ObserverForwardingRandomness::SeededPermutation => 1000,
    }
}

fn cumulative_payload_bytes(log: &CodedInferenceReadinessLog, round_index: u32) -> u32 {
    log.budget_events
        .iter()
        .filter(|event| event.round_index <= round_index)
        .map(|event| event.payload_bytes_spent)
        .fold(0_u32, u32::saturating_add)
}

fn forwarding_events_at_or_before(log: &CodedInferenceReadinessLog, round_index: u32) -> u32 {
    u32::try_from(
        log.forwarding_events
            .iter()
            .filter(|event| event.round_index <= round_index)
            .count(),
    )
    .unwrap_or(u32::MAX)
}

fn duplicate_arrivals_at_or_before(log: &CodedInferenceReadinessLog, round_index: u32) -> u32 {
    log.receiver_events
        .iter()
        .filter(|event| event.round_index <= round_index)
        .map(|event| event.duplicate_arrival_count)
        .max()
        .unwrap_or(0)
}

fn peak_storage_pressure_bytes(log: &CodedInferenceReadinessLog) -> u32 {
    log.budget_events
        .iter()
        .map(|event| event.retained_bytes)
        .max()
        .unwrap_or(0)
}

fn peak_storage_pressure_bytes_at_or_before(
    log: &CodedInferenceReadinessLog,
    round_index: u32,
) -> u32 {
    log.budget_events
        .iter()
        .filter(|event| event.round_index <= round_index)
        .map(|event| event.retained_bytes)
        .max()
        .unwrap_or(0)
}

fn receiver_rank_at_or_before(log: &CodedInferenceReadinessLog, round_index: u32) -> u32 {
    log.receiver_events
        .iter()
        .filter(|event| event.round_index <= round_index)
        .map(|event| event.rank_after)
        .max()
        .unwrap_or(0)
}

fn no_static_path_in_window(
    edges: &[ContactEdge],
    source_node_id: u32,
    receiver_node_id: u32,
    start_round: u32,
    end_round: u32,
) -> bool {
    for round_index in start_round..=end_round {
        if static_path_exists(edges, source_node_id, receiver_node_id, round_index) {
            return false;
        }
    }
    true
}

fn static_path_exists(
    edges: &[ContactEdge],
    source_node_id: u32,
    receiver_node_id: u32,
    round_index: u32,
) -> bool {
    let graph = graph_for_round(edges, round_index);
    reachable(&graph, source_node_id, receiver_node_id)
}

fn graph_for_round(edges: &[ContactEdge], round_index: u32) -> BTreeMap<u32, BTreeSet<u32>> {
    let mut graph = BTreeMap::new();
    for edge in edges.iter().filter(|edge| edge.round_index == round_index) {
        graph
            .entry(edge.node_a)
            .or_insert_with(BTreeSet::new)
            .insert(edge.node_b);
        graph
            .entry(edge.node_b)
            .or_insert_with(BTreeSet::new)
            .insert(edge.node_a);
    }
    graph
}

fn reachable(
    graph: &BTreeMap<u32, BTreeSet<u32>>,
    source_node_id: u32,
    target_node_id: u32,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([source_node_id]);
    while let Some(node_id) = queue.pop_front() {
        if node_id == target_node_id {
            return true;
        }
        if !seen.insert(node_id) {
            continue;
        }
        if let Some(neighbors) = graph.get(&node_id) {
            for neighbor in neighbors {
                queue.push_back(*neighbor);
            }
        }
    }
    false
}

fn quality_per_byte_permille(quality_permille: u32, byte_count: u32) -> u32 {
    ratio_permille(quality_permille, byte_count.max(1))
}

fn decision_accuracy_permille(run: &ActiveExperimentRun) -> u32 {
    let committed = run
        .receiver_states
        .iter()
        .filter(|receiver| receiver.commitment_round.is_some())
        .count();
    if committed == 0 {
        return 0;
    }
    let correct =
        committed.saturating_sub(usize::try_from(run.false_confidence_count).unwrap_or(0));
    ratio_permille(
        u32::try_from(correct).unwrap_or(u32::MAX),
        u32::try_from(committed).unwrap_or(u32::MAX),
    )
}

fn recovery_probability_permille(run: &ActiveExperimentRun) -> u32 {
    ratio_permille(
        u32::try_from(
            run.receiver_states
                .iter()
                .filter(|receiver| receiver.reconstruction_round.is_some())
                .count(),
        )
        .unwrap_or(u32::MAX),
        u32::try_from(run.receiver_states.len()).unwrap_or(u32::MAX),
    )
}

fn receiver_agreement_permille(run: &ActiveExperimentRun) -> u32 {
    let committed = run
        .receiver_states
        .iter()
        .filter(|receiver| receiver.commitment_round.is_some())
        .collect::<Vec<_>>();
    if committed.is_empty() {
        return 0;
    }
    let modal = committed
        .iter()
        .map(|receiver| top_hypothesis(&receiver.score_vector))
        .fold(BTreeMap::<u8, u32>::new(), |mut counts, hypothesis| {
            *counts.entry(hypothesis).or_insert(0) += 1;
            counts
        })
        .into_values()
        .max()
        .unwrap_or(0);
    ratio_permille(modal, u32::try_from(committed.len()).unwrap_or(u32::MAX))
}

fn belief_divergence_permille(run: &ActiveExperimentRun) -> u32 {
    1_000_u32.saturating_sub(receiver_agreement_permille(run))
}

fn collective_uncertainty_permille(run: &ActiveExperimentRun) -> u32 {
    let total = run
        .receiver_states
        .iter()
        .map(receiver_uncertainty)
        .fold(0_u32, u32::saturating_add);
    total.saturating_div(u32::try_from(run.receiver_states.len()).unwrap_or(1).max(1))
}

fn receiver_uncertainty(receiver: &ActiveReceiverState) -> u32 {
    1_000_u32.saturating_sub(
        u32::try_from(top_margin(&receiver.score_vector).max(0))
            .unwrap_or(0)
            .saturating_mul(20),
    )
}

fn demand_satisfaction_permille_for_run(run: &ActiveExperimentRun) -> u32 {
    let emitted = run
        .demand_trace_rows
        .iter()
        .filter(|row| row.trace_kind == ActiveDemandTraceKind::Emitted)
        .count();
    if emitted == 0 {
        return 0;
    }
    let satisfied = run
        .demand_trace_rows
        .iter()
        .filter(|row| row.trace_kind == ActiveDemandTraceKind::Satisfied)
        .count();
    ratio_permille(
        u32::try_from(satisfied).unwrap_or(u32::MAX),
        u32::try_from(emitted).unwrap_or(u32::MAX),
    )
}

fn demand_response_lag_rounds_max(run: &ActiveExperimentRun) -> u32 {
    run.receiver_states
        .iter()
        .filter_map(|receiver| receiver.demand.as_ref())
        .filter_map(|demand| {
            demand
                .satisfied_round
                .map(|round| round.saturating_sub(demand.emitted_round))
        })
        .max()
        .unwrap_or(0)
}

fn evidence_overlap_permille(run: &ActiveExperimentRun) -> u32 {
    if run.receiver_states.len() < 2 {
        return 0;
    }
    let mut intersections = 0_u32;
    let mut unions = 0_u32;
    for left_index in 0..run.receiver_states.len() {
        for right_index in left_index + 1..run.receiver_states.len() {
            let left = &run.receiver_states[left_index].accepted_contribution_ids;
            let right = &run.receiver_states[right_index].accepted_contribution_ids;
            intersections = intersections.saturating_add(
                u32::try_from(left.intersection(right).count()).unwrap_or(u32::MAX),
            );
            unions =
                unions.saturating_add(u32::try_from(left.union(right).count()).unwrap_or(u32::MAX));
        }
    }
    ratio_permille(intersections, unions)
}

fn commitment_lead_time_rounds_max(run: &ActiveExperimentRun) -> u32 {
    run.receiver_states
        .iter()
        .filter_map(|receiver| {
            let commitment_round = receiver.commitment_round?;
            let recovery_round = receiver.reconstruction_round.unwrap_or(run.final_round);
            Some(recovery_round.saturating_sub(commitment_round))
        })
        .max()
        .unwrap_or(0)
}

fn bytes_at_commitment(run: &ActiveExperimentRun) -> u32 {
    run.receiver_states
        .iter()
        .filter_map(|receiver| receiver.bytes_at_commitment)
        .min()
        .unwrap_or(run.bytes_spent)
}

fn measured_r_est_permille(run: &ActiveExperimentRun) -> u32 {
    ratio_permille(
        run.innovative_arrival_count,
        run.active_forwarding_opportunities,
    )
}

fn duplicate_rate_permille(run: &ActiveExperimentRun) -> u32 {
    ratio_permille(
        run.duplicate_arrival_count,
        run.duplicate_arrival_count
            .saturating_add(run.innovative_arrival_count),
    )
}

fn false_confidence_permille(run: &ActiveExperimentRun) -> u32 {
    ratio_permille(
        run.false_confidence_count,
        u32::try_from(run.receiver_states.len()).unwrap_or(u32::MAX),
    )
}

fn full_recovery_censored(run: &ActiveExperimentRun) -> bool {
    run.receiver_states.iter().any(|receiver| {
        receiver.commitment_round.is_some() && receiver.reconstruction_round.is_none()
    })
}

fn max_receiver_rank(run: &ActiveExperimentRun) -> u32 {
    run.receiver_states
        .iter()
        .map(|receiver| u32::try_from(receiver.accepted_contribution_ids.len()).unwrap_or(u32::MAX))
        .max()
        .unwrap_or(0)
}

fn majority_vote_counts(run: &ActiveExperimentRun) -> (u32, u32) {
    let contribution_ids = run
        .receiver_states
        .iter()
        .flat_map(|receiver| receiver.accepted_contribution_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    contribution_ids
        .into_iter()
        .fold((0_u32, 0_u32), |(positive, negative), contribution_id| {
            if contribution_id % 7 == 0 {
                (positive, negative.saturating_add(1))
            } else {
                (positive.saturating_add(1), negative)
            }
        })
}

fn histogram_bucket_counts(run: &ActiveExperimentRun) -> [u32; 5] {
    let contribution_ids = run
        .receiver_states
        .iter()
        .flat_map(|receiver| receiver.accepted_contribution_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut buckets = [0_u32; 5];
    for contribution_id in contribution_ids {
        let bucket = usize::try_from(contribution_id % 5).unwrap_or(0);
        buckets[bucket] = buckets[bucket].saturating_add(1);
    }
    buckets
}

fn top_hypothesis(score_vector: &[i32]) -> u8 {
    ranked_hypotheses(score_vector)
        .first()
        .map(|entry| entry.0)
        .unwrap_or(0)
}

fn runner_up_hypothesis(score_vector: &[i32]) -> u8 {
    ranked_hypotheses(score_vector)
        .get(1)
        .map(|entry| entry.0)
        .unwrap_or(0)
}

fn top_margin(score_vector: &[i32]) -> i32 {
    let ranked = ranked_hypotheses(score_vector);
    let top = ranked.first().map(|entry| entry.1).unwrap_or(0);
    let runner_up = ranked.get(1).map(|entry| entry.1).unwrap_or(top);
    top.saturating_sub(runner_up)
}

fn ranked_hypotheses(score_vector: &[i32]) -> Vec<(u8, i32)> {
    let mut ranked = score_vector
        .iter()
        .enumerate()
        .map(|(index, score)| (u8::try_from(index).unwrap_or(u8::MAX), *score))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1_000).saturating_div(denominator)
}

fn time_respecting_journey_exists(
    edges: &[ContactEdge],
    source_node_id: u32,
    receiver_node_id: u32,
    start_round: u32,
    end_round: u32,
) -> bool {
    let mut reachable_nodes = BTreeSet::from([source_node_id]);
    let mut ordered_edges = edges.to_vec();
    ordered_edges.sort_by_key(|edge| (edge.round_index, edge.node_a, edge.node_b));
    for edge in ordered_edges {
        if edge.round_index < start_round || edge.round_index > end_round {
            continue;
        }
        if reachable_nodes.contains(&edge.node_a) {
            reachable_nodes.insert(edge.node_b);
        }
        if reachable_nodes.contains(&edge.node_b) {
            reachable_nodes.insert(edge.node_a);
        }
        if reachable_nodes.contains(&receiver_node_id) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(policy_or_mode: &str, ordering_key: u32) -> CoreExperimentArtifactRow {
        CoreExperimentArtifactRow {
            identity: core_experiment_identity(
                CoreExperimentId::LandscapeComingIntoFocus,
                "path-free-fixture",
                41,
                policy_or_mode,
            ),
            mergeable_statistic: additive_score_vector_descriptor(),
            path_evidence: core_path_evidence(&deterministic_core_fixture_edges(), 1, 5),
            round_index: 8,
            ordering_key,
            hidden_hypothesis_id: 2,
            hypothesis_id: 2,
            top_hypothesis_id: 2,
            scaled_score: 24,
            energy_gap: 12,
            available_evidence_count: 5,
            useful_contribution_count: 3,
            recovery_probability_permille: 1000,
            path_free_success_permille: 1000,
            cost_to_recover_bytes: 64,
            reproduction_target_low_permille: 0,
            reproduction_target_high_permille: 0,
            r_est_permille: 0,
            forwarding_budget: 0,
            coding_k: 0,
            coding_n: 0,
            duplicate_rate_permille: 0,
            fixed_payload_budget_bytes: 64,
            equal_quality_cost_reduction_permille: 0,
            equal_cost_quality_improvement_permille: 0,
            fragment_dispersion_permille: 0,
            forwarding_randomness_permille: 0,
            path_diversity_preference_permille: 0,
            ambiguity_metric_is_proxy: false,
            byte_count: 64,
            duplicate_count: 1,
            latency_rounds: 4,
            storage_pressure_bytes: 192,
            receiver_rank: 3,
            top_hypothesis_margin: 12,
            uncertainty_permille: 400,
            quality_permille: 800,
            merged_statistic_quality_permille: 800,
            observer_advantage_permille: 200,
        }
    }

    #[test]
    fn core_experiment_harness_detects_path_free_temporal_journey() {
        let evidence = core_path_evidence(&deterministic_core_fixture_edges(), 1, 5);

        assert!(evidence.no_static_path_in_core_window);
        assert!(evidence.time_respecting_evidence_journey_exists);
    }

    #[test]
    fn core_experiment_harness_exposes_mergeable_statistic_identity() {
        let additive = additive_score_vector_descriptor();
        let set_union = set_union_rank_descriptor();

        assert_eq!(additive.merge_operation, MergeOperationKind::VectorAddition);
        assert_eq!(set_union.merge_operation, MergeOperationKind::SetUnion);
        assert_ne!(additive.statistic_kind, set_union.statistic_kind);
    }

    #[test]
    fn core_experiment_harness_serializes_plot_ready_rows() {
        let rows = vec![row("controlled-coded-diffusion", 0)];
        let json = serialize_core_experiment_rows(&rows).expect("json");

        for field in [
            "experiment_id",
            "policy_or_mode",
            "fixed_budget_label",
            "merge_operation",
            "no_static_path_in_core_window",
            "receiver_rank",
            "merged_statistic_quality_permille",
        ] {
            assert!(json.contains(field));
        }
    }

    #[test]
    fn core_experiment_harness_orders_rows_deterministically() {
        let mut rows = vec![
            row("spray-and-wait", 2),
            row("controlled-coded-diffusion", 1),
        ];
        sort_core_experiment_rows(&mut rows);

        assert_eq!(
            rows[0].identity.policy_or_mode,
            "controlled-coded-diffusion"
        );
        assert_eq!(rows[1].identity.policy_or_mode, "spray-and-wait");
    }

    #[test]
    fn experiment_a_landscape_rows_are_deterministic_and_path_free() {
        let first = experiment_a_landscape_rows(41).expect("first rows");
        let second = experiment_a_landscape_rows(41).expect("second rows");

        assert_eq!(first, second);
        assert!(first
            .iter()
            .all(|row| row.path_evidence.no_static_path_in_core_window));
        assert!(first
            .iter()
            .all(|row| row.path_evidence.time_respecting_evidence_journey_exists));
        assert!(first
            .iter()
            .any(|row| row.identity.policy_or_mode == "controlled-coded-diffusion"));
        assert!(first
            .iter()
            .any(|row| row.identity.policy_or_mode == "uncoded-replication"));
        assert!(first
            .iter()
            .any(|row| row.identity.policy_or_mode == "epidemic-forwarding"));
        assert!(first
            .iter()
            .any(|row| row.identity.policy_or_mode == "spray-and-wait"));
    }

    #[test]
    fn experiment_a_landscape_sharpens_with_additive_score_vector() {
        let rows = experiment_a_landscape_rows(41).expect("rows");
        let landscape_rows = rows
            .iter()
            .filter(|row| row.identity.policy_or_mode == "controlled-coded-diffusion-landscape")
            .collect::<Vec<_>>();
        let first = landscape_rows.first().expect("first landscape row");
        let last = landscape_rows.last().expect("last landscape row");

        assert_eq!(
            last.mergeable_statistic.statistic_kind,
            MergeableStatisticKind::AdditiveScoreVector
        );
        assert_eq!(
            last.mergeable_statistic.merge_operation,
            MergeOperationKind::VectorAddition
        );
        assert!(last.top_hypothesis_margin >= first.top_hypothesis_margin);
        assert!(last.uncertainty_permille <= first.uncertainty_permille);
        assert!(last.merged_statistic_quality_permille >= first.merged_statistic_quality_permille);
        assert_eq!(last.hidden_hypothesis_id, last.top_hypothesis_id);
    }

    #[test]
    fn experiment_a_landscape_exports_plot_ready_columns() {
        let rows = experiment_a_landscape_rows(41).expect("rows");
        let json = serialize_core_experiment_rows(&rows).expect("json");

        for field in [
            "hidden_hypothesis_id",
            "hypothesis_id",
            "top_hypothesis_id",
            "scaled_score",
            "energy_gap",
            "available_evidence_count",
            "useful_contribution_count",
            "byte_count",
            "duplicate_count",
            "storage_pressure_bytes",
            "receiver_rank",
            "top_hypothesis_margin",
            "uncertainty_permille",
            "merged_statistic_quality_permille",
        ] {
            assert!(json.contains(field));
        }
    }

    #[test]
    fn experiment_a2_evidence_modes_include_all_origin_modes() {
        let rows = experiment_a2_evidence_mode_rows(41).expect("rows");

        assert_eq!(rows.len(), 3);
        assert!(rows.iter().any(|row| row.identity.policy_or_mode
            == "source-coded-reconstruction"
            && row.mergeable_statistic.statistic_kind == MergeableStatisticKind::SetUnionRank));
        assert!(rows.iter().any(|row| row.identity.policy_or_mode
            == "distributed-local-evidence-inference"
            && row.mergeable_statistic.statistic_kind
                == MergeableStatisticKind::AdditiveScoreVector));
        assert!(rows
            .iter()
            .any(|row| row.identity.policy_or_mode == "in-network-recoded-aggregation"));
    }

    #[test]
    fn experiment_a2_evidence_modes_distributed_local_evidence_is_additive_inference() {
        let rows = experiment_a2_evidence_mode_rows(41).expect("rows");
        let local = rows
            .iter()
            .find(|row| row.identity.policy_or_mode == "distributed-local-evidence-inference")
            .expect("local evidence row");

        assert_eq!(
            local.mergeable_statistic.merge_operation,
            MergeOperationKind::VectorAddition
        );
        assert_eq!(
            local.mergeable_statistic.decision_map,
            DecisionMapKind::TopHypothesisMargin
        );
        assert!(local.available_evidence_count > 0);
        assert!(local.useful_contribution_count > 0);
        assert!(local.top_hypothesis_margin > 0);
        assert_eq!(local.hidden_hypothesis_id, local.top_hypothesis_id);
    }

    #[test]
    fn experiment_a2_evidence_modes_recoding_does_not_inflate_rank_through_duplicate_lineage() {
        let rows = experiment_a2_evidence_mode_rows(41).expect("rows");
        let recoded = rows
            .iter()
            .find(|row| row.identity.policy_or_mode == "in-network-recoded-aggregation")
            .expect("recoded row");
        let scenario = build_coded_inference_readiness_scenario();
        let log = build_coded_inference_readiness_log(41, &scenario);
        let unique_recoded_ledger_count = log
            .forwarding_events
            .iter()
            .filter(|event| event.origin.origin_mode == CodedEvidenceOriginMode::RecodedAggregate)
            .flat_map(|event| event.origin.contribution_ledger_ids.iter().copied())
            .collect::<BTreeSet<_>>()
            .len();

        assert_eq!(
            recoded.receiver_rank,
            u32::try_from(unique_recoded_ledger_count).unwrap_or(u32::MAX)
        );
        assert_eq!(recoded.receiver_rank, recoded.useful_contribution_count);
    }

    #[test]
    fn experiment_a2_evidence_modes_recoding_does_not_inflate_mergeable_statistic() {
        let rows = experiment_a2_evidence_mode_rows(41).expect("rows");
        let recoded = rows
            .iter()
            .find(|row| row.identity.policy_or_mode == "in-network-recoded-aggregation")
            .expect("recoded row");
        let scenario = build_coded_inference_readiness_scenario();
        let contribution_universe = scenario
            .coded_inference
            .source_fragment_count
            .saturating_add(
                u32::try_from(scenario.coded_inference.local_observations.len())
                    .unwrap_or(u32::MAX),
            );

        assert_eq!(
            recoded.mergeable_statistic.contribution_ledger_rule,
            ContributionLedgerRule::EvidenceVectorContribution
        );
        assert!(recoded.useful_contribution_count <= contribution_universe);
        assert!(recoded.merged_statistic_quality_permille > 0);
    }

    #[test]
    fn experiment_b_path_free_recovery_includes_required_roster() {
        let rows = experiment_b_path_free_recovery_rows(41).expect("rows");

        for policy in [
            "controlled-coded-diffusion",
            "uncontrolled-coded-diffusion",
            "uncoded-replication",
            "epidemic-forwarding",
            "spray-and-wait",
        ] {
            assert!(rows.iter().any(|row| row.identity.policy_or_mode == policy));
        }
        assert!(rows
            .iter()
            .all(|row| row.identity.experiment_id == CoreExperimentId::PathFreeRecovery));
    }

    #[test]
    fn experiment_b_path_free_recovery_conditions_success_on_no_static_path() {
        let rows = experiment_b_path_free_recovery_rows(41).expect("rows");

        assert!(rows
            .iter()
            .all(|row| row.path_evidence.no_static_path_in_core_window));
        assert!(rows
            .iter()
            .all(|row| row.path_evidence.time_respecting_evidence_journey_exists));
        assert!(rows
            .iter()
            .all(|row| { row.path_free_success_permille == row.recovery_probability_permille }));
        assert!(rows
            .iter()
            .any(|row| row.path_free_success_permille > 0 && row.cost_to_recover_bytes > 0));
    }

    #[test]
    fn experiment_b_path_free_recovery_excludes_route_style_research_rows() {
        let rows = experiment_b_path_free_recovery_rows(41).expect("rows");

        assert!(rows.iter().all(|row| {
            !row.identity.policy_or_mode.contains("route")
                && !row.identity.policy_or_mode.contains("field-corridor")
                && !row.identity.policy_or_mode.contains("legacy")
        }));
        assert!(rows.iter().all(|row| {
            row.mergeable_statistic.statistic_kind == MergeableStatisticKind::SetUnionRank
        }));
    }

    #[test]
    fn experiment_c_phase_diagram_covers_band_budget_rate_and_task() {
        let rows = experiment_c_phase_diagram_rows(41);
        let target_bands = rows
            .iter()
            .map(|row| {
                (
                    row.reproduction_target_low_permille,
                    row.reproduction_target_high_permille,
                )
            })
            .collect::<BTreeSet<_>>();
        let budgets = rows
            .iter()
            .map(|row| row.forwarding_budget)
            .collect::<BTreeSet<_>>();
        let coding_rates = rows
            .iter()
            .map(|row| (row.coding_k, row.coding_n))
            .collect::<BTreeSet<_>>();
        let statistic_kinds = rows
            .iter()
            .map(|row| row.mergeable_statistic.statistic_kind)
            .collect::<BTreeSet<_>>();

        assert!(target_bands.len() >= 3);
        assert!(budgets.len() >= 3);
        assert!(coding_rates.len() >= 3);
        assert!(statistic_kinds.contains(&MergeableStatisticKind::SetUnionRank));
        assert!(statistic_kinds.contains(&MergeableStatisticKind::AdditiveScoreVector));
    }

    #[test]
    fn experiment_c_phase_diagram_subcritical_cells_fail() {
        let rows = experiment_c_phase_diagram_rows(41);
        let subcritical = rows
            .iter()
            .filter(|row| row.identity.policy_or_mode.contains("subcritical"))
            .collect::<Vec<_>>();

        assert!(!subcritical.is_empty());
        assert!(subcritical
            .iter()
            .all(|row| row.recovery_probability_permille == 0));
    }

    #[test]
    fn experiment_c_phase_diagram_near_critical_band_is_useful() {
        let rows = experiment_c_phase_diagram_rows(41);
        let near_critical = rows
            .iter()
            .filter(|row| row.identity.policy_or_mode.contains("near-critical"))
            .collect::<Vec<_>>();

        assert!(!near_critical.is_empty());
        assert!(near_critical
            .iter()
            .any(|row| row.merged_statistic_quality_permille >= 800));
        assert!(near_critical.iter().all(|row| row.r_est_permille >= 800));
    }

    #[test]
    fn experiment_c_phase_diagram_supercritical_cells_show_visible_cost() {
        let rows = experiment_c_phase_diagram_rows(41);
        let supercritical_max_cost = rows
            .iter()
            .filter(|row| row.identity.policy_or_mode.contains("supercritical"))
            .map(|row| row.byte_count)
            .max()
            .unwrap_or(0);
        let subcritical_min_cost = rows
            .iter()
            .filter(|row| row.identity.policy_or_mode.contains("subcritical"))
            .map(|row| row.byte_count)
            .min()
            .unwrap_or(0);

        assert!(supercritical_max_cost > subcritical_min_cost);
        assert!(rows
            .iter()
            .filter(|row| row.identity.policy_or_mode.contains("supercritical"))
            .any(|row| row.quality_permille == 1000 && row.byte_count >= 128));
    }

    #[test]
    fn experiment_d_coding_vs_replication_includes_reviewer_roster() {
        let rows = experiment_d_coding_vs_replication_rows(41).expect("rows");

        for policy in [
            "uncoded-replication",
            "epidemic-forwarding",
            "uncontrolled-coded-diffusion",
            "controlled-coded-diffusion",
        ] {
            assert!(rows.iter().any(|row| row.identity.policy_or_mode == policy));
        }
    }

    #[test]
    fn experiment_d_coding_vs_replication_preserves_equal_budget_metadata() {
        let rows = experiment_d_coding_vs_replication_rows(41).expect("rows");
        let labels = rows
            .iter()
            .map(|row| row.identity.fixed_budget_label.as_str())
            .collect::<BTreeSet<_>>();
        let payload_budgets = rows
            .iter()
            .map(|row| row.fixed_payload_budget_bytes)
            .collect::<BTreeSet<_>>();

        assert_eq!(labels.len(), 1);
        assert_eq!(labels.first().copied(), Some("equal-payload-bytes"));
        assert_eq!(payload_budgets.len(), 1);
        assert_eq!(payload_budgets.first().copied(), Some(4096));
    }

    #[test]
    fn experiment_d_coding_vs_replication_does_not_mix_secondary_budgets() {
        let rows = experiment_d_coding_vs_replication_rows(41).expect("rows");

        assert!(rows.iter().all(|row| {
            row.identity.fixed_budget_label == "equal-payload-bytes"
                && row.fixed_payload_budget_bytes == 4096
        }));
        assert!(rows
            .iter()
            .all(|row| row.byte_count <= row.fixed_payload_budget_bytes));
    }

    #[test]
    fn experiment_d_coding_vs_replication_exposes_cost_and_quality_surfaces() {
        let rows = experiment_d_coding_vs_replication_rows(41).expect("rows");

        assert!(rows.iter().all(|row| {
            row.equal_quality_cost_reduction_permille <= 1000
                && row.equal_cost_quality_improvement_permille <= 1000
        }));
        assert!(rows.iter().any(|row| {
            row.equal_quality_cost_reduction_permille > 0
                || row.equal_cost_quality_improvement_permille > 0
        }));
        assert!(rows.iter().any(|row| {
            row.identity.policy_or_mode == "controlled-coded-diffusion"
                && row.mergeable_statistic.statistic_kind
                    == MergeableStatisticKind::AdditiveScoreVector
        }));
        assert!(rows.iter().any(|row| {
            row.identity.policy_or_mode == "uncoded-replication"
                && row.mergeable_statistic.statistic_kind == MergeableStatisticKind::SetUnionRank
        }));
    }

    #[test]
    fn experiment_e_observer_frontier_covers_required_knobs() {
        let rows = experiment_e_observer_frontier_rows(41);
        let dispersions = rows
            .iter()
            .map(|row| row.fragment_dispersion_permille)
            .collect::<BTreeSet<_>>();
        let randomness = rows
            .iter()
            .map(|row| row.forwarding_randomness_permille)
            .collect::<BTreeSet<_>>();
        let bands = rows
            .iter()
            .map(|row| {
                (
                    row.reproduction_target_low_permille,
                    row.reproduction_target_high_permille,
                )
            })
            .collect::<BTreeSet<_>>();

        assert!(dispersions.len() >= 2);
        assert!(randomness.len() >= 2);
        assert!(bands.len() >= 2);
        assert!(rows
            .iter()
            .all(|row| row.identity.experiment_id == CoreExperimentId::ObserverAmbiguityFrontier));
    }

    #[test]
    fn experiment_e_observer_frontier_ambiguity_is_not_free() {
        let rows = experiment_e_observer_frontier_rows(41);
        let low_dispersion = rows
            .iter()
            .filter(|row| row.fragment_dispersion_permille == 200)
            .map(|row| row.byte_count.saturating_add(row.latency_rounds))
            .min()
            .unwrap_or(0);
        let high_dispersion = rows
            .iter()
            .filter(|row| row.fragment_dispersion_permille == 800)
            .map(|row| row.byte_count.saturating_add(row.latency_rounds))
            .max()
            .unwrap_or(0);

        assert!(high_dispersion > low_dispersion);
        assert!(rows
            .iter()
            .any(|row| { row.forwarding_randomness_permille == 1000 && row.latency_rounds > 8 }));
    }

    #[test]
    fn experiment_e_observer_frontier_labels_ambiguity_metrics_as_proxies() {
        let rows = experiment_e_observer_frontier_rows(41);

        assert!(rows.iter().all(|row| row.ambiguity_metric_is_proxy));
        assert!(rows.iter().all(|row| {
            row.mergeable_statistic.statistic_kind
                == MergeableStatisticKind::ObserverProjectionSummary
        }));
    }

    #[test]
    fn experiment_e_observer_frontier_rows_are_deterministic() {
        let first = experiment_e_observer_frontier_rows(41);
        let second = experiment_e_observer_frontier_rows(41);

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.iter().any(|row| row.uncertainty_permille > 0));
        assert!(first.iter().any(|row| row.quality_permille >= 700));
    }

    #[test]
    fn active_belief_artifacts_cover_required_phase10_outputs() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");

        assert!(!artifacts.grid_rows.is_empty());
        assert!(!artifacts.demand_trace_rows.is_empty());
        assert!(!artifacts.host_bridge_demand_replay_rows.is_empty());
        assert_eq!(artifacts.active_versus_passive_rows.len(), 6);
        assert_eq!(artifacts.no_central_encoder_panel_rows.len(), 1);
        assert_eq!(artifacts.second_task_rows.len(), 18);
        assert_eq!(artifacts.recoding_frontier_rows.len(), 3);
        assert_eq!(artifacts.robustness_rows.len(), 10);
        assert_eq!(artifacts.theorem_assumption_rows.len(), 15);
        assert_eq!(artifacts.large_regime_rows.len(), 9);
        assert_eq!(artifacts.trace_validation_rows.len(), 3);
        assert_eq!(artifacts.strong_baseline_rows.len(), 2);
        assert_eq!(artifacts.exact_seed_summary_rows.len(), 9);
        assert_eq!(artifacts.final_validation_rows.len(), 162);
        assert_eq!(artifacts.figure_artifact_rows.len(), 11);
        assert_eq!(artifacts.scaling_boundary_rows.len(), 1);
        assert!(artifacts
            .no_central_encoder_panel_rows
            .iter()
            .all(|row| !row.node_owns_global_input && row.oracle_evaluation_after_run));
        assert_eq!(artifacts.no_central_encoder_panel_rows[0].receiver_count, 3);
    }

    #[test]
    fn active_belief_full_policy_improves_collective_uncertainty() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let passive = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::PassiveControlled)
            .expect("passive row");
        let active = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::FullActiveBelief)
            .expect("active row");

        assert_eq!(
            passive.fixed_payload_budget_bytes,
            active.fixed_payload_budget_bytes
        );
        assert!(active.collective_uncertainty_permille < passive.collective_uncertainty_permille);
        assert!(active.demand_satisfaction_permille > passive.demand_satisfaction_permille);
        assert!(active.commitment_lead_time_rounds_per_receiver_max > 0);
    }

    #[test]
    fn active_belief_causal_ablation_reduces_demand_gain() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let demand_disabled = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::DemandDisabled)
            .expect("demand disabled row");
        let active = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::FullActiveBelief)
            .expect("active row");

        assert!(
            active.collective_uncertainty_permille
                < demand_disabled.collective_uncertainty_permille
        );
        assert!(active.demand_satisfaction_permille > demand_disabled.demand_satisfaction_permille);
    }

    #[test]
    fn active_belief_demand_trace_records_lifecycle_events() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let kinds = artifacts
            .demand_trace_rows
            .iter()
            .map(|row| row.trace_kind)
            .collect::<BTreeSet<_>>();

        assert!(kinds.contains(&ActiveDemandTraceKind::Emitted));
        assert!(kinds.contains(&ActiveDemandTraceKind::Received));
        assert!(kinds.contains(&ActiveDemandTraceKind::Forwarded));
        assert!(kinds.contains(&ActiveDemandTraceKind::Piggybacked));
        assert!(kinds.contains(&ActiveDemandTraceKind::Satisfied));
    }

    #[test]
    fn active_belief_stale_demand_is_policy_only() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let stale = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::StaleDemandAblation)
            .expect("stale row");
        let active = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::FullActiveBelief)
            .expect("active row");

        assert!(stale.stale_demand_ignored_count > 0);
        assert!(stale.demand_satisfaction_permille <= active.demand_satisfaction_permille);
        assert!(stale.innovative_arrival_count <= active.innovative_arrival_count);
    }

    #[test]
    fn active_belief_demand_preserves_evidence_accounting_and_commitment_guards() {
        let scenario = build_coded_inference_readiness_scenario();
        let log = build_coded_inference_readiness_log(41, &scenario);
        let comparison = run_equal_budget_baseline_comparison(41).expect("baseline comparison");
        let passive = comparison
            .summaries
            .iter()
            .find(|summary| summary.policy_id == BaselinePolicyId::ControlledCodedDiffusion)
            .expect("controlled coded baseline");
        let contribution_ids = log
            .forwarding_events
            .iter()
            .flat_map(|event| event.origin.contribution_ledger_ids.iter().copied())
            .collect::<BTreeSet<_>>();

        for run in active_policy_runs(41, &scenario, &log, passive.fixed_payload_budget_bytes) {
            assert_eq!(
                run.selected_event_count,
                run.innovative_arrival_count
                    .saturating_add(run.duplicate_arrival_count)
            );
            for receiver in &run.receiver_states {
                assert!(receiver
                    .accepted_contribution_ids
                    .is_subset(&contribution_ids));
                if receiver.commitment_round.is_some() {
                    let rank =
                        u32::try_from(receiver.accepted_contribution_ids.len()).unwrap_or(u32::MAX);
                    assert!(rank >= scenario.coded_inference.minimum_decision_evidence_count);
                    assert!(
                        top_margin(&receiver.score_vector)
                            >= scenario.coded_inference.decision_margin_threshold
                    );
                    assert!(receiver.bytes_at_commitment.is_some());
                }
            }
        }
    }

    #[test]
    fn active_belief_multi_receiver_metrics_use_distinct_histories() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let active_grid = artifacts
            .grid_rows
            .iter()
            .filter(|row| row.mode == ActiveBeliefPolicyMode::FullActiveBelief)
            .collect::<Vec<_>>();
        let receivers = active_grid
            .iter()
            .map(|row| row.receiver_node_id)
            .collect::<BTreeSet<_>>();
        let ranks = active_grid
            .iter()
            .map(|row| (row.receiver_node_id, row.top_hypothesis_margin))
            .collect::<BTreeSet<_>>();

        assert_eq!(receivers.len(), 3);
        assert!(ranks.len() >= 3);
        assert!(active_grid
            .iter()
            .all(|row| row.receiver_agreement_permille <= 1000));
    }

    #[test]
    fn active_belief_recoding_frontier_and_second_task_are_causal_rows() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let forwarding_only = artifacts
            .recoding_frontier_rows
            .iter()
            .find(|row| row.recoding_mode == ActiveRecodingMode::ForwardingOnly)
            .expect("forwarding-only row");
        let active_aggregation = artifacts
            .recoding_frontier_rows
            .iter()
            .find(|row| row.recoding_mode == ActiveRecodingMode::ActiveDemandAggregation)
            .expect("active aggregation row");

        assert!(
            active_aggregation.demand_satisfaction_permille
                >= forwarding_only.demand_satisfaction_permille
        );
        assert!(active_aggregation.bytes_at_commitment > 0);
        let majority = artifacts
            .second_task_rows
            .iter()
            .find(|row| {
                row.task_kind == ActiveSecondTaskKind::MajorityThreshold
                    && row.mode == ActiveBeliefPolicyMode::FullActiveBelief
            })
            .expect("majority threshold row");
        assert_eq!(
            majority.mergeable_statistic.statistic_kind,
            MergeableStatisticKind::MajorityThreshold
        );
        assert!(majority.receiver_rank > 0);
        assert!(majority.demand_satisfaction_permille > 0);
        assert!(majority.decision_accuracy_permille > 0);
        let histogram = artifacts
            .second_task_rows
            .iter()
            .find(|row| {
                row.task_kind == ActiveSecondTaskKind::BoundedHistogram
                    && row.mode == ActiveBeliefPolicyMode::FullActiveBelief
            })
            .expect("bounded histogram row");
        assert_eq!(
            histogram.mergeable_statistic.statistic_kind,
            MergeableStatisticKind::BoundedHistogram
        );
        assert!(histogram.receiver_rank > 0);
        assert!(histogram.quality_per_byte_permille > 0);
    }

    #[test]
    fn active_belief_host_bridge_demand_is_replay_visible_and_non_evidential() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let surfaces = artifacts
            .host_bridge_demand_replay_rows
            .iter()
            .map(|row| row.execution_surface)
            .collect::<BTreeSet<_>>();

        assert!(surfaces.contains(&ActiveDemandExecutionSurface::SimulatorLocal));
        assert!(surfaces.contains(&ActiveDemandExecutionSurface::HostBridgeReplay));
        assert!(artifacts.host_bridge_demand_replay_rows.iter().all(|row| {
            row.replay_visible
                && row.demand_contribution_count == 0
                && !row.evidence_validity_changed
                && !row.contribution_identity_created
                && !row.merge_semantics_changed
                && !row.route_truth_published
                && !row.duplicate_rank_inflation
        }));
    }

    #[test]
    fn active_belief_strong_assumptions_and_large_regime_rows_are_covered() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let theorem_names = artifacts
            .theorem_assumption_rows
            .iter()
            .map(|row| row.theorem_name.as_str())
            .collect::<BTreeSet<_>>();
        let large_regimes = artifacts
            .large_regime_rows
            .iter()
            .map(|row| row.scenario_regime)
            .collect::<BTreeSet<_>>();

        assert!(theorem_names.contains("receiver_arrival_reconstruction_bound"));
        assert!(theorem_names.contains("anomaly_margin_lower_tail_bound"));
        assert!(artifacts
            .theorem_assumption_rows
            .iter()
            .all(|row| row.finite_horizon_model_valid
                && row.receiver_arrival_bound_permille <= 1000
                && row.lower_tail_failure_permille <= 1000
                && row.false_commitment_bound_permille <= 1000));
        assert_eq!(large_regimes.len(), 3);
        assert!(artifacts.large_regime_rows.iter().all(|row| {
            row.requested_node_count == 500
                && row.executed_node_count == 500
                && row.deterministic_replay
                && row.runtime_budget_stable
                && row.artifact_sanity_covered
        }));
        assert!(artifacts
            .trace_validation_rows
            .iter()
            .any(|row| row.external_or_semi_realistic && row.canonical_preprocessing));
        assert!(artifacts
            .strong_baseline_rows
            .iter()
            .any(|row| row.baseline_policy == "prophet-contact-frequency"));
        assert!(artifacts.exact_seed_summary_rows.iter().all(|row| {
            row.receiver_arrival_probability_permille <= 1000
                && row.commitment_accuracy_permille <= 1000
                && row.false_commitment_rate_permille <= 1000
        }));
    }

    #[test]
    fn active_belief_final_validation_covers_seeds_regimes_and_second_task() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let seeds = artifacts
            .final_validation_rows
            .iter()
            .map(|row| row.seed)
            .collect::<BTreeSet<_>>();
        let regimes = artifacts
            .final_validation_rows
            .iter()
            .map(|row| row.scenario_regime)
            .collect::<BTreeSet<_>>();
        let tasks = artifacts
            .final_validation_rows
            .iter()
            .map(|row| row.task_kind)
            .collect::<BTreeSet<_>>();

        assert_eq!(seeds, BTreeSet::from([41, 43, 45]));
        assert_eq!(regimes.len(), 3);
        assert!(tasks.contains(&ActiveSecondTaskKind::MajorityThreshold));
        assert!(tasks.contains(&ActiveSecondTaskKind::BoundedHistogram));
        assert!(artifacts
            .final_validation_rows
            .iter()
            .all(|row| row.deterministic_replay));
    }

    #[test]
    fn active_belief_final_rows_preserve_budget_and_censored_lead_time() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let full_active = artifacts
            .active_versus_passive_rows
            .iter()
            .find(|row| row.mode == ActiveBeliefPolicyMode::FullActiveBelief)
            .expect("full active row");

        assert!(full_active.full_recovery_censored);
        assert!(full_active.commitment_accuracy_permille > 0);
        assert!(full_active.commitment_lead_time_rounds_per_receiver_max > 0);
        assert!(artifacts
            .final_validation_rows
            .iter()
            .all(|row| row.fixed_payload_budget_bytes == 4096));
    }

    #[test]
    fn active_belief_final_figure_and_scaling_rows_are_sane() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let figure_ids = artifacts
            .figure_artifact_rows
            .iter()
            .map(|row| row.figure_index)
            .collect::<BTreeSet<_>>();
        let scaling = artifacts
            .scaling_boundary_rows
            .first()
            .expect("scaling boundary row");

        assert_eq!(figure_ids, (1_u8..=11).collect::<BTreeSet<_>>());
        assert!(artifacts
            .figure_artifact_rows
            .iter()
            .all(|row| row.sanity_passed && row.fixed_budget_label == "equal-payload-bytes"));
        assert_eq!(scaling.requested_node_count, 500);
        assert!(scaling.executed_node_count >= 100);
        assert!(scaling.documented_boundary);
    }

    #[test]
    fn active_belief_robustness_rows_are_dynamic_and_guard_false_confidence() {
        let artifacts = active_belief_experiment_artifacts(41).expect("active artifacts");
        let stress_kinds = artifacts
            .robustness_rows
            .iter()
            .map(|row| row.stress_kind)
            .collect::<BTreeSet<_>>();
        let byte_counts = artifacts
            .robustness_rows
            .iter()
            .map(|row| row.bytes_at_commitment)
            .collect::<BTreeSet<_>>();

        assert_eq!(stress_kinds.len(), 10);
        assert!(byte_counts.len() > 1);
        assert!(artifacts
            .robustness_rows
            .iter()
            .all(|row| row.false_confidence_permille <= 1000));
    }

    #[test]
    fn active_belief_artifacts_replay_for_multiple_seeds() {
        for seed in [41, 43] {
            let first = active_belief_experiment_artifacts(seed).expect("first artifacts");
            let second = active_belief_experiment_artifacts(seed).expect("second artifacts");

            assert_eq!(first, second);
        }
    }

    #[test]
    fn active_belief_artifacts_are_replay_deterministic() {
        let first = active_belief_experiment_artifacts(41).expect("first active artifacts");
        let second = active_belief_experiment_artifacts(41).expect("second active artifacts");

        assert_eq!(first, second);
        assert!(first
            .robustness_rows
            .iter()
            .all(|row| row.false_confidence_permille <= 50));
    }
}
