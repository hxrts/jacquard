//! Spec-faithful classic BATMAN next-hop routing engine.
//!
//! This engine implements the BATMAN protocol as described in the BATMAN IV
//! specification. The key behavioural properties:
//!
//! - **TQ carried in OGM** — each `OriginatorAdvertisement` encodes a `tq`
//!   field. Originators initialise it to 1000; re-broadcasting nodes apply
//!   `tq_product(local_link_tq, received_tq)` before forwarding. Downstream
//!   nodes read path quality directly from received OGMs.
//! - **Hop-limit-bounded propagation** — OGMs carry a `remaining_hop_limit`
//!   field decremented at each hop. OGMs reaching zero are not re-broadcast,
//!   bounding propagation to `DEFAULT_OGM_HOP_LIMIT` hops.
//! - **No Bellman-Ford** — path quality to remote originators is read from
//!   received OGM TQ values, not computed locally over a gossip-merged topology
//!   graph.
//! - **No TQ enrichment** — quality is derived solely from
//!   `ogm_equivalent_tq(LinkRuntimeState)`; Jacquard-specific link beliefs
//!   (delivery confidence, symmetry, transfer rate, stability horizon) are not
//!   incorporated.
//! - **Echo-only bidirectionality** — a neighbor is confirmed bidirectional
//!   only by receiving a local OGM echoed back via that neighbor. There is no
//!   topology fallback.
//! - **No bootstrap shortcut** — if no OGM receive-window data exists for a
//!   path, no route candidate is produced. The engine starts silent and becomes
//!   active only after OGMs have accumulated window data.
//!
//! These properties make `BatmanClassicEngine` a faithful baseline for
//! comparison against Babel (which was designed to fix exactly the weaknesses
//! of classic DV-gossip protocols: asymmetric-link handling, loop-freedom under
//! topology change, and triggered rather than periodic-only updates).

#![forbid(unsafe_code)]

mod gossip;
mod planner;
mod private_state;
mod public_state;
mod runtime;
mod scoring;
pub mod simulator;

use std::collections::BTreeMap;

use gossip::LearnedAdvertisement;
use jacquard_core::{
    Configuration, ConnectivityPosture, NodeId, Observation, RouteId, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteShapeVisibility, RoutingEngineCapabilities,
    RoutingEngineId,
};
pub use public_state::DecayWindow;
use public_state::{
    ActiveBatmanClassicRoute, BatmanClassicPlannerSnapshot, BestNextHop, NeighborRanking,
    OgmReceiveWindow, OriginatorObservationTable, ReceivedOgmInfo,
};

pub const BATMAN_CLASSIC_ENGINE_ID: RoutingEngineId =
    RoutingEngineId::from_contract_bytes(*b"jacquard.batmanc");

pub const BATMAN_CLASSIC_CAPABILITIES: RoutingEngineCapabilities = RoutingEngineCapabilities {
    engine: BATMAN_CLASSIC_ENGINE_ID,
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

pub struct BatmanClassicEngine<Transport, Effects> {
    local_node_id: NodeId,
    transport: Transport,
    effects: Effects,
    /// Most recently observed topology (direct, not gossip-merged).
    latest_topology: Option<Observation<Configuration>>,
    decay_window: DecayWindow,
    /// Per originator, per forwarding neighbor: OGM receive windows used to
    /// compute window-occupancy receive quality.
    originator_receive_windows: BTreeMap<NodeId, BTreeMap<NodeId, OgmReceiveWindow>>,
    /// Per originator, per forwarding neighbor: TQ scalar and hop count from
    /// the most recently received OGM. Replaces the Bellman-Ford path
    /// computation in the enhanced batman engine.
    received_ogm_info: BTreeMap<NodeId, BTreeMap<NodeId, ReceivedOgmInfo>>,
    /// Echo windows keyed by neighbor: populated when a local OGM is received
    /// back via that neighbor. Used exclusively for bidirectionality gating
    /// (no topology fallback).
    bidirectional_receive_windows: BTreeMap<NodeId, OgmReceiveWindow>,
    /// Best OGM per originator retained for TTL-bounded re-flooding.
    learned_advertisements: BTreeMap<NodeId, LearnedAdvertisement>,
    originator_observations: OriginatorObservationTable,
    neighbor_rankings: BTreeMap<NodeId, NeighborRanking>,
    best_next_hops: BTreeMap<NodeId, BestNextHop>,
    active_routes: BTreeMap<RouteId, ActiveBatmanClassicRoute>,
}

impl<Transport, Effects> BatmanClassicEngine<Transport, Effects> {
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
            originator_receive_windows: BTreeMap::new(),
            received_ogm_info: BTreeMap::new(),
            bidirectional_receive_windows: BTreeMap::new(),
            learned_advertisements: BTreeMap::new(),
            originator_observations: BTreeMap::new(),
            neighbor_rankings: BTreeMap::new(),
            best_next_hops: BTreeMap::new(),
            active_routes: BTreeMap::new(),
        }
    }

    pub(crate) fn planner_snapshot(&self) -> BatmanClassicPlannerSnapshot {
        BatmanClassicPlannerSnapshot {
            local_node_id: self.local_node_id,
            stale_after_ticks: self.decay_window.stale_after_ticks,
            best_next_hops: self.best_next_hops.clone(),
        }
    }
}
