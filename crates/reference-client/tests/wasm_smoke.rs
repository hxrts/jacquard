//! Executable wasm smoke test for the reference-client host bridge.
//!
//! This verifies that the wasm build can construct a client, activate a route,
//! project it, and drive one bridge round under `wasm-bindgen-test`.

#![cfg(target_arch = "wasm32")]

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment, FactSourceClass,
    NodeId, Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteEpoch,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, Tick,
};
use jacquard_reference_client::{
    topology, BridgeRoundProgress, ClientBuilder, ObservedRouteShape, SharedInMemoryNetwork,
    TopologyProjector,
};
use jacquard_traits::Router;
use wasm_bindgen_test::wasm_bindgen_test;

fn routed_topology(
    local_node_id: NodeId,
    relay_node_id: NodeId,
    remote_node_id: NodeId,
    alternate_node_id: NodeId,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(2),
            nodes: BTreeMap::from([
                (local_node_id, topology::node(1).pathway().build()),
                (relay_node_id, topology::node(2).pathway().build()),
                (remote_node_id, topology::node(3).pathway().build()),
                (alternate_node_id, topology::node(4).pathway().build()),
            ]),
            links: BTreeMap::from([
                (
                    (local_node_id, relay_node_id),
                    topology::link(2)
                        .with_confidence(RatioPermille(950))
                        .build(),
                ),
                (
                    (relay_node_id, remote_node_id),
                    topology::link(3)
                        .with_confidence(RatioPermille(875))
                        .build(),
                ),
                (
                    (local_node_id, alternate_node_id),
                    topology::link(4)
                        .with_confidence(RatioPermille(925))
                        .build(),
                ),
                (
                    (relay_node_id, alternate_node_id),
                    topology::link(4)
                        .with_confidence(RatioPermille(900))
                        .build(),
                ),
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

#[wasm_bindgen_test]
fn reference_client_executes_bridge_round_and_projects_active_route_on_wasm() {
    let local_node_id = NodeId([1; 32]);
    let relay_node_id = NodeId([2; 32]);
    let remote_node_id = NodeId([3; 32]);
    let alternate_node_id = NodeId([4; 32]);
    let topology = routed_topology(
        local_node_id,
        relay_node_id,
        remote_node_id,
        alternate_node_id,
    );
    let network = SharedInMemoryNetwork::default();
    let mut client = ClientBuilder::pathway(local_node_id, topology.clone(), network, Tick(2))
        .build()
        .expect("build wasm client");
    let mut bound = client.bind();

    let route = Router::activate_route(
        bound.router_mut(),
        objective(DestinationId::Node(remote_node_id)),
    )
    .expect("activate route on wasm");

    let mut projector = TopologyProjector::new(local_node_id, bound.topology().clone());
    for engine_id in bound.router().registered_engine_ids() {
        let capabilities = bound
            .router()
            .registered_engine_capabilities(&engine_id)
            .expect("registered capabilities");
        projector.ingest_engine_capabilities(capabilities);
    }
    projector.ingest_materialized_route(&route);

    let route_event = bound
        .router()
        .effects()
        .events
        .last()
        .cloned()
        .expect("reference client recorded route event");
    projector.ingest_route_event(&route_event);

    let snapshot = projector.snapshot();
    assert_eq!(snapshot.local_node_id, local_node_id);
    assert_eq!(snapshot.nodes.len(), 4);
    assert_eq!(snapshot.links.len(), 4);

    let projected = snapshot
        .active_routes
        .get(route.identity.route_id())
        .expect("projected active route");
    assert_eq!(projected.destination, DestinationId::Node(remote_node_id));
    assert_eq!(projected.route_shape, ObservedRouteShape::ExplicitPath);

    let progress = bound.advance_round().expect("advance wasm bridge round");
    match progress {
        BridgeRoundProgress::Advanced(report) => {
            assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(2));
        }
        BridgeRoundProgress::Waiting(wait_state) => {
            assert_eq!(wait_state.pending_transport_observations, 0);
        }
    }
}
