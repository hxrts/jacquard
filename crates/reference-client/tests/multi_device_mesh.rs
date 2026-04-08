use std::collections::BTreeMap;

use jacquard_core::{
    SelectedRoutingParameters, Configuration, DiversityFloor, OperatingMode,
    DestinationId, DurationMs, Environment, FactSourceClass, NodeId, Observation,
    OriginAuthenticationClass, PriorityPoints, RatioPermille, ConnectivityPosture,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteReplacementPolicy, RouteServiceKind, RoutingEngineFallbackPolicy,
    RoutingEvidenceClass, RoutingObjective, Tick,
};
use jacquard_mem_link_profile::SharedInMemoryNetwork;
use jacquard_reference_client::{
    build_mesh_client, build_mesh_client_with_profile, fixtures, MeshClient,
};
use jacquard_traits::{Router, RoutingControlPlane, RoutingDataPlane};

const NODE_A: NodeId = NodeId([1; 32]);
const NODE_B: NodeId = NodeId([2; 32]);
const NODE_C: NodeId = NodeId([3; 32]);
const NODE_D: NodeId = NodeId([4; 32]);

#[test]
fn multi_device_mesh_routing_uses_shared_router_transport_and_device_boundaries() {
    let topology = sample_configuration();
    let network = SharedInMemoryNetwork::default();
    let (mut client_a, mut client_b, mut client_c) =
        build_client_triplet(&topology, network);

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

    let payload = b"mesh-e2e";
    forward_and_assert_ingress(
        &mut client_a,
        &route_a_to_c.identity.handle.route_id,
        &mut client_b,
        payload,
        topology.value.epoch,
        "client A forwards toward B",
        "client B ingress tick",
    );
    forward_and_assert_ingress(
        &mut client_b,
        &route_b_to_c.identity.handle.route_id,
        &mut client_c,
        payload,
        topology.value.epoch,
        "client B forwards toward C",
        "client C ingress tick",
    );
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
                (NODE_A, fixtures::route_capable_node(1)),
                (NODE_B, fixtures::route_capable_node(2)),
                (NODE_C, fixtures::route_capable_node(3)),
                (NODE_D, fixtures::route_capable_node(4)),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), fixtures::active_link(2, 950)),
                ((NODE_B, NODE_C), fixtures::active_link(3, 875)),
                ((NODE_A, NODE_D), fixtures::active_link(4, 925)),
                ((NODE_B, NODE_D), fixtures::active_link(4, 900)),
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
