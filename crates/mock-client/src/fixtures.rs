//! Shared in-memory topology fixtures built from the mem-* profile crates.
//!
//! Control flow intuition: tests use these helpers to assemble shared `Node`
//! and `Link` objects from the isolated profile/state builders, then hand that
//! world state to the router and mesh engine through the normal composition
//! path.

use jacquard_core::{
    Belief, ByteCount, ControllerId, DiscoveryScopeId, DurationMs, Estimate, Link,
    Node, NodeId, RatioPermille, RouteServiceKind, ServiceScope, Tick, TimeWindow,
};
use jacquard_mem_link_profile::{ble_endpoint, SimulatedLinkProfile};
use jacquard_mem_node_profile::{
    NodeStateSnapshot, SimulatedNodeProfile, SimulatedServiceDescriptor,
};
use jacquard_mesh::MESH_ENGINE_ID;

#[must_use]
pub fn route_capable_node(node_byte: u8) -> Node {
    let node_id = NodeId([node_byte; 32]);
    let controller_id = ControllerId([node_byte; 32]);
    let endpoint = ble_endpoint(node_byte);

    let profile = SimulatedNodeProfile::new()
        .with_endpoint(endpoint.clone())
        .with_connection_count_max(8)
        .with_neighbor_state_count_max(8)
        .with_simultaneous_transfer_count_max(4)
        .with_active_route_count_max(4)
        .with_relay_budget(10)
        .with_maintenance_budget(10)
        .with_hold_item_count(8)
        .with_hold_capacity(ByteCount(8192))
        .with_service(
            SimulatedServiceDescriptor::new(RouteServiceKind::Discover)
                .with_endpoint(endpoint.clone())
                .with_scope(ServiceScope::Discovery(DiscoveryScopeId([7; 16])))
                .with_valid_for(
                    TimeWindow::new(Tick(1), Tick(20)).expect("valid window"),
                )
                .with_repair_capacity(4),
        )
        .with_service(
            SimulatedServiceDescriptor::new(RouteServiceKind::Move)
                .with_endpoint(endpoint.clone())
                .with_scope(ServiceScope::Discovery(DiscoveryScopeId([7; 16])))
                .with_valid_for(
                    TimeWindow::new(Tick(1), Tick(20)).expect("valid window"),
                )
                .with_repair_capacity(4),
        )
        .with_service(
            SimulatedServiceDescriptor::new(RouteServiceKind::Hold)
                .with_endpoint(endpoint)
                .with_scope(ServiceScope::Discovery(DiscoveryScopeId([7; 16])))
                .with_valid_for(
                    TimeWindow::new(Tick(1), Tick(20)).expect("valid window"),
                )
                .with_repair_capacity(4),
        );
    let state = NodeStateSnapshot::new()
        .with_relay_budget(8)
        .with_available_connections(4)
        .with_hold_capacity(ByteCount(4096))
        .with_information_summary(4, ByteCount(2048), RatioPermille(10))
        .with_observed_at_tick(Tick(1));

    let mut node = profile.build_node(node_id, controller_id, &state);
    for service in &mut node.profile.services {
        service.routing_engines = vec![MESH_ENGINE_ID];
        if matches!(service.service_kind, RouteServiceKind::Hold) {
            service.capacity = Belief::Estimated(Estimate {
                value: jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity: Belief::Estimated(Estimate {
                        value: 4,
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    hold_capacity_bytes: Belief::Estimated(Estimate {
                        value: ByteCount(4096),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            });
        }
    }
    node
}

#[must_use]
pub fn active_link(device_byte: u8, confidence: u16) -> Link {
    SimulatedLinkProfile::new(ble_endpoint(device_byte))
        .with_median_rtt(DurationMs(40))
        .with_transfer_rate(2048)
        .with_stability_horizon(DurationMs(500))
        .with_loss(RatioPermille(50))
        .with_delivery_confidence(RatioPermille(confidence))
        .with_symmetry(RatioPermille(900))
        .with_observed_at_tick(Tick(1))
        .build()
}
