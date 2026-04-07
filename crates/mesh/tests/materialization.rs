//! Integration tests for admitted-plan materialization in the mesh engine.
//!
//! These tests make sure materialization is driven by the admitted opaque
//! backend plan token rather than the planner cache.

mod common;

use jacquard_traits::{
    jacquard_core::{DestinationId, NodeId, RouteEpoch, RouteError, RouteRuntimeError, Tick},
    RoutingEngine, RoutingEnginePlanner,
};

use common::engine::{build_engine_at_tick, lease, materialization_input, objective, profile};
use common::fixtures::sample_configuration;

#[test]
fn materialize_route_succeeds_after_engine_tick_clears_candidate_cache() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("initial engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(2), Tick(20)));

    engine
        .engine_tick(&topology)
        .expect("second engine tick clears the planner cache");
    engine
        .materialize_route(input)
        .expect("materialization must not depend on planner cache state");
}

#[test]
fn materialize_route_succeeds_after_candidate_cache_rebuild() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let destination_three = objective(DestinationId::Node(NodeId([3; 32])));
    let destination_four = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&destination_three, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&destination_three, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(2), Tick(20)));

    let _rebuilt_cache = engine.candidate_routes(&destination_four, &policy, &topology);
    engine
        .materialize_route(input)
        .expect("materialization must survive unrelated cache rebuilds");
}

#[test]
fn materialize_route_fails_closed_for_mismatched_backend_plan_token() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let alternate_goal = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let mut input = materialization_input(admission, lease(Tick(2), Tick(20)));

    let mismatched_candidate = engine
        .candidate_routes(&alternate_goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("alternate candidate");
    input.admission.backend_ref = mismatched_candidate.backend_ref;

    let error = engine
        .materialize_route(input)
        .expect_err("mismatched admitted plan tokens must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

#[test]
fn materialize_route_rolls_back_when_event_logging_fails() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(2), Tick(20)));

    engine.runtime_effects_mut().fail_record_route_event = true;
    let error = engine
        .materialize_route(input.clone())
        .expect_err("event log failure must fail closed");

    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::MaintenanceFailed)
    ));
    assert_eq!(engine.active_route_count(), 0);
    assert!(engine.runtime_effects().events.is_empty());
    assert_eq!(
        engine.runtime_effects().storage.len(),
        1,
        "materialization rollback should leave only the topology epoch checkpoint"
    );
}

#[test]
fn materialize_route_fails_closed_for_mismatched_handle_epoch() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let mut input = materialization_input(admission, lease(Tick(2), Tick(20)));
    input.handle.topology_epoch = RouteEpoch(99);

    let error = engine
        .materialize_route(input)
        .expect_err("epoch-mismatched materialization must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

#[test]
fn materialize_route_fails_closed_when_latest_topology_epoch_has_advanced() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("initial engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(2), Tick(20)));

    let mut advanced_topology = topology.clone();
    advanced_topology.value.epoch = RouteEpoch(3);
    engine
        .engine_tick(&advanced_topology)
        .expect("advanced topology tick");

    let error = engine
        .materialize_route(input)
        .expect_err("stale admitted topology epoch must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

#[test]
fn materialize_route_fails_closed_when_plan_token_has_expired() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(20), Tick(30)));
    engine.runtime_effects_mut().now = Tick(20);

    let error = engine
        .materialize_route(input)
        .expect_err("expired admitted plan token must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}
