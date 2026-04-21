use std::collections::BTreeMap;

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Limit, MaterializedRoute, NodeId, Observation, OperatingMode,
    OriginAuthenticationClass, PublicationId, RatioPermille, RouteEpoch, RouteHandle,
    RouteIdentityStamp, RouteLease, RouteMaintenanceFailure, RouteMaintenanceOutcome,
    RouteMaintenanceTrigger, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteServiceKind, RoutingEvidenceClass, RoutingObjective, RoutingTickContext,
    SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
};
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_mercator::{evidence::MercatorSupportState, MercatorEngine, MERCATOR_ENGINE_ID};
use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(256))
}

fn mercator_node(byte: u8) -> jacquard_core::Node {
    NodePreset::route_capable(
        NodePresetOptions::new(
            NodeIdentity::new(node(byte), ControllerId([byte; 32])),
            endpoint(byte),
            Tick(1),
        ),
        &MERCATOR_ENGINE_ID,
    )
    .build()
}

fn topology(
    epoch: u64,
    tick: u64,
    node_bytes: &[u8],
    link_bytes: &[(u8, u8)],
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(epoch),
            nodes: node_bytes
                .iter()
                .copied()
                .map(|byte| (node(byte), mercator_node(byte)))
                .collect::<BTreeMap<_, _>>(),
            links: link_bytes
                .iter()
                .copied()
                .map(|(from, to)| {
                    (
                        (node(from), node(to)),
                        LinkPreset::active(LinkPresetOptions::new(endpoint(to), Tick(tick)))
                            .build(),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            environment: Environment {
                reachable_neighbor_count: u32::try_from(link_bytes.len()).unwrap_or(u32::MAX),
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(tick),
    }
}

fn objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::None,
        protection_floor: RouteProtectionClass::None,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
        latency_budget_ms: Limit::Bounded(DurationMs(100)),
        protection_priority: jacquard_core::PriorityPoints(1),
        connectivity_priority: jacquard_core::PriorityPoints(1),
    }
}

fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::None,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: OperatingMode::SparseLowPower,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

fn materialization_input(
    admission: jacquard_core::RouteAdmission,
    route_id: jacquard_core::RouteId,
    epoch: RouteEpoch,
) -> jacquard_core::RouteMaterializationInput {
    let lease = RouteLease {
        owner_node_id: node(1),
        lease_epoch: epoch,
        valid_for: TimeWindow::new(Tick(1), Tick(12)).expect("lease window"),
    };
    jacquard_core::RouteMaterializationInput {
        handle: RouteHandle {
            stamp: RouteIdentityStamp {
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

fn materialized_route(
    engine: &mut MercatorEngine,
    topology: &Observation<Configuration>,
) -> MaterializedRoute {
    let objective = objective(node(4));
    let profile = profile();
    let candidate = engine
        .candidate_routes(&objective, &profile, topology)
        .pop()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate.clone(), topology)
        .expect("admission");
    let input = materialization_input(admission, candidate.route_id, topology.value.epoch);
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialization");
    MaterializedRoute::from_installation(input, installation)
}

#[test]
fn repair_uses_surviving_corridor_alternate() {
    let initial = topology(1, 1, &[1, 2, 3, 4], &[(1, 2), (2, 4), (1, 3), (3, 4)]);
    let repaired = topology(2, 2, &[1, 2, 3, 4], &[(1, 3), (3, 4)]);
    let mut engine = MercatorEngine::new(node(1));
    let mut route = materialized_route(&mut engine, &initial);

    engine
        .engine_tick(&RoutingTickContext::new(repaired.clone()))
        .expect("tick");
    let result = engine
        .maintain_route(
            &route.identity,
            &mut route.runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    assert_eq!(
        route.runtime.last_lifecycle_event,
        jacquard_core::RouteLifecycleEvent::Repaired
    );
    assert_eq!(engine.diagnostics().repair_success_count, 1);
}

#[test]
fn stale_withdraws_when_no_surviving_support_remains() {
    let initial = topology(1, 1, &[1, 2, 4], &[(1, 2), (2, 4)]);
    let partitioned = topology(2, 3, &[1, 2, 4], &[]);
    let mut engine = MercatorEngine::new(node(1));
    let mut route = materialized_route(&mut engine, &initial);

    engine
        .engine_tick(&RoutingTickContext::new(partitioned.clone()))
        .expect("tick");
    let result = engine
        .maintain_route(
            &route.identity,
            &mut route.runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("maintenance");

    assert_eq!(result.event, jacquard_core::RouteLifecycleEvent::Expired);
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LostReachability)
    );
    assert_eq!(
        engine.evidence().route_support()[0].state,
        MercatorSupportState::Withdrawn
    );
    assert_eq!(engine.diagnostics().support_withdrawal_count, 1);
}

#[test]
fn stale_persistence_counts_only_active_unusable_routes_after_disruption() {
    let initial = topology(1, 1, &[1, 2, 4], &[(1, 2), (2, 4)]);
    let partitioned = topology(2, 4, &[1, 2, 4], &[]);
    let mut engine = MercatorEngine::new(node(1));
    let _route = materialized_route(&mut engine, &initial);

    engine
        .engine_tick(&RoutingTickContext::new(partitioned))
        .expect("tick");

    assert_eq!(engine.diagnostics().active_stale_route_count, 1);
    assert_eq!(engine.diagnostics().stale_persistence_rounds, 0);

    let still_partitioned = topology(2, 7, &[1, 2, 4], &[]);
    engine
        .engine_tick(&RoutingTickContext::new(still_partitioned))
        .expect("second tick");

    assert_eq!(engine.diagnostics().active_stale_route_count, 1);
    assert_eq!(engine.diagnostics().stale_persistence_rounds, 3);
}

#[test]
fn repair_records_recovery_rounds_from_first_stale_tick() {
    let initial = topology(1, 1, &[1, 2, 3, 4], &[(1, 2), (2, 4), (1, 3), (3, 4)]);
    let partitioned = topology(2, 2, &[1, 2, 3, 4], &[]);
    let repaired = topology(3, 6, &[1, 2, 3, 4], &[(1, 3), (3, 4)]);
    let mut engine = MercatorEngine::new(node(1));
    let mut route = materialized_route(&mut engine, &initial);

    engine
        .engine_tick(&RoutingTickContext::new(partitioned))
        .expect("partition tick");
    engine
        .engine_tick(&RoutingTickContext::new(repaired))
        .expect("repair tick");
    let result = engine
        .maintain_route(
            &route.identity,
            &mut route.runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    assert_eq!(engine.diagnostics().recovery_rounds, 4);
}
