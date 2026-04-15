//! Engine-public policy and state types for `ScatterEngine`.

use jacquard_core::{ByteCount, DurationMs, Tick};
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_EMERGENCY_EXPIRY_MS: u32 = 15_000;
pub(crate) const DEFAULT_NORMAL_EXPIRY_MS: u32 = 45_000;
pub(crate) const DEFAULT_BACKGROUND_EXPIRY_MS: u32 = 90_000;
pub(crate) const DEFAULT_EMERGENCY_COPY_BUDGET: u8 = 8;
pub(crate) const DEFAULT_NORMAL_COPY_BUDGET: u8 = 4;
pub(crate) const DEFAULT_BACKGROUND_COPY_BUDGET: u8 = 2;
pub(crate) const DEFAULT_SPARSE_NEIGHBOR_COUNT_MAX: u32 = 1;
pub(crate) const DEFAULT_DENSE_NEIGHBOR_COUNT_MIN: u32 = 4;
pub(crate) const DEFAULT_CONSTRAINED_HOLD_CAPACITY_FLOOR_BYTES: u32 = 512;
pub(crate) const DEFAULT_CONSTRAINED_RELAY_UTILIZATION_FLOOR_PERMILLE: u16 = 750;
pub(crate) const DEFAULT_BRIDGING_DIVERSITY_FLOOR: u32 = 2;
pub(crate) const DEFAULT_SPARSE_DELTA_FLOOR: i32 = 180;
pub(crate) const DEFAULT_DENSE_DELTA_FLOOR: i32 = 120;
pub(crate) const DEFAULT_BRIDGING_DELTA_FLOOR: i32 = 60;
pub(crate) const DEFAULT_CONSTRAINED_DELTA_FLOOR: i32 = 220;
pub(crate) const DEFAULT_PREFERENTIAL_HANDOFF_DELTA_FLOOR: i32 = 260;
pub(crate) const DEFAULT_MIN_TRANSFER_RATE_BYTES_PER_SEC: u32 = 64;
pub(crate) const DEFAULT_MIN_STABILITY_HORIZON_MS: u32 = 250;
pub(crate) const DEFAULT_LOW_RATE_PAYLOAD_BYTES_MAX: u32 = 128;
pub(crate) const DEFAULT_MESSAGE_COUNT_MAX: u32 = 32;
pub(crate) const DEFAULT_BYTE_COUNT_MAX: u32 = 16_384;
pub(crate) const DEFAULT_HOLD_BYTES_RESERVED: u32 = 1_024;
pub(crate) const DEFAULT_WORK_STEP_COUNT_MAX: u32 = 8;
pub(crate) const DEFAULT_VALIDITY_WINDOW_TICKS: u64 = 12;
pub(crate) const DEFAULT_HISTORY_WINDOW_TICKS: u64 = 8;
pub(crate) const DEFAULT_ENGINE_TICK_WITHIN_TICKS: u64 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScatterUrgencyClass {
    Emergency,
    #[default]
    Normal,
    Background,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScatterSizeClass {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScatterRegime {
    Sparse,
    #[default]
    Dense,
    Bridging,
    Constrained,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScatterAction {
    #[default]
    KeepCarrying,
    Replicate,
    PreferentialHandoff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterExpiryPolicy {
    pub emergency_expiry_ms: DurationMs,
    pub normal_expiry_ms: DurationMs,
    pub background_expiry_ms: DurationMs,
}

impl Default for ScatterExpiryPolicy {
    fn default() -> Self {
        Self {
            emergency_expiry_ms: DurationMs(DEFAULT_EMERGENCY_EXPIRY_MS),
            normal_expiry_ms: DurationMs(DEFAULT_NORMAL_EXPIRY_MS),
            background_expiry_ms: DurationMs(DEFAULT_BACKGROUND_EXPIRY_MS),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterBudgetPolicy {
    pub emergency_copy_budget: u8,
    pub normal_copy_budget: u8,
    pub background_copy_budget: u8,
}

impl Default for ScatterBudgetPolicy {
    fn default() -> Self {
        Self {
            emergency_copy_budget: DEFAULT_EMERGENCY_COPY_BUDGET,
            normal_copy_budget: DEFAULT_NORMAL_COPY_BUDGET,
            background_copy_budget: DEFAULT_BACKGROUND_COPY_BUDGET,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterRegimeThresholds {
    pub sparse_neighbor_count_max: u32,
    pub dense_neighbor_count_min: u32,
    pub constrained_hold_capacity_floor_bytes: ByteCount,
    pub constrained_relay_utilization_floor_permille: u16,
    pub bridging_diversity_floor: u32,
    pub history_window_ticks: u64,
}

impl Default for ScatterRegimeThresholds {
    fn default() -> Self {
        Self {
            sparse_neighbor_count_max: DEFAULT_SPARSE_NEIGHBOR_COUNT_MAX,
            dense_neighbor_count_min: DEFAULT_DENSE_NEIGHBOR_COUNT_MIN,
            constrained_hold_capacity_floor_bytes: ByteCount(u64::from(
                DEFAULT_CONSTRAINED_HOLD_CAPACITY_FLOOR_BYTES,
            )),
            constrained_relay_utilization_floor_permille:
                DEFAULT_CONSTRAINED_RELAY_UTILIZATION_FLOOR_PERMILLE,
            bridging_diversity_floor: DEFAULT_BRIDGING_DIVERSITY_FLOOR,
            history_window_ticks: DEFAULT_HISTORY_WINDOW_TICKS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterDecisionThresholds {
    pub sparse_delta_floor: i32,
    pub dense_delta_floor: i32,
    pub bridging_delta_floor: i32,
    pub constrained_delta_floor: i32,
    pub preferential_handoff_delta_floor: i32,
}

impl Default for ScatterDecisionThresholds {
    fn default() -> Self {
        Self {
            sparse_delta_floor: DEFAULT_SPARSE_DELTA_FLOOR,
            dense_delta_floor: DEFAULT_DENSE_DELTA_FLOOR,
            bridging_delta_floor: DEFAULT_BRIDGING_DELTA_FLOOR,
            constrained_delta_floor: DEFAULT_CONSTRAINED_DELTA_FLOOR,
            preferential_handoff_delta_floor: DEFAULT_PREFERENTIAL_HANDOFF_DELTA_FLOOR,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterTransportPolicy {
    pub min_transfer_rate_bytes_per_sec: u32,
    pub min_stability_horizon_ms: DurationMs,
    pub low_rate_payload_bytes_max: ByteCount,
}

impl Default for ScatterTransportPolicy {
    fn default() -> Self {
        Self {
            min_transfer_rate_bytes_per_sec: DEFAULT_MIN_TRANSFER_RATE_BYTES_PER_SEC,
            min_stability_horizon_ms: DurationMs(DEFAULT_MIN_STABILITY_HORIZON_MS),
            low_rate_payload_bytes_max: ByteCount(u64::from(DEFAULT_LOW_RATE_PAYLOAD_BYTES_MAX)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterOperationalBounds {
    pub message_count_max: u32,
    pub byte_count_max: ByteCount,
    pub hold_bytes_reserved: ByteCount,
    pub work_step_count_max: u32,
    pub validity_window_ticks: u64,
    pub engine_tick_within_ticks: u64,
}

impl Default for ScatterOperationalBounds {
    fn default() -> Self {
        Self {
            message_count_max: DEFAULT_MESSAGE_COUNT_MAX,
            byte_count_max: ByteCount(u64::from(DEFAULT_BYTE_COUNT_MAX)),
            hold_bytes_reserved: ByteCount(u64::from(DEFAULT_HOLD_BYTES_RESERVED)),
            work_step_count_max: DEFAULT_WORK_STEP_COUNT_MAX,
            validity_window_ticks: DEFAULT_VALIDITY_WINDOW_TICKS,
            engine_tick_within_ticks: DEFAULT_ENGINE_TICK_WITHIN_TICKS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScatterEngineConfig {
    pub expiry: ScatterExpiryPolicy,
    pub budget: ScatterBudgetPolicy,
    pub regime: ScatterRegimeThresholds,
    pub decision: ScatterDecisionThresholds,
    pub transport: ScatterTransportPolicy,
    pub bounds: ScatterOperationalBounds,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterLocalSummary {
    pub contact_rate: u32,
    pub distinct_peer_rate: u32,
    pub novelty_rate: u32,
    pub diversity_score: u32,
    pub resource_pressure_permille: u16,
    pub encounter_rate: u32,
    pub scope_encounter_rate: u32,
    pub bridge_score: u32,
    pub scope_bridge_score: u32,
    pub mobility_score: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScatterRouteProgress {
    pub retained_message_count: u32,
    pub delivered_message_count: u32,
    pub last_regime: ScatterRegime,
    pub last_action: ScatterAction,
    pub last_progress_at_tick: Option<Tick>,
}
