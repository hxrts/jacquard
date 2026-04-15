use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::experiments::ExperimentError;

mod execution;
mod posture;
mod scenarios;
mod scoring;
mod stats;

use execution::{
    aggregate_diffusion_runs, coverage_permille_for, is_target_node, is_terminal_target,
    node_by_id, scenario_target_cluster_count, simulate_diffusion_run,
    summarize_diffusion_boundaries,
};
use posture::{
    classify_field_transfer, compute_field_posture_signals, count_field_posture_round,
    covered_target_clusters, desired_field_posture, diffusion_bridge_candidate,
    diffusion_destination_cluster, diffusion_source_cluster, dominant_field_posture_name,
    field_budget_kind, field_forwarding_suppressed, holder_count_in_cluster, initial_field_budget,
    initial_field_posture, sender_energy_ratio_permille,
};
use scenarios::{
    build_adversarial_observation_scenario, build_bridge_drought_scenario,
    build_congestion_cascade_scenario, build_disaster_broadcast_scenario,
    build_energy_starved_relay_scenario, build_high_density_overload_scenario,
    build_mobility_shift_scenario, build_partitioned_clusters_scenario,
    build_random_waypoint_sanity_scenario, build_sparse_long_delay_scenario,
};
use scoring::forwarding_score;
use stats::{mean_option_u32, mean_u32, min_max_spread_u32, mode_option_string, mode_string};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionPolicyConfig {
    pub config_id: String,
    pub replication_budget: u32,
    #[serde(rename = "ttl_rounds")]
    pub message_horizon: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub target_cluster_bias_permille: i32,
    pub same_cluster_bias_permille: i32,
    pub observer_aversion_permille: i32,
    pub lora_bias_permille: i32,
    pub spread_restraint_permille: u32,
    pub energy_guard_permille: u32,
    pub forwarding_style: DiffusionForwardingStyle,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiffusionForwardingStyle {
    ConservativeLocal,
    BalancedDistanceVector,
    FreshnessAware,
    ServiceDirected,
    CorridorAware,
    Composite,
}

/// Diffusion-specific approximation of Field posture / regime control.
///
/// These postures let the diffusion simulator switch the `field` forwarding surface
/// between continuity-seeking and boundedness-protecting behavior instead of treating
/// Field as one static corridor-aware policy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionFieldPosture {
    /// Preserve corridor breadth and rare bridge optionality while delivery is
    /// still incomplete and boundedness pressure is tolerable.
    ContinuityBiased,
    /// Default middle posture when the engine can keep spreading without a
    /// stronger continuity, scarcity, congestion, or privacy signal.
    Balanced,
    /// Conserve transfer opportunities under low remaining energy or rising
    /// storage pressure by preferring fewer, cheaper continuations.
    ScarcityConservative,
    /// Spend protected budget on first-arrival target-cluster coverage before
    /// broad duplicate suppression takes over.
    ClusterSeeding,
    /// Suppress redundant spread once first-arrival cluster coverage is largely
    /// established and duplicate dissemination becomes the main risk.
    DuplicateSuppressed,
    /// Penalize observer-adjacent dissemination more aggressively when leakage
    /// signals dominate the local forwarding decision.
    PrivacyConservative,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionRegimeDescriptor {
    pub density: String,
    pub mobility_model: String,
    pub transport_mix: String,
    pub pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionMobilityProfile {
    Static,
    LocalMover,
    Bridger,
    LongRangeMover,
    Observer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionTransportKind {
    Ble,
    WifiAware,
    LoRa,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionMessageMode {
    Unicast,
    Broadcast,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionNodeSpec {
    pub node_id: u32,
    pub cluster_id: u8,
    pub mobility_profile: DiffusionMobilityProfile,
    pub energy_budget: u32,
    pub storage_capacity: u32,
    pub transport_capabilities: Vec<DiffusionTransportKind>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionScenarioSpec {
    pub family_id: String,
    pub regime: DiffusionRegimeDescriptor,
    pub round_count: u32,
    pub creation_round: u32,
    pub payload_bytes: u32,
    pub message_mode: DiffusionMessageMode,
    pub source_node_id: u32,
    pub destination_node_id: Option<u32>,
    pub nodes: Vec<DiffusionNodeSpec>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionRunSpec {
    pub suite_id: String,
    pub family_id: String,
    pub seed: u64,
    pub policy: DiffusionPolicyConfig,
    pub scenario: DiffusionScenarioSpec,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionContactEvent {
    pub round_index: u32,
    pub node_a: u32,
    pub node_b: u32,
    pub contact_window: u32,
    pub bandwidth_bytes: u32,
    pub transport_kind: DiffusionTransportKind,
    pub connection_delay: u32,
    pub energy_cost_per_byte: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionRunSummary {
    pub suite_id: String,
    pub family_id: String,
    pub config_id: String,
    pub seed: u64,
    pub density: String,
    pub mobility_model: String,
    pub transport_mix: String,
    pub pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
    pub replication_budget: u32,
    #[serde(rename = "ttl_rounds")]
    pub message_horizon: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub delivery_probability_permille: u32,
    #[serde(rename = "delivery_latency_rounds")]
    pub delivery_delay: Option<u32>,
    pub coverage_permille: u32,
    pub cluster_coverage_permille: u32,
    pub total_transmissions: u32,
    pub energy_spent_units: u32,
    pub energy_per_delivered_message: Option<u32>,
    pub storage_utilization_permille: u32,
    pub estimated_reproduction_permille: u32,
    pub corridor_persistence_permille: u32,
    pub decision_churn_count: u32,
    pub observer_leakage_permille: u32,
    pub bounded_state: String,
    pub message_persistence_rounds: u32,
    pub field_posture_mode: Option<String>,
    pub field_posture_transition_count: u32,
    pub field_continuity_biased_rounds: u32,
    pub field_balanced_rounds: u32,
    pub field_scarcity_conservative_rounds: u32,
    pub field_congestion_suppressed_rounds: u32,
    pub field_cluster_seeding_rounds: u32,
    pub field_duplicate_suppressed_rounds: u32,
    pub field_privacy_conservative_rounds: u32,
    pub field_first_scarcity_transition_round: Option<u32>,
    pub field_first_congestion_transition_round: Option<u32>,
    pub field_protected_budget_used: u32,
    pub field_generic_budget_used: u32,
    pub field_bridge_opportunity_count: u32,
    pub field_protected_bridge_usage_count: u32,
    pub field_cluster_seed_opportunity_count: u32,
    pub field_cluster_seed_usage_count: u32,
    pub field_cluster_coverage_starvation_count: u32,
    pub field_redundant_forward_suppression_count: u32,
    pub field_same_cluster_suppression_count: u32,
    pub field_expensive_transport_suppression_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionAggregateSummary {
    pub suite_id: String,
    pub family_id: String,
    pub config_id: String,
    pub density: String,
    pub mobility_model: String,
    pub transport_mix: String,
    pub pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
    pub replication_budget: u32,
    #[serde(rename = "ttl_rounds")]
    pub message_horizon: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub run_count: u32,
    pub delivery_probability_permille_mean: u32,
    pub delivery_probability_permille_min: u32,
    pub delivery_probability_permille_max: u32,
    pub delivery_probability_permille_spread: u32,
    #[serde(rename = "delivery_latency_rounds_mean")]
    pub delivery_delay_mean: Option<u32>,
    pub coverage_permille_mean: u32,
    pub cluster_coverage_permille_mean: u32,
    pub total_transmissions_mean: u32,
    pub energy_spent_units_mean: u32,
    pub energy_per_delivered_message_mean: Option<u32>,
    pub storage_utilization_permille_mean: u32,
    pub estimated_reproduction_permille_mean: u32,
    pub corridor_persistence_permille_mean: u32,
    pub decision_churn_count_mean: u32,
    pub observer_leakage_permille_mean: u32,
    pub message_persistence_rounds_mean: u32,
    pub bounded_state_mode: String,
    pub field_posture_mode: Option<String>,
    pub field_posture_transition_count_mean: u32,
    pub field_continuity_biased_rounds_mean: u32,
    pub field_balanced_rounds_mean: u32,
    pub field_scarcity_conservative_rounds_mean: u32,
    pub field_congestion_suppressed_rounds_mean: u32,
    pub field_cluster_seeding_rounds_mean: u32,
    pub field_duplicate_suppressed_rounds_mean: u32,
    pub field_privacy_conservative_rounds_mean: u32,
    pub field_first_scarcity_transition_round_mean: Option<u32>,
    pub field_first_congestion_transition_round_mean: Option<u32>,
    pub field_protected_budget_used_mean: u32,
    pub field_generic_budget_used_mean: u32,
    pub field_bridge_opportunity_count_mean: u32,
    pub field_protected_bridge_usage_count_mean: u32,
    pub field_cluster_seed_opportunity_count_mean: u32,
    pub field_cluster_seed_usage_count_mean: u32,
    pub field_cluster_coverage_starvation_count_mean: u32,
    pub field_redundant_forward_suppression_count_mean: u32,
    pub field_same_cluster_suppression_count_mean: u32,
    pub field_expensive_transport_suppression_count_mean: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionBoundarySummary {
    pub suite_id: String,
    pub config_id: String,
    pub viable_family_count: u32,
    pub first_collapse_family_id: Option<String>,
    pub first_collapse_stress_score: Option<u32>,
    pub first_explosive_family_id: Option<String>,
    pub first_explosive_stress_score: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionManifest {
    pub suite_id: String,
    pub run_count: u32,
    pub aggregate_count: u32,
    pub boundary_count: u32,
}

#[derive(Clone, Debug)]
pub struct DiffusionArtifacts {
    pub output_dir: PathBuf,
    pub manifest: DiffusionManifest,
    pub runs: Vec<DiffusionRunSummary>,
    pub aggregates: Vec<DiffusionAggregateSummary>,
    pub boundaries: Vec<DiffusionBoundarySummary>,
}

#[derive(Clone, Debug)]
pub struct DiffusionSuite {
    suite_id: String,
    runs: Vec<DiffusionRunSpec>,
}

impl DiffusionSuite {
    #[must_use]
    pub fn suite_id(&self) -> &str {
        &self.suite_id
    }
}

#[derive(Clone, Debug)]
struct PendingTransfer {
    arrival_round: u32,
    target_node_id: u32,
}

#[derive(Clone, Debug)]
struct HolderState {
    first_round: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct FieldPostureMetrics {
    transitions: u32,
    continuity_biased_rounds: u32,
    balanced_rounds: u32,
    scarcity_conservative_rounds: u32,
    cluster_seeding_rounds: u32,
    duplicate_suppressed_rounds: u32,
    privacy_conservative_rounds: u32,
    first_scarcity_transition_round: Option<u32>,
    first_congestion_transition_round: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
struct FieldPostureSignals {
    holder_count: usize,
    spread_growth: u32,
    remaining_energy_fraction_permille: u32,
    storage_pressure_permille: u32,
    recent_bridge_opportunity: bool,
    observer_exposure_permille: u32,
    delivery_progress_permille: u32,
    cluster_delivery_progress_permille: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct FieldBudgetState {
    protected_remaining: u32,
    generic_remaining: u32,
    protected_used: u32,
    generic_used: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct FieldExecutionMetrics {
    bridge_opportunity_count: u32,
    protected_bridge_usage_count: u32,
    cluster_seed_opportunity_count: u32,
    cluster_seed_usage_count: u32,
    cluster_coverage_starvation_count: u32,
    redundant_forward_suppression_count: u32,
    same_cluster_suppression_count: u32,
    expensive_transport_suppression_count: u32,
}

#[derive(Clone, Debug, Default)]
struct FieldSuppressionState {
    recent_cluster_forward_round: BTreeMap<u8, u32>,
    recent_same_cluster_forward_round: BTreeMap<u8, u32>,
    recent_corridor_forward_round: BTreeMap<(u8, u8), u32>,
}

#[derive(Clone, Copy, Debug)]
struct FieldTransferFeatures {
    from_cluster_id: u8,
    to_cluster_id: u8,
    receiver_is_target: bool,
    sender_is_observer: bool,
    receiver_is_observer: bool,
    same_cluster: bool,
    new_cluster_coverage: bool,
    expensive_transport: bool,
    continuity_value: bool,
    protected_opportunity: bool,
}

#[derive(Clone, Copy)]
struct ForwardingGeometry {
    toward_destination_cluster: bool,
    leaving_source_cluster: bool,
    bridge_candidate: bool,
}

#[derive(Clone, Copy)]
struct ForwardingOpportunity<'a> {
    scenario: &'a DiffusionScenarioSpec,
    contact: &'a DiffusionContactEvent,
}

#[derive(Clone, Copy)]
struct ForwardingNodes<'a> {
    from_node: &'a DiffusionNodeSpec,
    to_node: &'a DiffusionNodeSpec,
}

type FieldProfileOverrides = (u32, u32, u32, u32, i32, i32, i32, i32, u32, u32);
type DiffusionPolicySpec = (
    u32,
    u32,
    u32,
    u32,
    i32,
    i32,
    i32,
    i32,
    u32,
    u32,
    DiffusionForwardingStyle,
);

#[derive(Clone, Copy)]
struct ForwardingScoreContext<'a> {
    opp: ForwardingOpportunity<'a>,
    policy: &'a DiffusionPolicyConfig,
    nodes: ForwardingNodes<'a>,
    holder_count: usize,
    geometry: ForwardingGeometry,
    field_features: Option<&'a FieldTransferFeatures>,
}

#[derive(Clone, Copy, Debug)]
enum FieldBudgetKind {
    Target,
    Protected,
    Generic,
}

#[must_use]
pub fn diffusion_smoke_suite() -> DiffusionSuite {
    build_diffusion_suite("diffusion-smoke", &[41], true)
}

#[must_use]
pub fn diffusion_local_suite() -> DiffusionSuite {
    build_diffusion_suite("diffusion-local", &[41, 43, 47, 53], false)
}

pub fn run_diffusion_suite(
    suite: &DiffusionSuite,
    output_dir: &Path,
) -> Result<DiffusionArtifacts, ExperimentError> {
    fs::create_dir_all(output_dir)?;
    let mut runs = Vec::new();
    let run_path = output_dir.join("diffusion_runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);
    for spec in &suite.runs {
        let summary = simulate_diffusion_run(spec);
        serde_json::to_writer(&mut writer, &summary)?;
        writer.write_all(b"\n")?;
        runs.push(summary);
    }
    writer.flush()?;
    let aggregates = aggregate_diffusion_runs(&runs);
    let boundaries = summarize_diffusion_boundaries(&aggregates);
    let manifest = DiffusionManifest {
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

fn build_diffusion_suite(suite_id: &str, seeds: &[u64], _smoke: bool) -> DiffusionSuite {
    let mut configs = vec![
        diffusion_engine_profile("batman-bellman"),
        diffusion_engine_profile("batman-classic"),
        diffusion_engine_profile("babel"),
        diffusion_engine_profile("olsrv2"),
        diffusion_engine_profile("scatter"),
        diffusion_engine_profile("pathway"),
        diffusion_engine_profile("pathway-batman-bellman"),
    ];
    configs.extend(field_diffusion_profiles());
    let scenarios = vec![
        build_random_waypoint_sanity_scenario(),
        build_partitioned_clusters_scenario(),
        build_disaster_broadcast_scenario(),
        build_sparse_long_delay_scenario(),
        build_high_density_overload_scenario(),
        build_mobility_shift_scenario(),
        build_adversarial_observation_scenario(),
        build_bridge_drought_scenario(),
        build_energy_starved_relay_scenario(),
        build_congestion_cascade_scenario(),
    ];
    let mut runs = Vec::new();
    for seed in seeds {
        for scenario in &scenarios {
            for policy in &configs {
                runs.push(DiffusionRunSpec {
                    suite_id: suite_id.to_string(),
                    family_id: scenario.family_id.clone(),
                    seed: *seed,
                    policy: policy.clone(),
                    scenario: scenario.clone(),
                });
            }
        }
    }
    DiffusionSuite {
        suite_id: suite_id.to_string(),
        runs,
    }
}

fn field_diffusion_profiles() -> Vec<DiffusionPolicyConfig> {
    let mut variants = vec![("field".to_string(), "field".to_string(), None)];
    let search_templates: [(&str, &str, [FieldProfileOverrides; 4]); 4] = [
        (
            "field-continuity",
            "field-continuity",
            [
                (4, 34, 455, 360, 190, -10, 180, 140, 140, 120),
                (3, 28, 420, 340, 180, 30, 210, 80, 190, 150),
                (5, 38, 470, 390, 210, -30, 170, 160, 110, 110),
                (4, 32, 440, 350, 210, 0, 190, 120, 150, 130),
            ],
        ),
        (
            "field-scarcity",
            "field-scarcity",
            [
                (2, 20, 320, 210, 190, 95, 220, -90, 320, 260),
                (2, 18, 290, 230, 210, 110, 240, -120, 360, 300),
                (1, 16, 260, 220, 220, 130, 250, -150, 420, 340),
                (2, 18, 300, 250, 230, 120, 220, -140, 390, 320),
            ],
        ),
        (
            "field-congestion",
            "field-congestion",
            [
                (2, 18, 300, 150, 120, 140, 210, -120, 360, 240),
                (8, 26, 520, 180, 150, 20, 200, 0, 140, 120),
                (6, 22, 380, 170, 170, 120, 210, -80, 280, 190),
                (7, 24, 440, 210, 210, 50, 190, -20, 200, 130),
            ],
        ),
        (
            "field-privacy",
            "field-privacy",
            [
                (2, 22, 320, 210, 160, 90, 360, -40, 260, 220),
                (2, 22, 310, 200, 160, 100, 360, -40, 260, 220),
                (2, 24, 330, 240, 190, 70, 420, -80, 280, 240),
                (3, 24, 360, 250, 200, 40, 440, -120, 260, 210),
            ],
        ),
    ];
    for (base_id, prefix, overrides) in search_templates {
        append_field_profile_variants(&mut variants, base_id, prefix, &overrides);
    }
    variants
        .into_iter()
        .map(field_profile_from_variant)
        .collect()
}

fn append_field_profile_variants(
    variants: &mut Vec<(String, String, Option<FieldProfileOverrides>)>,
    base_id: &str,
    prefix: &str,
    overrides: &[FieldProfileOverrides],
) {
    variants.push((base_id.to_string(), prefix.to_string(), None));
    for (index, override_set) in overrides.iter().copied().enumerate() {
        variants.push((
            base_id.to_string(),
            format!("{prefix}-search-{}", index + 1),
            Some(override_set),
        ));
    }
}

fn field_profile_from_variant(
    (base_id, config_id, overrides): (String, String, Option<FieldProfileOverrides>),
) -> DiffusionPolicyConfig {
    let mut profile = diffusion_engine_profile(&base_id);
    profile.config_id = config_id;
    if let Some(overrides) = overrides {
        apply_field_profile_overrides(&mut profile, overrides);
    }
    profile
}

fn apply_field_profile_overrides(
    profile: &mut DiffusionPolicyConfig,
    (
        replication_budget,
        message_horizon,
        forward_probability_permille,
        bridge_bias_permille,
        target_cluster_bias_permille,
        same_cluster_bias_permille,
        observer_aversion_permille,
        lora_bias_permille,
        spread_restraint_permille,
        energy_guard_permille,
    ): FieldProfileOverrides,
) {
    profile.replication_budget = replication_budget;
    profile.message_horizon = message_horizon;
    profile.forward_probability_permille = forward_probability_permille;
    profile.bridge_bias_permille = bridge_bias_permille;
    profile.target_cluster_bias_permille = target_cluster_bias_permille;
    profile.same_cluster_bias_permille = same_cluster_bias_permille;
    profile.observer_aversion_permille = observer_aversion_permille;
    profile.lora_bias_permille = lora_bias_permille;
    profile.spread_restraint_permille = spread_restraint_permille;
    profile.energy_guard_permille = energy_guard_permille;
}

fn diffusion_policy_profile(
    config_id: &str,
    (
        replication_budget,
        message_horizon,
        forward_probability_permille,
        bridge_bias_permille,
        target_cluster_bias_permille,
        same_cluster_bias_permille,
        observer_aversion_permille,
        lora_bias_permille,
        spread_restraint_permille,
        energy_guard_permille,
        forwarding_style,
    ): DiffusionPolicySpec,
) -> DiffusionPolicyConfig {
    DiffusionPolicyConfig {
        config_id: config_id.to_string(),
        replication_budget,
        message_horizon,
        forward_probability_permille,
        bridge_bias_permille,
        target_cluster_bias_permille,
        same_cluster_bias_permille,
        observer_aversion_permille,
        lora_bias_permille,
        spread_restraint_permille,
        energy_guard_permille,
        forwarding_style,
    }
}

// long-block-exception: the diffusion engine profile catalog is maintained as a
// single tuning surface so per-engine defaults remain auditable in one place.
fn diffusion_engine_profile(engine_set: &str) -> DiffusionPolicyConfig {
    match engine_set {
        "batman-bellman" => diffusion_policy_profile(
            "batman-bellman",
            (
                3,
                20,
                380,
                80,
                90,
                45,
                130,
                -80,
                180,
                140,
                DiffusionForwardingStyle::BalancedDistanceVector,
            ),
        ),
        "batman-classic" => diffusion_policy_profile(
            "batman-classic",
            (
                2,
                24,
                320,
                60,
                80,
                90,
                150,
                -120,
                240,
                190,
                DiffusionForwardingStyle::ConservativeLocal,
            ),
        ),
        "babel" => diffusion_policy_profile(
            "babel",
            (
                3,
                22,
                430,
                90,
                105,
                25,
                120,
                -40,
                140,
                120,
                DiffusionForwardingStyle::FreshnessAware,
            ),
        ),
        "olsrv2" => diffusion_policy_profile(
            "olsrv2",
            (
                3,
                24,
                400,
                110,
                120,
                20,
                130,
                0,
                150,
                130,
                DiffusionForwardingStyle::FreshnessAware,
            ),
        ),
        "pathway" => diffusion_policy_profile(
            "pathway",
            (
                5,
                20,
                540,
                180,
                170,
                -50,
                90,
                40,
                90,
                80,
                DiffusionForwardingStyle::ServiceDirected,
            ),
        ),
        "scatter" => diffusion_policy_profile(
            "scatter",
            (
                4,
                28,
                470,
                260,
                150,
                -20,
                180,
                80,
                170,
                140,
                DiffusionForwardingStyle::ConservativeLocal,
            ),
        ),
        "field" => diffusion_policy_profile(
            "field",
            (
                3,
                26,
                430,
                240,
                150,
                35,
                190,
                40,
                180,
                150,
                DiffusionForwardingStyle::CorridorAware,
            ),
        ),
        "field-continuity" => diffusion_policy_profile(
            "field-continuity",
            (
                4,
                34,
                460,
                360,
                190,
                -10,
                180,
                140,
                140,
                120,
                DiffusionForwardingStyle::CorridorAware,
            ),
        ),
        "field-scarcity" => diffusion_policy_profile(
            "field-scarcity",
            (
                2,
                20,
                330,
                220,
                200,
                100,
                220,
                -90,
                320,
                260,
                DiffusionForwardingStyle::CorridorAware,
            ),
        ),
        "field-congestion" => diffusion_policy_profile(
            "field-congestion",
            (
                2,
                18,
                300,
                160,
                130,
                140,
                200,
                -120,
                360,
                240,
                DiffusionForwardingStyle::CorridorAware,
            ),
        ),
        "field-privacy" => diffusion_policy_profile(
            "field-privacy",
            (
                2,
                22,
                320,
                210,
                160,
                90,
                360,
                -40,
                260,
                220,
                DiffusionForwardingStyle::CorridorAware,
            ),
        ),
        "pathway-batman-bellman" => diffusion_policy_profile(
            "pathway-batman-bellman",
            (
                6,
                24,
                560,
                180,
                150,
                10,
                100,
                20,
                70,
                60,
                DiffusionForwardingStyle::Composite,
            ),
        ),
        _ => diffusion_policy_profile(
            engine_set,
            (
                4,
                24,
                450,
                120,
                100,
                0,
                100,
                0,
                120,
                120,
                DiffusionForwardingStyle::BalancedDistanceVector,
            ),
        ),
    }
}
