//! Engine-public state types for `OlsrV2Engine`.
use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{BackendRouteId, NodeId, RouteDegradation, RouteEpoch, Tick, TransportKind};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HoldWindow {
    pub last_observed_at_tick: Tick,
    pub stale_after_ticks: u64,
}

impl HoldWindow {
    #[must_use]
    pub(crate) fn is_live(self, now: Tick) -> bool {
        now.0.saturating_sub(self.last_observed_at_tick.0) <= self.stale_after_ticks
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NeighborLinkState {
    pub neighbor: NodeId,
    pub latest_sequence: u64,
    pub hold_window: HoldWindow,
    pub is_symmetric: bool,
    pub is_mpr_selector: bool,
    pub advertised_symmetric_neighbors: BTreeSet<NodeId>,
    pub advertised_mprs: BTreeSet<NodeId>,
    pub link_cost: u32,
    pub transport_kind: TransportKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TwoHopReachability {
    pub two_hop: NodeId,
    pub via_neighbors: BTreeSet<NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MprSelection {
    pub selected_relays: BTreeSet<NodeId>,
    pub covered_two_hops: BTreeSet<NodeId>,
    pub observed_at_tick: Option<Tick>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TopologyTuple {
    pub originator: NodeId,
    pub advertised_neighbor: NodeId,
    pub seqno: u64,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedOlsrRoute {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub hop_count: u8,
    pub path_cost: u32,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OlsrBestNextHop {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub hop_count: u8,
    pub path_cost: u32,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub updated_at_tick: Tick,
    pub topology_epoch: RouteEpoch,
    pub backend_route_id: BackendRouteId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OlsrPlannerSnapshot {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub best_next_hops: BTreeMap<NodeId, OlsrBestNextHop>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveOlsrRoute {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
}
