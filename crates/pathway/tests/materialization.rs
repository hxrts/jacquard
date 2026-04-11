//! Integration tests for admitted-plan materialization in the pathway engine.
//!
//! Materialization converts an admitted opaque backend plan token into an
//! installed active route. The plan token is cryptographically bound to the
//! topology epoch at admission time, so the engine must fail closed when that
//! binding is violated. These tests cover:
//!
//! - Successful materialization after the planner cache is cleared by a
//!   subsequent `engine_tick` call, confirming materialization is driven by the
//!   admitted token, not the cache.
//! - Successful materialization after an unrelated candidate rebuild, showing
//!   the installed token survives concurrent planning work.
//! - Fail-closed rejection for mismatched, corrupted, expired, and
//!   epoch-mismatched backend plan tokens.
//! - Independence from router event logging: materialization succeeds and
//!   writes the correct storage keys even when event recording is disabled.

mod common;

use common::{
    engine::{build_engine_at_tick, lease, materialization_input, objective, profile},
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{
        DestinationId, NodeId, RouteEpoch, RouteError, RouteRuntimeError, RoutingTickContext, Tick,
    },
    RoutingEngine, RoutingEnginePlanner,
};

#[test]
fn materialize_route_succeeds_after_engine_tick_clears_candidate_cache() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("initial engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
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

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&destination_three, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&destination_three, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));

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

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let mut input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));

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
fn materialize_route_fails_closed_for_corrupted_backend_plan_token() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let mut admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    admission.backend_ref.backend_route_id.0 = vec![0xff, 0x00, 0xaa];
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));

    let error = engine
        .materialize_route(input)
        .expect_err("corrupted backend token must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

#[test]
fn materialize_route_does_not_depend_on_router_event_logging() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));

    engine.effects.fail_record_route_event();
    let installation = engine
        .materialize_route(input.clone())
        .expect("engine materialization stays independent of router event logging");

    assert_eq!(engine.active_route_count(), 1);
    assert!(engine.effects.events().is_empty());
    assert_eq!(
        installation.materialization_proof.stamp.route_id,
        input.handle.stamp.route_id,
    );
    let remaining_keys = engine
        .effects
        .storage_keys()
        .into_iter()
        .map(|key| String::from_utf8_lossy(&key).into_owned())
        .collect::<Vec<_>>();
    assert!(remaining_keys
        .iter()
        .any(|key| key == &format!("pathway/{}/topology-epoch", "\u{1}".repeat(32))));
    assert!(remaining_keys
        .iter()
        .any(|key| key.starts_with(&format!("pathway/{}/route/", "\u{1}".repeat(32)))));
    assert!(remaining_keys
        .iter()
        .any(|key| key == "pathway/protocol/forwarding/tick-epoch-2"));
    assert!(remaining_keys
        .iter()
        .any(|key| key == "pathway/protocol/neighbor-advertisement/tick-epoch-2"));
}

#[test]
fn materialize_route_fails_closed_for_mismatched_handle_epoch() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let mut input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));
    input.handle.stamp.topology_epoch = RouteEpoch(99);

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

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("initial engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(20)));

    let mut advanced_topology = topology.clone();
    advanced_topology.value.epoch = RouteEpoch(3);
    engine
        .engine_tick(&RoutingTickContext::new(advanced_topology.clone()))
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

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission, lease(Tick(20), Tick(30)));
    engine.effects.set_now(Tick(20));

    let error = engine
        .materialize_route(input)
        .expect_err("expired admitted plan token must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}
