//! Shared reference topology builders built from the mem-* profile crates.
//!
//! Control flow: callers use these helpers to assemble canonical in-memory
//! `Node` and `Link` shapes from the isolated profile/state builders, then hand
//! that world state to the router and mesh engine through the normal
//! composition path. This module is reusable reference composition, not
//! engine-private business logic.

use jacquard_core::{
    ControllerId, DiscoveryScopeId, Link, Node, NodeId, ServiceScope, Tick, TimeWindow,
};
use jacquard_mem_link_profile::{ble_endpoint, SimulatedLinkProfile};
use jacquard_mem_node_profile::{NodeStateSnapshot, SimulatedNodeProfile};
use jacquard_mesh::MESH_ENGINE_ID;

#[must_use]
pub fn route_capable_node(node_byte: u8) -> Node {
    let node_id = NodeId([node_byte; 32]);
    let controller_id = ControllerId([node_byte; 32]);
    let endpoint = ble_endpoint(node_byte);
    SimulatedNodeProfile::route_capable(
        endpoint,
        &MESH_ENGINE_ID,
        ServiceScope::Discovery(DiscoveryScopeId([7; 16])),
        TimeWindow::new(Tick(1), Tick(20)).expect("valid window"),
        Tick(1),
    )
    .build_node(
        node_id,
        controller_id,
        &NodeStateSnapshot::route_capable(Tick(1)),
    )
}

#[must_use]
pub fn active_link(device_byte: u8, confidence: u16) -> Link {
    SimulatedLinkProfile::active_with_confidence(
        ble_endpoint(device_byte),
        jacquard_core::RatioPermille(confidence),
        Tick(1),
    )
    .build()
}
