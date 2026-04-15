//! Reference-client builder and projector smoke tests.
//!
//! These cover the minimal host-facing builder flow without crossing the full
//! shared-network forwarding path.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment, FactSourceClass,
    NodeId, Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteEpoch,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, Tick,
};
use jacquard_reference_client::{
    topology, BridgeQueueConfig, BridgeRoundProgress, ClientBuilder, ObservedRouteShape,
    SharedInMemoryNetwork, TopologyProjector,
};
use jacquard_traits::Router;

fn sample_topology(local_node_id: NodeId) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(local_node_id, topology::node(1).pathway().build())]),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

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
        observed_at_tick: Tick(1),
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

#[test]
fn client_builder_constructs_waiting_pathway_bridge() {
    let local_node_id = NodeId([1; 32]);
    let topology = sample_topology(local_node_id);
    let network = SharedInMemoryNetwork::default();
    let mut client = ClientBuilder::pathway(local_node_id, topology, network, Tick(1))
        .build()
        .expect("build pathway client");
    let mut bound = client.bind();

    let progress = bound.advance_round().expect("advance initial round");

    match progress {
        BridgeRoundProgress::Advanced(report) => {
            assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
        }
        BridgeRoundProgress::Waiting(_) => {}
    }
}

#[test]
fn client_builder_accepts_explicit_queue_config_and_profile() {
    let local_node_id = NodeId([1; 32]);
    let mut client = ClientBuilder::pathway(
        local_node_id,
        sample_topology(local_node_id),
        SharedInMemoryNetwork::default(),
        Tick(1),
    )
    .with_queue_config(BridgeQueueConfig::new(1, 1))
    .with_batman_bellman()
    .build()
    .expect("build configured client");
    let mut bound = client.bind();

    let progress = bound.advance_round().expect("advance initial round");

    match progress {
        BridgeRoundProgress::Advanced(report) => {
            assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
        }
        BridgeRoundProgress::Waiting(_) => {}
    }
}

#[test]
fn topology_projector_reads_stable_snapshot_from_reference_client_surfaces() {
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
    let mut client = ClientBuilder::pathway(local_node_id, topology.clone(), network, Tick(1))
        .build()
        .expect("build routed client");
    let mut bound = client.bind();
    let mut projector = TopologyProjector::new(local_node_id, bound.topology().clone());

    for engine_id in bound.router().registered_engine_ids() {
        let capabilities = bound
            .router()
            .registered_engine_capabilities(&engine_id)
            .expect("registered capabilities");
        projector.ingest_engine_capabilities(capabilities);
    }

    let route = Router::activate_route(
        bound.router_mut(),
        objective(DestinationId::Node(remote_node_id)),
    )
    .expect("activate route");
    projector.ingest_materialized_route(&route);
    let route_event = bound
        .router()
        .effects()
        .events
        .last()
        .cloned()
        .expect("recorded route event");
    projector.ingest_route_event(&route_event);

    let snapshot = projector.snapshot();
    assert_eq!(snapshot.local_node_id, local_node_id);
    assert_eq!(snapshot.nodes.len(), 4);
    assert_eq!(snapshot.links.len(), 4);
    let projected = snapshot
        .active_routes
        .get(route.identity.route_id())
        .expect("projected route");
    assert_eq!(projected.destination, DestinationId::Node(remote_node_id));
    assert_eq!(projected.route_shape, ObservedRouteShape::ExplicitPath);
}
