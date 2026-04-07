//! Integration tests for the mesh lease window boundaries.
//!
//! `TimeWindow::contains` is half-open: `start <= tick < end`. The mesh
//! engine uses `RouteLease::is_valid_at` and `ensure_valid_at` to gate
//! materialization and maintenance, so the boundary cases at `start`,
//! `end - 1`, and `end` must all behave correctly.

mod common;

use common::{build_engine_at_tick, sample_configuration};
use jacquard_traits::{
    jacquard_core::{
        DestinationId, MaterializedRouteIdentity, NodeId, PublicationId, RouteHandle, RouteLease,
        RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceTrigger,
        RouteMaterializationInput, RouteRuntimeError, RouteRuntimeState, Tick, TimeWindow,
    },
    RoutingEngine, RoutingEnginePlanner,
};

// Materialization at the exact lease start tick must succeed because
// `TimeWindow::contains` is inclusive on the lower bound.
#[test]
fn materialize_route_succeeds_at_lease_start_tick() {
    let mut engine = build_engine_at_tick(Tick(5));
    let topology = sample_configuration();
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admission");
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            route_id: admission.route_id,
            topology_epoch: topology.value.epoch,
            materialized_at_tick: Tick(5),
            publication_id: PublicationId([7; 16]),
        },
        admission: admission.clone(),
        // Lease starts exactly at the engine's current tick.
        lease: RouteLease {
            owner_node_id: NodeId([1; 32]),
            lease_epoch: topology.value.epoch,
            valid_for: TimeWindow::new(Tick(5), Tick(20)).expect("valid lease"),
        },
    };
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
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admission");
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            route_id: admission.route_id,
            topology_epoch: topology.value.epoch,
            materialized_at_tick: Tick(10),
            publication_id: PublicationId([7; 16]),
        },
        admission: admission.clone(),
        // Lease end is exclusive, so the engine clock at tick 10 is
        // already outside the [2, 10) window.
        lease: RouteLease {
            owner_node_id: NodeId([1; 32]),
            lease_epoch: topology.value.epoch,
            valid_for: TimeWindow::new(Tick(2), Tick(10)).expect("valid lease"),
        },
    };
    let error = engine
        .materialize_route(input)
        .expect_err("materialization should fail at the lease end tick");
    assert!(matches!(
        error,
        jacquard_traits::jacquard_core::RouteError::Runtime(RouteRuntimeError::LeaseExpired)
    ));
}

// Maintenance at one tick before the lease end must still succeed,
// confirming the upper-bound check uses strict less-than.
#[test]
fn maintain_route_succeeds_one_tick_before_lease_end() {
    let mut engine = build_engine_at_tick(Tick(2));
    let topology = sample_configuration();
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admission");
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            route_id: admission.route_id,
            topology_epoch: topology.value.epoch,
            materialized_at_tick: Tick(2),
            publication_id: PublicationId([7; 16]),
        },
        admission: admission.clone(),
        lease: RouteLease {
            owner_node_id: NodeId([1; 32]),
            lease_epoch: topology.value.epoch,
            valid_for: TimeWindow::new(Tick(2), Tick(10)).expect("valid lease"),
        },
    };
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialization");
    let mut runtime = RouteRuntimeState {
        last_lifecycle_event: installation.last_lifecycle_event,
        health: installation.health,
        progress: installation.progress,
    };
    let identity = MaterializedRouteIdentity {
        handle: input.handle,
        materialization_proof: installation.materialization_proof,
        admission: input.admission,
        lease: input.lease,
    };

    engine.runtime_effects_mut().now = Tick(9);
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
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admission");
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            route_id: admission.route_id,
            topology_epoch: topology.value.epoch,
            materialized_at_tick: Tick(2),
            publication_id: PublicationId([7; 16]),
        },
        admission: admission.clone(),
        lease: RouteLease {
            owner_node_id: NodeId([1; 32]),
            lease_epoch: topology.value.epoch,
            valid_for: TimeWindow::new(Tick(2), Tick(10)).expect("valid lease"),
        },
    };
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialization");
    let mut runtime = RouteRuntimeState {
        last_lifecycle_event: installation.last_lifecycle_event,
        health: installation.health,
        progress: installation.progress,
    };
    let identity = MaterializedRouteIdentity {
        handle: input.handle,
        materialization_proof: installation.materialization_proof,
        admission: input.admission,
        lease: input.lease,
    };

    engine.runtime_effects_mut().now = Tick(10);
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
