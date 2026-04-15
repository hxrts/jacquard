//! Shared deterministic helpers for cross-crate router-integration tests.

#![allow(dead_code, unused_imports, unused_macros)]

use jacquard_adapter::{dispatch_mailbox, opaque_endpoint, DispatchReceiver, DispatchSender};
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    HealthScore, IdentityAssuranceClass, Link, LinkEndpoint, Node, NodeId, Observation,
    RatioPermille, RouteAdmission, RouteError, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteReplacementPolicy, RoutingEngineFallbackPolicy, RoutingObjective,
    RoutingPolicyInputs, SelectedRoutingParameters, Tick, TransportError,
};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions, SharedInMemoryNetwork,
};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{
    effect_handler, RouterManagedEngine, RoutingControlPlane, RoutingEnginePlanner,
    TransportDriver, TransportSenderEffects,
};

pub type RouterIntegrationRouter = MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects>;

#[must_use]
pub fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

#[must_use]
pub fn endpoint(byte: u8) -> LinkEndpoint {
    opaque_endpoint(
        jacquard_core::TransportKind::WifiAware,
        vec![byte],
        ByteCount(128),
    )
}

#[must_use]
pub fn route_capable_node(byte: u8, engine_id: &jacquard_core::RoutingEngineId, now: Tick) -> Node {
    NodePreset::route_capable(
        NodePresetOptions::new(
            NodeIdentity::new(node(byte), ControllerId([byte; 32])),
            endpoint(byte),
            now,
        ),
        engine_id,
    )
    .build()
}

#[must_use]
pub fn active_link(byte: u8, now: Tick) -> Link {
    LinkPreset::active(LinkPresetOptions::new(endpoint(byte), now)).build()
}

#[must_use]
pub fn lossy_link(byte: u8, now: Tick, confidence: RatioPermille) -> Link {
    LinkPreset::lossy(LinkPresetOptions::new(endpoint(byte), now).with_confidence(confidence))
        .build()
}

#[must_use]
pub fn connected_profile() -> SelectedRoutingParameters {
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

#[must_use]
pub fn connected_objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
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

#[must_use]
pub fn routing_policy_inputs(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    routing_engine_count: usize,
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
        routing_engine_count: u32::try_from(routing_engine_count)
            .expect("routing engine count fits in u32"),
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(100),
        adversary_pressure_permille: RatioPermille(0),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

#[must_use]
pub fn build_router(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    profile: SelectedRoutingParameters,
    now: Tick,
    routing_engine_count: usize,
) -> RouterIntegrationRouter {
    MultiEngineRouter::new(
        local_node_id,
        FixedPolicyEngine::new(profile),
        InMemoryRuntimeEffects {
            now,
            ..Default::default()
        },
        topology.clone(),
        routing_policy_inputs(&topology, local_node_id, routing_engine_count),
    )
}

pub fn activate_route_within_rounds(
    router: &mut RouterIntegrationRouter,
    objective: &RoutingObjective,
    max_rounds: usize,
) -> Result<jacquard_core::MaterializedRoute, RouteError> {
    for round in 0..=max_rounds {
        if let Ok(route) = RoutingControlPlane::activate_route(router, objective.clone()) {
            return Ok(route);
        }
        if round < max_rounds {
            router.advance_round()?;
        }
    }
    Err(jacquard_core::RouteSelectionError::NoCandidate.into())
}

pub fn admitted_single_candidate<E>(
    engine: &E,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
) -> Result<RouteAdmission, RouteError>
where
    E: RoutingEnginePlanner,
{
    let mut candidates = engine.candidate_routes(objective, profile, topology);
    assert_eq!(
        candidates.len(),
        1,
        "expected one deterministic candidate but found {}",
        candidates.len()
    );
    engine.admit_route(objective, profile, candidates.remove(0), topology)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboundFrame {
    pub endpoint: LinkEndpoint,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct QueuedTransportSender {
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

pub struct RouterIntegrationHost {
    topology: Observation<Configuration>,
    router: RouterIntegrationRouter,
    driver: InMemoryTransport,
    outbound: DispatchReceiver<OutboundFrame>,
    next_tick: Tick,
}

impl RouterIntegrationHost {
    pub fn new<F>(
        local_node_id: NodeId,
        topology: Observation<Configuration>,
        network: SharedInMemoryNetwork,
        profile: SelectedRoutingParameters,
        routing_engine_count: usize,
        build_engine: F,
    ) -> Self
    where
        F: FnOnce(QueuedTransportSender, Tick) -> Box<dyn RouterManagedEngine>,
    {
        let next_tick = Tick(topology.observed_at_tick.0.saturating_add(1));
        let driver = InMemoryTransport::attach(
            local_node_id,
            topology.value.nodes[&local_node_id]
                .profile
                .endpoints
                .clone(),
            network,
        );
        let (outbound_tx, outbound_rx) = dispatch_mailbox(64);
        let mut router = build_router(
            local_node_id,
            topology.clone(),
            profile,
            topology.observed_at_tick,
            routing_engine_count,
        );
        router
            .register_engine(build_engine(
                QueuedTransportSender {
                    outbound: outbound_tx,
                },
                topology.observed_at_tick,
            ))
            .expect("register router integration engine");
        Self {
            topology,
            router,
            driver,
            outbound: outbound_rx,
            next_tick,
        }
    }

    pub fn advance_round(&mut self) {
        let tick = self.next_tick;
        self.next_tick = Tick(self.next_tick.0.saturating_add(1));
        self.router.effects_mut().now = tick;
        self.topology.observed_at_tick = tick;
        self.router
            .ingest_topology_observation(self.topology.clone());
        self.router.ingest_policy_inputs(routing_policy_inputs(
            &self.topology,
            self.router.local_node_id(),
            self.router.registered_engine_ids().len(),
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

    pub fn router_mut(&mut self) -> &mut RouterIntegrationRouter {
        &mut self.router
    }
}

#[macro_export]
macro_rules! homogeneous_router_integration_hosts {
    ($network:expr, $topology_fn:path, $profile:expr, $routing_engine_count:expr, [$($byte:expr),+ $(,)?], $builder:expr) => {{
        let network = $network;
        let profile = $profile;
        std::collections::BTreeMap::from([
            $(
                (
                    $crate::router_integration::node($byte),
                    $crate::router_integration::RouterIntegrationHost::new(
                        $crate::router_integration::node($byte),
                        $topology_fn(),
                        network.clone(),
                        profile.clone(),
                        $routing_engine_count,
                        |sender, now| ($builder)(
                            $crate::router_integration::node($byte),
                            sender,
                            now,
                        ),
                    ),
                )
            ),+
        ])
    }};
}
