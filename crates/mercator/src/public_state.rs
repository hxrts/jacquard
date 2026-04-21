//! Engine-public policy and state types for `MercatorEngine`.

use jacquard_core::{DurationMs, RouteEpoch, Tick};
use serde::{Deserialize, Serialize};

use crate::evidence::MercatorDiagnostics;

pub(crate) const DEFAULT_ENGINE_TICK_WITHIN_TICKS: u64 = 1;
pub(crate) const DEFAULT_EVIDENCE_VALIDITY_MS: u32 = 1_000;
pub(crate) const DEFAULT_REPAIR_ATTEMPT_COUNT_MAX: u32 = 4;
pub(crate) const DEFAULT_NEIGHBOR_COUNT_MAX: u32 = 16;
pub(crate) const DEFAULT_CANDIDATE_BROKER_COUNT_MAX: u32 = 8;
pub(crate) const DEFAULT_SERVICE_EVIDENCE_COUNT_MAX: u32 = 16;
pub(crate) const DEFAULT_CORRIDOR_ALTERNATE_COUNT_MAX: u32 = 8;
pub(crate) const DEFAULT_CUSTODY_OPPORTUNITY_COUNT_MAX: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorOperationalBounds {
    pub engine_tick_within: Tick,
    pub evidence_validity: DurationMs,
    pub repair_attempt_count_max: u32,
}

impl Default for MercatorOperationalBounds {
    fn default() -> Self {
        Self {
            engine_tick_within: Tick(DEFAULT_ENGINE_TICK_WITHIN_TICKS),
            evidence_validity: DurationMs(DEFAULT_EVIDENCE_VALIDITY_MS),
            repair_attempt_count_max: DEFAULT_REPAIR_ATTEMPT_COUNT_MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorEvidenceBounds {
    pub neighbor_count_max: u32,
    pub candidate_broker_count_max: u32,
    pub service_evidence_count_max: u32,
    pub corridor_alternate_count_max: u32,
    pub custody_opportunity_count_max: u32,
}

impl Default for MercatorEvidenceBounds {
    fn default() -> Self {
        Self {
            neighbor_count_max: DEFAULT_NEIGHBOR_COUNT_MAX,
            candidate_broker_count_max: DEFAULT_CANDIDATE_BROKER_COUNT_MAX,
            service_evidence_count_max: DEFAULT_SERVICE_EVIDENCE_COUNT_MAX,
            corridor_alternate_count_max: DEFAULT_CORRIDOR_ALTERNATE_COUNT_MAX,
            custody_opportunity_count_max: DEFAULT_CUSTODY_OPPORTUNITY_COUNT_MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorEngineConfig {
    pub bounds: MercatorOperationalBounds,
    pub evidence: MercatorEvidenceBounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorRouterAnalysisSnapshot {
    pub diagnostics: MercatorDiagnostics,
    pub active_route_count: u32,
    pub latest_topology_epoch: Option<RouteEpoch>,
}
