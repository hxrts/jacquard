//! Integration tests for the mesh lease window boundaries.
//!
//! `TimeWindow::contains` is half-open: `start <= tick < end`. The mesh
//! engine uses `RouteLease::is_valid_at` and `ensure_valid_at` to gate
//! materialization and maintenance, so the boundary cases at `start`,
//! `end - 1`, and `end` must all behave correctly.

mod common;

use common::{
    engine::{
        activate_route, build_engine_at_tick, lease, materialization_input, objective,
        profile,
    },
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{
        DestinationId, NodeId, RouteError, RouteMaintenanceFailure,
        RouteMaintenanceOutcome, RouteMaintenanceTrigger, RouteRuntimeError,
        RoutingTickContext, Tick,
    },
    RoutingEngine, RoutingEnginePlanner,
};

// Materialization at the exact lease start tick must succeed because
// `TimeWindow::contains` is inclusive on the lower bound.
#[test]
fn materialize_route_succeeds_at_lease_start_tick() {
    let mut engine = build_engine_at_tick(Tick(5));
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
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(5), Tick(20)));
    engine
        .materialize_route(input)
        .expect("materialization should succeed at the lease start tick");
}

// Materialization at the exact lease end tick must fail with a typed
// LeaseExpired runtime error because the upper bound is exclusive.
#[test]
fn materialize_route_fails_at_lease_end_tick() {
    let mut engine = build_engine_at_tick(Tick(10));
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
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admission");
    // Lease end is exclusive, so the engine clock at tick 10 is already
    // outside the [2, 10) window.
    let input = materialization_input(admission, lease(Tick(2), Tick(10)));
    let error = engine
        .materialize_route(input)
        .expect_err("materialization should fail at the lease end tick");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::LeaseExpired)
    ));
}

// Maintenance at one tick before the lease end must still succeed,
// confirming the upper-bound check uses strict less-than.
#[test]
fn maintain_route_succeeds_one_tick_before_lease_end() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(10)),
    );

    engine.effects.set_now(Tick(9));
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance at tick 9 should succeed");
    assert!(!matches!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired)
    ));
}

// Maintenance at the exact lease end tick must produce a typed
// LeaseExpired failure regardless of which trigger arrived.
#[test]
fn maintain_route_fails_at_lease_end_tick() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(10)),
    );

    engine.effects.set_now(Tick(10));
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance call returns Ok with a typed failure outcome");
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired)
    );
}
