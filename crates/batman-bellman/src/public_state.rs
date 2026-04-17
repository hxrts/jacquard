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

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    BackendRouteId, NodeId, RatioPermille, RouteDegradation, RouteEpoch, Tick, TransportKind,
};

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OgmReceiveWindow {
    pub latest_sequence: Option<u64>,
    pub received_sequences: BTreeSet<u64>,
    pub last_observed_at_tick: Option<Tick>,
}

impl OgmReceiveWindow {
    pub(crate) fn observe(&mut self, sequence: u64, observed_at_tick: Tick, window_span: u64) {
        self.latest_sequence = Some(
            self.latest_sequence
                .map_or(sequence, |known| known.max(sequence)),
        );
        self.received_sequences.insert(sequence);
        self.last_observed_at_tick = Some(observed_at_tick);
        self.prune(observed_at_tick, window_span, window_span);
    }

    pub(crate) fn prune(&mut self, now: Tick, stale_after_ticks: u64, window_span: u64) {
        if let Some(last_seen) = self.last_observed_at_tick {
            if now.0.saturating_sub(last_seen.0) > stale_after_ticks {
                self.latest_sequence = None;
                self.received_sequences.clear();
                self.last_observed_at_tick = None;
                return;
            }
        }
        if let Some(latest_sequence) = self.latest_sequence {
            let lower_bound = latest_sequence.saturating_sub(window_span.saturating_sub(1));
            self.received_sequences
                .retain(|sequence| *sequence >= lower_bound);
            if self.received_sequences.is_empty() {
                self.latest_sequence = None;
                self.last_observed_at_tick = None;
            }
        }
    }

    pub(crate) fn would_be_live_after_prune(
        &self,
        now: Tick,
        stale_after_ticks: u64,
        window_span: u64,
    ) -> bool {
        if let Some(last_seen) = self.last_observed_at_tick {
            if now.0.saturating_sub(last_seen.0) > stale_after_ticks {
                return false;
            }
        }
        if let Some(latest_sequence) = self.latest_sequence {
            let lower_bound = latest_sequence.saturating_sub(window_span.saturating_sub(1));
            self.received_sequences
                .iter()
                .any(|seq| *seq >= lower_bound)
        } else {
            false
        }
    }

    pub(crate) fn packet_count(&self) -> usize {
        self.received_sequences.len()
    }

    pub(crate) fn occupancy_permille(&self, window_span: u64) -> RatioPermille {
        if window_span == 0 {
            return RatioPermille(0);
        }
        let count = u64::try_from(self.packet_count()).unwrap_or(u64::MAX);
        let value = count.saturating_mul(1000) / window_span;
        RatioPermille(u16::try_from(value.min(1000)).expect("permille occupancy"))
    }

    pub(crate) fn is_live(&self) -> bool {
        !self.received_sequences.is_empty()
    }
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
    /// Neighbors ranked by descending TQ then ascending hop count then neighbor
    /// id.
    ///
    /// Length is bounded by the number of direct neighbours reachable from the
    /// local node in the current topology, which is bounded by
    /// `NodeProfile::neighbor_state_count_max`.
    pub ranked_neighbors: Vec<OriginatorObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BestNextHop {
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
pub(crate) struct BatmanBellmanPlannerSnapshot {
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
