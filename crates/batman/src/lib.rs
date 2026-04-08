//! Proactive BATMAN-style next-hop routing engine.
//!
//! Ownership:
//! - router owns canonical route publication, handles, leases, and route truth
//! - BATMAN owns proactive originator observations, neighbor ranking, and
//!   best-next-hop state
//! - route visibility is `NextHopOnly`; the engine does not pretend to expose a
//!   full explicit path
//!
//! This crate implements a proactive next-hop routing model over Jacquard's
//! shared world observations. It starts from an OGM-equivalent baseline for
//! route quality and optionally refines ranking with richer shared link
//! observations when they are present.

#![forbid(unsafe_code)]

mod planner;
mod private_state;
mod public_state;
mod runtime;
mod scoring;

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, NodeId, Observation, RouteId,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteShapeVisibility,
    RoutingEngineCapabilities, RoutingEngineId,
};
use public_state::{
    ActiveBatmanRoute, BestNextHop, DecayWindow, NeighborRanking,
    OriginatorObservationTable,
};

pub const BATMAN_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.batman.");

pub const BATMAN_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: BATMAN_ENGINE_ID,
    max_protection: RouteProtectionClass::LinkProtected,
    max_connectivity: ConnectivityPosture {
        repair: RouteRepairClass::Repairable,
        partition: RoutePartitionClass::ConnectedOnly,
    },
    repair_support: jacquard_core::RepairSupport::Unsupported,
    hold_support: jacquard_core::HoldSupport::Unsupported,
    decidable_admission: jacquard_core::DecidableSupport::Supported,
    quantitative_bounds: jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
    reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
    route_shape_visibility: RouteShapeVisibility::NextHopOnly,
};

pub struct BatmanEngine<Transport, Effects> {
    local_node_id: NodeId,
    transport: Transport,
    effects: Effects,
    latest_topology: Option<Observation<Configuration>>,
    decay_window: DecayWindow,
    originator_observations: OriginatorObservationTable,
    neighbor_rankings: BTreeMap<NodeId, NeighborRanking>,
    best_next_hops: BTreeMap<NodeId, BestNextHop>,
    active_routes: BTreeMap<RouteId, ActiveBatmanRoute>,
}

impl<Transport, Effects> BatmanEngine<Transport, Effects> {
    #[must_use]
    pub fn new(local_node_id: NodeId, transport: Transport, effects: Effects) -> Self {
        Self::with_decay_window(
            local_node_id,
            transport,
            effects,
            DecayWindow::default(),
        )
    }

    #[must_use]
    pub(crate) fn with_decay_window(
        local_node_id: NodeId,
        transport: Transport,
        effects: Effects,
        decay_window: DecayWindow,
    ) -> Self {
        Self {
            local_node_id,
            transport,
            effects,
            latest_topology: None,
            decay_window,
            originator_observations: BTreeMap::new(),
            neighbor_rankings: BTreeMap::new(),
            best_next_hops: BTreeMap::new(),
            active_routes: BTreeMap::new(),
        }
    }
}
