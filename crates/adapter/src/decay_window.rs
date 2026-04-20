//! Shared decay-window type for engines that age observations by tick count.
//!
//! `DecayWindow` configures how many ticks an observation or route entry
//! remains fresh and how soon the next engine refresh should run. Proactive
//! next-hop engines (BATMAN variants, Babel, OLSRv2) use this primitive to
//! prune stale per-neighbor evidence and keep refresh cadence legible.
// proc-macro-scope: adapter support primitive intentionally stays outside #[public_model].

use serde::{Deserialize, Serialize};

/// Per-engine staleness and refresh window configuration.
///
/// `stale_after_ticks` is the number of ticks an observation may age before it
/// is dropped. `next_refresh_within_ticks` is the upper bound on how soon the
/// next engine tick should refresh derived tables.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DecayWindow {
    pub stale_after_ticks: u64,
    pub next_refresh_within_ticks: u64,
}

impl DecayWindow {
    #[must_use]
    pub const fn new(stale_after_ticks: u64, next_refresh_within_ticks: u64) -> Self {
        Self {
            stale_after_ticks,
            next_refresh_within_ticks,
        }
    }
}

impl Default for DecayWindow {
    fn default() -> Self {
        Self {
            stale_after_ticks: 8,
            next_refresh_within_ticks: 4,
        }
    }
}
