//! Diffusion execution engine with round-level state management and transfer scoring.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use rayon::prelude::*;

use crate::experiments::ExperimentError;

pub(super) use super::model::{
    DiffusionAggregateSummary, DiffusionArtifacts, DiffusionBoundarySummary, DiffusionContactEvent,
    DiffusionFieldPosture, DiffusionMessageMode, DiffusionMobilityProfile, DiffusionNodeSpec,
    DiffusionPairDescriptor, DiffusionPolicyConfig, DiffusionRunSpec, DiffusionRunSummary,
    DiffusionScenarioSpec, DiffusionSuite, DiffusionTransportKind,
};
pub(super) use super::posture::{
    classify_field_transfer, compute_field_posture_signals, count_field_posture_round,
    covered_target_clusters, desired_field_posture, dominant_field_posture_name, field_budget_kind,
    field_forwarding_suppressed, holder_count_in_cluster, initial_field_budget,
    initial_field_posture, sender_energy_ratio_permille,
};
pub(super) use super::scoring::forwarding_score;
pub(super) use super::stats::{
    mean_option_u32, mean_u32, min_max_spread_u32, mode_option_string, mode_string,
};

pub(super) mod execution;

#[derive(Clone, Debug)]
pub(super) struct PendingTransfer {
    pub arrival_round: u32,
    pub target_node_id: u32,
}

#[derive(Clone, Debug)]
pub(super) struct HolderState {
    pub first_round: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FieldPostureMetrics {
    pub transitions: u32,
    pub continuity_biased_rounds: u32,
    pub balanced_rounds: u32,
    pub scarcity_conservative_rounds: u32,
    pub cluster_seeding_rounds: u32,
    pub duplicate_suppressed_rounds: u32,
    pub privacy_conservative_rounds: u32,
    pub first_scarcity_transition_round: Option<u32>,
    pub first_congestion_transition_round: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct FieldPostureSignals {
    pub holder_count: usize,
    pub spread_growth: u32,
    pub remaining_energy_fraction_permille: u32,
    pub storage_pressure_permille: u32,
    pub recent_bridge_opportunity: bool,
    pub observer_exposure_permille: u32,
    pub delivery_progress_permille: u32,
    pub cluster_delivery_progress_permille: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FieldBudgetState {
    pub protected_remaining: u32,
    pub generic_remaining: u32,
    pub protected_used: u32,
    pub generic_used: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FieldExecutionMetrics {
    pub bridge_opportunity_count: u32,
    pub protected_bridge_usage_count: u32,
    pub cluster_seed_opportunity_count: u32,
    pub cluster_seed_usage_count: u32,
    pub cluster_coverage_starvation_count: u32,
    pub redundant_forward_suppression_count: u32,
    pub same_cluster_suppression_count: u32,
    pub expensive_transport_suppression_count: u32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FieldSuppressionState {
    pub recent_cluster_forward_round: BTreeMap<u8, u32>,
    pub recent_same_cluster_forward_round: BTreeMap<u8, u32>,
    pub recent_cluster_pair_forward_round: BTreeMap<(u8, u8), u32>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct FieldTransferFeatures {
    pub from_cluster_id: u8,
    pub to_cluster_id: u8,
    pub receiver_is_target: bool,
    pub sender_is_observer: bool,
    pub receiver_is_observer: bool,
    pub same_cluster: bool,
    pub new_cluster_coverage: bool,
    pub expensive_transport: bool,
    pub continuity_value: bool,
    pub protected_opportunity: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ForwardingGeometry {
    pub toward_destination_cluster: bool,
    pub leaving_source_cluster: bool,
    pub bridge_candidate: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ForwardingOpportunity<'a> {
    pub scenario: &'a DiffusionScenarioSpec,
    pub contact: &'a DiffusionContactEvent,
}

#[derive(Clone, Copy)]
pub(super) struct ForwardingNodes<'a> {
    pub from_node: &'a DiffusionNodeSpec,
    pub to_node: &'a DiffusionNodeSpec,
}

#[derive(Clone, Copy)]
pub(super) struct ForwardingScoreContext<'a> {
    pub opp: ForwardingOpportunity<'a>,
    pub policy: &'a super::model::DiffusionPolicyConfig,
    pub nodes: ForwardingNodes<'a>,
    pub holder_count: usize,
    pub geometry: ForwardingGeometry,
    pub field_features: Option<&'a FieldTransferFeatures>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum FieldBudgetKind {
    Target,
    Protected,
    Generic,
}

#[cfg(test)]
pub(super) fn execute_diffusion_runs_serial(suite: &DiffusionSuite) -> Vec<DiffusionRunSummary> {
    suite.runs.iter().map(simulate_diffusion_run).collect()
}

pub(super) fn execute_diffusion_runs_parallel(suite: &DiffusionSuite) -> Vec<DiffusionRunSummary> {
    let mut indexed = suite
        .runs
        .par_iter()
        .enumerate()
        .map(|(index, spec)| (index, simulate_diffusion_run(spec)))
        .collect::<Vec<_>>();
    indexed.sort_by_key(|(index, _)| *index);
    indexed.into_iter().map(|(_, summary)| summary).collect()
}

pub fn run_diffusion_suite(
    suite: &DiffusionSuite,
    output_dir: &Path,
) -> Result<DiffusionArtifacts, ExperimentError> {
    fs::create_dir_all(output_dir)?;
    let runs = execute_diffusion_runs_parallel(suite);
    let run_path = output_dir.join("diffusion_runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);
    for summary in &runs {
        serde_json::to_writer(&mut writer, summary)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    let aggregates = aggregate_diffusion_runs(&runs);
    let boundaries = summarize_diffusion_boundaries(&aggregates);
    let manifest = super::model::DiffusionManifest {
        schema_version: super::model::DIFFUSION_ARTIFACT_SCHEMA_VERSION,
        suite_id: suite.suite_id.clone(),
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        boundary_count: u32::try_from(boundaries.len()).unwrap_or(u32::MAX),
    };
    serde_json::to_writer_pretty(
        File::create(output_dir.join("diffusion_manifest.json"))?,
        &manifest,
    )?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("diffusion_aggregates.json"))?,
        &aggregates,
    )?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("diffusion_boundaries.json"))?,
        &boundaries,
    )?;
    Ok(DiffusionArtifacts {
        output_dir: output_dir.to_path_buf(),
        manifest,
        runs,
        aggregates,
        boundaries,
    })
}

pub(super) fn simulate_diffusion_run(spec: &DiffusionRunSpec) -> DiffusionRunSummary {
    execution::simulate_diffusion_run(spec)
}

#[must_use]
pub fn aggregate_diffusion_runs(runs: &[DiffusionRunSummary]) -> Vec<DiffusionAggregateSummary> {
    execution::aggregate_diffusion_runs(runs)
}

#[must_use]
pub fn summarize_diffusion_boundaries(
    aggregates: &[DiffusionAggregateSummary],
) -> Vec<DiffusionBoundarySummary> {
    execution::summarize_diffusion_boundaries(aggregates)
}

pub(super) fn coverage_permille_for(target_count: usize, delivered_count: usize) -> u32 {
    execution::coverage_permille_for(target_count, delivered_count)
}

pub(super) fn scenario_target_cluster_count(scenario: &DiffusionScenarioSpec) -> usize {
    execution::scenario_target_cluster_count(scenario)
}

pub(super) fn is_target_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    execution::is_target_node(scenario, node_id)
}

pub(super) fn is_terminal_target(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    execution::is_terminal_target(scenario, node_id)
}

pub(super) fn node_by_id(
    scenario: &DiffusionScenarioSpec,
    node_id: u32,
) -> Option<&DiffusionNodeSpec> {
    execution::node_by_id(scenario, node_id)
}
