// long-file-exception: the maintained diffusion scenario catalog is kept as one
// explicit roster so regime definitions stay easy to review together.
use super::{
    DiffusionMessageMode, DiffusionMobilityProfile, DiffusionNodeSpec, DiffusionRegimeDescriptor,
    DiffusionScenarioSpec, DiffusionTransportKind,
};

pub(super) fn build_partitioned_clusters_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_random_waypoint_sanity_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_disaster_broadcast_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_sparse_long_delay_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_high_density_overload_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_mobility_shift_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_adversarial_observation_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_bridge_drought_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_energy_starved_relay_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

pub(super) fn build_congestion_cascade_scenario() -> DiffusionScenarioSpec {
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
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

// Analytical question: where does bounded unicast diffusion cross from
// under-seeded collapse into viable or explosive spread when a larger clustered
// population depends on scarce bridgers?
pub(super) fn build_large_sparse_threshold_moderate_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes_with_strides(24, 6, false, 9, 13);
    set_uniform_resources(&mut nodes, 9_000, 224);
    boost_bridge_resources(&mut nodes, 11_000, 256);
    DiffusionScenarioSpec {
        family_id: "diffusion-large-sparse-threshold-moderate".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "large-sparse".to_string(),
            mobility_model: "scarce-bridgers".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "threshold".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 84,
        },
        round_count: 72,
        creation_round: 2,
        payload_bytes: 80,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(24),
        nodes,
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

// Analytical question: does the same sparse bridger regime sharpen the
// collapse / viable / explosive boundary further at the high large-population
// band?
pub(super) fn build_large_sparse_threshold_high_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes_with_strides(36, 8, false, 12, 17);
    set_uniform_resources(&mut nodes, 8_400, 192);
    boost_bridge_resources(&mut nodes, 10_500, 224);
    DiffusionScenarioSpec {
        family_id: "diffusion-large-sparse-threshold-high".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "large-sparse".to_string(),
            mobility_model: "scarce-bridgers".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "threshold".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 92,
        },
        round_count: 96,
        creation_round: 2,
        payload_bytes: 84,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(36),
        nodes,
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

// Analytical question: where does larger-population broadcast diffusion tip
// from viable bounded spread into overload collapse once dense clusters and
// bridge brokers saturate?
pub(super) fn build_large_congestion_threshold_moderate_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes_with_strides(28, 5, false, 6, 11);
    set_uniform_resources(&mut nodes, 7_000, 160);
    boost_bridge_resources(&mut nodes, 9_000, 192);
    DiffusionScenarioSpec {
        family_id: "diffusion-large-congestion-threshold-moderate".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "large-dense".to_string(),
            mobility_model: "clustered".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "overload-threshold".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 88,
        },
        round_count: 34,
        creation_round: 1,
        payload_bytes: 96,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes,
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

// Analytical question: does the overload boundary stay stable or fail earlier
// once the same dense congestion family is pushed to the high population band?
pub(super) fn build_large_congestion_threshold_high_scenario() -> DiffusionScenarioSpec {
    let mut nodes = clustered_nodes_with_strides(40, 6, false, 7, 13);
    set_uniform_resources(&mut nodes, 8_000, 160);
    boost_bridge_resources(&mut nodes, 10_000, 192);
    DiffusionScenarioSpec {
        family_id: "diffusion-large-congestion-threshold-high".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "large-dense".to_string(),
            mobility_model: "clustered".to_string(),
            transport_mix: "ble-wifi".to_string(),
            pressure: "overload-threshold".to_string(),
            objective_regime: "broadcast".to_string(),
            stress_score: 96,
        },
        round_count: 38,
        creation_round: 1,
        payload_bytes: 104,
        message_mode: DiffusionMessageMode::Broadcast,
        source_node_id: 1,
        destination_node_id: None,
        nodes,
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

// Analytical question: how robust is store-carry diffusion when contact
// opportunities shift by region and only a few brokers preserve continuity
// across the larger population?
pub(super) fn build_large_regional_shift_moderate_scenario() -> DiffusionScenarioSpec {
    let mut nodes = regional_shift_nodes(24, 4);
    set_uniform_resources(&mut nodes, 7_500, 224);
    boost_bridge_resources(&mut nodes, 9_000, 256);
    DiffusionScenarioSpec {
        family_id: "diffusion-large-regional-shift-moderate".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "large-clustered".to_string(),
            mobility_model: "regional-shift".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "reconfiguration".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 76,
        },
        round_count: 60,
        creation_round: 3,
        payload_bytes: 72,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(24),
        nodes,
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

// Analytical question: does the same regional-shift mobility surface remain
// deterministic and viable once the broker and cluster count both grow?
pub(super) fn build_large_regional_shift_high_scenario() -> DiffusionScenarioSpec {
    let mut nodes = regional_shift_nodes(36, 6);
    set_uniform_resources(&mut nodes, 7_000, 192);
    boost_bridge_resources(&mut nodes, 8_600, 224);
    DiffusionScenarioSpec {
        family_id: "diffusion-large-regional-shift-high".to_string(),
        regime: DiffusionRegimeDescriptor {
            density: "large-clustered".to_string(),
            mobility_model: "regional-shift".to_string(),
            transport_mix: "ble-wifi-lora".to_string(),
            pressure: "reconfiguration".to_string(),
            objective_regime: "store-carry-unicast".to_string(),
            stress_score: 84,
        },
        round_count: 76,
        creation_round: 3,
        payload_bytes: 80,
        message_mode: DiffusionMessageMode::Unicast,
        source_node_id: 1,
        destination_node_id: Some(36),
        nodes,
        node_index_by_id: std::collections::BTreeMap::new(),
        pair_descriptors: Vec::new(),
    }
    .with_runtime_indexes()
}

fn clustered_nodes_with_strides(
    node_count: u32,
    cluster_count: u8,
    with_observers: bool,
    bridger_stride: u32,
    long_range_stride: u32,
) -> Vec<DiffusionNodeSpec> {
    let mut nodes = Vec::new();
    for node_id in 1..=node_count {
        let cluster_id = u8::try_from((node_id - 1) % u32::from(cluster_count)).unwrap_or(0);
        let mobility_profile = if with_observers && node_id % 6 == 0 {
            DiffusionMobilityProfile::Observer
        } else if node_id % long_range_stride == 0 {
            DiffusionMobilityProfile::LongRangeMover
        } else if node_id % bridger_stride == 0 {
            DiffusionMobilityProfile::Bridger
        } else if (node_id + u32::from(cluster_id)).is_multiple_of(2) {
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

fn regional_shift_nodes(node_count: u32, cluster_count: u8) -> Vec<DiffusionNodeSpec> {
    let mut nodes = clustered_nodes_with_strides(node_count, cluster_count, false, 8, 13);
    for node in &mut nodes {
        node.mobility_profile = match node.cluster_id % 3 {
            0 => DiffusionMobilityProfile::Static,
            1 => {
                if node.node_id % 4 == 0 {
                    DiffusionMobilityProfile::Bridger
                } else {
                    DiffusionMobilityProfile::LocalMover
                }
            }
            _ => {
                if node.node_id % 3 == 0 {
                    DiffusionMobilityProfile::LongRangeMover
                } else {
                    DiffusionMobilityProfile::LocalMover
                }
            }
        };
        node.transport_capabilities = match node.mobility_profile {
            DiffusionMobilityProfile::LongRangeMover => vec![
                DiffusionTransportKind::Ble,
                DiffusionTransportKind::WifiAware,
                DiffusionTransportKind::LoRa,
            ],
            _ => vec![
                DiffusionTransportKind::Ble,
                DiffusionTransportKind::WifiAware,
            ],
        };
    }
    nodes
}

fn set_uniform_resources(
    nodes: &mut [DiffusionNodeSpec],
    energy_budget: u32,
    storage_capacity: u32,
) {
    for node in nodes {
        node.energy_budget = energy_budget;
        node.storage_capacity = storage_capacity;
    }
}

fn boost_bridge_resources(
    nodes: &mut [DiffusionNodeSpec],
    bridge_energy_budget: u32,
    bridge_storage_capacity: u32,
) {
    for node in nodes {
        if matches!(
            node.mobility_profile,
            DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover
        ) {
            node.energy_budget = bridge_energy_budget;
            node.storage_capacity = bridge_storage_capacity;
        }
    }
}

fn clustered_nodes(
    node_count: u32,
    cluster_count: u8,
    with_observers: bool,
) -> Vec<DiffusionNodeSpec> {
    clustered_nodes_with_strides(node_count, cluster_count, with_observers, 5, 7)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::super::{
        diffusion_engine_profile, diffusion_smoke_suite, field_diffusion_profiles,
        transition_diffusion_profiles,
    };
    use super::{
        build_adversarial_observation_scenario, build_bridge_drought_scenario,
        build_congestion_cascade_scenario, build_disaster_broadcast_scenario,
        build_energy_starved_relay_scenario, build_high_density_overload_scenario,
        build_large_congestion_threshold_high_scenario,
        build_large_congestion_threshold_moderate_scenario,
        build_large_regional_shift_high_scenario, build_large_regional_shift_moderate_scenario,
        build_large_sparse_threshold_high_scenario, build_large_sparse_threshold_moderate_scenario,
        build_partitioned_clusters_scenario,
    };
    use crate::diffusion::{
        posture::{field_budget_kind, field_forwarding_suppressed},
        runtime::execution::{
            aggregate_diffusion_runs, bounded_state, generate_contacts, simulate_diffusion_run,
        },
        runtime::DiffusionRunSpec,
        DiffusionFieldPosture, DiffusionForwardingStyle, DiffusionPolicyConfig, FieldBudgetState,
        FieldExecutionMetrics, FieldSuppressionState, FieldTransferFeatures,
    };

    fn transition_profile(config_id: &str) -> DiffusionPolicyConfig {
        transition_diffusion_profiles(false)
            .into_iter()
            .find(|profile| profile.config_id == config_id)
            .expect("transition profile present")
    }

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
            message_horizon: 30,
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
    fn diffusion_smoke_suite_includes_maintained_olsrv2_baseline() {
        let suite = diffusion_smoke_suite();
        assert!(
            suite
                .runs
                .iter()
                .any(|run| run.policy.config_id == "olsrv2"),
            "diffusion suite should include the maintained OLSRv2 baseline"
        );
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
            .find(|candidate| candidate.config_id == "field-continuity-search-1")
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
    fn field_congestion_cascade_transitions_into_congestion_control_posture() {
        let scenario = build_congestion_cascade_scenario();
        let field = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("field"),
            scenario,
        });
        assert_eq!(field.field_posture_mode.as_deref(), Some("cluster_seeding"));
        assert!(field.field_first_congestion_transition_round.is_some());
    }

    #[test]
    fn scarcity_variant_reduces_energy_and_expensive_transport_use() {
        let scenario = build_energy_starved_relay_scenario();
        let scarcity_policy = field_diffusion_profiles()
            .into_iter()
            .find(|candidate| candidate.config_id == "field-scarcity-search-2")
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
            .find(|candidate| candidate.config_id == "field-congestion-search-2")
            .expect("field congestion memory profile");
        let congestion = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: congestion_policy,
            scenario,
        });
        assert!(
            matches!(
                congestion.field_posture_mode.as_deref(),
                Some("cluster_seeding") | Some("duplicate_suppressed")
            ),
            "unexpected posture: {:?}",
            congestion.field_posture_mode
        );
        assert!(
            congestion.field_redundant_forward_suppression_count
                + congestion.field_same_cluster_suppression_count
                > 0
        );
        assert!(congestion.field_cluster_seed_usage_count > 0);
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
            new_cluster_coverage: false,
            expensive_transport: false,
            continuity_value: false,
            protected_opportunity: false,
        };
        let suppressed = field_forwarding_suppressed(
            DiffusionFieldPosture::DuplicateSuppressed,
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
    fn cluster_seeding_budget_blocks_non_novel_broadcast_forwarding() {
        let scenario = build_congestion_cascade_scenario();
        let covered_clusters = BTreeSet::from([1_u8]);
        let features = FieldTransferFeatures {
            from_cluster_id: 0,
            to_cluster_id: 1,
            receiver_is_target: false,
            sender_is_observer: false,
            receiver_is_observer: false,
            same_cluster: false,
            new_cluster_coverage: false,
            expensive_transport: false,
            continuity_value: false,
            protected_opportunity: false,
        };
        let budget = FieldBudgetState {
            protected_remaining: 2,
            generic_remaining: 2,
            protected_used: 0,
            generic_used: 0,
        };
        let decision = field_budget_kind(
            &scenario,
            Some(DiffusionFieldPosture::ClusterSeeding),
            &features,
            &budget,
            &covered_clusters,
        );
        assert!(decision.is_none());
    }

    #[test]
    fn duplicate_suppressed_allows_one_bounded_completion_forward() {
        let mut metrics = FieldExecutionMetrics::default();
        let suppression_state = FieldSuppressionState::default();
        let features = FieldTransferFeatures {
            from_cluster_id: 1,
            to_cluster_id: 1,
            receiver_is_target: false,
            sender_is_observer: false,
            receiver_is_observer: false,
            same_cluster: true,
            new_cluster_coverage: false,
            expensive_transport: false,
            continuity_value: false,
            protected_opportunity: false,
        };
        let suppressed = field_forwarding_suppressed(
            DiffusionFieldPosture::DuplicateSuppressed,
            6,
            4,
            1,
            900,
            &features,
            &suppression_state,
            &mut metrics,
        );
        assert!(!suppressed);
    }

    #[test]
    fn privacy_variant_reduces_observer_leakage_relative_to_balanced() {
        let scenario = build_adversarial_observation_scenario();
        let privacy_policy = field_diffusion_profiles()
            .into_iter()
            .find(|candidate| candidate.config_id == "field-privacy-search-2")
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
    fn congestion_cascade_tracks_cluster_coverage_separately_from_node_coverage() {
        let scenario = build_congestion_cascade_scenario();
        let summary = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("pathway"),
            scenario,
        });
        assert!(summary.cluster_coverage_permille > summary.coverage_permille);
    }

    #[test]
    fn adversarial_observation_reports_non_zero_leakage_for_broad_baseline() {
        let scenario = build_adversarial_observation_scenario();
        let summary = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: diffusion_engine_profile("pathway"),
            scenario,
        });
        assert!(summary.observer_leakage_permille > 0);
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

    #[test]
    fn diffusion_smoke_suite_includes_large_population_moderate_families() {
        let suite = diffusion_smoke_suite();
        let families = suite
            .runs
            .iter()
            .map(|run| run.family_id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(families.contains("diffusion-large-sparse-threshold-moderate"));
        assert!(families.contains("diffusion-large-congestion-threshold-moderate"));
        assert!(families.contains("diffusion-large-regional-shift-moderate"));
    }

    #[test]
    fn large_sparse_threshold_high_spans_collapse_and_viable_regions() {
        let scenario = build_large_sparse_threshold_high_scenario();
        let summaries = transition_diffusion_profiles(false)
            .into_iter()
            .map(|policy| {
                simulate_diffusion_run(&DiffusionRunSpec {
                    suite_id: "test".to_string(),
                    family_id: scenario.family_id.clone(),
                    seed: 41,
                    policy,
                    scenario: scenario.clone(),
                })
            })
            .collect::<Vec<_>>();
        let states = summaries
            .iter()
            .map(|summary| summary.bounded_state.clone())
            .collect::<BTreeSet<_>>();
        assert!(states.contains("explosive"), "states: {:?}", states);
        assert!(states.contains("viable"), "states: {:?}", states);
    }

    #[test]
    fn large_population_diffusion_families_are_deterministic_and_aggregate() {
        let scenarios = vec![
            build_large_sparse_threshold_moderate_scenario(),
            build_large_sparse_threshold_high_scenario(),
            build_large_congestion_threshold_moderate_scenario(),
            build_large_congestion_threshold_high_scenario(),
            build_large_regional_shift_moderate_scenario(),
            build_large_regional_shift_high_scenario(),
        ];
        let mut summaries = Vec::new();
        for scenario in scenarios {
            let policy =
                if scenario.message_mode == crate::diffusion::DiffusionMessageMode::Broadcast {
                    transition_profile("transition-balanced")
                } else {
                    transition_profile("transition-bridge-biased")
                };
            let first = simulate_diffusion_run(&DiffusionRunSpec {
                suite_id: "test".to_string(),
                family_id: scenario.family_id.clone(),
                seed: 41,
                policy: policy.clone(),
                scenario: scenario.clone(),
            });
            let second = simulate_diffusion_run(&DiffusionRunSpec {
                suite_id: "test".to_string(),
                family_id: scenario.family_id.clone(),
                seed: 41,
                policy,
                scenario,
            });
            assert_eq!(first, second);
            summaries.push(first);
        }
        assert!(!aggregate_diffusion_runs(&summaries).is_empty());
    }

    #[test]
    fn large_congestion_threshold_moderate_spans_collapse_and_viable_regions() {
        let scenario = build_large_congestion_threshold_moderate_scenario();
        let profiles = vec![
            transition_profile("transition-balanced"),
            transition_profile("transition-broad"),
            diffusion_engine_profile("field-congestion"),
        ];
        let states = profiles
            .into_iter()
            .map(|policy| {
                simulate_diffusion_run(&DiffusionRunSpec {
                    suite_id: "test".to_string(),
                    family_id: scenario.family_id.clone(),
                    seed: 41,
                    policy,
                    scenario: scenario.clone(),
                })
                .bounded_state
            })
            .collect::<BTreeSet<_>>();
        assert!(states.contains("collapse"), "states: {:?}", states);
        assert!(states.contains("viable"), "states: {:?}", states);
    }

    #[test]
    fn large_regional_shift_high_is_deterministic_for_transition_bridge_bias() {
        let scenario = build_large_regional_shift_high_scenario();
        let policy = transition_profile("transition-bridge-biased");
        let first = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy: policy.clone(),
            scenario: scenario.clone(),
        });
        let second = simulate_diffusion_run(&DiffusionRunSpec {
            suite_id: "test".to_string(),
            family_id: scenario.family_id.clone(),
            seed: 41,
            policy,
            scenario,
        });
        assert_eq!(first, second);
    }

    #[test]
    fn mercator_profile_keeps_broadcast_overload_above_collapse_floor() {
        let policy = diffusion_engine_profile("mercator");
        let cases = [
            (build_disaster_broadcast_scenario(), 500, "viable"),
            (build_high_density_overload_scenario(), 500, "viable"),
            (build_congestion_cascade_scenario(), 500, "viable"),
            (
                build_large_congestion_threshold_moderate_scenario(),
                500,
                "viable",
            ),
            (
                build_large_congestion_threshold_high_scenario(),
                500,
                "viable",
            ),
        ];
        for (scenario, floor, expected_state) in cases {
            let summary = simulate_diffusion_run(&DiffusionRunSpec {
                suite_id: "test".to_string(),
                family_id: scenario.family_id.clone(),
                seed: 41,
                policy: policy.clone(),
                scenario,
            });
            assert!(
                summary.delivery_probability_permille >= floor,
                "{} delivered {} below floor {floor}",
                summary.family_id,
                summary.delivery_probability_permille,
            );
            assert_eq!(
                summary.bounded_state, expected_state,
                "{} bounded state should remain {expected_state}",
                summary.family_id,
            );
        }
    }

    #[test]
    fn large_regional_shift_moderate_changes_contact_shape_by_phase() {
        let scenario = build_large_regional_shift_moderate_scenario();
        let early = generate_contacts(41, &scenario, 6).len();
        let middle = generate_contacts(41, &scenario, scenario.round_count / 2).len();
        let late = generate_contacts(41, &scenario, scenario.round_count - 4).len();
        let distinct: std::collections::HashSet<usize> =
            [early, middle, late].into_iter().collect();
        assert!(distinct.len() > 1);
    }
}
