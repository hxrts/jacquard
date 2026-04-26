//! Diffusion policy config, forwarding styles, field postures, and scenario specs.

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DIFFUSION_ARTIFACT_SCHEMA_VERSION: u32 = 1;

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
    #[serde(alias = "CorridorAware")]
    ContinuityBiased,
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
pub enum DiffusionMobilityProfile {
    Static,
    LocalMover,
    Bridger,
    LongRangeMover,
    Observer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiffusionTransportKind {
    Ble,
    WifiAware,
    LoRa,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiffusionMessageMode {
    Unicast,
    Broadcast,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffusionNodeSpec {
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
pub struct CustomDiffusionScenarioSpec {
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

impl CustomDiffusionScenarioSpec {
    #[must_use]
    pub(crate) fn into_scenario_spec(self) -> DiffusionScenarioSpec {
        DiffusionScenarioSpec {
            family_id: self.family_id,
            regime: self.regime,
            round_count: self.round_count,
            creation_round: self.creation_round,
            payload_bytes: self.payload_bytes,
            message_mode: self.message_mode,
            source_node_id: self.source_node_id,
            destination_node_id: self.destination_node_id,
            nodes: self.nodes,
            node_index_by_id: BTreeMap::new(),
            pair_descriptors: Vec::new(),
        }
        .with_runtime_indexes()
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
pub struct CustomDiffusionRunSpec {
    pub family_id: String,
    pub seed: u64,
    pub policy: DiffusionPolicyConfig,
    pub scenario: CustomDiffusionScenarioSpec,
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
    pub continuity_persistence_permille: u32,
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
    pub continuity_persistence_permille_mean: u32,
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
    #[serde(default = "default_diffusion_artifact_schema_version")]
    pub schema_version: u32,
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
    pub fn from_custom_runs(
        suite_id: impl Into<String>,
        runs: Vec<CustomDiffusionRunSpec>,
    ) -> Result<Self, DiffusionSuiteBuildError> {
        let suite_id = suite_id.into();
        validate_suite_id(&suite_id)?;
        let mut seen = BTreeMap::<(String, String, u64), String>::new();
        let mut materialized = Vec::with_capacity(runs.len());
        for run in runs {
            validate_suite_id(&run.family_id)?;
            validate_suite_id(&run.policy.config_id)?;
            validate_diffusion_scenario(&run.scenario)?;
            let key = (
                run.family_id.clone(),
                run.policy.config_id.clone(),
                run.seed,
            );
            if let Some(previous) = seen.insert(key.clone(), run.family_id.clone()) {
                return Err(DiffusionSuiteBuildError::DuplicateRun {
                    family_id: key.0,
                    config_id: key.1,
                    seed: key.2,
                    previous_family_id: previous,
                });
            }
            materialized.push(DiffusionRunSpec {
                suite_id: suite_id.clone(),
                family_id: run.family_id,
                seed: run.seed,
                policy: run.policy,
                scenario: run.scenario.into_scenario_spec(),
            });
        }
        if materialized.is_empty() {
            return Err(DiffusionSuiteBuildError::EmptySuite { suite_id });
        }
        Ok(Self {
            suite_id,
            runs: materialized,
        })
    }

    #[must_use]
    pub fn suite_id(&self) -> &str {
        &self.suite_id
    }

    #[must_use]
    pub fn run_count(&self) -> usize {
        self.runs.len()
    }
}

#[derive(Debug, Error)]
pub enum DiffusionSuiteBuildError {
    #[error("diffusion suite id must not be empty")]
    EmptySuiteId,
    #[error("diffusion suite '{suite_id}' has no runs")]
    EmptySuite { suite_id: String },
    #[error("diffusion id '{id}' contains unsupported characters")]
    InvalidId { id: String },
    #[error("diffusion scenario '{family_id}' has no nodes")]
    EmptyScenario { family_id: String },
    #[error("diffusion scenario '{family_id}' source node {source_node_id} is missing")]
    MissingSourceNode {
        family_id: String,
        source_node_id: u32,
    },
    #[error("diffusion scenario '{family_id}' destination node {destination_node_id} is missing")]
    MissingDestinationNode {
        family_id: String,
        destination_node_id: u32,
    },
    #[error("diffusion scenario '{family_id}' has duplicate node id {node_id}")]
    DuplicateNodeId { family_id: String, node_id: u32 },
    #[error(
        "duplicate diffusion run for family '{family_id}', config '{config_id}', seed {seed}; previous family id '{previous_family_id}'"
    )]
    DuplicateRun {
        family_id: String,
        config_id: String,
        seed: u64,
        previous_family_id: String,
    },
}

fn default_diffusion_artifact_schema_version() -> u32 {
    DIFFUSION_ARTIFACT_SCHEMA_VERSION
}

fn validate_suite_id(id: &str) -> Result<(), DiffusionSuiteBuildError> {
    if id.is_empty() {
        return Err(DiffusionSuiteBuildError::EmptySuiteId);
    }
    if id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Ok(());
    }
    Err(DiffusionSuiteBuildError::InvalidId { id: id.to_string() })
}

fn validate_diffusion_scenario(
    scenario: &CustomDiffusionScenarioSpec,
) -> Result<(), DiffusionSuiteBuildError> {
    if scenario.nodes.is_empty() {
        return Err(DiffusionSuiteBuildError::EmptyScenario {
            family_id: scenario.family_id.clone(),
        });
    }
    let mut seen = BTreeMap::<u32, ()>::new();
    for node in &scenario.nodes {
        if seen.insert(node.node_id, ()).is_some() {
            return Err(DiffusionSuiteBuildError::DuplicateNodeId {
                family_id: scenario.family_id.clone(),
                node_id: node.node_id,
            });
        }
    }
    if !seen.contains_key(&scenario.source_node_id) {
        return Err(DiffusionSuiteBuildError::MissingSourceNode {
            family_id: scenario.family_id.clone(),
            source_node_id: scenario.source_node_id,
        });
    }
    if let Some(destination_node_id) = scenario.destination_node_id {
        if !seen.contains_key(&destination_node_id) {
            return Err(DiffusionSuiteBuildError::MissingDestinationNode {
                family_id: scenario.family_id.clone(),
                destination_node_id,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn diffusion_research_names_stay_continuity_facing() {
        let source = include_str!("model.rs");
        for forbidden in [
            concat!("pub ", "cor", "ridor_"),
            concat!("recent_", "cor", "ridor"),
            concat!("::", "Cor", "ridorAware"),
        ] {
            assert!(
                !source.contains(forbidden),
                "diffusion research model exposes route-stack token `{forbidden}`"
            );
        }
        assert!(source.contains("ContinuityBiased"));
        assert!(source.contains("continuity_persistence_permille"));
    }
}
