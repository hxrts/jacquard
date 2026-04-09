//! Contract tests for the static mesh capability envelope.

mod common;

use common::{
    engine::{
        activate_route, build_engine, lease, objective, profile,
        profile_with_connectivity,
    },
    fixtures::sample_configuration,
};
use jacquard_pathway::PATHWAY_CAPABILITIES;
use jacquard_traits::{
    jacquard_core::{
        AdmissionDecision, DecidableSupport, DestinationId, HoldSupport, NodeId,
        RepairSupport, RouteLifecycleEvent, RouteMaintenanceOutcome,
        RouteMaintenanceTrigger, RoutePartitionClass, RouteRepairClass,
        RouteShapeVisibility, Tick,
    },
    RouterManagedEngine, RoutingEngine, RoutingEnginePlanner,
};

#[test]
fn mesh_capability_surface_matches_the_advertised_constant() {
    let engine = build_engine();
    assert_eq!(engine.capabilities(), PATHWAY_CAPABILITIES);
}

#[test]
fn advertised_hold_and_partition_tolerance_are_exercised_by_deferred_delivery_admission(
) {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("partition-tolerant candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");

    assert_eq!(PATHWAY_CAPABILITIES.hold_support, HoldSupport::Supported);
    assert_eq!(
        PATHWAY_CAPABILITIES.max_connectivity.partition,
        RoutePartitionClass::PartitionTolerant
    );
    assert_eq!(
        admission.summary.connectivity.partition,
        RoutePartitionClass::PartitionTolerant
    );
}

#[test]
fn advertised_hold_support_is_exercised_by_partition_buffering() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );

    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("partition maintenance");
    engine
        .forward_payload_for_router(&identity.stamp.route_id, b"buffer-me")
        .expect("partitioned forwarding retains payload");

    assert_eq!(PATHWAY_CAPABILITIES.hold_support, HoldSupport::Supported);
    assert_eq!(
        engine
            .active_route(&identity.stamp.route_id)
            .expect("active route")
            .retention
            .retained_object_count,
        1
    );
}

#[test]
fn advertised_repair_support_is_exercised_by_link_degraded_maintenance() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );

    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance succeeds");

    assert_eq!(
        PATHWAY_CAPABILITIES.repair_support,
        RepairSupport::Supported
    );
    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    assert_eq!(runtime.last_lifecycle_event, RouteLifecycleEvent::Repaired);
}

#[test]
fn advertised_decidable_admission_is_exercised_by_typed_rejection() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::PartitionTolerant,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("typed admission check");

    assert_eq!(
        PATHWAY_CAPABILITIES.decidable_admission,
        DecidableSupport::Supported
    );
    assert!(matches!(check.decision, AdmissionDecision::Rejected(_)));
}

#[test]
fn advertised_explicit_route_shape_is_visible_in_active_route_state() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, _runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );
    let active_route = engine
        .active_route(&identity.stamp.route_id)
        .expect("active route");

    assert_eq!(
        PATHWAY_CAPABILITIES.route_shape_visibility,
        RouteShapeVisibility::ExplicitPath
    );
    assert!(active_route.segment_count > 0);
}
