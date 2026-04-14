//! Babel RFC 8966 distance-vector routing engine for Jacquard.
//!
//! This engine implements the Babel routing protocol as described in RFC 8966.
//! Key behavioural properties that differentiate it from classic BATMAN:
//!
//! - **Bidirectional ETX link cost** — link cost uses both forward and reverse
//!   delivery: `256 * 1_000_000 / (fwd_delivery_permille * rev_delivery_permille)`.
//!   This correctly penalises asymmetric links where the reverse path is poor.
//! - **Additive metric** — path metric is `cost + neighbor_metric`, lower is better.
//!   0 = perfect, 0xFFFF = unreachable. Unlike BATMAN's multiplicative TQ.
//! - **No TTL** — OGM-equivalent updates carry no hop limit. Route freshness is
//!   controlled by `DecayWindow` (same as batman-classic).
//! - **No bidirectionality gate** — asymmetry is encoded in the link cost itself.
//!   If the reverse link is absent, cost = `BABEL_INFINITY`, making the route
//!   unusable without a separate echo-window check.
//! - **Selected-route flooding** — each tick floods the originated update plus
//!   re-advertisements of the best (selected) route per destination only. Non-
//!   selected routes are not re-broadcast.
//! - **Simplified route selection** — lowest finite metric wins. No strict
//!   RFC 8966 feasibility distance is enforced; seqno freshness is used instead.
//!   See `private_state` module documentation for the deviation rationale.

#![forbid(unsafe_code)]

mod gossip;
mod planner;
mod private_state;
mod public_state;
mod runtime;
mod scoring;

use std::collections::BTreeMap;

use jacquard_core::{
    BackendRouteId, Configuration, ConnectivityPosture, NodeId, Observation, RouteId,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteShapeVisibility,
    RoutingEngineCapabilities, RoutingEngineId,
};
pub use public_state::DecayWindow;
use public_state::{ActiveBabelRoute, BabelBestNextHop, RouteEntry, SelectedBabelRoute};

pub const BABEL_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.babel..");

pub const BABEL_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: BABEL_ENGINE_ID,
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

pub struct BabelEngine<Transport, Effects> {
    local_node_id: NodeId,
    transport: Transport,
    effects: Effects,
    /// Most recently observed topology (direct, not gossip-merged).
    latest_topology: Option<Observation<Configuration>>,
    decay_window: DecayWindow,
    /// Local originator sequence number, incremented every
    /// `SEQNO_REFRESH_INTERVAL_TICKS` ticks.
    local_seqno: u16,
    /// Route table: destination → (via_neighbor → RouteEntry).
    route_table: BTreeMap<NodeId, BTreeMap<NodeId, RouteEntry>>,
    /// Best selected route per destination, rebuilt from `route_table` each tick.
    selected_routes: BTreeMap<NodeId, SelectedBabelRoute>,
    /// Best next-hop per destination, derived from `selected_routes`.
    best_next_hops: BTreeMap<NodeId, BabelBestNextHop>,
    /// Currently active (materialized) routes keyed by `RouteId`.
    active_routes: BTreeMap<RouteId, ActiveBabelRoute>,
}

impl<Transport, Effects> BabelEngine<Transport, Effects> {
    #[must_use]
    pub fn new(local_node_id: NodeId, transport: Transport, effects: Effects) -> Self {
        Self::with_decay_window(local_node_id, transport, effects, DecayWindow::default())
    }

    #[must_use]
    pub fn with_decay_window(
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
            local_seqno: 0,
            route_table: BTreeMap::new(),
            selected_routes: BTreeMap::new(),
            best_next_hops: BTreeMap::new(),
            active_routes: BTreeMap::new(),
        }
    }
}
