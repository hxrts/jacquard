//! Router integration tests for `jacquard-batman`.
//!
//! Exercises `BatmanEngine` wired into a `MultiEngineRouter` with
//! `InMemoryTransport` and `InMemoryRuntimeEffects`, verifying end-to-end
//! behavior from route activation through payload forwarding.
//!
//! Tests in this module confirm:
//! - `BatmanEngine` can be registered with a `MultiEngineRouter` and its
//!   `BATMAN_ENGINE_ID` capabilities are accessible after registration.
//! - `Router::activate_route` selects a next-hop-only route with
//!   `RouteShapeVisibility::NextHopOnly` via the BATMAN engine.
//! - After an `advance_round` tick, `forward_payload` delivers a payload to the
//!   correct next-hop `InMemoryTransport` node, verifiable by draining the
//!   ingress queue of the expected neighbor.
//!
//! The sample topology uses four nodes with two paths to destination node 4:
//! a high-quality path via node 2 and a lossier path via node 3.

use std::collections::BTreeMap;

use jacquard_batman::{BatmanEngine, BATMAN_ENGINE_ID};
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId,
    DurationMs, EndpointLocator, Environment, HealthScore, IdentityAssuranceClass,
    LinkEndpoint, Observation, RatioPermille, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
    RoutingEngineFallbackPolicy, RoutingPolicyInputs, RoutingTickChange,
    SelectedRoutingParameters, Tick, TransportKind,
};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, ReferenceLink, SharedInMemoryNetwork,
};
use jacquard_mem_node_profile::ReferenceNode;
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{Router, RoutingControlPlane, RoutingDataPlane, TransportDriver};

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn endpoint(byte: u8) -> LinkEndpoint {
    LinkEndpoint::new(
        TransportKind::WifiAware,
        EndpointLocator::Opaque(vec![byte]),
        ByteCount(128),
    )
}

// long-block-exception: integration topology intentionally kept inline so the
// mixed next-hop path and weaker fallback path remain readable together.
fn sample_topology() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (
                    node(1),
                    ReferenceNode::route_capable(
                        node(1),
                        ControllerId([1; 32]),
                        endpoint(1),
                        &BATMAN_ENGINE_ID,
                        Tick(1),
                    )
                    .build(),
                ),
                (
                    node(2),
                    ReferenceNode::route_capable(
                        node(2),
                        ControllerId([2; 32]),
                        endpoint(2),
                        &BATMAN_ENGINE_ID,
                        Tick(1),
                    )
                    .build(),
                ),
                (
                    node(3),
                    ReferenceNode::route_capable(
                        node(3),
                        ControllerId([3; 32]),
                        endpoint(3),
                        &BATMAN_ENGINE_ID,
                        Tick(1),
                    )
                    .build(),
                ),
                (
                    node(4),
                    ReferenceNode::route_capable(
                        node(4),
                        ControllerId([4; 32]),
                        endpoint(4),
                        &BATMAN_ENGINE_ID,
                        Tick(1),
                    )
                    .build(),
                ),
            ]),
            links: BTreeMap::from([
                (
                    (node(1), node(2)),
                    ReferenceLink::active(endpoint(2), Tick(1)).build(),
                ),
                (
                    (node(2), node(4)),
                    ReferenceLink::active(endpoint(4), Tick(1)).build(),
                ),
                (
                    (node(1), node(3)),
                    ReferenceLink::lossy(endpoint(3), RatioPermille(650), Tick(1))
                        .build(),
                ),
                (
                    (node(3), node(4)),
                    ReferenceLink::lossy(endpoint(4), RatioPermille(600), Tick(1))
                        .build(),
                ),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(50),
                contention_permille: RatioPermille(25),
            },
        },
        source_class: jacquard_core::FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn sample_policy_inputs(topology: &Observation<Configuration>) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&node(1)].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(100),
        adversary_pressure_permille: RatioPermille(0),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

fn sample_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn sample_objective() -> jacquard_core::RoutingObjective {
    jacquard_core::RoutingObjective {
        destination: DestinationId::Node(node(4)),
        service_kind: jacquard_core::RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(100)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(10),
    }
}

#[test]
fn batman_router_activates_next_hop_only_routes() {
    let topology = sample_topology();
    let network = SharedInMemoryNetwork::default();
    let local_transport = InMemoryTransport::attach(
        node(1),
        topology.value.nodes[&node(1)].profile.endpoints.clone(),
        network,
    );
    let engine = BatmanEngine::new(
        node(1),
        local_transport,
        InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
    );
    let mut router = MultiEngineRouter::new(
        node(1),
        FixedPolicyEngine::new(sample_profile()),
        InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
        topology.clone(),
        sample_policy_inputs(&topology),
    );
    router
        .register_engine(Box::new(engine))
        .expect("register BATMAN engine");

    let route = Router::activate_route(&mut router, sample_objective())
        .expect("activate route");

    assert_eq!(route.identity.admission.summary.engine, BATMAN_ENGINE_ID);
    assert_eq!(
        router
            .registered_engine_capabilities(&BATMAN_ENGINE_ID)
            .expect("registered BATMAN capabilities")
            .route_shape_visibility,
        jacquard_core::RouteShapeVisibility::NextHopOnly
    );
}

#[test]
fn batman_router_composes_with_in_memory_transport_and_private_ticks() {
    let topology = sample_topology();
    let network = SharedInMemoryNetwork::default();
    let local_transport = InMemoryTransport::attach(
        node(1),
        topology.value.nodes[&node(1)].profile.endpoints.clone(),
        network.clone(),
    );
    let mut next_hop_transport = InMemoryTransport::attach(
        node(2),
        topology.value.nodes[&node(2)].profile.endpoints.clone(),
        network,
    );

    let engine = BatmanEngine::new(
        node(1),
        local_transport,
        InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
    );
    let mut router = MultiEngineRouter::new(
        node(1),
        FixedPolicyEngine::new(sample_profile()),
        InMemoryRuntimeEffects { now: Tick(1), ..Default::default() },
        topology.clone(),
        sample_policy_inputs(&topology),
    );
    router
        .register_engine(Box::new(engine))
        .expect("register BATMAN engine");

    let tick = router.advance_round().expect("pre-activation router round");
    assert_eq!(tick.engine_change, RoutingTickChange::PrivateStateUpdated);

    let route = Router::activate_route(&mut router, sample_objective())
        .expect("activate route");
    router
        .forward_payload(route.identity.route_id(), b"batman-payload")
        .expect("forward payload");

    let observations = next_hop_transport
        .drain_transport_ingress()
        .expect("drain next-hop ingress");
    assert_eq!(observations.len(), 1);
    match &observations[0] {
        | jacquard_core::TransportIngressEvent::PayloadReceived {
            from_node_id,
            payload,
            ..
        } => {
            assert_eq!(from_node_id, &node(1));
            assert_eq!(payload, b"batman-payload");
        },
        | other => panic!("unexpected observation: {other:?}"),
    }
}
