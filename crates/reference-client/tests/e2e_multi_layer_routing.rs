//! End-to-end routing tests over one `SharedInMemoryNetwork`. One test
//! covers pathway-only forwarding across a four-node topology. The other
//! covers mixed batman-plus-pathway forwarding across a three-node topology
//! where B is the hinge between the two engines. Both assert that route
//! activation, forwarding, and receiver-side anti-entropy ticks stay
//! consistent across device boundaries.
//!
//! Reading order is bottom-up: world topologies, routing parameters,
//! client builders, observation helpers, then the two tests at the end.

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
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_reference_client::{
    build_pathway_batman_client, build_pathway_batman_client_with_profile,
    build_pathway_client, build_pathway_client_with_profile, topology, PathwayClient,
};
use jacquard_traits::{
    Router, RoutingControlPlane, RoutingDataPlane, TransportEffects,
};

const NODE_A: NodeId = NodeId([1; 32]);
const NODE_B: NodeId = NodeId([2; 32]);
const NODE_C: NodeId = NodeId([3; 32]);
const NODE_D: NodeId = NodeId([4; 32]);

// -- World topologies --------------------------------------------------

/// Four-node pathway topology used by the pathway-only test. A, B, C, D are all
/// pathway route-capable, with links A-B, B-C, A-D, and B-D.
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

/// Three-node mixed-engine topology. A and B are dual-engine (batman plus
/// mesh). C is pathway-only. Links are A-B and B-C, so B is the hinge where
/// the batman leg hands off to the pathway leg.
fn mixed_engine_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(3),
            nodes: BTreeMap::from([
                (NODE_A, topology::dual_engine_route_capable_node(1)),
                (NODE_B, topology::dual_engine_route_capable_node(2)),
                (
                    NODE_C,
                    topology::route_capable_node_for_engine(3, &PATHWAY_ENGINE_ID),
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

// -- Routing parameters ------------------------------------------------

/// Stock `Move`-kind routing objective with link-protected, repairable,
/// partition-tolerant defaults. Parameterized by destination so the tests
/// can activate routes to any node.
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

/// Routing parameters for a dense-interactive relay client. Used for the
/// middle-hop B client in both tests.
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

// -- Client builders ---------------------------------------------------

/// Build three pathway-only clients (A, B, C) attached to one shared network.
/// B gets the relay profile so it can sit on the middle hop.
fn build_client_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (PathwayClient, PathwayClient, PathwayClient) {
    let client_a =
        build_pathway_client(NODE_A, topology.clone(), network.clone(), Tick(2));
    let client_b = build_pathway_client_with_profile(
        NODE_B,
        topology.clone(),
        network.clone(),
        Tick(2),
        relay_profile(),
    );
    let client_c = build_pathway_client(NODE_C, topology.clone(), network, Tick(2));
    (client_a, client_b, client_c)
}

/// Build three dual-engine clients (batman + pathway) plus side-channel
/// observers attached to B and C. The observers read ingress straight off
/// the shared network without going through a client's own transport.
fn build_mixed_engine_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (
    PathwayClient,
    PathwayClient,
    PathwayClient,
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
        build_pathway_batman_client(NODE_A, topology.clone(), network.clone(), Tick(2));
    let client_b = build_pathway_batman_client_with_profile(
        NODE_B,
        topology.clone(),
        network.clone(),
        Tick(2),
        relay_profile(),
    );
    let client_c =
        build_pathway_batman_client(NODE_C, topology.clone(), network, Tick(2));

    (client_a, client_b, client_c, observer_b, observer_c)
}

// -- Observation helpers -----------------------------------------------

/// Run one anti-entropy tick on the receiver and assert the router
/// reported the expected topology epoch, a private-state update, and a
/// one-tick scheduling hint. Used to confirm a pathway forward landed.
fn assert_tick_after_forward(
    receiver: &mut PathwayClient,
    expected_epoch: jacquard_core::RouteEpoch,
    tick_context: &str,
) {
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

/// Poll the observer transport once, assert exactly one `PayloadReceived`
/// observation, and return its bytes.
fn drain_payload(transport: &mut InMemoryTransport, context: &str) -> Vec<u8> {
    let observations = transport.poll_transport().expect(context);
    assert_eq!(observations.len(), 1);
    match &observations[0] {
        | TransportObservation::PayloadReceived { payload, .. } => payload.clone(),
        | other => panic!("unexpected observation: {other:?}"),
    }
}

/// Lowercase hex encoding of a byte slice. The pathway carrier hex-encodes
/// payloads on this network, so the second-hop assertion compares against
/// this form rather than the raw bytes.
fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

// -- Tests -------------------------------------------------------------

#[test]
fn pathway_forwarding_across_shared_network() {
    // 1. World. A four-node topology that every client will observe.
    let topology = sample_configuration();

    // 2. Fabric. One in-memory network plays the role of the shared radio.
    let network = SharedInMemoryNetwork::default();

    // 3. Clients. Three pathway clients, each wrapping its own router and engine.
    //    Client B takes the relay profile because it sits on the middle hop.
    let (mut client_a, mut client_b, mut client_c) =
        build_client_triplet(&topology, network);

    // 4. Activation. A and B each ask their router for a route to C. The router
    //    picks candidates, admits one, and returns a canonical handle.
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

    // 5. Hop one. A forwards a payload along its A-to-C route. The first hop lands
    //    at B through the shared network.
    let payload = b"pathway-e2e";
    client_a
        .router_mut()
        .forward_payload(&route_a_to_c.identity.stamp.route_id, payload)
        .expect("client A forwards toward B");
    assert_tick_after_forward(
        &mut client_b,
        topology.value.epoch,
        "client B ingress tick",
    );

    // 6. Hop two. B forwards along its own B-to-C route. Same pattern, next
    //    receiver.
    client_b
        .router_mut()
        .forward_payload(&route_b_to_c.identity.stamp.route_id, payload)
        .expect("client B forwards toward C");
    assert_tick_after_forward(
        &mut client_c,
        topology.value.epoch,
        "client C ingress tick",
    );
}

// long-block-exception: end-to-end scenario traces a single linear routing
// narrative across two engines; splitting would obscure the hop-by-hop
// sequence.
#[test]
fn routing_spans_batman_then_pathway() {
    // 1. World. A three-node topology where A and B run both batman and pathway
    //    engines, and C is pathway-only.
    let topology = mixed_engine_configuration();
    let network = SharedInMemoryNetwork::default();

    // 2. Clients plus two side-channel observers attached to B and C. The observers
    //    let the test read ingress directly off the shared network without routing
    //    through a client's own transport.
    let (mut client_a, mut client_b, mut client_c, mut observer_b, mut observer_c) =
        build_mixed_engine_triplet(&topology, network);

    // 3. Activation. A requests a route to B. The router picks batman because
    //    batman holds next-hop data for that overlay. B requests a route to C. The
    //    router picks pathway for that one.
    let route_a_to_b = Router::activate_route(
        client_a.router_mut(),
        objective(DestinationId::Node(NODE_B)),
    )
    .expect("client A batman route activation");
    let route_b_to_c = Router::activate_route(
        client_b.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("client B pathway route activation");

    // 4. Engine check. Verify that the router actually picked the expected engine
    //    per objective rather than silently falling back.
    assert_eq!(
        route_a_to_b.identity.admission.summary.engine,
        BATMAN_ENGINE_ID
    );
    assert_eq!(
        route_b_to_c.identity.admission.summary.engine,
        PATHWAY_ENGINE_ID
    );

    // 5. Hop one, batman. A forwards the raw payload and B's observer reads it
    //    verbatim because batman relays bytes as-is on this carrier.
    let payload = b"dual-engine-hop";
    client_a
        .router_mut()
        .forward_payload(route_a_to_b.identity.route_id(), payload)
        .expect("client A forwards over batman");
    let received_by_b = drain_payload(&mut observer_b, "observe batman ingress at B");
    assert_eq!(received_by_b, payload);

    // 6. Hop two, pathway. B re-forwards the payload. Pathway hex-encodes payloads
    //    on this carrier, so C observes the hex form instead of the raw bytes.
    client_b
        .router_mut()
        .forward_payload(route_b_to_c.identity.route_id(), &received_by_b)
        .expect("client B forwards over pathway");
    let received_by_c = drain_payload(&mut observer_c, "observe pathway ingress at C");
    assert_eq!(received_by_c, hex_bytes(payload).into_bytes());

    // 7. Epoch check. C's router tick still reports the current topology epoch
    //    after the dual-engine path has completed.
    let outcome = client_c
        .router_mut()
        .anti_entropy_tick()
        .expect("client C anti-entropy tick");
    assert_eq!(outcome.topology_epoch, jacquard_core::RouteEpoch(3));
}
