//! End-to-end multi-device routing tests over one `SharedInMemoryNetwork`.
//! Builds several `MeshClient` and mesh-plus-batman clients on a four-node
//! topology, activates routes across them through the router-owned
//! canonical path, and asserts that transport, route events, and route
//! handles stay consistent across device boundaries.

use std::collections::BTreeMap;

use jacquard_batman::BATMAN_ENGINE_ID;
use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs,
    Environment, FactSourceClass, NodeId, Observation, OperatingMode,
    OriginAuthenticationClass, PriorityPoints, RatioPermille, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy, RouteServiceKind,
    RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
    SelectedRoutingParameters, Tick, TransportObservation,
};
use jacquard_mem_link_profile::{InMemoryTransport, SharedInMemoryNetwork};
use jacquard_mesh::MESH_ENGINE_ID;
use jacquard_reference_client::{
    build_mesh_batman_client, build_mesh_batman_client_with_profile, build_mesh_client,
    build_mesh_client_with_profile, topology, MeshClient,
};
use jacquard_traits::{
    Router, RoutingControlPlane, RoutingDataPlane, TransportEffects,
};

const NODE_A: NodeId = NodeId([1; 32]);
const NODE_B: NodeId = NodeId([2; 32]);
const NODE_C: NodeId = NodeId([3; 32]);
const NODE_D: NodeId = NodeId([4; 32]);

#[test]
fn multi_device_mesh_routing_uses_shared_router_transport_and_device_boundaries() {
    // Shared four-node topology plus one in-memory network backing all clients.
    let topology = sample_configuration();
    let network = SharedInMemoryNetwork::default();
    let (mut client_a, mut client_b, mut client_c) =
        build_client_triplet(&topology, network);

    // Each client activates its own route to C through its router-owned path.
    let route_a_to_c = Router::activate_route(
        client_a.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("client A route activation");
    let route_b_to_c = Router::activate_route(
        client_b.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("client B route activation");

    // Forward A to B to C and assert that each receiving client observes the
    // payload through its own anti-entropy tick with the expected epoch.
    let payload = b"mesh-e2e";
    forward_and_assert_ingress(
        &mut client_a,
        &route_a_to_c.identity.stamp.route_id,
        &mut client_b,
        payload,
        topology.value.epoch,
        "client A forwards toward B",
        "client B ingress tick",
    );
    forward_and_assert_ingress(
        &mut client_b,
        &route_b_to_c.identity.stamp.route_id,
        &mut client_c,
        payload,
        topology.value.epoch,
        "client B forwards toward C",
        "client C ingress tick",
    );
}

#[test]
fn multi_device_routing_can_span_batman_then_mesh_layers() {
    let topology = mixed_engine_configuration();
    let network = SharedInMemoryNetwork::default();
    let (mut client_a, mut client_b, mut client_c, mut observer_b, mut observer_c) =
        build_mixed_engine_triplet(&topology, network);
    let (route_a_to_b, route_b_to_c) =
        activate_mixed_engine_routes(&mut client_a, &mut client_b);

    assert_route_engines(&route_a_to_b, &route_b_to_c);
    forward_across_batman_then_mesh(
        &mut client_a,
        &mut client_b,
        &mut observer_b,
        &mut observer_c,
        &route_a_to_b.identity.stamp.route_id,
        &route_b_to_c.identity.stamp.route_id,
    );

    let outcome = client_c
        .router_mut()
        .anti_entropy_tick()
        .expect("client C anti-entropy tick");
    assert_eq!(outcome.topology_epoch, jacquard_core::RouteEpoch(3));
}

fn build_client_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (MeshClient, MeshClient, MeshClient) {
    let client_a =
        build_mesh_client(NODE_A, topology.clone(), network.clone(), Tick(2));
    let client_b = build_mesh_client_with_profile(
        NODE_B,
        topology.clone(),
        network.clone(),
        Tick(2),
        relay_profile(),
    );
    let client_c = build_mesh_client(NODE_C, topology.clone(), network, Tick(2));
    (client_a, client_b, client_c)
}

fn build_mixed_engine_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (
    MeshClient,
    MeshClient,
    MeshClient,
    InMemoryTransport,
    InMemoryTransport,
) {
    let b_endpoint = topology.value.nodes[&NODE_B].profile.endpoints.clone();
    let c_endpoint = topology.value.nodes[&NODE_C].profile.endpoints.clone();
    let mut observer_b = InMemoryTransport::attach(NODE_B, b_endpoint, network.clone());
    observer_b.set_ingress_tick(Tick(2));
    let mut observer_c = InMemoryTransport::attach(NODE_C, c_endpoint, network.clone());
    observer_c.set_ingress_tick(Tick(2));
    let client_a =
        build_mesh_batman_client(NODE_A, topology.clone(), network.clone(), Tick(2));
    let client_b = build_mesh_batman_client_with_profile(
        NODE_B,
        topology.clone(),
        network.clone(),
        Tick(2),
        relay_profile(),
    );
    let client_c = build_mesh_batman_client(NODE_C, topology.clone(), network, Tick(2));

    (client_a, client_b, client_c, observer_b, observer_c)
}

fn activate_mixed_engine_routes(
    client_a: &mut MeshClient,
    client_b: &mut MeshClient,
) -> (
    jacquard_core::MaterializedRoute,
    jacquard_core::MaterializedRoute,
) {
    let route_a_to_b = Router::activate_route(
        client_a.router_mut(),
        objective(DestinationId::Node(NODE_B)),
    )
    .expect("client A BATMAN route activation");
    let route_b_to_c = Router::activate_route(
        client_b.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("client B mesh route activation");

    (route_a_to_b, route_b_to_c)
}

fn assert_route_engines(
    route_a_to_b: &jacquard_core::MaterializedRoute,
    route_b_to_c: &jacquard_core::MaterializedRoute,
) {
    assert_eq!(
        route_a_to_b.identity.admission.summary.engine,
        BATMAN_ENGINE_ID
    );
    assert_eq!(
        route_b_to_c.identity.admission.summary.engine,
        MESH_ENGINE_ID
    );
}

fn forward_across_batman_then_mesh(
    client_a: &mut MeshClient,
    client_b: &mut MeshClient,
    observer_b: &mut InMemoryTransport,
    observer_c: &mut InMemoryTransport,
    route_a_to_b: &jacquard_core::RouteId,
    route_b_to_c: &jacquard_core::RouteId,
) {
    let payload = b"dual-engine-hop";
    client_a
        .router_mut()
        .forward_payload(route_a_to_b, payload)
        .expect("client A forwards over BATMAN");
    let received_by_b = drain_payload(observer_b, "observe BATMAN ingress at B");
    assert_eq!(received_by_b, payload);

    client_b
        .router_mut()
        .forward_payload(route_b_to_c, &received_by_b)
        .expect("client B forwards over mesh");
    let received_by_c = drain_payload(observer_c, "observe mesh ingress at C");
    assert_eq!(received_by_c, hex_bytes(payload).into_bytes());
}

fn forward_and_assert_ingress(
    sender: &mut MeshClient,
    route_id: &jacquard_core::RouteId,
    receiver: &mut MeshClient,
    payload: &[u8],
    expected_epoch: jacquard_core::RouteEpoch,
    forward_context: &str,
    tick_context: &str,
) {
    sender
        .router_mut()
        .forward_payload(route_id, payload)
        .expect(forward_context);
    let outcome = receiver
        .router_mut()
        .anti_entropy_tick()
        .expect(tick_context);

    assert_eq!(outcome.topology_epoch, expected_epoch);
    assert_eq!(
        outcome.engine_change,
        jacquard_core::RoutingTickChange::PrivateStateUpdated,
    );
    assert_eq!(
        outcome.engine_tick_hint,
        jacquard_core::RoutingTickHint::WithinTicks(jacquard_core::Tick(1)),
    );
}

fn drain_payload(transport: &mut InMemoryTransport, context: &str) -> Vec<u8> {
    let observations = transport.poll_transport().expect(context);
    assert_eq!(observations.len(), 1);
    match &observations[0] {
        | TransportObservation::PayloadReceived { payload, .. } => payload.clone(),
        | other => panic!("unexpected observation: {other:?}"),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn relay_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: OperatingMode::DenseInteractive,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn sample_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (NODE_A, topology::route_capable_node(1)),
                (NODE_B, topology::route_capable_node(2)),
                (NODE_C, topology::route_capable_node(3)),
                (NODE_D, topology::route_capable_node(4)),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::active_link(2, 950)),
                ((NODE_B, NODE_C), topology::active_link(3, 875)),
                ((NODE_A, NODE_D), topology::active_link(4, 925)),
                ((NODE_B, NODE_D), topology::active_link(4, 900)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille: RatioPermille(150),
                contention_permille: RatioPermille(120),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn mixed_engine_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(3),
            nodes: BTreeMap::from([
                (NODE_A, topology::dual_engine_route_capable_node(1)),
                (NODE_B, topology::dual_engine_route_capable_node(2)),
                (
                    NODE_C,
                    topology::route_capable_node_for_engine(3, &MESH_ENGINE_ID),
                ),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::active_link(2, 940)),
                ((NODE_B, NODE_C), topology::active_link(3, 910)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(75),
                contention_permille: RatioPermille(60),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}
