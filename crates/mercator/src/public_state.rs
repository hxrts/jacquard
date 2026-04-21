//! Engine-public policy and state types for `MercatorEngine`.

use jacquard_core::{DurationMs, Tick};
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_ENGINE_TICK_WITHIN_TICKS: u64 = 1;
pub(crate) const DEFAULT_EVIDENCE_VALIDITY_MS: u32 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorOperationalBounds {
    pub engine_tick_within: Tick,
    pub evidence_validity: DurationMs,
}

impl Default for MercatorOperationalBounds {
    fn default() -> Self {
        Self {
            engine_tick_within: Tick(DEFAULT_ENGINE_TICK_WITHIN_TICKS),
            evidence_validity: DurationMs(DEFAULT_EVIDENCE_VALIDITY_MS),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorEngineConfig {
    pub bounds: MercatorOperationalBounds,
}
