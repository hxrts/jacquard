use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum DiffusionFieldPosture {
    ContinuityBiased,
    Balanced,
    ScarcityConservative,
    ClusterSeeding,
    DuplicateSuppressed,
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
    #[serde(skip)]
    pub node_index_by_id: BTreeMap<u32, usize>,
    #[serde(skip)]
    pub pair_descriptors: Vec<DiffusionPairDescriptor>,
}

impl DiffusionScenarioSpec {
    pub(crate) fn rebuild_runtime_indexes(&mut self) {
        self.node_index_by_id = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.node_id, index))
            .collect();
        self.pair_descriptors = Vec::new();
        for left_index in 0..self.nodes.len() {
            for right_index in left_index + 1..self.nodes.len() {
                let left = &self.nodes[left_index];
                let right = &self.nodes[right_index];
                self.pair_descriptors.push(DiffusionPairDescriptor {
                    left_index,
                    right_index,
                    left_node_id: left.node_id,
                    right_node_id: right.node_id,
                    same_cluster: left.cluster_id == right.cluster_id,
                    bridged: matches!(
                        left.mobility_profile,
                        DiffusionMobilityProfile::Bridger
                            | DiffusionMobilityProfile::LongRangeMover
                    ) || matches!(
                        right.mobility_profile,
                        DiffusionMobilityProfile::Bridger
                            | DiffusionMobilityProfile::LongRangeMover
                    ),
                });
            }
        }
    }

    #[must_use]
    pub(crate) fn with_runtime_indexes(mut self) -> Self {
        self.rebuild_runtime_indexes();
        self
    }
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DiffusionPairDescriptor {
    pub left_index: usize,
    pub right_index: usize,
    pub left_node_id: u32,
    pub right_node_id: u32,
    pub same_cluster: bool,
    pub bridged: bool,
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
    pub(crate) suite_id: String,
    pub(crate) runs: Vec<DiffusionRunSpec>,
}

impl DiffusionSuite {
    #[must_use]
    pub fn suite_id(&self) -> &str {
        &self.suite_id
    }
}
