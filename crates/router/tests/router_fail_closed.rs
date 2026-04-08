mod common;

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use common::{
    build_router, build_router_with_effects, build_router_with_recoverable_engine,
    build_router_with_selector, objective, AdvisoryCommitteeSelector, FAR_NODE_ID,
};
use jacquard_core::{DestinationId, RouteMaintenanceTrigger, Tick};
use jacquard_mock_transport::InMemoryRuntimeEffects;
use jacquard_traits::{Router, RoutingControlPlane, RoutingDataPlane};

#[test]
fn failing_committee_selector_cannot_publish_canonical_route_truth() {
    let mut router =
        build_router_with_selector(Tick(2), AdvisoryCommitteeSelector { fail: true });

    let error = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect_err("selector failure must block proof-bearing activation");

    assert!(matches!(
        error,
        jacquard_core::RouteError::Selection(
            jacquard_core::RouteSelectionError::Inadmissible(_)
        )
    ));
    assert_eq!(router.active_route_count(), 0);
}

#[test]
fn activation_fails_closed_when_router_event_logging_fails() {
    let mut router = build_router_with_effects(
        Tick(2),
        InMemoryRuntimeEffects {
            now: Tick(2),
            fail_record_route_event: true,
            ..Default::default()
        },
    );

    let error = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect_err("router must fail closed when canonical event logging fails");

    assert!(matches!(
        error,
        jacquard_core::RouteError::Runtime(
            jacquard_core::RouteRuntimeError::Invalidated
        )
    ));
    assert_eq!(router.active_route_count(), 0);
    assert!(router.effects().events.is_empty());
}

#[test]
fn activation_reselection_and_maintenance_are_deterministic_for_equal_inputs() {
    let mut left = build_router(Tick(2));
    let mut right = build_router(Tick(2));

    let left_route =
        Router::activate_route(&mut left, objective(DestinationId::Node(FAR_NODE_ID)))
            .expect("left activation");
    let right_route =
        Router::activate_route(&mut right, objective(DestinationId::Node(FAR_NODE_ID)))
            .expect("right activation");
    assert_eq!(left_route.identity, right_route.identity);

    let left_maintenance = left
        .maintain_route(
            &left_route.identity.handle.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("left maintenance");
    let right_maintenance = right
        .maintain_route(
            &right_route.identity.handle.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("right maintenance");
    assert_eq!(left_maintenance, right_maintenance);

    let left_reselected = left
        .reselect_route(
            &left_route.identity.handle.route_id,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("left reselection");
    let right_reselected = right
        .reselect_route(
            &right_route.identity.handle.route_id,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("right reselection");
    assert_eq!(left_reselected.identity, right_reselected.identity);
}

#[test]
fn recovery_restores_router_and_mesh_state_from_router_owned_registry() {
    let shared_state = Arc::new(Mutex::new(BTreeSet::new()));
    let mut router = build_router_with_recoverable_engine(
        Tick(2),
        InMemoryRuntimeEffects { now: Tick(2), ..Default::default() },
        shared_state.clone(),
    );
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");
    let persisted_router_effects = router.effects().clone();
    let mut recovered = build_router_with_recoverable_engine(
        Tick(2),
        persisted_router_effects,
        shared_state,
    );
    let restored_count = recovered
        .recover_checkpointed_routes()
        .expect("recover router and engine state");

    assert_eq!(restored_count, 1);
    assert!(recovered
        .active_route(&route.identity.handle.route_id)
        .is_some());
    recovered
        .forward_payload(&route.identity.handle.route_id, b"recovered")
        .expect("recovered router forwards using restored engine-private state");
}
