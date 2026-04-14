//! End-to-end routing tests over one `SharedInMemoryNetwork`.
//!
//! Two tests exercise the full host-side composition stack: topology fixtures,
//! client builders, bridge-driven round advancement, outbound flush, and
//! receiver-side ingress stamping.
//!
//! `pathway_forwarding_across_shared_network` uses a four-node pathway-only
//! topology (A, B, C, D) and forwards a payload two hops from A to C through B,
//! asserting route activation, bridge flush, and receiver tick consistency.
//!
//! `routing_spans_batman_then_pathway` uses a three-node mixed-engine topology
//! where A and B run both batman and pathway engines and C is pathway-only.
//! A forwards over batman to B; B re-forwards over pathway to C. The test
//! verifies that the router selects the expected engine per hop, that batman
//! relays bytes verbatim, and that pathway hex-encodes the payload on this
//! carrier.
//!
//! Reading order is bottom-up: world topologies, routing parameters, client
//! builders, observation helpers, then the two tests at the end.

use std::collections::BTreeMap;

use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs, Environment,
    FactSourceClass, NodeId, Observation, OperatingMode, OriginAuthenticationClass, PriorityPoints,
    RatioPermille, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteReplacementPolicy, RouteServiceKind, RoutingEngineFallbackPolicy, RoutingEvidenceClass,
    RoutingObjective, SelectedRoutingParameters, Tick,
};
use jacquard_olsrv2::OLSRV2_ENGINE_ID;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_reference_client::{
    topology, BoundHostBridge, BridgeRoundProgress, ClientBuilder, ReferenceClient,
    ReferenceRouter, SharedInMemoryNetwork,
};
use jacquard_traits::{Router, RoutingDataPlane};

const NODE_A: NodeId = NodeId([1; 32]);
const NODE_B: NodeId = NodeId([2; 32]);
const NODE_C: NodeId = NodeId([3; 32]);
const NODE_D: NodeId = NodeId([4; 32]);
const BATMAN_GOSSIP_MAGIC: &[u8; 8] = b"JQBATMAN";

// -- World topologies --------------------------------------------------

/// Four-node pathway topology used by the pathway-only test. A, B, C, D are all
/// pathway route-capable, with links A-B, B-C, A-D, and B-D.
fn sample_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (NODE_A, topology::node(1).pathway().build()),
                (NODE_B, topology::node(2).pathway().build()),
                (NODE_C, topology::node(3).pathway().build()),
                (NODE_D, topology::node(4).pathway().build()),
            ]),
            links: BTreeMap::from([
                (
                    (NODE_A, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(950))
                        .build(),
                ),
                (
                    (NODE_B, NODE_C),
                    topology::link(3)
                        .with_confidence(RatioPermille(875))
                        .build(),
                ),
                (
                    (NODE_A, NODE_D),
                    topology::link(4)
                        .with_confidence(RatioPermille(925))
                        .build(),
                ),
                (
                    (NODE_B, NODE_D),
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

/// Three-node mixed-engine topology. A is BATMAN-only, B is dual-engine
/// (batman plus pathway), and C is pathway-only. Links are A-B and B-C, so B
/// is the hinge where the BATMAN leg hands off to the Pathway leg.
fn mixed_engine_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(3),
            nodes: BTreeMap::from([
                (NODE_A, topology::node(1).batman_bellman().build()),
                (
                    NODE_B,
                    topology::node(2).pathway_and_batman_bellman().build(),
                ),
                (
                    NODE_C,
                    topology::node(3).for_engine(&PATHWAY_ENGINE_ID).build(),
                ),
            ]),
            links: BTreeMap::from([
                (
                    (NODE_A, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(940))
                        .build(),
                ),
                (
                    (NODE_B, NODE_A),
                    topology::link(1)
                        .with_confidence(RatioPermille(940))
                        .build(),
                ),
                (
                    (NODE_B, NODE_C),
                    topology::link(3)
                        .with_confidence(RatioPermille(910))
                        .build(),
                ),
                (
                    (NODE_C, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(910))
                        .build(),
                ),
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

/// Three-node OLSRv2-only line topology. A and C are not directly linked, so A
/// must learn C through B before it can route there.
fn olsrv2_line_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(4),
            nodes: BTreeMap::from([
                (NODE_A, topology::node(1).olsrv2().build()),
                (NODE_B, topology::node(2).olsrv2().build()),
                (NODE_C, topology::node(3).olsrv2().build()),
            ]),
            links: BTreeMap::from([
                (
                    (NODE_A, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(940))
                        .build(),
                ),
                (
                    (NODE_B, NODE_A),
                    topology::link(1)
                        .with_confidence(RatioPermille(940))
                        .build(),
                ),
                (
                    (NODE_B, NODE_C),
                    topology::link(3)
                        .with_confidence(RatioPermille(930))
                        .build(),
                ),
                (
                    (NODE_C, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(930))
                        .build(),
                ),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(40),
                contention_permille: RatioPermille(20),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

/// Mixed three-node topology. A runs OLSRv2 only, B can hand off from OLSRv2
/// to Pathway, and C is pathway-only.
fn olsrv2_pathway_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(5),
            nodes: BTreeMap::from([
                (NODE_A, topology::node(1).olsrv2().build()),
                (NODE_B, topology::node(2).pathway_and_olsrv2().build()),
                (NODE_C, topology::node(3).pathway().build()),
            ]),
            links: BTreeMap::from([
                (
                    (NODE_A, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(945))
                        .build(),
                ),
                (
                    (NODE_B, NODE_A),
                    topology::link(1)
                        .with_confidence(RatioPermille(945))
                        .build(),
                ),
                (
                    (NODE_B, NODE_C),
                    topology::link(3)
                        .with_confidence(RatioPermille(915))
                        .build(),
                ),
                (
                    (NODE_C, NODE_B),
                    topology::link(2)
                        .with_confidence(RatioPermille(915))
                        .build(),
                ),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(50),
                contention_permille: RatioPermille(25),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn connected_objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
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
fn client_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (ReferenceClient, ReferenceClient, ReferenceClient) {
    let client_a =
        ClientBuilder::pathway(NODE_A, topology.clone(), network.clone(), Tick(2)).build();
    let client_b = ClientBuilder::pathway(NODE_B, topology.clone(), network.clone(), Tick(2))
        .with_profile(relay_profile())
        .build();
    let client_c = ClientBuilder::pathway(NODE_C, topology.clone(), network, Tick(2)).build();
    (client_a, client_b, client_c)
}

/// Build a mixed client triplet attached to one shared network. A is BATMAN
/// only, B is dual-engine for the handoff point, and C is pathway-only.
fn mixed_engine_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (ReferenceClient, ReferenceClient, ReferenceClient) {
    let client_a =
        ClientBuilder::batman_bellman(NODE_A, topology.clone(), network.clone(), Tick(2))
            .with_profile(relay_profile())
            .build();
    let client_b = ClientBuilder::pathway_and_batman_bellman(
        NODE_B,
        topology.clone(),
        network.clone(),
        Tick(2),
    )
    .with_profile(relay_profile())
    .build();
    let client_c = ClientBuilder::pathway(NODE_C, topology.clone(), network, Tick(2)).build();

    (client_a, client_b, client_c)
}

fn olsrv2_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (ReferenceClient, ReferenceClient, ReferenceClient) {
    let client_a =
        ClientBuilder::olsrv2(NODE_A, topology.clone(), network.clone(), Tick(2)).build();
    let client_b = ClientBuilder::olsrv2(NODE_B, topology.clone(), network.clone(), Tick(2))
        .with_profile(relay_profile())
        .build();
    let client_c = ClientBuilder::olsrv2(NODE_C, topology.clone(), network, Tick(2)).build();
    (client_a, client_b, client_c)
}

fn olsrv2_pathway_triplet(
    topology: &Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> (ReferenceClient, ReferenceClient, ReferenceClient) {
    let client_a =
        ClientBuilder::olsrv2(NODE_A, topology.clone(), network.clone(), Tick(2)).build();
    let client_b =
        ClientBuilder::pathway_and_olsrv2(NODE_B, topology.clone(), network.clone(), Tick(2))
            .with_profile(relay_profile())
            .build();
    let client_c = ClientBuilder::pathway(NODE_C, topology.clone(), network, Tick(2)).build();
    (client_a, client_b, client_c)
}

// -- Observation helpers -----------------------------------------------

/// Run one bridge round on the receiver and assert the router reported the
/// expected topology epoch, a private-state update, and a one-tick scheduling
/// hint. Used to confirm a pathway forward landed.
fn assert_tick_after_forward(
    receiver: &mut BoundHostBridge<'_, ReferenceRouter>,
    expected_epoch: jacquard_core::RouteEpoch,
    tick_context: &str,
) {
    let BridgeRoundProgress::Advanced(report) = receiver.advance_round().expect(tick_context)
    else {
        panic!("expected a bridge-driven round with ingress")
    };
    let outcome = report.router_outcome;

    assert_eq!(outcome.topology_epoch, expected_epoch);
    assert_eq!(
        outcome.engine_change,
        jacquard_core::RoutingTickChange::PrivateStateUpdated,
    );
    assert_eq!(
        outcome.next_round_hint,
        jacquard_core::RoutingTickHint::WithinTicks(jacquard_core::Tick(1)),
    );
}

/// Advance the receiver bridge once, assert at least one `PayloadReceived`
/// observation, and return the first payload bytes.
fn advance_and_capture_payload(
    receiver: &mut BoundHostBridge<'_, ReferenceRouter>,
    expected_epoch: jacquard_core::RouteEpoch,
    context: &str,
) -> Vec<u8> {
    let BridgeRoundProgress::Advanced(report) = receiver.advance_round().expect(context) else {
        panic!("expected a bridge-driven round with ingress")
    };
    assert_eq!(report.router_outcome.topology_epoch, expected_epoch);
    report
        .ingested_transport_observations
        .iter()
        .find_map(|observation| match observation {
            jacquard_core::TransportObservation::PayloadReceived { payload, .. }
                if !payload.starts_with(BATMAN_GOSSIP_MAGIC) =>
            {
                Some(payload.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "expected payload observation, got {:?}",
                report.ingested_transport_observations
            )
        })
}

fn advance_and_capture_exact_payload(
    receiver: &mut BoundHostBridge<'_, ReferenceRouter>,
    expected_epoch: jacquard_core::RouteEpoch,
    expected_payload: &[u8],
    context: &str,
) -> Vec<u8> {
    let BridgeRoundProgress::Advanced(report) = receiver.advance_round().expect(context) else {
        panic!("expected a bridge-driven round with ingress")
    };
    assert_eq!(report.router_outcome.topology_epoch, expected_epoch);
    report
        .ingested_transport_observations
        .iter()
        .find_map(|observation| match observation {
            jacquard_core::TransportObservation::PayloadReceived { payload, .. }
                if payload == expected_payload =>
            {
                Some(payload.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "expected exact payload {:?}, got {:?}",
                expected_payload, report.ingested_transport_observations
            )
        })
}

/// Advance the sender bridge once and assert that it flushed at least one
/// queued transport command after the synchronous router round.
fn flush_sender_round(sender: &mut BoundHostBridge<'_, ReferenceRouter>, context: &str) {
    let BridgeRoundProgress::Advanced(report) = sender.advance_round().expect(context) else {
        panic!("expected a bridge-driven round with outbound flush")
    };
    assert!(report.flushed_transport_commands >= 1);
}

fn advance_convergence_rounds(
    client_a: &mut BoundHostBridge<'_, ReferenceRouter>,
    client_b: &mut BoundHostBridge<'_, ReferenceRouter>,
    client_c: &mut BoundHostBridge<'_, ReferenceRouter>,
    rounds: usize,
) {
    for _ in 0..rounds {
        let _ = client_a
            .advance_round()
            .expect("client A convergence round");
        let _ = client_b
            .advance_round()
            .expect("client B convergence round");
        let _ = client_c
            .advance_round()
            .expect("client C convergence round");
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
    let (mut client_a, mut client_b, mut client_c) = client_triplet(&topology, network);
    let mut client_a = client_a.bind();
    let mut client_b = client_b.bind();
    let mut client_c = client_c.bind();

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
    flush_sender_round(&mut client_a, "client A flush round");
    assert_tick_after_forward(&mut client_b, topology.value.epoch, "client B ingress tick");

    // 6. Hop two. B forwards along its own B-to-C route. Same pattern, next
    //    receiver.
    client_b
        .router_mut()
        .forward_payload(&route_b_to_c.identity.stamp.route_id, payload)
        .expect("client B forwards toward C");
    flush_sender_round(&mut client_b, "client B flush round");
    assert_tick_after_forward(&mut client_c, topology.value.epoch, "client C ingress tick");
}

// long-block-exception: end-to-end scenario traces a single linear routing
// narrative across two engines; splitting would obscure the hop-by-hop
// sequence.
#[test]
fn routing_spans_batman_then_pathway() {
    // 1. World. A three-node topology where A is BATMAN-only, B can hand off
    //    from BATMAN to Pathway, and C is pathway-only.
    let topology = mixed_engine_configuration();
    let network = SharedInMemoryNetwork::default();

    // 2. Clients plus two side-channel observers attached to B and C. The observers
    //    let the test read ingress directly off the shared network without routing
    //    through a client's own transport.
    let (mut client_a, mut client_b, mut client_c) = mixed_engine_triplet(&topology, network);
    let mut client_a = client_a.bind();
    let mut client_b = client_b.bind();
    let mut client_c = client_c.bind();

    let route_b_to_c = Router::activate_route(
        client_b.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("client B pathway route activation");

    // 3. Activation. A requests a route to B. The router picks BATMAN because
    //    BATMAN's direct-neighbor route is admissible on this connected-only
    //    objective. B's route to C stays on pathway.
    let route_a_to_b = Router::activate_route(
        client_a.router_mut(),
        connected_objective(DestinationId::Node(NODE_B)),
    )
    .expect("client A batman route activation");

    // 5. Engine check. Verify that the router actually picked the expected engine
    //    per objective rather than silently falling back.
    assert_eq!(
        route_a_to_b.identity.admission.summary.engine,
        BATMAN_BELLMAN_ENGINE_ID
    );
    assert_eq!(
        route_b_to_c.identity.admission.summary.engine,
        PATHWAY_ENGINE_ID
    );

    // 6. Hop one, batman. A forwards the raw payload and B's bridge delivers the
    //    stamped ingress observation verbatim because batman relays bytes as-is on
    //    this carrier.
    let payload = b"dual-engine-hop";
    client_a
        .router_mut()
        .forward_payload(route_a_to_b.identity.route_id(), payload)
        .expect("client A forwards over batman");
    flush_sender_round(&mut client_a, "client A batman flush round");
    let received_by_b =
        advance_and_capture_payload(&mut client_b, topology.value.epoch, "client B bridge round");
    assert_eq!(received_by_b, payload);

    // 7. Hop two, pathway. B re-forwards the payload. Pathway hex-encodes payloads
    //    on this carrier, so C observes the hex form instead of the raw bytes.
    client_b
        .router_mut()
        .forward_payload(route_b_to_c.identity.route_id(), &received_by_b)
        .expect("client B forwards over pathway");
    flush_sender_round(&mut client_b, "client B pathway flush round");
    let received_by_c =
        advance_and_capture_payload(&mut client_c, topology.value.epoch, "client C bridge round");
    assert_eq!(received_by_c, hex_bytes(payload).into_bytes());

    // 8. Epoch check. C's router tick still reports the current topology epoch
    //    after the dual-engine path has completed, regardless of whether the bridge
    //    reports an idle wait state or a proactive private-state round.
    match client_c.advance_round().expect("client C router round") {
        BridgeRoundProgress::Advanced(report) => {
            assert_eq!(
                report.router_outcome.topology_epoch,
                jacquard_core::RouteEpoch(3)
            );
        }
        BridgeRoundProgress::Waiting(_) => {}
    }
}

#[test]
fn olsrv2_forwarding_across_shared_network() {
    let topology = olsrv2_line_configuration();
    let network = SharedInMemoryNetwork::default();
    let (mut client_a, mut client_b, mut client_c) = olsrv2_triplet(&topology, network);
    let mut client_a = client_a.bind();
    let mut client_b = client_b.bind();
    let mut client_c = client_c.bind();

    advance_convergence_rounds(&mut client_a, &mut client_b, &mut client_c, 3);

    let route_a_to_c = Router::activate_route(
        client_a.router_mut(),
        connected_objective(DestinationId::Node(NODE_C)),
    )
    .expect("client A OLSRv2 route activation");
    let route_b_to_c = Router::activate_route(
        client_b.router_mut(),
        connected_objective(DestinationId::Node(NODE_C)),
    )
    .expect("client B OLSRv2 route activation");

    assert_eq!(
        route_a_to_c.identity.admission.summary.engine,
        OLSRV2_ENGINE_ID
    );
    assert_eq!(
        route_b_to_c.identity.admission.summary.engine,
        OLSRV2_ENGINE_ID
    );

    let payload = b"olsrv2-e2e";
    client_a
        .router_mut()
        .forward_payload(route_a_to_c.identity.route_id(), payload)
        .expect("client A forwards over OLSRv2");
    flush_sender_round(&mut client_a, "client A OLSRv2 flush round");
    let received_by_b = advance_and_capture_exact_payload(
        &mut client_b,
        topology.value.epoch,
        payload,
        "client B OLSRv2 round",
    );
    assert_eq!(received_by_b, payload);

    client_b
        .router_mut()
        .forward_payload(route_b_to_c.identity.route_id(), &received_by_b)
        .expect("client B forwards over OLSRv2");
    flush_sender_round(&mut client_b, "client B OLSRv2 flush round");
    let received_by_c = advance_and_capture_exact_payload(
        &mut client_c,
        topology.value.epoch,
        payload,
        "client C OLSRv2 round",
    );
    assert_eq!(received_by_c, payload);
}

#[test]
fn routing_spans_olsrv2_then_pathway() {
    let topology = olsrv2_pathway_configuration();
    let network = SharedInMemoryNetwork::default();
    let (mut client_a, mut client_b, mut client_c) = olsrv2_pathway_triplet(&topology, network);
    let mut client_a = client_a.bind();
    let mut client_b = client_b.bind();
    let mut client_c = client_c.bind();

    let route_b_to_c = Router::activate_route(
        client_b.router_mut(),
        objective(DestinationId::Node(NODE_C)),
    )
    .expect("client B pathway route activation");
    advance_convergence_rounds(&mut client_a, &mut client_b, &mut client_c, 2);
    let route_a_to_b = Router::activate_route(
        client_a.router_mut(),
        connected_objective(DestinationId::Node(NODE_B)),
    )
    .expect("client A OLSRv2 route activation");

    assert_eq!(
        route_a_to_b.identity.admission.summary.engine,
        OLSRV2_ENGINE_ID
    );
    assert_eq!(
        route_b_to_c.identity.admission.summary.engine,
        PATHWAY_ENGINE_ID
    );

    let payload = b"olsrv2-pathway-hop";
    client_a
        .router_mut()
        .forward_payload(route_a_to_b.identity.route_id(), payload)
        .expect("client A forwards over OLSRv2");
    flush_sender_round(&mut client_a, "client A OLSRv2 flush round");
    let received_by_b = advance_and_capture_exact_payload(
        &mut client_b,
        topology.value.epoch,
        payload,
        "client B ingress round",
    );
    assert_eq!(received_by_b, payload);

    client_b
        .router_mut()
        .forward_payload(route_b_to_c.identity.route_id(), &received_by_b)
        .expect("client B forwards over pathway");
    flush_sender_round(&mut client_b, "client B pathway flush round");
    let expected_hex = hex_bytes(payload).into_bytes();
    let received_by_c = advance_and_capture_exact_payload(
        &mut client_c,
        topology.value.epoch,
        &expected_hex,
        "client C ingress round",
    );
    assert_eq!(received_by_c, expected_hex);
}
