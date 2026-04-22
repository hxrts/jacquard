//! Engine-public state types for `BatmanClassicEngine`.
//!
//! The routing observation, ranking, and best-next-hop types follow the shared
//! next-hop shape used across Jacquard's proactive engines.
//!
//! Additional type for the classic engine:
//! - `ReceivedOgmInfo` — stores the TQ and derived hop count from a received
//!   OGM, keyed by `(originator, via_neighbor)` in the engine's
//!   `received_ogm_info` table. Carries the path-quality signal that classic
//!   BATMAN propagates through the OGM itself rather than computing locally.

use std::collections::BTreeMap;

use jacquard_core::{
    BackendRouteId, NodeId, RatioPermille, RouteDegradation, RouteEpoch, Tick, TransportKind,
};

pub use jacquard_host_support::DecayWindow;

pub(crate) use jacquard_host_support::OgmReceiveWindow;

/// Path-quality data extracted from a received OGM for a single
/// `(originator, via_neighbor)` pair.
///
/// `tq` is the TQ scalar the forwarding neighbor encoded in the OGM — their
/// computed path quality from themselves to the originator. `hop_count` is the
/// total path length from the local node to the originator via this neighbor,
/// derived from the OGM's received hop limit:
/// `DEFAULT_OGM_HOP_LIMIT - received_hop_limit + 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReceivedOgmInfo {
    /// TQ the forwarding neighbor encoded in the OGM (their path quality to the
    /// originator). 1000 when received directly from the originator itself.
    pub tq: RatioPermille,
    /// Total hops from local node to originator via this neighbor: 1 for a
    /// direct neighbor, 2 for one relay hop, etc.
    pub hop_count: u8,
}

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
    /// Neighbors ranked by descending receive-window quality, then descending
    /// TQ, then ascending hop count, then neighbor id.
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
pub struct BatmanClassicPlannerSnapshot {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub best_next_hops: BTreeMap<NodeId, BestNextHop>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveBatmanClassicRoute {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
}

pub(crate) type OriginatorObservationTable =
    BTreeMap<NodeId, BTreeMap<NodeId, OriginatorObservation>>;
