//! Router integration test for `jacquard-olsrv2`.

use std::collections::BTreeMap;

use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, HealthScore, IdentityAssuranceClass, LinkEndpoint, Observation,
    OriginAuthenticationClass, RatioPermille, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteReplacementPolicy, RoutingEngineFallbackPolicy, RoutingPolicyInputs,
    RoutingTickContext, SelectedRoutingParameters, Tick, TransportKind,
};
use jacquard_host_support::opaque_endpoint;
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_olsrv2::{OlsrV2Engine, OLSRV2_ENGINE_ID};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn endpoint(byte: u8) -> LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
}

fn topology() -> Observation<Configuration> {
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
                        &OLSRV2_ENGINE_ID,
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
                        &OLSRV2_ENGINE_ID,
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
            ]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn profile() -> SelectedRoutingParameters {
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

fn objective() -> jacquard_core::RoutingObjective {
    jacquard_core::RoutingObjective {
        destination: DestinationId::Node(node(2)),
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
fn olsrv2_registers_and_exposes_capabilities() {
    let topology = topology();
    let engine = OlsrV2Engine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let mut router = MultiEngineRouter::new(
        node(1),
        FixedPolicyEngine::new(profile()),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
        topology.clone(),
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
            loss_permille: RatioPermille(0),
            partition_risk_permille: RatioPermille(0),
            adversary_pressure_permille: RatioPermille(0),
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            direct_reachability_score: HealthScore(950),
        },
    );
    router
        .register_engine(Box::new(engine))
        .expect("register engine");
    let mut engine = OlsrV2Engine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    engine
        .engine_tick(&RoutingTickContext::new(topology))
        .expect("seed topology");

    assert_eq!(engine.engine_id(), OLSRV2_ENGINE_ID);
    let _objective = objective();
}
