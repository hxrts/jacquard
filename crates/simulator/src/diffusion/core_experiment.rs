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
pub(crate) enum MergeableStatisticKind {
    SetUnionRank,
    AdditiveScoreVector,
    ObserverProjectionSummary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum MergeOperationKind {
    SetUnion,
    VectorAddition,
    ProjectionAggregation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum ContributionLedgerRule {
    CanonicalContributionLedger,
    EvidenceVectorContribution,
    ProjectionErasure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DecisionMapKind {
    ReconstructionThreshold,
    TopHypothesisMargin,
    AttackerAdvantage,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum QualityMapKind {
    ReceiverRank,
    LandscapeUncertainty,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContactEdge {
    pub round_index: u32,
    pub node_a: u32,
    pub node_b: u32,
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
            accumulator,
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
    accumulator: OriginModeAccumulator,
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
}
