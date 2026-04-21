//! Engine-public policy and state types for `MercatorEngine`.

// proc-macro-scope: Mercator public state uses explicit derives to preserve its engine boundary.

use jacquard_core::{DurationMs, RouteEpoch, Tick};
use serde::{Deserialize, Serialize};

use crate::evidence::MercatorDiagnostics;

pub(crate) const DEFAULT_ENGINE_TICK_WITHIN_TICKS: u64 = 1;
pub(crate) const DEFAULT_EVIDENCE_VALIDITY_MS: u32 = 1_000;
pub(crate) const DEFAULT_REPAIR_ATTEMPT_COUNT_MAX: u32 = 4;
pub(crate) const DEFAULT_BROKER_OVERLOAD_PRESSURE_THRESHOLD: u16 = 500;
pub(crate) const DEFAULT_CUSTODY_COPY_BUDGET_MAX: u32 = 4;
pub(crate) const DEFAULT_CUSTODY_PROTECTED_BRIDGE_BUDGET: u32 = 1;
pub(crate) const DEFAULT_CUSTODY_PAYLOAD_BYTES_MAX: u32 = 16_384;
pub(crate) const DEFAULT_CUSTODY_LOW_GAIN_FLOOR: u16 = 40;
pub(crate) const DEFAULT_CUSTODY_ENERGY_PRESSURE_THRESHOLD: u16 = 900;
pub(crate) const DEFAULT_CUSTODY_LEAKAGE_RISK_THRESHOLD: u16 = 900;
pub(crate) const DEFAULT_NEIGHBOR_COUNT_MAX: u32 = 16;
pub(crate) const DEFAULT_CANDIDATE_BROKER_COUNT_MAX: u32 = 8;
pub(crate) const DEFAULT_SERVICE_EVIDENCE_COUNT_MAX: u32 = 16;
pub(crate) const DEFAULT_CORRIDOR_ALTERNATE_COUNT_MAX: u32 = 8;
pub(crate) const DEFAULT_CUSTODY_OPPORTUNITY_COUNT_MAX: u32 = 16;
pub(crate) const DEFAULT_CUSTODY_RECORD_COUNT_MAX: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercatorOperationalBounds {
    pub engine_tick_within: Tick,
    pub evidence_validity: DurationMs,
    pub repair_attempt_count_max: u32,
    pub broker_overload_pressure_threshold: u16,
    pub custody_copy_budget_max: u32,
    pub custody_protected_bridge_budget: u32,
    pub custody_payload_bytes_max: u32,
    pub custody_low_gain_floor: u16,
    pub custody_energy_pressure_threshold: u16,
    pub custody_leakage_risk_threshold: u16,
}

impl Default for MercatorOperationalBounds {
    fn default() -> Self {
        Self {
            engine_tick_within: Tick(DEFAULT_ENGINE_TICK_WITHIN_TICKS),
            evidence_validity: DurationMs(DEFAULT_EVIDENCE_VALIDITY_MS),
            repair_attempt_count_max: DEFAULT_REPAIR_ATTEMPT_COUNT_MAX,
            broker_overload_pressure_threshold: DEFAULT_BROKER_OVERLOAD_PRESSURE_THRESHOLD,
            custody_copy_budget_max: DEFAULT_CUSTODY_COPY_BUDGET_MAX,
            custody_protected_bridge_budget: DEFAULT_CUSTODY_PROTECTED_BRIDGE_BUDGET,
            custody_payload_bytes_max: DEFAULT_CUSTODY_PAYLOAD_BYTES_MAX,
            custody_low_gain_floor: DEFAULT_CUSTODY_LOW_GAIN_FLOOR,
            custody_energy_pressure_threshold: DEFAULT_CUSTODY_ENERGY_PRESSURE_THRESHOLD,
            custody_leakage_risk_threshold: DEFAULT_CUSTODY_LEAKAGE_RISK_THRESHOLD,
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
    pub custody_record_count_max: u32,
}

impl Default for MercatorEvidenceBounds {
    fn default() -> Self {
        Self {
            neighbor_count_max: DEFAULT_NEIGHBOR_COUNT_MAX,
            candidate_broker_count_max: DEFAULT_CANDIDATE_BROKER_COUNT_MAX,
            service_evidence_count_max: DEFAULT_SERVICE_EVIDENCE_COUNT_MAX,
            corridor_alternate_count_max: DEFAULT_CORRIDOR_ALTERNATE_COUNT_MAX,
            custody_opportunity_count_max: DEFAULT_CUSTODY_OPPORTUNITY_COUNT_MAX,
            custody_record_count_max: DEFAULT_CUSTODY_RECORD_COUNT_MAX,
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
