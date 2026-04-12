//! Search and evidence integration tests for `jacquard-field`.

use std::collections::BTreeMap;

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Link, MaterializedRoute, Node, Observation,
    OriginAuthenticationClass, PublicationId, RatioPermille, RouteEpoch, RouteHandle, RouteLease,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, RoutingTickContext, SelectedRoutingParameters,
    ServiceId, Tick, TimeWindow, TransportKind,
};
use jacquard_field::{
    FieldEngine, FieldForwardSummaryObservation, FieldSearchPlanningFailure,
    FieldSearchTransitionClass, FIELD_ENGINE_ID,
};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_traits::{RoutingEngine, RoutingEnginePlanner};
use telltale_search::SearchQuery;

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(128))
}

fn fixture_node(byte: u8) -> Node {
    NodePreset::route_capable(
        NodePresetOptions::new(
            NodeIdentity::new(node(byte), ControllerId([byte; 32])),
            endpoint(byte),
            Tick(1),
        ),
        &FIELD_ENGINE_ID,
    )
    .build()
}

fn fixture_link(byte: u8) -> Link {
    LinkPreset::active(LinkPresetOptions::new(endpoint(byte), Tick(1))).build()
}

fn topology_config() -> Configuration {
    Configuration {
        epoch: RouteEpoch(7),
        nodes: BTreeMap::from([
            (node(1), fixture_node(1)),
            (node(2), fixture_node(2)),
            (node(3), fixture_node(3)),
        ]),
        links: BTreeMap::from([
            ((node(1), node(2)), fixture_link(2)),
            ((node(2), node(3)), fixture_link(3)),
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    }
}

fn topology(observed_at_tick: Tick) -> Observation<Configuration> {
    Observation {
        value: topology_config(),
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick,
    }
}

fn objective() -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(node(3)),
        service_kind: jacquard_core::RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(20),
    }
}

fn service_objective() -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(ServiceId(vec![9, 9])),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: jacquard_core::PriorityPoints(10),
        connectivity_priority: jacquard_core::PriorityPoints(20),
    }
}

fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

fn lease(start_tick: Tick, end_tick: Tick, epoch: RouteEpoch) -> RouteLease {
    RouteLease {
        owner_node_id: node(1),
        lease_epoch: epoch,
        valid_for: TimeWindow::new(start_tick, end_tick).expect("lease window"),
    }
}

fn materialized_route_for_objective(
    engine: &mut FieldEngine<InMemoryTransport, InMemoryRuntimeEffects>,
    objective: &RoutingObjective,
    topology: &Observation<Configuration>,
    lease: RouteLease,
) -> MaterializedRoute {
    let candidate = engine
        .candidate_routes(objective, &profile(), topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(objective, &profile(), candidate, topology)
        .expect("admission");
    let input = jacquard_core::RouteMaterializationInput {
        handle: RouteHandle {
            stamp: jacquard_core::RouteIdentityStamp {
                route_id,
                topology_epoch: lease.lease_epoch,
                materialized_at_tick: lease.valid_for.start_tick(),
                publication_id: PublicationId([4; 16]),
            },
        },
        admission,
        lease,
    };
    let installation = engine
        .materialize_route(input.clone())
        .expect("installation");
    MaterializedRoute::from_installation(input, installation)
}

#[test]
fn forwarded_summary_enables_search_backed_candidate_selection() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");
    assert!(engine
        .candidate_routes(&objective(), &profile(), &first)
        .is_empty());

    engine.record_forward_summary(
        &DestinationId::Node(node(3)),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 900, 1, 1),
    );
    engine.record_reverse_feedback(&DestinationId::Node(node(3)), node(2), 850, Tick(2));

    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("refresh tick");

    let candidates = engine.candidate_routes(&objective(), &profile(), &second);
    assert_eq!(candidates.len(), 1);

    let record = engine.last_search_record().expect("field search record");
    assert_eq!(
        record.query,
        Some(SearchQuery::single_goal(node(1), node(3))),
    );
    assert_eq!(record.planning_failure, None);
    assert_eq!(
        record
            .selected_continuation
            .as_ref()
            .expect("selected continuation")
            .chosen_neighbor,
        node(2),
    );
    let run = record.run.as_ref().expect("search run");
    assert_eq!(
        run.selected_node_path,
        Some(vec![node(1), node(2), node(3)])
    );
    assert_eq!(
        run.report.observation.selected_result_witness,
        Some(vec![node(1), node(2), node(3)]),
    );
}

#[test]
fn service_objective_uses_candidate_set_query_and_one_selected_candidate() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");

    engine.record_forward_summary(
        &DestinationId::Service(ServiceId(vec![9, 9])),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 900, 1, 1),
    );
    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("refresh tick");

    let candidates = engine.candidate_routes(&service_objective(), &profile(), &second);
    assert_eq!(candidates.len(), 1);

    let record = engine.last_search_record().expect("field search record");
    assert!(matches!(
        record.query,
        Some(SearchQuery::CandidateSet { .. })
    ));
    assert_eq!(record.planning_failure, None);
    let continuation = record
        .selected_continuation
        .as_ref()
        .expect("selected continuation");
    assert_eq!(continuation.chosen_neighbor, node(2));
}

#[test]
fn admitted_query_without_selected_result_fails_closed() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");
    let second = topology(Tick(2));

    let candidates = engine.candidate_routes(&objective(), &profile(), &second);
    assert!(candidates.is_empty());

    let record = engine.last_search_record().expect("field search record");
    assert_eq!(
        record.query,
        Some(SearchQuery::single_goal(node(1), node(3))),
    );
    assert_eq!(
        record.planning_failure,
        Some(FieldSearchPlanningFailure::NoSelectedResult),
    );
    assert!(record.selected_continuation.is_none());
}

#[test]
fn changing_field_evidence_reconfigures_search_snapshot() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");

    engine.record_forward_summary(
        &DestinationId::Node(node(3)),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 700, 1, 1),
    );
    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("second tick");
    // allow-ignored-result: this call refreshes the last search record used by the assertions below.
    let _ = engine.candidate_routes(&objective(), &profile(), &second);
    let first_record = engine.last_search_record().expect("first record");
    assert_eq!(
        first_record
            .run
            .as_ref()
            .expect("first run")
            .topology_transition,
        FieldSearchTransitionClass::InitialSnapshot,
    );

    engine.record_forward_summary(
        &DestinationId::Node(node(3)),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(3), 950, 1, 1),
    );
    let third = topology(Tick(3));
    engine
        .engine_tick(&RoutingTickContext::new(third.clone()))
        .expect("third tick");
    // allow-ignored-result: this call refreshes the last search record used by the assertions below.
    let _ = engine.candidate_routes(&objective(), &profile(), &third);
    let second_record = engine.last_search_record().expect("second record");
    assert_eq!(
        second_record
            .run
            .as_ref()
            .expect("second run")
            .topology_transition,
        FieldSearchTransitionClass::SameEpochNewSnapshot,
    );
}

#[test]
fn runtime_round_artifacts_capture_observational_route_projection() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");

    engine.record_forward_summary(
        &DestinationId::Node(node(3)),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 900, 1, 1),
    );
    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("refresh tick");

    let artifacts = engine.runtime_round_artifacts();
    assert!(!artifacts.is_empty());
    assert!(artifacts.iter().any(|artifact| {
        artifact.destination == Some(DestinationId::Node(node(3)))
            && artifact.router_artifact.as_ref().is_some_and(|route| {
                route.destination == DestinationId::Node(node(3))
                    && route.route_shape == jacquard_core::RouteShapeVisibility::CorridorEnvelope
                    && route.route_support > 0
            })
    }));
    assert!(engine
        .protocol_artifacts()
        .iter()
        .any(|artifact| artifact.session().destination() == Some(DestinationId::Node(node(3)))));
}

#[test]
fn route_commitments_track_pending_then_lease_expiry() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");
    engine.record_forward_summary(
        &DestinationId::Node(node(3)),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 900, 1, 1),
    );
    engine.record_reverse_feedback(&DestinationId::Node(node(3)), node(2), 850, Tick(2));
    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("refresh tick");

    let route = materialized_route_for_objective(
        &mut engine,
        &objective(),
        &second,
        lease(Tick(2), Tick(4), second.value.epoch),
    );
    let pending = engine.route_commitments(&route);
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].resolution,
        jacquard_core::RouteCommitmentResolution::Pending,
    );

    let expired = topology(Tick(5));
    engine
        .engine_tick(&RoutingTickContext::new(expired))
        .expect("expiry tick");
    let expired_commitments = engine.route_commitments(&route);
    assert_eq!(
        expired_commitments[0].resolution,
        jacquard_core::RouteCommitmentResolution::Invalidated(
            jacquard_core::RouteInvalidationReason::LeaseExpired,
        ),
    );
}

#[test]
fn exact_node_lineage_runs_from_evidence_to_materialized_route() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");
    engine.record_forward_summary(
        &DestinationId::Node(node(3)),
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 900, 1, 1),
    );
    engine.record_reverse_feedback(&DestinationId::Node(node(3)), node(2), 850, Tick(2));
    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("refresh tick");

    let objective = objective();
    let route = materialized_route_for_objective(
        &mut engine,
        &objective,
        &second,
        lease(Tick(2), Tick(6), second.value.epoch),
    );
    let record = engine.last_search_record().expect("field search record");
    assert_eq!(
        record
            .selected_continuation
            .as_ref()
            .expect("selected continuation")
            .chosen_neighbor,
        node(2),
    );
    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        jacquard_core::RouteCommitmentResolution::Pending,
    );
}

#[test]
fn candidate_set_lineage_runs_from_evidence_to_materialized_route() {
    let mut engine = FieldEngine::new(
        node(1),
        InMemoryTransport::new(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
    );
    let first = topology(Tick(1));
    engine
        .engine_tick(&RoutingTickContext::new(first.clone()))
        .expect("initial tick");
    let objective = service_objective();
    engine.record_forward_summary(
        &objective.destination,
        node(2),
        FieldForwardSummaryObservation::new(first.value.epoch, Tick(2), 1000, 1, 1),
    );
    engine.record_reverse_feedback(&objective.destination, node(2), 950, Tick(2));
    let second = topology(Tick(2));
    engine
        .engine_tick(&RoutingTickContext::new(second.clone()))
        .expect("refresh tick");

    let route = materialized_route_for_objective(
        &mut engine,
        &objective,
        &second,
        lease(Tick(2), Tick(6), second.value.epoch),
    );
    let record = engine.last_search_record().expect("field search record");
    assert!(matches!(
        record.query,
        Some(SearchQuery::CandidateSet { .. })
    ));
    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        jacquard_core::RouteCommitmentResolution::Pending,
    );
}

#[test]
fn field_search_source_avoids_removed_alias_surfaces() {
    let sources = [
        include_str!("../src/search/mod.rs"),
        include_str!("../src/search/domain.rs"),
        include_str!("../src/search/runner.rs"),
    ];
    for source in sources {
        assert!(!source.contains("telltale_search::compat"));
        assert!(!source.contains("shim"));
    }
}
