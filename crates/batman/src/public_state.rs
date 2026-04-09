//! Engine-public state types for `BatmanEngine`. Defines the observation
//! records, ranked neighbor tables, and best next-hop entries that travel
//! across planner and runtime boundaries, along with the decay window that
//! governs freshness and refresh cadence.

use std::collections::BTreeMap;

use jacquard_core::{
    BackendRouteId, NodeId, RatioPermille, RouteDegradation, RouteEpoch, Tick,
    TransportKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecayWindow {
    pub stale_after_ticks: u64,
    pub next_refresh_within_ticks: u64,
}

impl Default for DecayWindow {
    fn default() -> Self {
        Self {
            stale_after_ticks: 8,
            next_refresh_within_ticks: 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OriginatorObservation {
    pub originator: NodeId,
    pub via_neighbor: NodeId,
    pub tq: RatioPermille,
    pub hop_count: u8,
    pub observed_at_tick: Tick,
    pub transport_kind: TransportKind,
    pub degradation: RouteDegradation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NeighborRanking {
    pub originator: NodeId,
    pub ranked_neighbors: Vec<OriginatorObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BestNextHop {
    pub originator: NodeId,
    pub next_hop: NodeId,
    pub tq: RatioPermille,
    pub hop_count: u8,
    pub updated_at_tick: Tick,
    pub transport_kind: TransportKind,
    pub degradation: RouteDegradation,
    pub backend_route_id: BackendRouteId,
    pub topology_epoch: RouteEpoch,
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
