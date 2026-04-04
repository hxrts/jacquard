//! Deterministic time model and integer-scaled metric types.

use contour_macros::{bounded_value, id_type, public_model};
use serde::{Deserialize, Serialize};

/// Local monotonic time. Not wall clock.
#[id_type]
pub struct Tick(pub u64);

/// Local duration. Used for timeouts, backoff, and validity windows.
#[id_type]
pub struct DurationMs(pub u32);

/// Deterministic ordering that does not depend on wall clock.
#[id_type]
pub struct OrderStamp(pub u64);

/// Topology and reconfiguration version, distinct from elapsed time.
#[id_type]
pub struct RouteEpoch(pub u64);

/// Integer-scaled ratio, 0..=1000.
#[bounded_value(max = 1000)]
pub struct RatioPermille(pub u16);

#[id_type]
pub struct PriorityPoints(pub u32);

#[id_type]
pub struct HealthScore(pub u32);

#[id_type]
pub struct PenaltyPoints(pub u32);

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_tick: Tick,
    pub end_tick: Tick,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub attempt_count_max: u32,
    pub initial_backoff_ms: DurationMs,
    pub backoff_multiplier_permille: RatioPermille,
    pub backoff_ms_max: DurationMs,
    pub overall_deadline_ms: DurationMs,
}
