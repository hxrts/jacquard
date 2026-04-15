//! Router and contract-level integration tests for `jacquard-scatter`.

use std::collections::BTreeMap;

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, LinkEndpoint, MaterializedRoute, Observation,
    OriginAuthenticationClass, PublicationId, RatioPermille, RouteEpoch, RouteHandle, RouteLease,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
    RoutingEngineFallbackPolicy, RoutingObjective, RoutingTickContext, SelectedRoutingParameters,
    Tick, TimeWindow, TransportKind,
};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_scatter::{ScatterEngine, SCATTER_CAPABILITIES, SCATTER_ENGINE_ID};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner};

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn endpoint(byte: u8) -> LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
}

fn topology(with_direct_link: bool) -> Observation<Configuration> {
    let mut links = BTreeMap::new();
    if with_direct_link {
        links.insert(
            (node(1), node(2)),
            LinkPreset::active(LinkPresetOptions::new(endpoint(2), Tick(1))).build(),
        );
        links.insert(
            (node(2), node(1)),
            LinkPreset::active(LinkPresetOptions::new(endpoint(1), Tick(1))).build(),
        );
    }
    Observation {
        value: Configuration {
            epoch: RouteEpoch(2),
            nodes: BTreeMap::from([
                (
                    node(1),
                    NodePreset::route_capable(
                        NodePresetOptions::new(
                            NodeIdentity::new(node(1), ControllerId([1; 32])),
                            endpoint(1),
                            Tick(1),
                        ),
                        &SCATTER_ENGINE_ID,
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
                        &SCATTER_ENGINE_ID,
                    )
                    .build(),
                ),
            ]),
            links,
            environment: Environment {
                reachable_neighbor_count: if with_direct_link { 1 } else { 0 },
                churn_permille: RatioPermille(50),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn profile(partition: RoutePartitionClass) -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition,
        },
        deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn objective(partition: RoutePartitionClass) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(node(2)),
        service_kind: jacquard_core::RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(100)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(10),
    }
}

fn materialization_input(
    admission: jacquard_core::RouteAdmission,
    route_id: jacquard_core::RouteId,
) -> jacquard_core::RouteMaterializationInput {
    let lease = RouteLease {
        owner_node_id: node(1),
        lease_epoch: RouteEpoch(2),
        valid_for: TimeWindow::new(Tick(1), Tick(9)).expect("lease window"),
    };
    jacquard_core::RouteMaterializationInput {
        handle: RouteHandle {
            stamp: jacquard_core::RouteIdentityStamp {
                route_id,
                topology_epoch: lease.lease_epoch,
                materialized_at_tick: lease.valid_for.start_tick(),
                publication_id: PublicationId([9; 16]),
            },
        },
        admission,
        lease,
    }
}

#[test]
fn scatter_advertises_partition_tolerant_hold_capabilities() {
    assert_eq!(SCATTER_CAPABILITIES.engine, SCATTER_ENGINE_ID);
    assert_eq!(
        SCATTER_CAPABILITIES.max_connectivity.partition,
        RoutePartitionClass::PartitionTolerant
    );
    assert_eq!(
        SCATTER_CAPABILITIES.hold_support,
        jacquard_core::HoldSupport::Supported
    );
}

#[test]
fn scatter_produces_candidate_without_direct_link_when_hold_allowed() {
    let topology = topology(false);
    let engine = ScatterEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let candidates = engine.candidate_routes(
        &objective(RoutePartitionClass::PartitionTolerant),
        &profile(RoutePartitionClass::PartitionTolerant),
        &topology,
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].summary.connectivity.partition,
        RoutePartitionClass::PartitionTolerant
    );
}

#[test]
fn scatter_materializes_and_accepts_forwarded_payloads() {
    let topology = topology(true);
    let mut engine = ScatterEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("seed topology");
    let objective = objective(RoutePartitionClass::PartitionTolerant);
    let profile = profile(RoutePartitionClass::PartitionTolerant);
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .pop()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate.clone(), &topology)
        .expect("admission");
    engine
        .materialize_route(materialization_input(admission, candidate.route_id))
        .expect("materialize");
    engine
        .forward_payload_for_router(&candidate.route_id, b"scatter-payload")
        .expect("forward");

    assert_eq!(engine.retained_message_count(), 1);
}

#[test]
fn scatter_maintenance_reports_hold_fallback_when_direct_link_is_absent() {
    let topology = topology(false);
    let mut engine = ScatterEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("seed topology");
    let objective = objective(RoutePartitionClass::PartitionTolerant);
    let profile = profile(RoutePartitionClass::PartitionTolerant);
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .pop()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate.clone(), &topology)
        .expect("admission");
    let input = materialization_input(admission.clone(), candidate.route_id);
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialize");
    engine
        .forward_payload_for_router(&candidate.route_id, b"scatter-payload")
        .expect("forward");
    let mut materialized = MaterializedRoute::from_installation(input, installation);
    let maintenance = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            jacquard_core::RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("maintenance");

    match maintenance.outcome {
        jacquard_core::RouteMaintenanceOutcome::HoldFallback { .. } => {}
        other => panic!("expected hold fallback, got {other:?}"),
    }
}
