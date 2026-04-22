//! Engine-public state types for `BatmanBellmanEngine`.
//!
//! Defines the observation records, ranked neighbor tables, and best next-hop
//! entries that travel across the planner and runtime module boundaries, along
//! with the decay window that governs observation freshness and refresh
//! cadence.
//!
//! Key types:
//! - `DecayWindow` — configures how many ticks an observation remains fresh
//!   (`stale_after_ticks`) and how often the engine should re-run its tick to
//!   refresh tables (`next_refresh_within_ticks`).
//! - `OgmReceiveWindow` — the classic B.A.T.M.A.N.-style per-neighbor receive
//!   window keyed by originator sequence number.
//! - `OriginatorObservation` — one entry in the per-originator, per-neighbor
//!   observation table: TQ score, hop count, tick, transport kind, degradation.
//! - `NeighborRanking` — the ranked list of neighbors for a single originator,
//!   sorted by descending TQ then ascending hop count then neighbor id.
//! - `BestNextHop` — the best neighbor entry selected from the ranking table,
//!   plus the derived `BackendRouteId`, `RouteEpoch`, and replay-visible
//!   BATMAN-native receive-window summary for route construction.
//! - `ActiveBatmanRoute` — the runtime record of an installed route, keyed by
//!   `RouteId`, tracking destination, next-hop, backend id, and install tick.
//! - `OriginatorObservationTable` — the nested `BTreeMap` type alias.

use std::collections::BTreeMap;

use jacquard_core::{
    BackendRouteId, NodeId, RatioPermille, RouteDegradation, RouteEpoch, Tick, TransportKind,
};

pub use jacquard_host_support::DecayWindow;

pub(crate) use jacquard_host_support::OgmReceiveWindow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OriginatorObservation {
    pub originator: NodeId,
    pub via_neighbor: NodeId,
    pub tq: RatioPermille,
    pub receive_quality: RatioPermille,
    pub hop_count: u8,
    pub observed_at_tick: Tick,
    pub transport_kind: TransportKind,
    pub degradation: RouteDegradation,
    pub is_bidirectional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NeighborRanking {
    pub originator: NodeId,
    /// Neighbors ranked by descending TQ then ascending hop count then neighbor
    /// id.
    ///
    /// Length is bounded by the number of direct neighbours reachable from the
    /// local node in the current topology, which is bounded by
    /// `NodeProfile::neighbor_state_count_max`.
    pub ranked_neighbors: Vec<OriginatorObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BestNextHop {
    pub originator: NodeId,
    pub next_hop: NodeId,
    pub tq: RatioPermille,
    pub receive_quality: RatioPermille,
    pub hop_count: u8,
    pub updated_at_tick: Tick,
    pub transport_kind: TransportKind,
    pub degradation: RouteDegradation,
    pub backend_route_id: BackendRouteId,
    pub topology_epoch: RouteEpoch,
    pub is_bidirectional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatmanBellmanPlannerSnapshot {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub best_next_hops: BTreeMap<NodeId, BestNextHop>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveBatmanRoute {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
}

pub(crate) type OriginatorObservationTable =
    BTreeMap<NodeId, BTreeMap<NodeId, OriginatorObservation>>;
