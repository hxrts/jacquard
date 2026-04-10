//! Deterministic time model and integer-scaled metric types.
//!
//! Jacquard is fully deterministic: no floating-point types, no wall-clock
//! APIs, and no ambient randomness appear in routing or protocol state. This
//! module defines the typed time and metric primitives that enforce that
//! invariant throughout the workspace.
//!
//! Core time types: [`Tick`] (local monotonic time, not wall clock),
//! [`DurationMs`] (integer millisecond duration for timeouts and windows),
//! [`OrderStamp`] (deterministic ordering independent of wall clock), and
//! [`RouteEpoch`] (topology and reconfiguration version counter).
//!
//! Metric types: [`ByteCount`] (deterministic byte quantity), [`RatioPermille`]
//! (bounded 0..=1000 integer-scaled ratio), [`PriorityPoints`],
//! [`HealthScore`], and [`PenaltyPoints`]. [`TimeWindow`] models half-open
//! `[start_tick, end_tick)` lease intervals. [`TimeoutPolicy`] packages retry
//! and backoff parameters without referencing wall time.

use std::{
    fmt,
    ops::{Add, Sub},
};

use jacquard_macros::{bounded_value, id_type, public_model};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Generates an `#[id_type]` newtype over a primitive integer, and adds
/// `Display`, `From<$inner>`, `Add`, and `Sub` delegating impls.
/// Use for scalar metrics where arithmetic is semantically valid.
macro_rules! arithmetic_newtype {
    ($name:ident: $inner:ty) => {
        #[id_type]
        pub struct $name(pub $inner);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl Add for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self(self.0.saturating_add(rhs.0))
            }
        }

        impl Sub for $name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self {
                Self(self.0.saturating_sub(rhs.0))
            }
        }
    };
}

/// Local monotonic time. Not wall clock.
#[id_type]
pub struct Tick(pub u64);

// Local duration. Used for timeouts, backoff, and validity windows.
arithmetic_newtype!(DurationMs: u32);

/// Deterministic ordering that does not depend on wall clock.
#[id_type]
pub struct OrderStamp(pub u64);

/// Topology and reconfiguration version, distinct from elapsed time.
#[id_type]
pub struct RouteEpoch(pub u64);

/// Deterministic seed for routing scenario simulation.
#[id_type]
pub struct SimulationSeed(pub u64);

// Deterministic quantity of bytes for budgets, capacities, and size limits.
arithmetic_newtype!(ByteCount: u64);

/// Integer-scaled ratio, 0..=1000.
#[bounded_value(max = 1000)]
pub struct RatioPermille(pub u16);

#[id_type]
pub struct PriorityPoints(pub u32);

arithmetic_newtype!(HealthScore: u32);
arithmetic_newtype!(PenaltyPoints: u32);

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

        Ok(Self {
            start_tick,
            end_tick,
        })
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
