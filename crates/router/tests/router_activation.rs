mod common;

use common::{build_router, objective, FAR_NODE_ID};
use jacquard_core::{
    DestinationId, RouteMaintenanceTrigger, RouterCanonicalMutation,
    RoutingEvidenceClass, Tick,
};
use jacquard_traits::{Router, RoutingControlPlane, RoutingDataPlane};

#[test]
fn activate_route_publishes_router_owned_materialized_route() {
    let mut router = build_router(Tick(2));

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("router activation");

    let stored = router
        .active_route(&route.identity.handle.route_id)
        .expect("router stores active route");
    assert_eq!(stored.identity.handle, route.identity.handle);
    assert_eq!(
        stored.identity.materialization_proof.publication_id,
        route.identity.handle.publication_id,
    );
}

#[test]
fn route_commitments_use_router_published_route_identity() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("router activation");

    let commitments = router
        .route_commitments(&route.identity.handle.route_id)
        .expect("route commitments");

    assert!(!commitments.is_empty());
    assert!(commitments.iter().all(|commitment| commitment.route_binding
        == jacquard_core::RouteBinding::Bound(route.identity.handle.route_id)));
}

#[test]
fn reselect_route_replaces_router_published_route() {
    let mut router = build_router(Tick(2));
    let first = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("first activation");

    let replacement = router
        .reselect_route(
            &first.identity.handle.route_id,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("reselection");

    assert_ne!(
        first.identity.handle.publication_id,
        replacement.identity.handle.publication_id,
    );
    assert!(router
        .active_route(&replacement.identity.handle.route_id)
        .is_some());
}

#[test]
fn maintain_route_dispatches_to_engine_via_control_plane() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");

    let result = router
        .maintain_route(
            &route.identity.handle.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance result");

    assert_eq!(
        result.engine_result.event,
        jacquard_core::RouteLifecycleEvent::Activated,
    );
    assert_eq!(
        result.engine_result.outcome,
        jacquard_core::RouteMaintenanceOutcome::Continued,
    );
    assert_eq!(result.canonical_mutation, RouterCanonicalMutation::None);
}

#[test]
fn anti_entropy_tick_drives_engine_tick_without_exposing_private_state() {
    let mut router = build_router(Tick(2));

    let outcome = router.anti_entropy_tick().expect("anti-entropy tick");

    assert_eq!(
        router.registered_engine_ids(),
        vec![jacquard_mesh::MESH_ENGINE_ID]
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&jacquard_mesh::MESH_ENGINE_ID)
            .expect("registered capabilities")
            .engine,
        jacquard_mesh::MESH_ENGINE_ID
    );
    assert_eq!(
        outcome.topology_epoch,
        common::sample_configuration().value.epoch
    );
    assert_eq!(outcome.canonical_mutation, RouterCanonicalMutation::None);
}

#[test]
fn anti_entropy_tick_drives_mesh_cooperative_choreographies_through_router_cadence() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");
    let _ = router
        .maintain_route(
            &route.identity.handle.route_id,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("enter partition mode");

    let outcome = router.anti_entropy_tick().expect("anti-entropy tick");

    assert_eq!(
        outcome.topology_epoch,
        common::sample_configuration().value.epoch
    );
    assert_eq!(
        outcome.engine_change,
        jacquard_core::RoutingTickChange::PrivateStateUpdated,
    );
    assert!(router
        .active_route(&route.identity.handle.route_id)
        .is_some());
}

#[test]
fn observe_route_health_reports_router_owned_observation() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");

    let observed = router
        .observe_route_health(&route.identity.handle.route_id)
        .expect("health observation");

    assert_eq!(observed.value, route.runtime.health);
    assert_eq!(observed.source_class, jacquard_core::FactSourceClass::Local);
    assert_eq!(
        observed.evidence_class,
        RoutingEvidenceClass::AdmissionWitnessed
    );
}
