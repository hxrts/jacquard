//! Stable exported replay fixture tests for `jacquard-field`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Link, MaterializedRoute, Node, Observation,
    OriginAuthenticationClass, PublicationId, RatioPermille, RouteEpoch, RouteHandle, RouteLease,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, RoutingTickContext, SelectedRoutingParameters,
    ServiceId, Tick, TimeWindow, TransportKind,
};
use jacquard_field::{FieldEngine, FieldExportedReplayBundle, FieldForwardSummaryObservation};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, LinkPreset, LinkPresetOptions,
};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner};

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
        &jacquard_field::FIELD_ENGINE_ID,
    )
    .build()
}

fn fixture_link(byte: u8) -> Link {
    LinkPreset::active(LinkPresetOptions::new(endpoint(byte), Tick(1))).build()
}

fn topology(observed_at_tick: Tick) -> Observation<Configuration> {
    Observation {
        value: Configuration {
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
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick,
    }
}

fn objective() -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(node(3)),
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

fn exact_node_bundle() -> FieldExportedReplayBundle {
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
        lease(Tick(2), Tick(6), second.value.epoch),
    );
    engine.exported_replay_bundle(&[route])
}

fn candidate_set_bundle() -> FieldExportedReplayBundle {
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
    engine.exported_replay_bundle(&[route])
}

fn recovery_bundle() -> FieldExportedReplayBundle {
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
        lease(Tick(2), Tick(6), second.value.epoch),
    );
    let route_id = route.identity.stamp.route_id;
    engine
        .suspend_route_runtime_for_recovery(&route_id)
        .expect("suspend");
    assert!(engine
        .restore_route_runtime_for_router(&route_id)
        .expect("restore"));
    engine.exported_replay_bundle(&[route])
}

fn assert_fixture(name: &str, bundle: &FieldExportedReplayBundle) {
    let actual = serde_json::to_string_pretty(bundle).expect("serialize replay bundle");
    let relative_path = match name {
        "exact-node-activation" => "../fixtures/replay/exact-node-activation.json",
        "candidate-set-activation" => "../fixtures/replay/candidate-set-activation.json",
        "checkpoint-restore" => "../fixtures/replay/checkpoint-restore.json",
        _ => panic!("unknown fixture"),
    };
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(relative_path);
    if std::env::var_os("JACQUARD_UPDATE_FIELD_REPLAY_FIXTURES").is_some() {
        std::fs::write(&fixture_path, format!("{actual}\n")).expect("write replay fixture");
    }
    let expected = std::fs::read_to_string(&fixture_path).expect("read replay fixture");
    assert_eq!(actual, expected.trim_end());
}

#[test]
fn exported_exact_node_bundle_matches_golden_fixture() {
    assert_fixture("exact-node-activation", &exact_node_bundle());
}

#[test]
fn exported_candidate_set_bundle_matches_golden_fixture() {
    assert_fixture("candidate-set-activation", &candidate_set_bundle());
}

#[test]
fn exported_checkpoint_restore_bundle_matches_golden_fixture() {
    assert_fixture("checkpoint-restore", &recovery_bundle());
}

#[test]
fn rust_lean_replay_fixture_generation_tracks_exported_replay_families() {
    let exact = exact_node_bundle().lean_replay_fixture("exact-node-activation");
    assert_eq!(exact.scenario, "exact-node-activation");
    assert_eq!(
        exact
            .search
            .as_ref()
            .map(|search| search.query_kind.as_str()),
        Some("SingleGoal")
    );
    assert_eq!(exact.protocol.route_bound_reconfiguration_count, 0);
    assert_eq!(exact.runtime.bootstrap_route_artifact_count, 0);

    let candidate = candidate_set_bundle().lean_replay_fixture("candidate-set-activation");
    assert_eq!(
        candidate
            .search
            .as_ref()
            .map(|search| search.query_kind.as_str()),
        Some("CandidateSet")
    );

    let recovery = recovery_bundle().lean_replay_fixture("checkpoint-restore");
    assert_eq!(
        recovery
            .recovery
            .as_ref()
            .and_then(|state| state.last_outcome.as_deref()),
        Some("CheckpointRestored")
    );
    assert_eq!(
        recovery
            .recovery
            .as_ref()
            .map(|state| state.bootstrap_active),
        Some(false)
    );
    assert_eq!(
        recovery.protocol.reconfiguration_causes,
        vec!["CheckpointRestore"]
    );
}

#[test]
fn rust_lean_replay_fixture_vocabulary_stays_aligned_with_lean_surface() {
    let lean = include_str!("../../../verification/Field/Adequacy/ReplayFixtures.lean");

    for required in [
        "structure RustReplayFixture",
        "structure RustReplaySearchFixture",
        "structure RustReplayProtocolFixture",
        "structure RustReplayRuntimeLinkageFixture",
        "structure RustReplayRecoveryFixture",
        "bootstrapRouteArtifactCount",
        "bootstrapActive",
        "lastPromotionDecision",
        "lastPromotionBlocker",
        "bootstrapHoldCount",
        "bootstrapNarrowCount",
        "exact-node-activation",
        "candidate-set-activation",
        "continuation-shift",
        "checkpoint-restore",
        "\"SingleGoal\"",
        "\"CandidateSet\"",
        "\"ContinuationShift\"",
        "\"CheckpointRestore\"",
    ] {
        assert!(
            lean.contains(required),
            "missing Lean replay fixture term: {required}"
        );
    }
}
