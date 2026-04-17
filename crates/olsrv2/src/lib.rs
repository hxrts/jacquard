//! Deterministic OLSRv2-class link-state routing engine for Jacquard.
//!
//! This crate preserves the key proactive link-state properties that matter for
//! Jacquard's comparison corpus:
//!
//! - HELLO-style neighbor discovery with symmetric-link confirmation
//! - deterministic MPR election over fresh one-hop and two-hop state
//! - TC-style topology advertisement and flooding through MPR selectors
//! - shortest-path computation over an engine-private topology database
//! - next-hop route realization through the shared Jacquard engine contract
//!
//! It intentionally simplifies some RFC 7181 details for the first in-tree
//! baseline:
//!
//! - no wire-compatibility goal with external OLSRv2 daemons
//! - no multi-interface semantics beyond Jacquard's shared link model
//! - no async driver ownership inside the engine
//! - integer-only link metrics derived from shared link observations
//! - advertised-neighbor TC payloads rather than full RFC feature parity

#![forbid(unsafe_code)]

mod gossip;
mod mpr;
mod planner;
mod private_state;
mod public_state;
mod runtime;
pub mod simulator;
mod spf;

use std::collections::{BTreeMap, BTreeSet};

use gossip::TcMessage;
use jacquard_core::{
    Configuration, ConnectivityPosture, NodeId, Observation, RouteId, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteShapeVisibility, RoutingEngineCapabilities,
    RoutingEngineId,
};
pub use public_state::DecayWindow;
use public_state::{
    ActiveOlsrRoute, MprSelection, NeighborLinkState, OlsrBestNextHop, OlsrPlannerSnapshot,
    SelectedOlsrRoute, TopologyTuple, TwoHopReachability,
};

pub const OLSRV2_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.olsrv2.");

pub const OLSRV2_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: OLSRV2_ENGINE_ID,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingTcForward {
    tc: TcMessage,
    received_from: NodeId,
}

pub struct OlsrV2Engine<Transport, Effects> {
    local_node_id: NodeId,
    transport: Transport,
    effects: Effects,
    latest_topology: Option<Observation<Configuration>>,
    decay_window: DecayWindow,
    hello_sequence: u64,
    tc_sequence: u64,
    neighbor_table: BTreeMap<NodeId, NeighborLinkState>,
    two_hop_reachability: BTreeMap<NodeId, TwoHopReachability>,
    local_mpr_selection: MprSelection,
    topology_tuples: BTreeMap<(NodeId, NodeId), TopologyTuple>,
    topology_latest_sequences: BTreeMap<NodeId, (u64, jacquard_core::Tick)>,
    last_forwarded_tc_sequences: BTreeMap<NodeId, u64>,
    pending_tc_forwards: BTreeMap<(NodeId, u64), PendingTcForward>,
    last_originated_tc_neighbors: BTreeSet<NodeId>,
    selected_routes: BTreeMap<NodeId, SelectedOlsrRoute>,
    best_next_hops: BTreeMap<NodeId, OlsrBestNextHop>,
    active_routes: BTreeMap<RouteId, ActiveOlsrRoute>,
}

impl<Transport, Effects> OlsrV2Engine<Transport, Effects> {
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
            hello_sequence: 0,
            tc_sequence: 0,
            neighbor_table: BTreeMap::new(),
            two_hop_reachability: BTreeMap::new(),
            local_mpr_selection: MprSelection::default(),
            topology_tuples: BTreeMap::new(),
            topology_latest_sequences: BTreeMap::new(),
            last_forwarded_tc_sequences: BTreeMap::new(),
            pending_tc_forwards: BTreeMap::new(),
            last_originated_tc_neighbors: BTreeSet::new(),
            selected_routes: BTreeMap::new(),
            best_next_hops: BTreeMap::new(),
            active_routes: BTreeMap::new(),
        }
    }

    pub(crate) fn planner_snapshot(&self) -> OlsrPlannerSnapshot {
        OlsrPlannerSnapshot {
            local_node_id: self.local_node_id,
            stale_after_ticks: self.decay_window.stale_after_ticks,
            best_next_hops: self.best_next_hops.clone(),
        }
    }
}
