//! Reusable topology fixture builders used across integration tests.
//! Assembles `Node` and `Link` values from the mem-node-profile and
//! mem-link-profile builders. Provides route-capable nodes for a named
//! routing engine and `dual_engine_route_capable_node` for pathway-plus-
//! batman composition.

use jacquard_batman::BATMAN_ENGINE_ID;
use jacquard_core::{
    ByteCount, ControllerId, EndpointLocator, Link, LinkEndpoint, Node, NodeId,
    RoutingEngineId, Tick, TransportKind,
};
use jacquard_mem_link_profile::ReferenceLink;
use jacquard_mem_node_profile::ReferenceNode;
use jacquard_pathway::PATHWAY_ENGINE_ID;

fn reference_endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    LinkEndpoint::new(
        TransportKind::WifiAware,
        EndpointLocator::Opaque(vec![byte]),
        ByteCount(256),
    )
}

#[must_use]
pub fn route_capable_node(node_byte: u8) -> Node {
    route_capable_node_for_engine(node_byte, &PATHWAY_ENGINE_ID)
}

#[must_use]
pub fn route_capable_node_for_engine(node_byte: u8, engine: &RoutingEngineId) -> Node {
    ReferenceNode::route_capable(
        NodeId([node_byte; 32]),
        ControllerId([node_byte; 32]),
        reference_endpoint(node_byte),
        engine,
        Tick(1),
    )
    .build()
}

#[must_use]
pub fn route_capable_node_for_engines(
    node_byte: u8,
    engines: &[RoutingEngineId],
) -> Node {
    ReferenceNode::route_capable_for_engines(
        NodeId([node_byte; 32]),
        ControllerId([node_byte; 32]),
        reference_endpoint(node_byte),
        engines,
        Tick(1),
    )
    .build()
}

#[must_use]
pub fn dual_engine_route_capable_node(node_byte: u8) -> Node {
    route_capable_node_for_engines(node_byte, &[PATHWAY_ENGINE_ID, BATMAN_ENGINE_ID])
}

#[must_use]
pub fn active_link(device_byte: u8, confidence: u16) -> Link {
    ReferenceLink::lossy(
        reference_endpoint(device_byte),
        jacquard_core::RatioPermille(confidence),
        Tick(1),
    )
    .build()
}
