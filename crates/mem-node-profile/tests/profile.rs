//! Integration tests for `jacquard-mem-node-profile`.
//!
//! These tests exercise the full construction pipeline from builder to
//! `jacquard-core` model types without importing any router or engine logic.
//! They verify that `SimulatedNodeProfile` and `NodeStateSnapshot` produce
//! correctly shaped `NodeProfile` and `NodeState` values, that service
//! descriptors carry the expected provider identity, and that assembled `Node`
//! values round-trip through `serde_json` without loss.
//!
//! Mutation helpers (`consume_relay_budget`, `reserve_hold_capacity`,
//! `open_connection`) are tested against their expected arithmetic so that
//! test scenarios relying on budget-tracking remain trustworthy.

use jacquard_core::{
    ByteCount, ControllerId, EndpointLocator, LinkEndpoint, NodeId, RatioPermille, RelayWorkBudget,
    RouteServiceKind, ServiceScope, Tick, TimeWindow, TransportKind,
};
use jacquard_mem_node_profile::{
    NodeStateSnapshot, SimulatedNodeProfile, SimulatedServiceDescriptor,
};

fn endpoint(byte: u8) -> LinkEndpoint {
    LinkEndpoint::new(
        TransportKind::WifiAware,
        EndpointLocator::Opaque(vec![byte]),
        ByteCount(256),
    )
}

#[test]
fn simulated_profile_builds_node_profile_and_services() {
    let node_id = NodeId([7; 32]);
    let controller_id = ControllerId([8; 32]);
    let profile = SimulatedNodeProfile::new()
        .with_connection_limits(8, 4, 2, 2)
        .with_work_budgets(10, 4)
        .with_hold_limits(4, ByteCount(8192))
        .with_endpoint(endpoint(1))
        .with_service(
            SimulatedServiceDescriptor::new(RouteServiceKind::Move)
                .with_endpoint(endpoint(1))
                .with_scope(ServiceScope::Discovery(jacquard_core::DiscoveryScopeId(
                    [1; 16],
                )))
                .with_valid_for(TimeWindow::new(Tick(1), Tick(10)).expect("valid service window")),
        )
        .build(node_id, controller_id);

    assert_eq!(profile.connection_count_max, 8);
    assert_eq!(profile.relay_work_budget_max, RelayWorkBudget(10));
    assert_eq!(profile.hold_capacity_bytes_max, ByteCount(8192));
    assert_eq!(profile.services.len(), 1);
    assert_eq!(profile.services[0].provider_node_id, node_id);
}

#[test]
fn node_state_snapshot_tracks_budget_and_capacity_changes() {
    let mut state = NodeStateSnapshot::new()
        .with_relay_state(10, RatioPermille(0), jacquard_core::DurationMs(500))
        .with_available_connections(3)
        .with_hold_capacity(ByteCount(2048))
        .with_information_set(4, ByteCount(1024), RatioPermille(20))
        .with_observed_at_tick(Tick(5));

    state.consume_relay_budget(4);
    state.reserve_hold_capacity(ByteCount(256));
    state.open_connection();

    let built = state.build();
    let relay = match built.relay_budget {
        jacquard_core::Belief::Estimated(estimate) => estimate,
        _ => panic!("expected estimated relay budget"),
    };
    let available_connections = match built.available_connection_count {
        jacquard_core::Belief::Estimated(estimate) => estimate.value,
        _ => panic!("expected estimated connection count"),
    };
    let hold_bytes = match built.hold_capacity_available_bytes {
        jacquard_core::Belief::Estimated(estimate) => estimate.value,
        _ => panic!("expected estimated hold capacity"),
    };

    let relay_budget = match relay.value.relay_work_budget {
        jacquard_core::Belief::Estimated(estimate) => estimate.value,
        _ => panic!("expected relay work budget"),
    };

    assert_eq!(relay_budget, RelayWorkBudget(6));
    assert_eq!(available_connections, 2);
    assert_eq!(hold_bytes, ByteCount(1792));
}

#[test]
fn simulated_node_profile_serializes_through_core_models() {
    let node = SimulatedNodeProfile::new()
        .with_endpoint(endpoint(1))
        .build_node(
            NodeId([1; 32]),
            ControllerId([2; 32]),
            &NodeStateSnapshot::new(),
        );

    let json = serde_json::to_string(&node).expect("serialize node");
    let restored: jacquard_core::Node = serde_json::from_str(&json).expect("deserialize node");
    assert_eq!(restored.profile.endpoints.len(), 1);
}
