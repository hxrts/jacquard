//! Deterministic bounded deferred-delivery routing engine for Jacquard.
//!
//! `Scatter` is a true router-managed engine. It publishes a conservative
//! route claim for deferred-delivery objectives, then performs bounded
//! store-carry-forward data movement through its own engine-private transport
//! packets behind the shared `RoutingEngine` / `RouterManagedEngine` boundary.
//!
//! Architectural shape:
//! - planner: one conservative advisory route per supported objective
//! - runtime: route materialization, hold-fallback maintenance, and bounded
//!   diffusion of retained payloads across direct contacts
//! - support: deterministic backend-token, wire-packet, regime, and scoring
//!   helpers
//!
//! The first in-tree implementation deliberately keeps the public route shape
//! opaque. It claims bounded deferred delivery and hold support without
//! pretending to expose an explicit path or a stable next hop when none exists.

#![forbid(unsafe_code)]

mod planner;
mod planner_model;
mod public_state;
mod runtime;
mod support;
#[cfg(test)]
mod validation;

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Configuration, ConnectivityPosture, NodeId, Observation, RouteId, RoutePartitionClass,
    RouteProtectionClass, RouteShapeVisibility, RoutingEngineCapabilities, RoutingEngineId,
};
pub use planner_model::{ScatterPlannerModel, ScatterPlannerSeed};
use public_state::ScatterPlannerSnapshot;
pub use public_state::{
    ScatterAction, ScatterBudgetPolicy, ScatterDecisionThresholds, ScatterEngineConfig,
    ScatterExpiryPolicy, ScatterLocalSummary, ScatterOperationalBounds, ScatterRegime,
    ScatterRegimeThresholds, ScatterRouteProgress, ScatterSizeClass, ScatterTransportPolicy,
    ScatterUrgencyClass,
};
use support::{ActiveScatterRoute, PeerObservationState, ScatterMessageId, StoredScatterMessage};

pub const SCATTER_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.scatter");

pub const SCATTER_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: SCATTER_ENGINE_ID,
    max_protection: RouteProtectionClass::LinkProtected,
    max_connectivity: ConnectivityPosture {
        repair: jacquard_core::RouteRepairClass::BestEffort,
        partition: RoutePartitionClass::PartitionTolerant,
    },
    repair_support: jacquard_core::RepairSupport::Unsupported,
    hold_support: jacquard_core::HoldSupport::Supported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds: jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
    reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
    route_shape_visibility: RouteShapeVisibility::Opaque,
};

pub struct ScatterEngine<Transport, Effects> {
    local_node_id: NodeId,
    transport: Transport,
    effects: Effects,
    config: ScatterEngineConfig,
    latest_topology: Option<Observation<Configuration>>,
    next_message_sequence: u64,
    peer_observations: BTreeMap<NodeId, PeerObservationState>,
    seen_messages: BTreeSet<ScatterMessageId>,
    stored_messages: BTreeMap<ScatterMessageId, StoredScatterMessage>,
    active_routes: BTreeMap<RouteId, ActiveScatterRoute>,
    current_regime: ScatterRegime,
    last_local_summary: ScatterLocalSummary,
}

impl<Transport, Effects> ScatterEngine<Transport, Effects> {
    #[must_use]
    pub fn new(local_node_id: NodeId, transport: Transport, effects: Effects) -> Self {
        Self::with_config(
            local_node_id,
            transport,
            effects,
            ScatterEngineConfig::default(),
        )
    }

    #[must_use]
    pub fn with_config(
        local_node_id: NodeId,
        transport: Transport,
        effects: Effects,
        config: ScatterEngineConfig,
    ) -> Self {
        Self {
            local_node_id,
            transport,
            effects,
            config,
            latest_topology: None,
            next_message_sequence: 0,
            peer_observations: BTreeMap::new(),
            seen_messages: BTreeSet::new(),
            stored_messages: BTreeMap::new(),
            active_routes: BTreeMap::new(),
            current_regime: ScatterRegime::Dense,
            last_local_summary: ScatterLocalSummary::default(),
        }
    }

    #[must_use]
    pub fn config(&self) -> ScatterEngineConfig {
        self.config
    }

    #[must_use]
    pub fn last_local_summary(&self) -> ScatterLocalSummary {
        self.last_local_summary
    }

    #[must_use]
    pub fn current_regime(&self) -> ScatterRegime {
        self.current_regime
    }

    #[must_use]
    pub fn retained_message_count(&self) -> usize {
        self.stored_messages.len()
    }

    pub(crate) fn planner_snapshot(&self) -> ScatterPlannerSnapshot {
        ScatterPlannerSnapshot {
            local_node_id: self.local_node_id,
            config: self.config,
            current_regime: self.current_regime,
            last_local_summary: self.last_local_summary,
        }
    }
}
