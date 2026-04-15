//! Router integration tests for `jacquard-batman`.
//!
//! Exercises `BatmanBellmanEngine` wired into a `MultiEngineRouter` with
//! `InMemoryTransport` and `InMemoryRuntimeEffects`, verifying end-to-end
//! behavior from route activation through payload forwarding.
//!
//! Tests in this module confirm:
//! - `BatmanBellmanEngine` can be registered with a `MultiEngineRouter` and its
//!   `BATMAN_BELLMAN_ENGINE_ID` capabilities are accessible after registration.
//! - `Router::activate_route` selects a next-hop-only route with
//!   `RouteShapeVisibility::NextHopOnly` via the BATMAN engine.
//! - After an `advance_round` tick, `forward_payload` delivers a payload to the
//!   correct next-hop `InMemoryTransport` node, verifiable by draining the
//!   ingress queue of the expected neighbor.
//!
//! The sample topology uses four nodes with two paths to destination node 4:
//! a high-quality path via node 2 and a lossier path via node 3.

use std::collections::BTreeMap;

use jacquard_adapter::{dispatch_mailbox, opaque_endpoint, DispatchReceiver, DispatchSender};
use jacquard_batman_bellman::{BatmanBellmanEngine, BATMAN_BELLMAN_ENGINE_ID};
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, HealthScore, IdentityAssuranceClass, LinkEndpoint, Observation, RatioPermille,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
    RoutingEngineFallbackPolicy, RoutingPolicyInputs, RoutingTickChange, SelectedRoutingParameters,
    Tick, TransportError, TransportIngressEvent, TransportKind,
};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions, SharedInMemoryNetwork,
};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{
    effect_handler, Router, RoutingControlPlane, RoutingDataPlane, TransportDriver,
    TransportSenderEffects,
};

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn endpoint(byte: u8) -> LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
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
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(1), ControllerId([1; 32])),
                            endpoint(1),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(2),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(2), ControllerId([2; 32])),
                            endpoint(2),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(3),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(3), ControllerId([3; 32])),
                            endpoint(3),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(4),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(4), ControllerId([4; 32])),
                            endpoint(4),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
            ]),
            links: BTreeMap::from([
                (
                    (node(1), node(2)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
                ),
                (
                    (node(2), node(1)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(1), Tick(1))).build(),
                ),
                (
                    (node(2), node(4)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(4), Tick(1))).build(),
                ),
                (
                    (node(4), node(2)),
                    LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
                ),
                (
                    (node(1), node(3)),
                    LinkPreset::lossy(
                        LinkPresetOptions::new(endpoint(3), Tick(1))
                            .with_confidence(RatioPermille(650)),
                    )
                    .build(),
                ),
                (
                    (node(3), node(1)),
                    LinkPreset::lossy(
                        LinkPresetOptions::new(endpoint(1), Tick(1))
                            .with_confidence(RatioPermille(650)),
                    )
                    .build(),
                ),
                (
                    (node(3), node(4)),
                    LinkPreset::lossy(
                        LinkPresetOptions::new(endpoint(4), Tick(1))
                            .with_confidence(RatioPermille(600)),
                    )
                    .build(),
                ),
                (
                    (node(4), node(3)),
                    LinkPreset::lossy(
                        LinkPresetOptions::new(endpoint(3), Tick(1))
                            .with_confidence(RatioPermille(600)),
                    )
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct OutboundFrame {
    endpoint: LinkEndpoint,
    payload: Vec<u8>,
}

#[derive(Clone)]
struct QueuedTransportSender {
    outbound: DispatchSender<OutboundFrame>,
}

#[effect_handler]
impl TransportSenderEffects for QueuedTransportSender {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.outbound
            .send(OutboundFrame {
                endpoint: endpoint.clone(),
                payload: payload.to_vec(),
            })
            .map(|_| ())
            .map_err(|_| TransportError::Unavailable)
    }
}

struct BatmanHost {
    topology: Observation<Configuration>,
    router: MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>,
    driver: InMemoryTransport,
    outbound: DispatchReceiver<OutboundFrame>,
    next_tick: Tick,
}

impl BatmanHost {
    fn new(
        local_node_id: jacquard_core::NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
    ) -> Self {
        let driver = InMemoryTransport::attach(
            local_node_id,
            topology.value.nodes[&local_node_id]
                .profile
                .endpoints
                .clone(),
            network,
        );
        let (outbound_tx, outbound_rx) = dispatch_mailbox(64);
        let sender = QueuedTransportSender {
            outbound: outbound_tx,
        };
        let engine = BatmanBellmanEngine::new(
            local_node_id,
            sender,
            InMemoryRuntimeEffects {
                now: topology.observed_at_tick,
                ..Default::default()
            },
        );
        let mut router = MultiEngineRouter::new(
            local_node_id,
            FixedPolicyEngine::new(sample_profile()),
            InMemoryRuntimeEffects {
                now: topology.observed_at_tick,
                ..Default::default()
            },
            topology.clone(),
            sample_policy_inputs_for(&topology, local_node_id),
        );
        router
            .register_engine(Box::new(engine))
            .expect("register BATMAN engine");
        Self {
            topology,
            router,
            driver,
            outbound: outbound_rx,
            next_tick: Tick(2),
        }
    }

    fn advance_round(&mut self) {
        let tick = self.next_tick;
        self.next_tick = Tick(self.next_tick.0.saturating_add(1));
        self.router.effects_mut().now = tick;
        self.topology.observed_at_tick = tick;
        self.router
            .ingest_topology_observation(self.topology.clone());
        self.router.ingest_policy_inputs(sample_policy_inputs_for(
            &self.topology,
            self.router.local_node_id(),
        ));

        let ingress = self
            .driver
            .drain_transport_ingress()
            .expect("drain transport ingress");
        for event in ingress {
            self.router
                .ingest_transport_observation(&event.observe_at(tick))
                .expect("ingest transport observation");
        }
        self.router.advance_round().expect("advance router round");
        for frame in self.outbound.drain() {
            self.driver
                .send_transport(&frame.endpoint, &frame.payload)
                .expect("flush outbound frame");
        }
    }
}

fn sample_policy_inputs_for(
    topology: &Observation<Configuration>,
    local_node_id: jacquard_core::NodeId,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&local_node_id].clone(),
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

// long-block-exception: this fixture intentionally keeps the five-node roster
// and per-node direct-neighbor edges together so the gossip-learning setup is
// readable as one world description.
fn five_node_roster_topology(local_node_id: jacquard_core::NodeId) -> Observation<Configuration> {
    let neighbors = match local_node_id {
        id if id == node(1) => vec![node(2)],
        id if id == node(2) => vec![node(1), node(3)],
        id if id == node(3) => vec![node(2), node(4)],
        id if id == node(4) => vec![node(3), node(5)],
        id if id == node(5) => vec![node(4)],
        _ => Vec::new(),
    };
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(3),
            nodes: BTreeMap::from([
                (
                    node(1),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(1), ControllerId([1; 32])),
                            endpoint(1),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(2),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(2), ControllerId([2; 32])),
                            endpoint(2),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(3),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(3), ControllerId([3; 32])),
                            endpoint(3),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(4),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(4), ControllerId([4; 32])),
                            endpoint(4),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
                (
                    node(5),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(5), ControllerId([5; 32])),
                            endpoint(5),
                            Tick(1),
                        ),
                        &BATMAN_BELLMAN_ENGINE_ID,
                    )
                    .build(),
                ),
            ]),
            links: neighbors
                .into_iter()
                .map(|neighbor| {
                    (
                        (local_node_id, neighbor),
                        LinkPreset::active(LinkPresetOptions::new(
                            endpoint(neighbor.0[0]),
                            Tick(1),
                        ))
                        .build(),
                    )
                })
                .collect(),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: jacquard_core::FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
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
    let engine = BatmanBellmanEngine::new(
        node(1),
        local_transport,
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let mut router = MultiEngineRouter::new(
        node(1),
        FixedPolicyEngine::new(sample_profile()),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
        topology.clone(),
        sample_policy_inputs(&topology),
    );
    router
        .register_engine(Box::new(engine))
        .expect("register BATMAN engine");

    let route = Router::activate_route(&mut router, sample_objective()).expect("activate route");

    assert_eq!(
        route.identity.admission.summary.engine,
        BATMAN_BELLMAN_ENGINE_ID
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&BATMAN_BELLMAN_ENGINE_ID)
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

    let engine = BatmanBellmanEngine::new(
        node(1),
        local_transport,
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let mut router = MultiEngineRouter::new(
        node(1),
        FixedPolicyEngine::new(sample_profile()),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
        topology.clone(),
        sample_policy_inputs(&topology),
    );
    router
        .register_engine(Box::new(engine))
        .expect("register BATMAN engine");

    let tick = router.advance_round().expect("pre-activation router round");
    assert_eq!(tick.engine_change, RoutingTickChange::PrivateStateUpdated);

    let route = Router::activate_route(&mut router, sample_objective()).expect("activate route");
    router
        .forward_payload(route.identity.route_id(), b"batman-payload")
        .expect("forward payload");

    let observations = next_hop_transport
        .drain_transport_ingress()
        .expect("drain next-hop ingress");
    assert!(observations.iter().any(|observation| {
        matches!(
            observation,
            jacquard_core::TransportIngressEvent::PayloadReceived {
                from_node_id,
                payload,
                ..
            } if *from_node_id == node(1) && payload == b"batman-payload"
        )
    }));
}

// long-block-exception: this integration test traces one complete five-node
// BATMAN convergence narrative from host setup through route activation and
// next-hop forwarding; splitting it would obscure the sequence under test.
#[test]
fn batman_router_activates_a_five_node_route_from_direct_neighbor_observations() {
    let network = SharedInMemoryNetwork::default();
    let mut hosts = BTreeMap::from([
        (
            node(1),
            BatmanHost::new(node(1), five_node_roster_topology(node(1)), network.clone()),
        ),
        (
            node(2),
            BatmanHost::new(node(2), five_node_roster_topology(node(2)), network.clone()),
        ),
        (
            node(3),
            BatmanHost::new(node(3), five_node_roster_topology(node(3)), network.clone()),
        ),
        (
            node(4),
            BatmanHost::new(node(4), five_node_roster_topology(node(4)), network.clone()),
        ),
        (
            node(5),
            BatmanHost::new(node(5), five_node_roster_topology(node(5)), network),
        ),
    ]);

    for _ in 0..12 {
        for host in hosts.values_mut() {
            host.advance_round();
        }
    }

    let route = Router::activate_route(
        &mut hosts.get_mut(&node(1)).expect("host A").router,
        jacquard_core::RoutingObjective {
            destination: DestinationId::Node(node(5)),
            ..sample_objective()
        },
    )
    .expect("activate route across learned five-node line");

    assert_eq!(
        route.identity.admission.summary.engine,
        BATMAN_BELLMAN_ENGINE_ID
    );
    assert_eq!(
        route.identity.admission.summary.hop_count_hint.value_or(0),
        4
    );

    hosts
        .get_mut(&node(1))
        .expect("host A")
        .router
        .forward_payload(route.identity.route_id(), b"batman-gossip")
        .expect("forward payload over learned route");

    hosts.get_mut(&node(1)).expect("host A").advance_round();

    let delivered = hosts
        .get_mut(&node(2))
        .expect("host B")
        .driver
        .drain_transport_ingress()
        .expect("drain next-hop ingress");
    assert!(
        delivered.iter().any(|event| {
            matches!(
                event,
                TransportIngressEvent::PayloadReceived { from_node_id, payload, .. }
                    if *from_node_id == node(1) && payload == b"batman-gossip"
            )
        }),
        "next-hop ingress: {delivered:?}"
    );
}
