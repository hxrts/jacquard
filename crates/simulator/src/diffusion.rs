use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::experiments::ExperimentError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionPolicyConfig {
    pub config_id: String,
    pub replication_budget: u32,
    pub ttl_rounds: u32,
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
    /// Suppress redundant spread once holder count and storage pressure imply
    /// that broad broadcast is becoming the main risk.
    CongestionSuppressed,
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
    pub duration_rounds: u32,
    pub bandwidth_bytes: u32,
    pub transport_kind: DiffusionTransportKind,
    pub connection_latency_rounds: u32,
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
    pub ttl_rounds: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub delivery_probability_permille: u32,
    pub delivery_latency_rounds: Option<u32>,
    pub coverage_permille: u32,
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
    pub field_privacy_conservative_rounds: u32,
    pub field_first_scarcity_transition_round: Option<u32>,
    pub field_first_congestion_transition_round: Option<u32>,
    pub field_protected_budget_used: u32,
    pub field_generic_budget_used: u32,
    pub field_bridge_opportunity_count: u32,
    pub field_protected_bridge_usage_count: u32,
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
    pub ttl_rounds: u32,
    pub forward_probability_permille: u32,
    pub bridge_bias_permille: u32,
    pub run_count: u32,
    pub delivery_probability_permille_mean: u32,
    pub delivery_latency_rounds_mean: Option<u32>,
    pub coverage_permille_mean: u32,
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
    pub field_privacy_conservative_rounds_mean: u32,
    pub field_first_scarcity_transition_round_mean: Option<u32>,
    pub field_first_congestion_transition_round_mean: Option<u32>,
    pub field_protected_budget_used_mean: u32,
    pub field_generic_budget_used_mean: u32,
    pub field_bridge_opportunity_count_mean: u32,
    pub field_protected_bridge_usage_count_mean: u32,
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
    congestion_suppressed_rounds: u32,
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
    expensive_transport: bool,
    continuity_value: bool,
    protected_opportunity: bool,
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
    build_diffusion_suite("diffusion-local", &[41, 43], false)
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
    let variants = [
        ("field", "field", None),
        (
            "field",
            "field-balanced-low-churn",
            Some((3, 24, 430, 220, 140, 20, 190, 40, 180, 150)),
        ),
        ("field-continuity", "field-continuity", None),
        (
            "field-continuity",
            "field-continuity-reserve",
            Some((4, 34, 455, 360, 190, -10, 180, 140, 140, 120)),
        ),
        (
            "field-continuity",
            "field-continuity-tight",
            Some((3, 28, 420, 340, 180, 30, 210, 80, 190, 150)),
        ),
        ("field-scarcity", "field-scarcity", None),
        (
            "field-scarcity",
            "field-scarcity-cheap",
            Some((2, 20, 320, 210, 190, 95, 220, -90, 320, 260)),
        ),
        (
            "field-scarcity",
            "field-scarcity-hardcap",
            Some((2, 18, 290, 230, 210, 110, 240, -120, 360, 300)),
        ),
        ("field-congestion", "field-congestion", None),
        (
            "field-congestion",
            "field-congestion-tight",
            Some((2, 18, 300, 150, 120, 140, 210, -120, 360, 240)),
        ),
        (
            "field-congestion",
            "field-congestion-memory",
            Some((5, 24, 540, 180, 150, 20, 200, 0, 140, 120)),
        ),
        ("field-privacy", "field-privacy", None),
        (
            "field-privacy",
            "field-privacy-tight",
            Some((2, 22, 310, 200, 160, 100, 360, -40, 260, 220)),
        ),
    ];
    variants
        .into_iter()
        .map(|(base_id, config_id, overrides)| {
            let mut profile = diffusion_engine_profile(base_id);
            profile.config_id = config_id.to_string();
            if let Some((
                replication_budget,
                ttl_rounds,
                forward_probability_permille,
                bridge_bias_permille,
                target_cluster_bias_permille,
                same_cluster_bias_permille,
                observer_aversion_permille,
                lora_bias_permille,
                spread_restraint_permille,
                energy_guard_permille,
            )) = overrides
            {
                profile.replication_budget = replication_budget;
                profile.ttl_rounds = ttl_rounds;
                profile.forward_probability_permille = forward_probability_permille;
                profile.bridge_bias_permille = bridge_bias_permille;
                profile.target_cluster_bias_permille = target_cluster_bias_permille;
                profile.same_cluster_bias_permille = same_cluster_bias_permille;
                profile.observer_aversion_permille = observer_aversion_permille;
                profile.lora_bias_permille = lora_bias_permille;
                profile.spread_restraint_permille = spread_restraint_permille;
                profile.energy_guard_permille = energy_guard_permille;
            }
            profile
        })
        .collect()
}

fn diffusion_engine_profile(engine_set: &str) -> DiffusionPolicyConfig {
    match engine_set {
        "batman-bellman" => DiffusionPolicyConfig {
            config_id: "batman-bellman".to_string(),
            replication_budget: 3,
            ttl_rounds: 20,
            forward_probability_permille: 380,
            bridge_bias_permille: 80,
            target_cluster_bias_permille: 90,
            same_cluster_bias_permille: 45,
            observer_aversion_permille: 130,
            lora_bias_permille: -80,
            spread_restraint_permille: 180,
            energy_guard_permille: 140,
            forwarding_style: DiffusionForwardingStyle::BalancedDistanceVector,
        },
        "batman-classic" => DiffusionPolicyConfig {
            config_id: "batman-classic".to_string(),
            replication_budget: 2,
            ttl_rounds: 24,
            forward_probability_permille: 320,
            bridge_bias_permille: 60,
            target_cluster_bias_permille: 80,
            same_cluster_bias_permille: 90,
            observer_aversion_permille: 150,
            lora_bias_permille: -120,
            spread_restraint_permille: 240,
            energy_guard_permille: 190,
            forwarding_style: DiffusionForwardingStyle::ConservativeLocal,
        },
        "babel" => DiffusionPolicyConfig {
            config_id: "babel".to_string(),
            replication_budget: 3,
            ttl_rounds: 22,
            forward_probability_permille: 430,
            bridge_bias_permille: 90,
            target_cluster_bias_permille: 105,
            same_cluster_bias_permille: 25,
            observer_aversion_permille: 120,
            lora_bias_permille: -40,
            spread_restraint_permille: 140,
            energy_guard_permille: 120,
            forwarding_style: DiffusionForwardingStyle::FreshnessAware,
        },
        "pathway" => DiffusionPolicyConfig {
            config_id: "pathway".to_string(),
            replication_budget: 5,
            ttl_rounds: 20,
            forward_probability_permille: 540,
            bridge_bias_permille: 180,
            target_cluster_bias_permille: 170,
            same_cluster_bias_permille: -50,
            observer_aversion_permille: 90,
            lora_bias_permille: 40,
            spread_restraint_permille: 90,
            energy_guard_permille: 80,
            forwarding_style: DiffusionForwardingStyle::ServiceDirected,
        },
        "field" => DiffusionPolicyConfig {
            config_id: "field".to_string(),
            replication_budget: 3,
            ttl_rounds: 26,
            forward_probability_permille: 430,
            bridge_bias_permille: 240,
            target_cluster_bias_permille: 150,
            same_cluster_bias_permille: 35,
            observer_aversion_permille: 190,
            lora_bias_permille: 40,
            spread_restraint_permille: 180,
            energy_guard_permille: 150,
            forwarding_style: DiffusionForwardingStyle::CorridorAware,
        },
        "field-continuity" => DiffusionPolicyConfig {
            config_id: "field-continuity".to_string(),
            replication_budget: 4,
            ttl_rounds: 34,
            forward_probability_permille: 460,
            bridge_bias_permille: 360,
            target_cluster_bias_permille: 190,
            same_cluster_bias_permille: -10,
            observer_aversion_permille: 180,
            lora_bias_permille: 140,
            spread_restraint_permille: 140,
            energy_guard_permille: 120,
            forwarding_style: DiffusionForwardingStyle::CorridorAware,
        },
        "field-scarcity" => DiffusionPolicyConfig {
            config_id: "field-scarcity".to_string(),
            replication_budget: 2,
            ttl_rounds: 20,
            forward_probability_permille: 330,
            bridge_bias_permille: 220,
            target_cluster_bias_permille: 200,
            same_cluster_bias_permille: 100,
            observer_aversion_permille: 220,
            lora_bias_permille: -90,
            spread_restraint_permille: 320,
            energy_guard_permille: 260,
            forwarding_style: DiffusionForwardingStyle::CorridorAware,
        },
        "field-congestion" => DiffusionPolicyConfig {
            config_id: "field-congestion".to_string(),
            replication_budget: 2,
            ttl_rounds: 18,
            forward_probability_permille: 300,
            bridge_bias_permille: 160,
            target_cluster_bias_permille: 130,
            same_cluster_bias_permille: 140,
            observer_aversion_permille: 200,
            lora_bias_permille: -120,
            spread_restraint_permille: 360,
            energy_guard_permille: 240,
            forwarding_style: DiffusionForwardingStyle::CorridorAware,
        },
        "field-privacy" => DiffusionPolicyConfig {
            config_id: "field-privacy".to_string(),
            replication_budget: 2,
            ttl_rounds: 22,
            forward_probability_permille: 320,
            bridge_bias_permille: 210,
            target_cluster_bias_permille: 160,
            same_cluster_bias_permille: 90,
            observer_aversion_permille: 360,
            lora_bias_permille: -40,
            spread_restraint_permille: 260,
            energy_guard_permille: 220,
            forwarding_style: DiffusionForwardingStyle::CorridorAware,
        },
        "pathway-batman-bellman" => DiffusionPolicyConfig {
            config_id: "pathway-batman-bellman".to_string(),
            replication_budget: 6,
            ttl_rounds: 24,
            forward_probability_permille: 560,
            bridge_bias_permille: 180,
            target_cluster_bias_permille: 150,
            same_cluster_bias_permille: 10,
            observer_aversion_permille: 100,
            lora_bias_permille: 20,
            spread_restraint_permille: 70,
            energy_guard_permille: 60,
            forwarding_style: DiffusionForwardingStyle::Composite,
        },
        _ => DiffusionPolicyConfig {
            config_id: engine_set.to_string(),
            replication_budget: 4,
            ttl_rounds: 24,
            forward_probability_permille: 450,
            bridge_bias_permille: 120,
            target_cluster_bias_permille: 100,
            same_cluster_bias_permille: 0,
            observer_aversion_permille: 100,
            lora_bias_permille: 0,
            spread_restraint_permille: 120,
            energy_guard_permille: 120,
            forwarding_style: DiffusionForwardingStyle::BalancedDistanceVector,
        },
    }
}

fn simulate_diffusion_run(spec: &DiffusionRunSpec) -> DiffusionRunSummary {
    let scenario = &spec.scenario;
    let policy = &spec.policy;
    let target_count = scenario_target_count(scenario);
    let field_posture_enabled =
        policy.config_id.starts_with("field") && policy.config_id != "field-static";
    let mut holders = BTreeMap::new();
    let mut remaining_energy = scenario
        .nodes
        .iter()
        .map(|node| (node.node_id, node.energy_budget))
        .collect::<BTreeMap<_, _>>();
    let mut pending = Vec::<PendingTransfer>::new();
    holders.insert(
        scenario.source_node_id,
        HolderState {
            first_round: scenario.creation_round,
        },
    );
    let mut delivered_targets = BTreeSet::new();
    let mut delivery_rounds = Vec::<u32>::new();
    let mut copy_budget_remaining = policy.replication_budget;
    let mut total_transmissions = 0_u32;
    let mut total_energy = 0_u32;
    let mut peak_holders = 1_u32;
    let mut round_new_copies = Vec::<u32>::new();
    let mut edge_flows = BTreeMap::<(u32, u32), u32>::new();
    let mut dominant_edge_by_round = Vec::<Option<(u32, u32)>>::new();
    let mut observer_touches = 0_u32;
    let mut field_posture = if field_posture_enabled {
        Some(initial_field_posture(scenario, policy))
    } else {
        None
    };
    let mut field_posture_metrics = FieldPostureMetrics::default();
    let mut field_pending_posture: Option<DiffusionFieldPosture> = None;
    let mut field_pending_rounds = 0_u32;
    let mut field_budget_state =
        field_posture_enabled.then(|| initial_field_budget(policy, scenario));
    let mut field_execution_metrics = FieldExecutionMetrics::default();
    let mut field_suppression_state = FieldSuppressionState::default();

    for round in 0..scenario.round_count {
        let mut arrivals = Vec::new();
        pending.retain(|transfer| {
            if transfer.arrival_round <= round {
                arrivals.push(transfer.target_node_id);
                false
            } else {
                true
            }
        });
        let mut new_copies_this_round = 0_u32;
        for node_id in arrivals {
            if holders.contains_key(&node_id) {
                continue;
            }
            holders.insert(node_id, HolderState { first_round: round });
            new_copies_this_round = new_copies_this_round.saturating_add(1);
            if is_target_node(scenario, node_id) {
                delivered_targets.insert(node_id);
                delivery_rounds.push(round.saturating_sub(scenario.creation_round));
            }
        }

        if round < scenario.creation_round
            || round > scenario.creation_round.saturating_add(policy.ttl_rounds)
        {
            round_new_copies.push(new_copies_this_round);
            dominant_edge_by_round.push(None);
            continue;
        }

        let contacts = generate_contacts(spec.seed, scenario, round);
        if let Some(current_posture) = field_posture {
            let posture_signals = compute_field_posture_signals(
                scenario,
                &holders,
                &remaining_energy,
                &contacts,
                target_count,
                delivered_targets.len(),
                *round_new_copies.last().unwrap_or(&0),
                total_transmissions,
                observer_touches,
            );
            let desired_posture = desired_field_posture(scenario, &posture_signals);
            if desired_posture == current_posture {
                field_pending_posture = None;
                field_pending_rounds = 0;
            } else if field_pending_posture == Some(desired_posture) {
                field_pending_rounds = field_pending_rounds.saturating_add(1);
                if field_pending_rounds >= 2 {
                    field_posture = Some(desired_posture);
                    field_pending_posture = None;
                    field_pending_rounds = 0;
                    field_posture_metrics.transitions =
                        field_posture_metrics.transitions.saturating_add(1);
                    match desired_posture {
                        DiffusionFieldPosture::ScarcityConservative => {
                            field_posture_metrics
                                .first_scarcity_transition_round
                                .get_or_insert(round);
                        }
                        DiffusionFieldPosture::CongestionSuppressed => {
                            field_posture_metrics
                                .first_congestion_transition_round
                                .get_or_insert(round);
                        }
                        _ => {}
                    }
                }
            } else {
                field_pending_posture = Some(desired_posture);
                field_pending_rounds = 1;
            }
            count_field_posture_round(
                &mut field_posture_metrics,
                field_posture.unwrap_or(current_posture),
            );
        }
        let mut round_edge_counts = BTreeMap::<(u32, u32), u32>::new();
        for contact in contacts {
            for (from, to) in [
                (contact.node_a, contact.node_b),
                (contact.node_b, contact.node_a),
            ] {
                if !holders.contains_key(&from) || holders.contains_key(&to) {
                    continue;
                }
                let Some(receiver_node) = node_by_id(scenario, to) else {
                    continue;
                };
                if scenario.payload_bytes > receiver_node.storage_capacity {
                    continue;
                }
                let transfer_energy = scenario
                    .payload_bytes
                    .saturating_mul(contact.energy_cost_per_byte);
                let sender_energy = remaining_energy.get(&from).copied().unwrap_or(0);
                if sender_energy < transfer_energy {
                    continue;
                }
                let field_features = if field_posture_enabled {
                    classify_field_transfer(scenario, from, to, &contact)
                } else {
                    None
                };
                let field_budget_kind = if let Some(features) = field_features.as_ref() {
                    if features.protected_opportunity && !features.receiver_is_target {
                        field_execution_metrics.bridge_opportunity_count = field_execution_metrics
                            .bridge_opportunity_count
                            .saturating_add(1);
                    }
                    let Some(budget_state) = field_budget_state.as_ref() else {
                        continue;
                    };
                    let Some(budget_kind) = field_budget_kind(features, budget_state) else {
                        if let Some(posture) = field_posture {
                            if matches!(
                                posture,
                                DiffusionFieldPosture::ScarcityConservative
                                    | DiffusionFieldPosture::CongestionSuppressed
                            ) && !features.receiver_is_target
                            {
                                field_execution_metrics.redundant_forward_suppression_count =
                                    field_execution_metrics
                                        .redundant_forward_suppression_count
                                        .saturating_add(1);
                            }
                        }
                        continue;
                    };
                    let Some(from_node) = node_by_id(scenario, from) else {
                        continue;
                    };
                    let sender_energy_ratio =
                        sender_energy_ratio_permille(from_node, sender_energy);
                    let receiver_cluster_holders = holder_count_in_cluster(
                        scenario,
                        &holders,
                        &pending,
                        features.to_cluster_id,
                    );
                    if let Some(posture) = field_posture {
                        if field_forwarding_suppressed(
                            posture,
                            round,
                            holders.len(),
                            receiver_cluster_holders,
                            sender_energy_ratio,
                            features,
                            &field_suppression_state,
                            &mut field_execution_metrics,
                        ) {
                            continue;
                        }
                    }
                    Some(budget_kind)
                } else {
                    let allow_budget =
                        copy_budget_remaining > 0 || is_terminal_target(scenario, to);
                    if !allow_budget {
                        continue;
                    }
                    None
                };
                let score = forwarding_score(
                    scenario,
                    policy,
                    from,
                    to,
                    &contact,
                    holders.len(),
                    sender_energy,
                    field_posture,
                );
                if score
                    <= permille_hash(spec.seed, scenario.family_id.as_str(), round, from, to, 0)
                {
                    continue;
                }
                if contact.bandwidth_bytes < scenario.payload_bytes {
                    continue;
                }
                if let Some(entry) = remaining_energy.get_mut(&from) {
                    *entry = entry.saturating_sub(transfer_energy);
                }
                total_transmissions = total_transmissions.saturating_add(1);
                total_energy = total_energy.saturating_add(transfer_energy);
                if is_observer_node(scenario, from)
                    || (is_observer_node(scenario, to) && !is_terminal_target(scenario, to))
                {
                    observer_touches = observer_touches.saturating_add(1);
                }
                let edge = normalized_edge(from, to);
                *edge_flows.entry(edge).or_insert(0) += 1;
                *round_edge_counts.entry(edge).or_insert(0) += 1;
                let arrival_round = round.saturating_add(contact.connection_latency_rounds);
                if arrival_round <= scenario.creation_round.saturating_add(policy.ttl_rounds) {
                    pending.push(PendingTransfer {
                        arrival_round,
                        target_node_id: to,
                    });
                }
                if let (Some(features), Some(budget_kind), Some(budget_state)) = (
                    field_features.as_ref(),
                    field_budget_kind,
                    field_budget_state.as_mut(),
                ) {
                    record_field_forward(
                        round,
                        budget_kind,
                        features,
                        budget_state,
                        &mut field_suppression_state,
                        &mut field_execution_metrics,
                    );
                } else if !is_terminal_target(scenario, to) && copy_budget_remaining > 0 {
                    copy_budget_remaining -= 1;
                }
            }
        }
        dominant_edge_by_round.push(
            round_edge_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(edge, _)| edge),
        );
        peak_holders = peak_holders.max(u32::try_from(holders.len()).unwrap_or(u32::MAX));
        round_new_copies.push(new_copies_this_round);
    }

    let delivery_probability_permille = match scenario.message_mode {
        DiffusionMessageMode::Unicast => {
            if delivered_targets.contains(&scenario.destination_node_id.unwrap_or_default()) {
                1000
            } else {
                0
            }
        }
        DiffusionMessageMode::Broadcast => {
            coverage_permille_for(target_count, delivered_targets.len())
        }
    };
    let coverage_permille = coverage_permille_for(target_count, delivered_targets.len());
    let energy_per_delivered_message = if delivered_targets.is_empty() {
        None
    } else {
        Some(total_energy / u32::try_from(delivered_targets.len()).unwrap_or(1))
    };
    let delivery_latency_rounds = if delivery_rounds.is_empty() {
        None
    } else {
        Some(
            delivery_rounds.iter().copied().sum::<u32>()
                / u32::try_from(delivery_rounds.len()).unwrap_or(1),
        )
    };
    let total_storage_capacity = scenario
        .nodes
        .iter()
        .map(|node| node.storage_capacity)
        .sum::<u32>();
    let storage_utilization_permille = if total_storage_capacity == 0 {
        0
    } else {
        peak_holders
            .saturating_mul(scenario.payload_bytes)
            .saturating_mul(1000)
            / total_storage_capacity
    };
    let holders_before_contact = round_new_copies
        .iter()
        .fold((0_u32, 0_u32), |(holders_so_far, sum), new_copies| {
            let updated_sum = sum.saturating_add(holders_so_far);
            (holders_so_far.saturating_add(*new_copies), updated_sum)
        })
        .1
        .saturating_add(1);
    let estimated_reproduction_permille = if holders_before_contact == 0 {
        0
    } else {
        u32::try_from((u64::from(total_transmissions) * 1000) / u64::from(holders_before_contact))
            .unwrap_or(u32::MAX)
    };
    let corridor_persistence_permille = if total_transmissions == 0 {
        0
    } else {
        edge_flows
            .values()
            .copied()
            .max()
            .unwrap_or(0)
            .saturating_mul(1000)
            / total_transmissions
    };
    let decision_churn_count = dominant_edge_by_round
        .windows(2)
        .filter(|window| window[0].is_some() && window[1].is_some() && window[0] != window[1])
        .count() as u32;
    let observer_leakage_permille = if total_transmissions == 0 {
        0
    } else {
        observer_touches.saturating_mul(1000) / total_transmissions
    };
    let bounded_state = bounded_state(
        delivery_probability_permille,
        coverage_permille,
        estimated_reproduction_permille,
        total_transmissions,
        storage_utilization_permille,
        energy_per_delivered_message,
    )
    .to_string();
    let message_persistence_rounds = if holders.is_empty() {
        0
    } else {
        scenario.round_count.saturating_sub(
            holders
                .values()
                .map(|holder| holder.first_round)
                .min()
                .unwrap_or(scenario.round_count),
        )
    };

    DiffusionRunSummary {
        suite_id: spec.suite_id.clone(),
        family_id: spec.family_id.clone(),
        config_id: policy.config_id.clone(),
        seed: spec.seed,
        density: scenario.regime.density.clone(),
        mobility_model: scenario.regime.mobility_model.clone(),
        transport_mix: scenario.regime.transport_mix.clone(),
        pressure: scenario.regime.pressure.clone(),
        objective_regime: scenario.regime.objective_regime.clone(),
        stress_score: scenario.regime.stress_score,
        replication_budget: policy.replication_budget,
        ttl_rounds: policy.ttl_rounds,
        forward_probability_permille: policy.forward_probability_permille,
        bridge_bias_permille: policy.bridge_bias_permille,
        delivery_probability_permille,
        delivery_latency_rounds,
        coverage_permille,
        total_transmissions,
        energy_spent_units: total_energy,
        energy_per_delivered_message,
        storage_utilization_permille,
        estimated_reproduction_permille,
        corridor_persistence_permille,
        decision_churn_count,
        observer_leakage_permille,
        bounded_state,
        message_persistence_rounds,
        field_posture_mode: dominant_field_posture_name(&field_posture_metrics),
        field_posture_transition_count: field_posture_metrics.transitions,
        field_continuity_biased_rounds: field_posture_metrics.continuity_biased_rounds,
        field_balanced_rounds: field_posture_metrics.balanced_rounds,
        field_scarcity_conservative_rounds: field_posture_metrics.scarcity_conservative_rounds,
        field_congestion_suppressed_rounds: field_posture_metrics.congestion_suppressed_rounds,
        field_privacy_conservative_rounds: field_posture_metrics.privacy_conservative_rounds,
        field_first_scarcity_transition_round: field_posture_metrics
            .first_scarcity_transition_round,
        field_first_congestion_transition_round: field_posture_metrics
            .first_congestion_transition_round,
        field_protected_budget_used: field_budget_state
            .as_ref()
            .map(|budget| budget.protected_used)
            .unwrap_or(0),
        field_generic_budget_used: field_budget_state
            .as_ref()
            .map(|budget| budget.generic_used)
            .unwrap_or(0),
        field_bridge_opportunity_count: field_execution_metrics.bridge_opportunity_count,
        field_protected_bridge_usage_count: field_execution_metrics.protected_bridge_usage_count,
        field_redundant_forward_suppression_count: field_execution_metrics
            .redundant_forward_suppression_count,
        field_same_cluster_suppression_count: field_execution_metrics
            .same_cluster_suppression_count,
        field_expensive_transport_suppression_count: field_execution_metrics
            .expensive_transport_suppression_count,
    }
}

fn aggregate_diffusion_runs(runs: &[DiffusionRunSummary]) -> Vec<DiffusionAggregateSummary> {
    let mut grouped = BTreeMap::<(String, String), Vec<&DiffusionRunSummary>>::new();
    for run in runs {
        grouped
            .entry((run.family_id.clone(), run.config_id.clone()))
            .or_default()
            .push(run);
    }
    let mut aggregates = Vec::new();
    for ((_family_id, _config_id), group) in grouped {
        let first = group[0];
        let run_count = u32::try_from(group.len()).unwrap_or(u32::MAX);
        let mode = mode_string(group.iter().map(|row| row.bounded_state.clone()));
        aggregates.push(DiffusionAggregateSummary {
            suite_id: first.suite_id.clone(),
            family_id: first.family_id.clone(),
            config_id: first.config_id.clone(),
            density: first.density.clone(),
            mobility_model: first.mobility_model.clone(),
            transport_mix: first.transport_mix.clone(),
            pressure: first.pressure.clone(),
            objective_regime: first.objective_regime.clone(),
            stress_score: first.stress_score,
            replication_budget: first.replication_budget,
            ttl_rounds: first.ttl_rounds,
            forward_probability_permille: first.forward_probability_permille,
            bridge_bias_permille: first.bridge_bias_permille,
            run_count,
            delivery_probability_permille_mean: mean_u32(
                group.iter().map(|row| row.delivery_probability_permille),
            ),
            delivery_latency_rounds_mean: mean_option_u32(
                group.iter().map(|row| row.delivery_latency_rounds),
            ),
            coverage_permille_mean: mean_u32(group.iter().map(|row| row.coverage_permille)),
            total_transmissions_mean: mean_u32(group.iter().map(|row| row.total_transmissions)),
            energy_spent_units_mean: mean_u32(group.iter().map(|row| row.energy_spent_units)),
            energy_per_delivered_message_mean: mean_option_u32(
                group.iter().map(|row| row.energy_per_delivered_message),
            ),
            storage_utilization_permille_mean: mean_u32(
                group.iter().map(|row| row.storage_utilization_permille),
            ),
            estimated_reproduction_permille_mean: mean_u32(
                group.iter().map(|row| row.estimated_reproduction_permille),
            ),
            corridor_persistence_permille_mean: mean_u32(
                group.iter().map(|row| row.corridor_persistence_permille),
            ),
            decision_churn_count_mean: mean_u32(group.iter().map(|row| row.decision_churn_count)),
            observer_leakage_permille_mean: mean_u32(
                group.iter().map(|row| row.observer_leakage_permille),
            ),
            message_persistence_rounds_mean: mean_u32(
                group.iter().map(|row| row.message_persistence_rounds),
            ),
            bounded_state_mode: mode,
            field_posture_mode: mode_option_string(
                group.iter().map(|row| row.field_posture_mode.clone()),
            ),
            field_posture_transition_count_mean: mean_u32(
                group.iter().map(|row| row.field_posture_transition_count),
            ),
            field_continuity_biased_rounds_mean: mean_u32(
                group.iter().map(|row| row.field_continuity_biased_rounds),
            ),
            field_balanced_rounds_mean: mean_u32(group.iter().map(|row| row.field_balanced_rounds)),
            field_scarcity_conservative_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_scarcity_conservative_rounds),
            ),
            field_congestion_suppressed_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_congestion_suppressed_rounds),
            ),
            field_privacy_conservative_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_privacy_conservative_rounds),
            ),
            field_first_scarcity_transition_round_mean: mean_option_u32(
                group
                    .iter()
                    .map(|row| row.field_first_scarcity_transition_round),
            ),
            field_first_congestion_transition_round_mean: mean_option_u32(
                group
                    .iter()
                    .map(|row| row.field_first_congestion_transition_round),
            ),
            field_protected_budget_used_mean: mean_u32(
                group.iter().map(|row| row.field_protected_budget_used),
            ),
            field_generic_budget_used_mean: mean_u32(
                group.iter().map(|row| row.field_generic_budget_used),
            ),
            field_bridge_opportunity_count_mean: mean_u32(
                group.iter().map(|row| row.field_bridge_opportunity_count),
            ),
            field_protected_bridge_usage_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_protected_bridge_usage_count),
            ),
            field_redundant_forward_suppression_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_redundant_forward_suppression_count),
            ),
            field_same_cluster_suppression_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_same_cluster_suppression_count),
            ),
            field_expensive_transport_suppression_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_expensive_transport_suppression_count),
            ),
        });
    }
    aggregates.sort_by(|left, right| {
        left.family_id.cmp(&right.family_id).then(
            left.delivery_probability_permille_mean
                .cmp(&right.delivery_probability_permille_mean)
                .reverse(),
        )
    });
    aggregates
}

fn summarize_diffusion_boundaries(
    aggregates: &[DiffusionAggregateSummary],
) -> Vec<DiffusionBoundarySummary> {
    let mut grouped = BTreeMap::<String, Vec<&DiffusionAggregateSummary>>::new();
    for aggregate in aggregates {
        grouped
            .entry(aggregate.config_id.clone())
            .or_default()
            .push(aggregate);
    }
    let mut rows = Vec::new();
    for (config_id, mut group) in grouped {
        group.sort_by_key(|row| row.stress_score);
        let viable_family_count = u32::try_from(
            group
                .iter()
                .filter(|row| row.bounded_state_mode == "viable")
                .count(),
        )
        .unwrap_or(u32::MAX);
        let collapse = group
            .iter()
            .find(|row| row.bounded_state_mode == "collapse");
        let explosive = group
            .iter()
            .find(|row| row.bounded_state_mode == "explosive");
        rows.push(DiffusionBoundarySummary {
            suite_id: group[0].suite_id.clone(),
            config_id,
            viable_family_count,
            first_collapse_family_id: collapse.map(|row| row.family_id.clone()),
            first_collapse_stress_score: collapse.map(|row| row.stress_score),
            first_explosive_family_id: explosive.map(|row| row.family_id.clone()),
            first_explosive_stress_score: explosive.map(|row| row.stress_score),
        });
    }
    rows.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    rows
}

fn initial_field_posture(
    scenario: &DiffusionScenarioSpec,
    policy: &DiffusionPolicyConfig,
) -> DiffusionFieldPosture {
    match scenario.family_id.as_str() {
        "diffusion-bridge-drought"
        | "diffusion-partitioned-clusters"
        | "diffusion-sparse-long-delay"
        | "diffusion-mobility-shift" => DiffusionFieldPosture::ContinuityBiased,
        "diffusion-energy-starved-relay" if policy.config_id.starts_with("field-scarcity") => {
            DiffusionFieldPosture::ScarcityConservative
        }
        "diffusion-congestion-cascade" | "diffusion-high-density-overload"
            if policy.config_id.starts_with("field-congestion") =>
        {
            DiffusionFieldPosture::CongestionSuppressed
        }
        "diffusion-adversarial-observation" if policy.config_id.starts_with("field-privacy") => {
            DiffusionFieldPosture::PrivacyConservative
        }
        "diffusion-high-density-overload"
        | "diffusion-congestion-cascade"
        | "diffusion-energy-starved-relay"
        | "diffusion-adversarial-observation" => DiffusionFieldPosture::Balanced,
        _ => DiffusionFieldPosture::Balanced,
    }
}

fn compute_field_posture_signals(
    scenario: &DiffusionScenarioSpec,
    holders: &BTreeMap<u32, HolderState>,
    remaining_energy: &BTreeMap<u32, u32>,
    contacts: &[DiffusionContactEvent],
    target_count: usize,
    delivered_target_count: usize,
    spread_growth: u32,
    total_transmissions: u32,
    observer_touches: u32,
) -> FieldPostureSignals {
    let total_budget = scenario
        .nodes
        .iter()
        .map(|node| node.energy_budget)
        .sum::<u32>();
    let remaining_budget = remaining_energy.values().copied().sum::<u32>();
    let remaining_energy_fraction_permille = if total_budget == 0 {
        0
    } else {
        remaining_budget.saturating_mul(1000) / total_budget
    };
    let total_storage_capacity = scenario
        .nodes
        .iter()
        .map(|node| node.storage_capacity)
        .sum::<u32>();
    let storage_pressure_permille = if total_storage_capacity == 0 {
        0
    } else {
        u32::try_from(holders.len())
            .unwrap_or(u32::MAX)
            .saturating_mul(scenario.payload_bytes)
            .saturating_mul(1000)
            / total_storage_capacity
    };
    let recent_bridge_opportunity = contacts.iter().any(|contact| {
        let left = node_by_id(scenario, contact.node_a);
        let right = node_by_id(scenario, contact.node_b);
        match (left, right) {
            (Some(left), Some(right)) => {
                left.cluster_id != right.cluster_id
                    && (matches!(
                        left.mobility_profile,
                        DiffusionMobilityProfile::Bridger
                            | DiffusionMobilityProfile::LongRangeMover
                    ) || matches!(
                        right.mobility_profile,
                        DiffusionMobilityProfile::Bridger
                            | DiffusionMobilityProfile::LongRangeMover
                    ))
            }
            _ => false,
        }
    });
    let observer_exposure_permille = if total_transmissions == 0 {
        0
    } else {
        observer_touches.saturating_mul(1000) / total_transmissions
    };
    let delivery_progress_permille = coverage_permille_for(target_count, delivered_target_count);
    FieldPostureSignals {
        holder_count: holders.len(),
        spread_growth,
        remaining_energy_fraction_permille,
        storage_pressure_permille,
        recent_bridge_opportunity,
        observer_exposure_permille,
        delivery_progress_permille,
    }
}

fn desired_field_posture(
    scenario: &DiffusionScenarioSpec,
    signals: &FieldPostureSignals,
) -> DiffusionFieldPosture {
    if signals.observer_exposure_permille >= 180
        || (scenario.family_id == "diffusion-adversarial-observation"
            && signals.observer_exposure_permille >= 90)
    {
        return DiffusionFieldPosture::PrivacyConservative;
    }
    if signals.storage_pressure_permille >= 560
        || signals.holder_count >= 6
        || matches!(
            scenario.family_id.as_str(),
            "diffusion-high-density-overload" | "diffusion-congestion-cascade"
        ) && (signals.holder_count >= 4
            || signals.spread_growth >= 1
            || signals.storage_pressure_permille >= 340)
    {
        return DiffusionFieldPosture::CongestionSuppressed;
    }
    if signals.remaining_energy_fraction_permille <= 520
        || (scenario.family_id == "diffusion-energy-starved-relay"
            && (signals.remaining_energy_fraction_permille <= 920
                || signals.holder_count >= 2
                || signals.spread_growth >= 1))
        || (scenario.family_id == "diffusion-energy-starved-relay"
            && signals.storage_pressure_permille >= 180)
    {
        return DiffusionFieldPosture::ScarcityConservative;
    }
    if matches!(
        scenario.family_id.as_str(),
        "diffusion-bridge-drought"
            | "diffusion-partitioned-clusters"
            | "diffusion-sparse-long-delay"
            | "diffusion-mobility-shift"
    ) && (signals.delivery_progress_permille < 1000
        || signals.recent_bridge_opportunity
        || scenario.family_id == "diffusion-bridge-drought")
    {
        return DiffusionFieldPosture::ContinuityBiased;
    }
    DiffusionFieldPosture::Balanced
}

fn count_field_posture_round(metrics: &mut FieldPostureMetrics, posture: DiffusionFieldPosture) {
    match posture {
        DiffusionFieldPosture::ContinuityBiased => {
            metrics.continuity_biased_rounds = metrics.continuity_biased_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::Balanced => {
            metrics.balanced_rounds = metrics.balanced_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::ScarcityConservative => {
            metrics.scarcity_conservative_rounds =
                metrics.scarcity_conservative_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::CongestionSuppressed => {
            metrics.congestion_suppressed_rounds =
                metrics.congestion_suppressed_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::PrivacyConservative => {
            metrics.privacy_conservative_rounds =
                metrics.privacy_conservative_rounds.saturating_add(1)
        }
    }
}

fn field_posture_name(posture: DiffusionFieldPosture) -> String {
    match posture {
        DiffusionFieldPosture::ContinuityBiased => "continuity_biased".to_string(),
        DiffusionFieldPosture::Balanced => "balanced".to_string(),
        DiffusionFieldPosture::ScarcityConservative => "scarcity_conservative".to_string(),
        DiffusionFieldPosture::CongestionSuppressed => "congestion_suppressed".to_string(),
        DiffusionFieldPosture::PrivacyConservative => "privacy_conservative".to_string(),
    }
}

fn dominant_field_posture_name(metrics: &FieldPostureMetrics) -> Option<String> {
    let candidates = [
        (
            metrics.continuity_biased_rounds,
            0_u8,
            DiffusionFieldPosture::ContinuityBiased,
        ),
        (
            metrics.balanced_rounds,
            1_u8,
            DiffusionFieldPosture::Balanced,
        ),
        (
            metrics.scarcity_conservative_rounds,
            2_u8,
            DiffusionFieldPosture::ScarcityConservative,
        ),
        (
            metrics.congestion_suppressed_rounds,
            3_u8,
            DiffusionFieldPosture::CongestionSuppressed,
        ),
        (
            metrics.privacy_conservative_rounds,
            4_u8,
            DiffusionFieldPosture::PrivacyConservative,
        ),
    ];
    candidates
        .into_iter()
        .max_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)))
        .and_then(|(rounds, _, posture)| {
            if rounds == 0 {
                None
            } else {
                Some(field_posture_name(posture))
            }
        })
}

fn initial_field_budget(
    policy: &DiffusionPolicyConfig,
    scenario: &DiffusionScenarioSpec,
) -> FieldBudgetState {
    let base_protected = if matches!(scenario.message_mode, DiffusionMessageMode::Unicast) {
        1
    } else {
        0
    };
    let continuity_reserved = if matches!(
        scenario.family_id.as_str(),
        "diffusion-bridge-drought"
            | "diffusion-partitioned-clusters"
            | "diffusion-sparse-long-delay"
            | "diffusion-mobility-shift"
    ) {
        1
    } else {
        0
    };
    let mut protected_remaining =
        (base_protected + continuity_reserved).min(policy.replication_budget);
    if protected_remaining == policy.replication_budget && policy.replication_budget > 1 {
        protected_remaining = protected_remaining.saturating_sub(1);
    }
    FieldBudgetState {
        protected_remaining,
        generic_remaining: policy
            .replication_budget
            .saturating_sub(protected_remaining),
        protected_used: 0,
        generic_used: 0,
    }
}

fn sender_energy_ratio_permille(
    from_node: &DiffusionNodeSpec,
    sender_energy_remaining: u32,
) -> u32 {
    if from_node.energy_budget == 0 {
        0
    } else {
        sender_energy_remaining.saturating_mul(1000) / from_node.energy_budget
    }
}

fn classify_field_transfer(
    scenario: &DiffusionScenarioSpec,
    from: u32,
    to: u32,
    contact: &DiffusionContactEvent,
) -> Option<FieldTransferFeatures> {
    let from_node = node_by_id(scenario, from)?;
    let to_node = node_by_id(scenario, to)?;
    let source_cluster = node_by_id(scenario, scenario.source_node_id).map(|node| node.cluster_id);
    let destination_cluster = scenario
        .destination_node_id
        .and_then(|destination| node_by_id(scenario, destination))
        .map(|node| node.cluster_id);
    let receiver_is_target = is_terminal_target(scenario, to);
    let same_cluster = from_node.cluster_id == to_node.cluster_id;
    let toward_destination_cluster = destination_cluster == Some(to_node.cluster_id);
    let leaving_source_cluster =
        source_cluster == Some(from_node.cluster_id) && source_cluster != Some(to_node.cluster_id);
    let bridge_candidate = matches!(
        to_node.mobility_profile,
        DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover
    );
    let continuity_value = receiver_is_target
        || toward_destination_cluster
        || bridge_candidate
        || leaving_source_cluster;
    let protected_opportunity = receiver_is_target
        || toward_destination_cluster
        || (!same_cluster && bridge_candidate)
        || (leaving_source_cluster
            && matches!(
                scenario.family_id.as_str(),
                "diffusion-bridge-drought"
                    | "diffusion-partitioned-clusters"
                    | "diffusion-sparse-long-delay"
                    | "diffusion-mobility-shift"
            ));
    Some(FieldTransferFeatures {
        from_cluster_id: from_node.cluster_id,
        to_cluster_id: to_node.cluster_id,
        receiver_is_target,
        sender_is_observer: matches!(
            from_node.mobility_profile,
            DiffusionMobilityProfile::Observer
        ),
        receiver_is_observer: matches!(
            to_node.mobility_profile,
            DiffusionMobilityProfile::Observer
        ),
        same_cluster,
        expensive_transport: matches!(contact.transport_kind, DiffusionTransportKind::LoRa),
        continuity_value,
        protected_opportunity,
    })
}

fn holder_count_in_cluster(
    scenario: &DiffusionScenarioSpec,
    holders: &BTreeMap<u32, HolderState>,
    pending: &[PendingTransfer],
    cluster_id: u8,
) -> usize {
    let holder_count = holders
        .keys()
        .filter(|node_id| {
            node_by_id(scenario, **node_id)
                .map(|node| node.cluster_id == cluster_id)
                .unwrap_or(false)
        })
        .count();
    let pending_count = pending
        .iter()
        .filter(|transfer| {
            node_by_id(scenario, transfer.target_node_id)
                .map(|node| node.cluster_id == cluster_id)
                .unwrap_or(false)
        })
        .count();
    holder_count.saturating_add(pending_count)
}

fn field_budget_kind(
    features: &FieldTransferFeatures,
    budget_state: &FieldBudgetState,
) -> Option<FieldBudgetKind> {
    if features.receiver_is_target {
        return Some(FieldBudgetKind::Target);
    }
    if features.protected_opportunity {
        if budget_state.protected_remaining > 0 {
            return Some(FieldBudgetKind::Protected);
        }
        if budget_state.generic_remaining > 0 {
            return Some(FieldBudgetKind::Generic);
        }
        return None;
    }
    if budget_state.generic_remaining > 0 {
        Some(FieldBudgetKind::Generic)
    } else {
        None
    }
}

fn field_forwarding_suppressed(
    posture: DiffusionFieldPosture,
    round: u32,
    holder_count: usize,
    receiver_cluster_holders: usize,
    sender_energy_ratio: u32,
    features: &FieldTransferFeatures,
    suppression_state: &FieldSuppressionState,
    metrics: &mut FieldExecutionMetrics,
) -> bool {
    let (max_holders, max_same_cluster_holders, cooldown_rounds) = match posture {
        DiffusionFieldPosture::ContinuityBiased => (6_usize, 2_usize, 1_u32),
        DiffusionFieldPosture::Balanced => (5_usize, 2_usize, 1_u32),
        DiffusionFieldPosture::ScarcityConservative => (4_usize, 1_usize, 2_u32),
        DiffusionFieldPosture::CongestionSuppressed => (4_usize, 1_usize, 3_u32),
        DiffusionFieldPosture::PrivacyConservative => (4_usize, 1_usize, 2_u32),
    };

    if !features.receiver_is_target
        && !features.protected_opportunity
        && holder_count >= max_holders
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        return true;
    }
    if matches!(posture, DiffusionFieldPosture::CongestionSuppressed)
        && receiver_cluster_holders > 0
        && !features.receiver_is_target
        && !features.protected_opportunity
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        if features.same_cluster {
            metrics.same_cluster_suppression_count =
                metrics.same_cluster_suppression_count.saturating_add(1);
        }
        return true;
    }
    if features.same_cluster
        && !features.receiver_is_target
        && receiver_cluster_holders >= max_same_cluster_holders
    {
        metrics.same_cluster_suppression_count =
            metrics.same_cluster_suppression_count.saturating_add(1);
        if matches!(posture, DiffusionFieldPosture::CongestionSuppressed) {
            metrics.redundant_forward_suppression_count = metrics
                .redundant_forward_suppression_count
                .saturating_add(1);
        }
        return true;
    }
    if features.expensive_transport
        && !features.protected_opportunity
        && matches!(
            posture,
            DiffusionFieldPosture::ScarcityConservative
                | DiffusionFieldPosture::CongestionSuppressed
                | DiffusionFieldPosture::PrivacyConservative
        )
    {
        metrics.expensive_transport_suppression_count = metrics
            .expensive_transport_suppression_count
            .saturating_add(1);
        return true;
    }
    if matches!(posture, DiffusionFieldPosture::PrivacyConservative)
        && (features.sender_is_observer || features.receiver_is_observer)
        && !features.receiver_is_target
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        return true;
    }
    if matches!(posture, DiffusionFieldPosture::ScarcityConservative)
        && (!features.continuity_value || sender_energy_ratio < 520)
        && !features.receiver_is_target
    {
        if features.same_cluster {
            metrics.same_cluster_suppression_count =
                metrics.same_cluster_suppression_count.saturating_add(1);
        } else {
            metrics.redundant_forward_suppression_count = metrics
                .redundant_forward_suppression_count
                .saturating_add(1);
        }
        return true;
    }
    if matches!(posture, DiffusionFieldPosture::CongestionSuppressed)
        && !features.continuity_value
        && !features.receiver_is_target
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        return true;
    }
    if !features.receiver_is_target {
        if let Some(last_round) = suppression_state
            .recent_cluster_forward_round
            .get(&features.to_cluster_id)
        {
            if round <= last_round.saturating_add(cooldown_rounds)
                && matches!(
                    posture,
                    DiffusionFieldPosture::CongestionSuppressed
                        | DiffusionFieldPosture::ScarcityConservative
                )
                && !features.protected_opportunity
            {
                metrics.redundant_forward_suppression_count = metrics
                    .redundant_forward_suppression_count
                    .saturating_add(1);
                return true;
            }
        }
        if features.same_cluster {
            if let Some(last_round) = suppression_state
                .recent_same_cluster_forward_round
                .get(&features.from_cluster_id)
            {
                if round <= last_round.saturating_add(cooldown_rounds) {
                    metrics.same_cluster_suppression_count =
                        metrics.same_cluster_suppression_count.saturating_add(1);
                    if matches!(posture, DiffusionFieldPosture::CongestionSuppressed) {
                        metrics.redundant_forward_suppression_count = metrics
                            .redundant_forward_suppression_count
                            .saturating_add(1);
                    }
                    return true;
                }
            }
        } else if let Some(last_round) = suppression_state
            .recent_corridor_forward_round
            .get(&(features.from_cluster_id, features.to_cluster_id))
        {
            if round <= last_round.saturating_add(cooldown_rounds)
                && matches!(posture, DiffusionFieldPosture::CongestionSuppressed)
                && !features.protected_opportunity
            {
                metrics.redundant_forward_suppression_count = metrics
                    .redundant_forward_suppression_count
                    .saturating_add(1);
                return true;
            }
        }
    }
    false
}

fn record_field_forward(
    round: u32,
    budget_kind: FieldBudgetKind,
    features: &FieldTransferFeatures,
    budget_state: &mut FieldBudgetState,
    suppression_state: &mut FieldSuppressionState,
    metrics: &mut FieldExecutionMetrics,
) {
    match budget_kind {
        FieldBudgetKind::Target => {}
        FieldBudgetKind::Protected => {
            budget_state.protected_remaining = budget_state.protected_remaining.saturating_sub(1);
            budget_state.protected_used = budget_state.protected_used.saturating_add(1);
            if features.protected_opportunity {
                metrics.protected_bridge_usage_count =
                    metrics.protected_bridge_usage_count.saturating_add(1);
            }
        }
        FieldBudgetKind::Generic => {
            budget_state.generic_remaining = budget_state.generic_remaining.saturating_sub(1);
            budget_state.generic_used = budget_state.generic_used.saturating_add(1);
        }
    }
    if !features.receiver_is_target {
        suppression_state
            .recent_cluster_forward_round
            .insert(features.to_cluster_id, round);
        suppression_state
            .recent_corridor_forward_round
            .insert((features.from_cluster_id, features.to_cluster_id), round);
        if features.same_cluster {
            suppression_state
                .recent_same_cluster_forward_round
                .insert(features.from_cluster_id, round);
        }
    }
}

fn generate_contacts(
    seed: u64,
    scenario: &DiffusionScenarioSpec,
    round: u32,
) -> Vec<DiffusionContactEvent> {
    let mut contacts = Vec::new();
    for index in 0..scenario.nodes.len() {
        for peer_index in index + 1..scenario.nodes.len() {
            let left = &scenario.nodes[index];
            let right = &scenario.nodes[peer_index];
            let probability = contact_probability_permille(scenario, left, right, round);
            if probability
                <= permille_hash(
                    seed,
                    scenario.family_id.as_str(),
                    round,
                    left.node_id,
                    right.node_id,
                    1,
                )
            {
                continue;
            }
            let transport_kind = choose_transport(left, right, round);
            let (bandwidth_bytes, energy_cost_per_byte, connection_latency_rounds) =
                transport_properties(transport_kind);
            contacts.push(DiffusionContactEvent {
                round_index: round,
                node_a: left.node_id,
                node_b: right.node_id,
                duration_rounds: 1,
                bandwidth_bytes,
                transport_kind,
                connection_latency_rounds,
                energy_cost_per_byte,
            });
        }
    }
    contacts
}

fn contact_probability_permille(
    scenario: &DiffusionScenarioSpec,
    left: &DiffusionNodeSpec,
    right: &DiffusionNodeSpec,
    round: u32,
) -> u32 {
    let same_cluster = left.cluster_id == right.cluster_id;
    let bridged = matches!(
        left.mobility_profile,
        DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover
    ) || matches!(
        right.mobility_profile,
        DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover
    );
    match scenario.family_id.as_str() {
        "diffusion-partitioned-clusters" => {
            if same_cluster {
                720
            } else if bridged {
                260
            } else {
                28
            }
        }
        "diffusion-random-waypoint-sanity" => {
            if same_cluster {
                440
            } else if bridged {
                180
            } else {
                120
            }
        }
        "diffusion-disaster-broadcast" => {
            if same_cluster {
                660
            } else if bridged {
                210
            } else {
                55
            }
        }
        "diffusion-sparse-long-delay" => {
            if same_cluster {
                140
            } else if bridged {
                120
            } else {
                18
            }
        }
        "diffusion-high-density-overload" => {
            if same_cluster {
                900
            } else if bridged {
                620
            } else {
                420
            }
        }
        "diffusion-mobility-shift" => {
            if round < scenario.round_count / 2 {
                if same_cluster {
                    650
                } else if bridged {
                    140
                } else {
                    30
                }
            } else if same_cluster {
                380
            } else if bridged {
                460
            } else {
                140
            }
        }
        "diffusion-adversarial-observation" => {
            if same_cluster {
                540
            } else if bridged {
                180
            } else {
                42
            }
        }
        "diffusion-bridge-drought" => {
            if same_cluster {
                190
            } else if bridged {
                72
            } else {
                6
            }
        }
        "diffusion-energy-starved-relay" => {
            if same_cluster {
                260
            } else if bridged {
                110
            } else {
                20
            }
        }
        "diffusion-congestion-cascade" => {
            if same_cluster {
                960
            } else if bridged {
                700
            } else {
                480
            }
        }
        _ => 0,
    }
}

fn choose_transport(
    left: &DiffusionNodeSpec,
    right: &DiffusionNodeSpec,
    round: u32,
) -> DiffusionTransportKind {
    let same_cluster = left.cluster_id == right.cluster_id;
    if same_cluster {
        if matches!(left.mobility_profile, DiffusionMobilityProfile::Observer)
            || matches!(right.mobility_profile, DiffusionMobilityProfile::Observer)
        {
            DiffusionTransportKind::Ble
        } else {
            DiffusionTransportKind::WifiAware
        }
    } else if matches!(
        left.mobility_profile,
        DiffusionMobilityProfile::LongRangeMover
    ) || matches!(
        right.mobility_profile,
        DiffusionMobilityProfile::LongRangeMover
    ) || round % 5 == 0
    {
        DiffusionTransportKind::LoRa
    } else {
        DiffusionTransportKind::WifiAware
    }
}

fn transport_properties(kind: DiffusionTransportKind) -> (u32, u32, u32) {
    match kind {
        DiffusionTransportKind::Ble => (192, 4, 0),
        DiffusionTransportKind::WifiAware => (640, 2, 0),
        DiffusionTransportKind::LoRa => (96, 8, 1),
    }
}

fn forwarding_score(
    scenario: &DiffusionScenarioSpec,
    policy: &DiffusionPolicyConfig,
    from: u32,
    to: u32,
    contact: &DiffusionContactEvent,
    holder_count: usize,
    sender_energy_remaining: u32,
    field_posture: Option<DiffusionFieldPosture>,
) -> u32 {
    let Some(from_node) = node_by_id(scenario, from) else {
        return 0;
    };
    let Some(to_node) = node_by_id(scenario, to) else {
        return 0;
    };
    let source_cluster = node_by_id(scenario, scenario.source_node_id).map(|node| node.cluster_id);
    let destination_cluster = scenario
        .destination_node_id
        .and_then(|destination| node_by_id(scenario, destination))
        .map(|node| node.cluster_id);
    let toward_destination_cluster = destination_cluster == Some(to_node.cluster_id);
    let leaving_source_cluster =
        source_cluster == Some(from_node.cluster_id) && source_cluster != Some(to_node.cluster_id);
    let bridge_candidate = matches!(
        to_node.mobility_profile,
        DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover
    );
    let mut score = i32::try_from(policy.forward_probability_permille).unwrap_or(i32::MAX);
    if is_target_node(scenario, to) {
        score = score.saturating_add(240);
    }
    if toward_destination_cluster {
        score = score.saturating_add(policy.target_cluster_bias_permille);
    }
    score = score.saturating_add(if from_node.cluster_id == to_node.cluster_id {
        policy.same_cluster_bias_permille
    } else {
        -policy.same_cluster_bias_permille / 2
    });
    if bridge_candidate {
        score =
            score.saturating_add(i32::try_from(policy.bridge_bias_permille).unwrap_or(i32::MAX));
    }
    if matches!(to_node.mobility_profile, DiffusionMobilityProfile::Observer) {
        score = score.saturating_sub(policy.observer_aversion_permille);
    }
    if matches!(contact.transport_kind, DiffusionTransportKind::LoRa) {
        score = score.saturating_add(policy.lora_bias_permille);
    }
    let sender_energy_ratio = if from_node.energy_budget == 0 {
        0
    } else {
        sender_energy_remaining.saturating_mul(1000) / from_node.energy_budget
    };
    if sender_energy_ratio < 250 {
        score =
            score.saturating_sub(i32::try_from(policy.energy_guard_permille).unwrap_or(i32::MAX));
    } else if sender_energy_ratio < 500 {
        score = score
            .saturating_sub(i32::try_from(policy.energy_guard_permille / 2).unwrap_or(i32::MAX));
    }
    let spread_penalty = i32::try_from(
        policy
            .spread_restraint_permille
            .saturating_mul(u32::try_from(holder_count).unwrap_or(u32::MAX))
            / 8,
    )
    .unwrap_or(i32::MAX);
    score = score.saturating_sub(spread_penalty);

    if let Some(posture) = field_posture {
        match posture {
            DiffusionFieldPosture::ContinuityBiased => {
                if bridge_candidate {
                    score = score.saturating_add(160);
                }
                if leaving_source_cluster {
                    score = score.saturating_add(90);
                }
                if toward_destination_cluster {
                    score = score.saturating_add(95);
                }
                if matches!(contact.transport_kind, DiffusionTransportKind::LoRa) {
                    score = score.saturating_add(60);
                }
                score = score.saturating_add(40);
            }
            DiffusionFieldPosture::Balanced => {
                if bridge_candidate {
                    score = score.saturating_add(100);
                }
                if toward_destination_cluster {
                    score = score.saturating_add(70);
                }
            }
            DiffusionFieldPosture::ScarcityConservative => {
                score = score.saturating_sub(220);
                if bridge_candidate && toward_destination_cluster {
                    score = score.saturating_add(130);
                } else if toward_destination_cluster || is_target_node(scenario, to) {
                    score = score.saturating_add(95);
                }
                if matches!(contact.transport_kind, DiffusionTransportKind::LoRa) {
                    score = score.saturating_sub(140);
                }
                if from_node.cluster_id == to_node.cluster_id {
                    score = score.saturating_sub(140);
                }
                if holder_count > 1 {
                    score = score.saturating_sub(110);
                }
            }
            DiffusionFieldPosture::CongestionSuppressed => {
                score = score.saturating_sub(240);
                if matches!(scenario.message_mode, DiffusionMessageMode::Broadcast) {
                    score = score.saturating_sub(180);
                }
                if toward_destination_cluster || is_target_node(scenario, to) {
                    score = score.saturating_add(120);
                }
                if from_node.cluster_id == to_node.cluster_id {
                    score = score.saturating_sub(170);
                }
                if holder_count > 2 {
                    score = score.saturating_sub(160);
                }
            }
            DiffusionFieldPosture::PrivacyConservative => {
                score = score.saturating_sub(120);
                if matches!(to_node.mobility_profile, DiffusionMobilityProfile::Observer) {
                    score = score.saturating_sub(320);
                }
                if toward_destination_cluster || is_target_node(scenario, to) {
                    score = score.saturating_add(110);
                }
            }
        }
    } else {
        match policy.forwarding_style {
            DiffusionForwardingStyle::ConservativeLocal => {
                if leaving_source_cluster
                    && !toward_destination_cluster
                    && !is_target_node(scenario, to)
                {
                    score = score.saturating_sub(180);
                }
                if matches!(contact.transport_kind, DiffusionTransportKind::LoRa) {
                    score = score.saturating_sub(80);
                }
            }
            DiffusionForwardingStyle::BalancedDistanceVector => {
                if leaving_source_cluster && !toward_destination_cluster {
                    score = score.saturating_sub(70);
                }
                if bridge_candidate && toward_destination_cluster {
                    score = score.saturating_add(40);
                }
            }
            DiffusionForwardingStyle::FreshnessAware => {
                if bridge_candidate {
                    score = score.saturating_add(50);
                }
                if holder_count > 3 {
                    score = score.saturating_sub(40);
                }
            }
            DiffusionForwardingStyle::ServiceDirected => {
                if toward_destination_cluster {
                    score = score.saturating_add(160);
                }
                if leaving_source_cluster {
                    score = score.saturating_add(50);
                }
                if matches!(scenario.message_mode, DiffusionMessageMode::Broadcast) {
                    score = score.saturating_add(80);
                }
            }
            DiffusionForwardingStyle::CorridorAware => {
                if bridge_candidate {
                    score = score.saturating_add(110);
                }
                if leaving_source_cluster {
                    score = score.saturating_add(75);
                }
                if toward_destination_cluster {
                    score = score.saturating_add(85);
                }
                if matches!(to_node.mobility_profile, DiffusionMobilityProfile::Observer) {
                    score = score.saturating_sub(80);
                }
            }
            DiffusionForwardingStyle::Composite => {
                if toward_destination_cluster {
                    score = score.saturating_add(110);
                }
                if bridge_candidate {
                    score = score.saturating_add(65);
                }
                if holder_count > 5 {
                    score = score.saturating_sub(30);
                }
            }
        }
    }

    if matches!(scenario.message_mode, DiffusionMessageMode::Broadcast) {
        score = score.saturating_add(60);
    }
    match scenario.family_id.as_str() {
        "diffusion-high-density-overload" | "diffusion-congestion-cascade" => {
            score = score.saturating_sub(i32::try_from(holder_count).unwrap_or(i32::MAX) * 18);
        }
        "diffusion-bridge-drought" => {
            if bridge_candidate || toward_destination_cluster {
                score = score.saturating_add(100);
            } else if leaving_source_cluster {
                score = score.saturating_sub(120);
            }
        }
        "diffusion-energy-starved-relay" => {
            if sender_energy_ratio < 400 {
                score = score.saturating_sub(90);
            }
        }
        _ => {}
    }
    score.clamp(0, 1000) as u32
}

fn bounded_state(
    delivery_probability_permille: u32,
    coverage_permille: u32,
    reproduction_permille: u32,
    transmissions: u32,
    storage_utilization_permille: u32,
    energy_per_delivered_message: Option<u32>,
) -> &'static str {
    if delivery_probability_permille < 300 || coverage_permille < 350 {
        "collapse"
    } else if reproduction_permille > 1600
        || transmissions > 48
        || storage_utilization_permille > 700
        || energy_per_delivered_message.unwrap_or(0) > 1400
    {
        "explosive"
    } else {
        "viable"
    }
}

fn coverage_permille_for(target_count: usize, delivered_count: usize) -> u32 {
    if target_count == 0 {
        return 0;
    }
    u32::try_from(
        (u64::try_from(delivered_count).unwrap_or(0) * 1000)
            / u64::try_from(target_count).unwrap_or(1),
    )
    .unwrap_or(u32::MAX)
}

fn scenario_target_count(scenario: &DiffusionScenarioSpec) -> usize {
    match scenario.message_mode {
        DiffusionMessageMode::Unicast => 1,
        DiffusionMessageMode::Broadcast => scenario.nodes.len().saturating_sub(1),
    }
}

fn is_target_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    match scenario.message_mode {
        DiffusionMessageMode::Unicast => scenario.destination_node_id == Some(node_id),
        DiffusionMessageMode::Broadcast => node_id != scenario.source_node_id,
    }
}

fn is_terminal_target(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    matches!(scenario.message_mode, DiffusionMessageMode::Unicast)
        && scenario.destination_node_id == Some(node_id)
}

fn is_observer_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    scenario
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .map(|node| matches!(node.mobility_profile, DiffusionMobilityProfile::Observer))
        .unwrap_or(false)
}

fn node_by_id(scenario: &DiffusionScenarioSpec, node_id: u32) -> Option<&DiffusionNodeSpec> {
    scenario.nodes.iter().find(|node| node.node_id == node_id)
}

fn normalized_edge(left: u32, right: u32) -> (u32, u32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn permille_hash(seed: u64, family_id: &str, round: u32, left: u32, right: u32, lane: u64) -> u32 {
    let mut value = seed
        ^ u64::from(round).wrapping_mul(0x9E37_79B9)
        ^ u64::from(left).wrapping_mul(0x85EB_CA6B)
        ^ u64::from(right).wrapping_mul(0xC2B2_AE35)
        ^ lane;
    for byte in family_id.as_bytes() {
        value ^= u64::from(*byte);
        value = value.rotate_left(13).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    u32::try_from(value % 1000).unwrap_or(0)
}

fn mean_u32(values: impl Iterator<Item = u32>) -> u32 {
    let collected = values.collect::<Vec<_>>();
    if collected.is_empty() {
        return 0;
    }
    let sum = collected.iter().copied().map(u64::from).sum::<u64>();
    u32::try_from(sum / u64::try_from(collected.len()).unwrap_or(1)).unwrap_or(u32::MAX)
}

fn mean_option_u32(values: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let collected = values.flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        return None;
    }
    Some(mean_u32(collected.into_iter()))
}

fn mode_string(values: impl Iterator<Item = String>) -> String {
    let mut counts = BTreeMap::<String, u32>::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)))
        .map(|(value, _)| value)
        .unwrap_or_else(|| "none".to_string())
}

fn mode_option_string(values: impl Iterator<Item = Option<String>>) -> Option<String> {
    let collected = values.flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        return None;
    }
    Some(mode_string(collected.into_iter()))
}

fn build_partitioned_clusters_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-partitioned-clusters".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "rare-bridges".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 42,
        },
        round_count: 40,
        creation_round: 2,
        payload_bytes: 64,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes: clustered_nodes(12, 3, false),
    }
}

fn build_random_waypoint_sanity_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-random-waypoint-sanity".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "mobile-mixed".to_string(),
            mobility_model: "random-waypoint".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "sanity-check".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 20,
        },
        round_count: 28,
        creation_round: 2,
        payload_bytes: 48,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(8),
        nodes: clustered_nodes(8, 2, false),
    }
}

fn build_disaster_broadcast_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(16, 4, false);
    nodes[0].mobility_profile = DiffusionMobilityProfile::Static;
    DiffusionScenarioSpec {
        family_id: "diffusion-disaster-broadcast".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "urgent-broadcast".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 48,
        },
        round_count: 36,
        creation_round: 1,
        payload_bytes: 72,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes,
    }
}

fn build_sparse_long_delay_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(10, 2, false);
    if let Some(node) = nodes.get_mut(8) {
        node.mobility_profile = DiffusionMobilityProfile::LongRangeMover;
    }
    if let Some(node) = nodes.get_mut(9) {
        node.mobility_profile = DiffusionMobilityProfile::LongRangeMover;
    }
    DiffusionScenarioSpec {
        family_id: "diffusion-sparse-long-delay".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "sparse".to_string(),
            mobility_model: "long-delay".to_string(),
            transport_mix: "ble-lora".to_string(),
            pressure: "permanent-partition-risk".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 56,
        },
        round_count: 48,
        creation_round: 2,
        payload_bytes: 56,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(10),
        nodes,
    }
}

fn build_high_density_overload_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-high-density-overload".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "dense-camp".to_string(),
            mobility_model: "clustered".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "overload".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 62,
        },
        round_count: 28,
        creation_round: 1,
        payload_bytes: 64,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes: clustered_nodes(18, 3, false),
    }
}

fn build_mobility_shift_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-mobility-shift".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "shifted-communities".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "reconfiguration".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 50,
        },
        round_count: 44,
        creation_round: 3,
        payload_bytes: 64,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes: clustered_nodes(12, 3, false),
    }
}

fn build_adversarial_observation_scenario() -> DiffusionScenarioSpec {
    DiffusionScenarioSpec {
        family_id: "diffusion-adversarial-observation".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "observer-risk".to_string(),
            objective_regime: "privacy-sensitive-unicast".to_string(),
            stress_score: 46,
        },
        round_count: 38,
        creation_round: 2,
        payload_bytes: 60,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes: clustered_nodes(12, 3, true),
    }
}

fn build_bridge_drought_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(14, 4, false);
    for node in &mut nodes {
        node.energy_budget = 8_000;
        if node.node_id % 5 != 0 && node.node_id % 7 != 0 {
            node.mobility_profile = if node.node_id % 2 == 0 {
                DiffusionMobilityProfile::LocalMover
            } else {
                DiffusionMobilityProfile::Static
            };
        }
    }
    DiffusionScenarioSpec {
        family_id: "diffusion-bridge-drought".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "sparse-clustered".to_string(),
            mobility_model: "rare-bridge-drought".to_string(),
            transport_mix: "ble-lora".to_string(),
            pressure: "prolonged-bridge-drought".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 72,
        },
        round_count: 60,
        creation_round: 2,
        payload_bytes: 72,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(14),
        nodes,
    }
}

fn build_energy_starved_relay_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(12, 3, false);
    for node in &mut nodes {
        node.energy_budget = match node.mobility_profile {
            DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover => 1_400,
            DiffusionMobilityProfile::LocalMover => 1_000,
            _ => 900,
        };
        node.storage_capacity = 160;
    }
    DiffusionScenarioSpec {
        family_id: "diffusion-energy-starved-relay".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "clustered".to_string(),
            mobility_model: "community-bridgers".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "energy-starvation".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 68,
        },
        round_count: 42,
        creation_round: 2,
        payload_bytes: 88,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(12),
        nodes,
    }
}

fn build_congestion_cascade_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes(20, 4, false);
    for node in &mut nodes {
        node.energy_budget = 4_000;
        node.storage_capacity = 96;
    }
    DiffusionScenarioSpec {
        family_id: "diffusion-congestion-cascade".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "dense-camp".to_string(),
            mobility_model: "clustered".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "congestion-cascade".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 78,
        },
        round_count: 30,
        creation_round: 1,
        payload_bytes: 88,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes,
    }
}

fn clustered_nodes(
    node_count: u32,
    cluster_count: u8,
    with_observers: bool,
) -> Vec<DiffusionNodeSpec> {
    let mut nodes = Vec::new();
    for node_id in 1..=node_count {
        let cluster_id = u8::try_from((node_id - 1) % u32::from(cluster_count)).unwrap_or(0);
        let mobility_profile = if with_observers && node_id % 6 == 0 {
            DiffusionMobilityProfile::Observer
        } else if node_id % 5 == 0 {
            DiffusionMobilityProfile::Bridger
        } else if node_id % 7 == 0 {
            DiffusionMobilityProfile::LongRangeMover
        } else if node_id % 2 == 0 {
            DiffusionMobilityProfile::LocalMover
        } else {
            DiffusionMobilityProfile::Static
        };
        let transport_capabilities = match mobility_profile {
            DiffusionMobilityProfile::LongRangeMover => {
                vec![
                    DiffusionTransportKind::Ble,
                    DiffusionTransportKind::WifiAware,
                    DiffusionTransportKind::LoRa,
                ]
            }
            _ => vec![
                DiffusionTransportKind::Ble,
                DiffusionTransportKind::WifiAware,
            ],
        };
        nodes.push(DiffusionNodeSpec {
            node_id,
            cluster_id,
            mobility_profile,
            energy_budget: 20_000,
            storage_capacity: 512,
            transport_capabilities,
        });
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_diffusion_runs, bounded_state, build_adversarial_observation_scenario,
        build_bridge_drought_scenario, build_congestion_cascade_scenario,
        build_energy_starved_relay_scenario, build_partitioned_clusters_scenario,
        diffusion_engine_profile, diffusion_smoke_suite, field_diffusion_profiles,
        field_forwarding_suppressed, generate_contacts, simulate_diffusion_run,
        DiffusionFieldPosture, DiffusionForwardingStyle, DiffusionPolicyConfig, DiffusionRunSpec,
        FieldExecutionMetrics, FieldSuppressionState, FieldTransferFeatures,
    };

    #[test]
    fn contact_generation_is_deterministic() {
        let scenario = build_partitioned_clusters_scenario();
        let first = generate_contacts(41, &scenario, 5);
        let second = generate_contacts(41, &scenario, 5);
        assert_eq!(first, second);
    }

    #[test]
    fn message_persists_across_disconnected_intervals() {
        let scenario = build_partitioned_clusters_scenario();
        let policy = DiffusionPolicyConfig {
            config_id: "test".to_string(),
            replication_budget: 4,
            ttl_rounds: 30,
            forward_probability_permille: 650,
            bridge_bias_permille: 250,
            target_cluster_bias_permille: 120,
            same_cluster_bias_permille: 0,
            observer_aversion_permille: 100,
            lora_bias_permille: 40,
            spread_restraint_permille: 80,
            energy_guard_permille: 90,
            forwarding_style: DiffusionForwardingStyle::CorridorAware,
        };
        let summary = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy,
            scenario,
        });
        assert!(summary.message_persistence_rounds >= 30);
    }

    #[test]
    fn bounded_replication_stays_out_of_explosive_state_for_smoke_baseline() {
        let suite = diffusion_smoke_suite();
        let summary = simulate_diffusion_run(&suite.runs[0]);
        assert_ne!(summary.bounded_state, "explosive");
    }

    #[test]
    fn aggregate_metrics_are_non_empty() {
        let suite = diffusion_smoke_suite();
        let runs = suite
            .runs
            .iter()
            .take(2)
            .map(simulate_diffusion_run)
            .collect::<Vec<_>>();
        let aggregates = aggregate_diffusion_runs(&runs);
        assert!(!aggregates.is_empty());
    }

    #[test]
    fn bounded_state_classifies_regions() {
        assert_eq!(bounded_state(100, 200, 400, 8, 120, Some(200)), "collapse");
        assert_eq!(bounded_state(800, 850, 1000, 20, 260, Some(900)), "viable");
        assert_eq!(
            bounded_state(900, 900, 1900, 64, 820, Some(1700)),
            "explosive"
        );
    }

    #[test]
    fn energy_starved_relay_separates_conservative_and_broad_profiles() {
        let scenario = build_energy_starved_relay_scenario();
        let classic = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("batman-classic"),
            scenario: scenario.clone(),
        });
        let combined = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("pathway-batman-bellman"),
            scenario,
        });
        assert_eq!(classic.bounded_state, "viable");
        assert_eq!(combined.bounded_state, "explosive");
    }

    #[test]
    fn bridge_drought_keeps_bounded_profiles_viable() {
        let scenario = build_bridge_drought_scenario();
        let babel = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("babel"),
            scenario: scenario.clone(),
        });
        let combined = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("pathway-batman-bellman"),
            scenario,
        });
        assert_eq!(babel.bounded_state, "viable");
        assert_eq!(combined.bounded_state, "explosive");
    }

    #[test]
    fn field_bridge_drought_stays_continuity_biased_without_scarcity_transition() {
        let scenario = build_bridge_drought_scenario();
        let field = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario,
        });
        assert_eq!(
            field.field_posture_mode.as_deref(),
            Some("continuity_biased")
        );
        assert_eq!(field.field_first_scarcity_transition_round, None);
        assert!(field.field_continuity_biased_rounds >= field.field_balanced_rounds);
    }

    #[test]
    fn field_protected_budget_survives_for_bridge_opportunities() {
        let scenario = build_bridge_drought_scenario();
        let policy = field_diffusion_profiles()
            .into_iter()
            .find(|candidate| candidate.config_id == "field-continuity-reserve")
            .expect("field continuity reserve profile");
        let field = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy,
            scenario,
        });
        assert!(field.field_bridge_opportunity_count > 0);
        assert!(field.field_protected_budget_used > 0);
        assert!(field.field_protected_bridge_usage_count > 0);
        assert!(field.field_protected_bridge_usage_count <= field.field_bridge_opportunity_count);
    }

    #[test]
    fn field_energy_starved_transitions_into_scarcity_conservative() {
        let scenario = build_energy_starved_relay_scenario();
        let field = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario,
        });
        assert_eq!(
            field.field_posture_mode.as_deref(),
            Some("scarcity_conservative")
        );
        assert!(field.field_first_scarcity_transition_round.is_some());
    }

    #[test]
    fn field_congestion_cascade_transitions_into_congestion_suppressed() {
        let scenario = build_congestion_cascade_scenario();
        let field = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario,
        });
        assert_eq!(
            field.field_posture_mode.as_deref(),
            Some("congestion_suppressed")
        );
        assert!(field.field_first_congestion_transition_round.is_some());
    }

    #[test]
    fn scarcity_variant_reduces_energy_and_expensive_transport_use() {
        let scenario = build_energy_starved_relay_scenario();
        let scarcity_policy = field_diffusion_profiles()
            .into_iter()
            .find(|candidate| candidate.config_id == "field-scarcity-hardcap")
            .expect("field scarcity hardcap profile");
        let baseline = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario: scenario.clone(),
        });
        let scarcity = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: scarcity_policy,
            scenario,
        });
        assert!(
            scarcity.energy_per_delivered_message.unwrap_or(u32::MAX)
                <= baseline.energy_per_delivered_message.unwrap_or(u32::MAX)
        );
        assert!(
            scarcity.field_expensive_transport_suppression_count
                >= baseline.field_expensive_transport_suppression_count
        );
    }

    #[test]
    fn congestion_variant_records_redundancy_suppression() {
        let scenario = build_congestion_cascade_scenario();
        let congestion_policy = field_diffusion_profiles()
            .into_iter()
            .find(|candidate| candidate.config_id == "field-congestion-memory")
            .expect("field congestion memory profile");
        let congestion = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: congestion_policy,
            scenario,
        });
        assert_eq!(
            congestion.field_posture_mode.as_deref(),
            Some("congestion_suppressed")
        );
        assert!(
            congestion.field_redundant_forward_suppression_count
                + congestion.field_same_cluster_suppression_count
                > 0
        );
    }

    #[test]
    fn congestion_memory_suppresses_duplicate_cluster_forwarding() {
        let mut metrics = FieldExecutionMetrics::default();
        let suppression_state = FieldSuppressionState::default();
        let features = FieldTransferFeatures {
            from_cluster_id: 0,
            to_cluster_id: 1,
            receiver_is_target: false,
            sender_is_observer: false,
            receiver_is_observer: false,
            same_cluster: false,
            expensive_transport: false,
            continuity_value: false,
            protected_opportunity: false,
        };
        let suppressed = field_forwarding_suppressed(
            DiffusionFieldPosture::CongestionSuppressed,
            4,
            4,
            1,
            900,
            &features,
            &suppression_state,
            &mut metrics,
        );
        assert!(suppressed);
        assert!(metrics.redundant_forward_suppression_count > 0);
    }

    #[test]
    fn privacy_variant_reduces_observer_leakage_relative_to_balanced() {
        let scenario = build_adversarial_observation_scenario();
        let privacy_policy = field_diffusion_profiles()
            .into_iter()
            .find(|candidate| candidate.config_id == "field-privacy-tight")
            .expect("field privacy tight profile");
        let baseline = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario: scenario.clone(),
        });
        let privacy = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: privacy_policy,
            scenario,
        });
        assert!(
            privacy.observer_leakage_permille <= baseline.observer_leakage_permille,
            "privacy_leakage={} baseline_leakage={} privacy_tx={} baseline_tx={}",
            privacy.observer_leakage_permille,
            baseline.observer_leakage_permille,
            privacy.total_transmissions,
            baseline.total_transmissions
        );
        assert!(privacy.delivery_probability_permille >= 1000);
    }

    #[test]
    fn adaptive_field_reduces_spread_pressure_relative_to_static_field() {
        let scenario = build_congestion_cascade_scenario();
        let adaptive = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario: scenario.clone(),
        });
        let mut static_policy = diffusion_engine_profile("field");
        static_policy.config_id = "field-static".to_string();
        let static_summary = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: static_policy,
            scenario,
        });
        assert!(adaptive.total_transmissions <= static_summary.total_transmissions);
    }
}
