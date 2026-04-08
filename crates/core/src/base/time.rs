//! Deterministic time model and integer-scaled metric types.

use jacquard_macros::{bounded_value, id_type, public_model};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Deterministic seed for routing scenario simulation.
#[id_type]
pub struct SimulationSeed(pub u64);

/// Deterministic quantity of bytes for budgets, capacities, and size limits.
#[id_type]
pub struct ByteCount(pub u64);

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
    start_tick: Tick,
    end_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum TimeWindowError {
    #[error("time window end tick must be greater than start tick")]
    EndNotAfterStart,
}

impl TimeWindow {
    pub fn new(start_tick: Tick, end_tick: Tick) -> Result<Self, TimeWindowError> {
        if end_tick <= start_tick {
            return Err(TimeWindowError::EndNotAfterStart);
        }

        Ok(Self { start_tick, end_tick })
    }

    #[must_use]
    pub fn start_tick(&self) -> Tick {
        self.start_tick
    }

    #[must_use]
    pub fn end_tick(&self) -> Tick {
        self.end_tick
    }

    // Half-open interval [start_tick, end_tick): start is included, end is
    // excluded. Matches standard lease-boundary semantics.
    #[must_use]
    pub fn contains(&self, tick: Tick) -> bool {
        self.start_tick <= tick && tick < self.end_tick
    }
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub attempt_count_max: u32,
    pub initial_backoff_ms: DurationMs,
    pub backoff_multiplier_permille: RatioPermille,
    pub backoff_ms_max: DurationMs,
    pub overall_timeout_ms: DurationMs,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_window_rejects_non_increasing_bounds() {
        assert_eq!(
            TimeWindow::new(Tick(5), Tick(5)),
            Err(TimeWindowError::EndNotAfterStart)
        );
        assert_eq!(
            TimeWindow::new(Tick(6), Tick(5)),
            Err(TimeWindowError::EndNotAfterStart)
        );
    }

    #[test]
    fn time_window_accepts_strictly_increasing_bounds() {
        let window = TimeWindow::new(Tick(5), Tick(6)).expect("valid window");
        assert_eq!(window.start_tick(), Tick(5));
        assert_eq!(window.end_tick(), Tick(6));
        assert!(window.contains(Tick(5)));
        assert!(!window.contains(Tick(6)));
    }
}
