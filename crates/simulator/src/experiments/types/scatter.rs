use super::*;
use jacquard_core::ByteCount;
use jacquard_scatter::{
    ScatterBudgetPolicy, ScatterDecisionThresholds, ScatterEngineConfig, ScatterExpiryPolicy,
    ScatterOperationalBounds, ScatterRegimeThresholds, ScatterTransportPolicy,
};

fn conservative_scatter_config() -> ScatterEngineConfig {
    ScatterEngineConfig {
        expiry: ScatterExpiryPolicy {
            emergency_expiry_ms: DurationMs(10_000),
            normal_expiry_ms: DurationMs(30_000),
            background_expiry_ms: DurationMs(60_000),
        },
        budget: ScatterBudgetPolicy {
            emergency_copy_budget: 6,
            normal_copy_budget: 3,
            background_copy_budget: 1,
        },
        regime: ScatterRegimeThresholds {
            sparse_neighbor_count_max: 1,
            dense_neighbor_count_min: 5,
            constrained_hold_capacity_floor_bytes: ByteCount(2_048),
            constrained_relay_utilization_floor_permille: 650,
            bridging_diversity_floor: 2,
            history_window_ticks: 8,
        },
        decision: ScatterDecisionThresholds {
            sparse_delta_floor: 220,
            dense_delta_floor: 160,
            bridging_delta_floor: 100,
            constrained_delta_floor: 260,
            preferential_handoff_delta_floor: 300,
        },
        transport: ScatterTransportPolicy {
            min_transfer_rate_bytes_per_sec: 96,
            min_stability_horizon_ms: DurationMs(350),
            low_rate_payload_bytes_max: ByteCount(96),
        },
        bounds: ScatterOperationalBounds {
            message_count_max: 24,
            byte_count_max: ByteCount(12_288),
            hold_bytes_reserved: ByteCount(2_048),
            work_step_count_max: 6,
            validity_window_ticks: 10,
            engine_tick_within_ticks: 2,
        },
    }
}

fn degraded_network_scatter_config() -> ScatterEngineConfig {
    ScatterEngineConfig {
        expiry: ScatterExpiryPolicy {
            emergency_expiry_ms: DurationMs(25_000),
            normal_expiry_ms: DurationMs(60_000),
            background_expiry_ms: DurationMs(120_000),
        },
        budget: ScatterBudgetPolicy {
            emergency_copy_budget: 10,
            normal_copy_budget: 5,
            background_copy_budget: 3,
        },
        regime: ScatterRegimeThresholds {
            sparse_neighbor_count_max: 2,
            dense_neighbor_count_min: 3,
            constrained_hold_capacity_floor_bytes: ByteCount(384),
            constrained_relay_utilization_floor_permille: 800,
            bridging_diversity_floor: 1,
            history_window_ticks: 12,
        },
        decision: ScatterDecisionThresholds {
            sparse_delta_floor: 140,
            dense_delta_floor: 90,
            bridging_delta_floor: 40,
            constrained_delta_floor: 170,
            preferential_handoff_delta_floor: 220,
        },
        transport: ScatterTransportPolicy {
            min_transfer_rate_bytes_per_sec: 48,
            min_stability_horizon_ms: DurationMs(180),
            low_rate_payload_bytes_max: ByteCount(192),
        },
        bounds: ScatterOperationalBounds {
            message_count_max: 48,
            byte_count_max: ByteCount(24_576),
            hold_bytes_reserved: ByteCount(768),
            work_step_count_max: 10,
            validity_window_ticks: 16,
            engine_tick_within_ticks: 2,
        },
    }
}

impl ExperimentParameterSet {
    #[must_use]
    pub fn scatter(profile_id: &str) -> Self {
        Self {
            engine_family: "scatter".to_string(),
            config_id: format!("scatter-{profile_id}"),
            comparison_engine_set: Some("scatter".to_string()),
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            scatter_profile_id: Some(profile_id.to_string()),
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
        }
    }

    #[must_use]
    pub fn scatter_config(&self) -> Option<ScatterEngineConfig> {
        match self.scatter_profile_id.as_deref()? {
            "balanced" => Some(ScatterEngineConfig::default()),
            "conservative" => Some(conservative_scatter_config()),
            "degraded-network" => Some(degraded_network_scatter_config()),
            _ => None,
        }
    }
}
