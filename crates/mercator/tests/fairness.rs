use std::collections::BTreeMap;

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Limit, MaterializedRoute, NodeId, Observation, OperatingMode,
    OriginAuthenticationClass, PublicationId, RatioPermille, RouteEpoch, RouteHandle,
    RouteIdentityStamp, RouteLease, RouteMaintenanceOutcome, RouteMaintenanceTrigger,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, RoutingTickContext, SelectedRoutingParameters, Tick,
    TimeWindow, TransportKind,
};
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_mercator::{
    corridor::{plan_corridor, MercatorPlanningOutcome},
    evidence::{MercatorBrokerPressure, MercatorEvidenceMeta},
    selected_neighbor_from_backend_route_id, MercatorEngine, MercatorEngineConfig,
    MERCATOR_ENGINE_ID,
};
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
        valid_for: TimeWindow::new(Tick(1), Tick(16)).expect("lease window"),
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
    destination: NodeId,
) -> MaterializedRoute {
    let objective = objective(destination);
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

fn pressure(broker: u8, score: u16) -> MercatorBrokerPressure {
    MercatorBrokerPressure {
        broker: node(broker),
        participation_count: 2,
        pressure_score: score,
        meta: MercatorEvidenceMeta::new(
            RouteEpoch(1),
            Tick(1),
            DurationMs(1_000),
            jacquard_core::OrderStamp(u64::from(broker)),
        ),
    }
}

#[test]
fn broker_penalty_prefers_viable_detour_over_overloaded_corridor() {
    let topology = topology(1, 1, &[1, 2, 3, 4], &[(1, 2), (2, 4), (1, 3), (3, 4)]);
    let mut engine = MercatorEngine::new(node(1));
    engine
        .evidence_mut()
        .record_broker_pressure(pressure(2, 600));
    let candidate = engine
        .candidate_routes(&objective(node(4)), &profile(), &topology)
        .pop()
        .expect("candidate");

    assert_eq!(
        selected_neighbor_from_backend_route_id(&candidate.backend_ref.backend_route_id),
        Some(node(3))
    );
    assert_eq!(engine.diagnostics().overloaded_broker_penalty_count, 1);
}

#[test]
fn fairness_reserves_search_budget_for_weakest_unserved_objective() {
    let mut config = MercatorEngineConfig::default();
    config.evidence.corridor_alternate_count_max = 1;
    config.bounds.repair_attempt_count_max = 3;
    let topology = topology(1, 1, &[1, 2, 3, 4, 5], &[(1, 2), (2, 3), (3, 4), (4, 5)]);
    let objective = objective(node(5));
    let evidence = jacquard_mercator::MercatorEvidenceGraph::new(config.evidence);
    assert_eq!(
        plan_corridor(node(1), &topology, &objective, &config, &evidence,),
        MercatorPlanningOutcome::NoCandidate
    );

    let engine = MercatorEngine::with_config(node(1), config);
    let candidate = engine
        .candidate_routes(&objective, &profile(), &topology)
        .pop();

    assert!(candidate.is_some());
    assert_eq!(engine.diagnostics().weakest_flow_reserved_search_count, 1);
}

#[test]
fn fairness_tracks_per_objective_presence_and_zero_service_tails() {
    let topology = topology(1, 1, &[1, 2, 3, 4, 5], &[(1, 2), (2, 4), (1, 3), (3, 5)]);
    let mut engine = MercatorEngine::new(node(1));
    let _first = materialized_route(&mut engine, &topology, node(4));
    assert!(engine
        .candidate_routes(&objective(node(5)), &profile(), &topology)
        .pop()
        .is_some());
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("tick");

    assert_eq!(engine.diagnostics().objective_count, 2);
    assert_eq!(engine.diagnostics().active_objective_count, 1);
    assert_eq!(engine.diagnostics().zero_service_objective_count, 1);

    let _second = materialized_route(&mut engine, &topology, node(5));
    engine
        .engine_tick(&RoutingTickContext::new(topology))
        .expect("second tick");

    assert_eq!(engine.diagnostics().active_objective_count, 2);
    assert_eq!(engine.diagnostics().zero_service_objective_count, 0);
    assert_eq!(engine.diagnostics().weakest_objective_presence_rounds, 1);
}

#[test]
fn broker_switch_accounting_records_deterministic_repair_switch() {
    let initial = topology(1, 1, &[1, 2, 3, 4], &[(1, 2), (2, 4), (1, 3), (3, 4)]);
    let repaired = topology(2, 2, &[1, 2, 3, 4], &[(1, 3), (3, 4)]);
    let mut engine = MercatorEngine::new(node(1));
    let mut route = materialized_route(&mut engine, &initial, node(4));

    engine
        .engine_tick(&RoutingTickContext::new(repaired))
        .expect("tick");
    let result = engine
        .maintain_route(
            &route.identity,
            &mut route.runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    assert_eq!(engine.diagnostics().broker_switch_count, 1);
    assert_eq!(engine.diagnostics().broker_participation_count, 1);
    assert_eq!(
        engine.diagnostics().hottest_broker_concentration_permille,
        1_000
    );
}
