//! Discoverable reference defaults for the in-memory node preset surface.
//!
//! These values seed the human-facing node preset path. Callers may override
//! them through [`crate::NodePreset`] or by dropping to the lower-level
//! builders.

use jacquard_core::{DiscoveryScopeId, Tick, TimeWindow};

pub use crate::profile::DEFAULT_HOLD_CAPACITY_BYTES;

/// Default discovery scope token used by the standard route-capable node
/// preset.
pub const DEFAULT_ROUTE_SERVICE_SCOPE_ID: [u8; 16] = DiscoveryScopeId([7; 16]).0;
/// Default route-service window length used by the standard node preset.
pub const DEFAULT_ROUTE_SERVICE_WINDOW_TICKS: u64 = 20;

#[must_use]
pub fn default_route_service_window(observed_at_tick: Tick) -> TimeWindow {
    TimeWindow::new(
        observed_at_tick,
        Tick(
            observed_at_tick
                .0
                .saturating_add(DEFAULT_ROUTE_SERVICE_WINDOW_TICKS.saturating_sub(1)),
        ),
    )
    .expect("reference node defaults use a valid service window")
}
