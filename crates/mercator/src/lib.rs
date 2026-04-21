//! Deterministic hybrid corridor routing engine skeleton for Jacquard.
//!
//! `Mercator` is intended to combine explicit objective search, maintained
//! evidence, corridor continuity, fail-closed stale repair, weakest-flow
//! fairness, and bounded custody. Phase 0 establishes only the crate boundary
//! and router-facing trait surface. It deliberately produces no candidates
//! until the bounded evidence graph and corridor planner exist.

#![forbid(unsafe_code)]

pub mod evidence;
mod planner;
mod public_state;
mod runtime;

pub use evidence::{MercatorDiagnostics, MercatorEvidenceGraph};
use jacquard_core::{
    ConnectivityPosture, NodeId, RouteEpoch, RoutePartitionClass, RouteProtectionClass,
    RouteShapeVisibility, RoutingEngineCapabilities, RoutingEngineId,
};
pub use public_state::{MercatorEngineConfig, MercatorEvidenceBounds, MercatorOperationalBounds};

pub const MERCATOR_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.mercatr");

pub const MERCATOR_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: MERCATOR_ENGINE_ID,
    max_protection: RouteProtectionClass::None,
    max_connectivity: ConnectivityPosture {
        repair: jacquard_core::RouteRepairClass::BestEffort,
        partition: RoutePartitionClass::ConnectedOnly,
    },
    repair_support: jacquard_core::RepairSupport::Unsupported,
    hold_support: jacquard_core::HoldSupport::Unsupported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds: jacquard_core::QuantitativeBoundSupport::Unsupported,
    reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
    route_shape_visibility: RouteShapeVisibility::CorridorEnvelope,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MercatorEngine {
    local_node_id: NodeId,
    config: MercatorEngineConfig,
    latest_topology_epoch: Option<RouteEpoch>,
    evidence: MercatorEvidenceGraph,
}

impl MercatorEngine {
    #[must_use]
    pub fn new(local_node_id: NodeId) -> Self {
        Self::with_config(local_node_id, MercatorEngineConfig::default())
    }

    #[must_use]
    pub fn with_config(local_node_id: NodeId, config: MercatorEngineConfig) -> Self {
        Self {
            local_node_id,
            config,
            latest_topology_epoch: None,
            evidence: MercatorEvidenceGraph::new(config.evidence),
        }
    }

    #[must_use]
    pub fn local_node_id(&self) -> NodeId {
        self.local_node_id
    }

    #[must_use]
    pub fn config(&self) -> MercatorEngineConfig {
        self.config
    }

    #[must_use]
    pub fn latest_topology_epoch(&self) -> Option<RouteEpoch> {
        self.latest_topology_epoch
    }

    #[must_use]
    pub fn evidence(&self) -> &MercatorEvidenceGraph {
        &self.evidence
    }

    #[must_use]
    pub fn diagnostics(&self) -> MercatorDiagnostics {
        self.evidence.diagnostics()
    }
}
