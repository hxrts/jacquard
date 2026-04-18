//! Engine-public state types for `BabelEngine`.
//!
//! These types represent the Babel-specific route state: route table entries,
//! selected routes, best next-hops, and active (materialized) routes.

use std::collections::BTreeMap;

use jacquard_core::{
    BackendRouteId, NodeId, RatioPermille, RouteDegradation, RouteEpoch, Tick, TransportKind,
};

/// Controls how long route entries remain valid before being pruned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Feasibility distance for a destination: the `(seqno, metric)` of the last
/// feasibly selected route.
///
/// A route entry passes the RFC 8966 feasibility condition if:
/// - `seqno_is_newer(entry.seqno, fd.seqno)`, OR
/// - `entry.seqno == fd.seqno && entry.metric < fd.metric`
///
/// An absent `FeasibilityEntry` for a destination means FD = ∞: any route
/// with a finite metric is feasible (the destination has never been selected,
/// or all routes expired and FD was cleared).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FeasibilityEntry {
    /// Seqno of the last feasibly selected route for this destination.
    pub seqno: u16,
    /// Total path metric of the last feasibly selected route.
    pub metric: u16,
}

/// A route entry in the route table, keyed by (destination, via_neighbor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RouteEntry {
    pub router_id: NodeId,
    pub seqno: u16,
    pub metric: u16,
    pub observed_at_tick: Tick,
}

/// The best selected route for a given destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedBabelRoute {
    pub destination: NodeId,
    pub via_neighbor: NodeId,
    pub metric: u16,
    pub seqno: u16,
    pub router_id: NodeId,
    pub tq: RatioPermille,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub observed_at_tick: Tick,
}

/// Best next-hop for a destination, derived from the selected route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelBestNextHop {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub metric: u16,
    pub tq: RatioPermille,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub updated_at_tick: Tick,
    pub topology_epoch: RouteEpoch,
    pub backend_route_id: BackendRouteId,
}

/// Read-only route-choice view projected from Babel runtime state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelPlannerSnapshot {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub best_next_hops: BTreeMap<NodeId, BabelBestNextHop>,
}

/// An active (materialized) route entry, keyed by `RouteId`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveBabelRoute {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
}
