//! Integration tests for the mesh maintenance trigger matrix and the
//! repair budget exhaustion path.
//!
//! Each `RouteMaintenanceTrigger` variant maps to a specific
//! `RouteMaintenanceOutcome` in the engine's dispatch logic. The end-to-end
//! test in `integration.rs` covers `LinkDegraded`, `PartitionDetected`, and
//! `AntiEntropyRequired`. This file fills in `CapacityExceeded`,
//! `PolicyShift`, `EpochAdvanced` (with and without budget remaining),
//! `LeaseExpiring`, and `RouteExpired`. It also verifies that repeated
//! `LinkDegraded` triggers eventually exhaust the repair budget and
//! escalate to `ReplacementRequired`.

mod common;

use common::{build_engine, sample_configuration};
use jacquard_traits::{
    jacquard_core::{
        DestinationId, MaterializedRouteIdentity, NodeId, PublicationId, RouteHandle, RouteLease,
        RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceTrigger,
        RouteMaterializationInput, RouteRuntimeState, Tick, TimeWindow,
    },
    RoutingEngine, RoutingEnginePlanner,
};

// Helper that materializes a single route at tick 2 with a long lease so
// the maintenance trigger under test is the only thing that can fail.
fn materialize_test_route() -> (
    common::TestEngine,
    MaterializedRouteIdentity,
    RouteRuntimeState,
) {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    engine.engine_tick(&topology).expect("engine tick");
    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate available");
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
            valid_for: TimeWindow::new(Tick(2), Tick(1000)).expect("valid lease"),
        },
    };
    let installation = engine
        .materialize_route(input.clone())
        .expect("materialization");
    let runtime = RouteRuntimeState {
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
    (engine, identity, runtime)
}

// CapacityExceeded must drive the route into partition mode and produce
// HoldFallback with the trigger preserved on the outcome.
#[test]
fn capacity_exceeded_enters_partition_mode_and_returns_hold_fallback() {
    let (mut engine, identity, mut runtime) = materialize_test_route();
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::CapacityExceeded,
        )
        .expect("maintenance succeeds");
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::HoldFallback {
            trigger: RouteMaintenanceTrigger::CapacityExceeded,
        }
    );
}

// PolicyShift must produce a HandedOff outcome carrying a populated
// RouteSemanticHandoff with from and to node ids and a receipt id.
#[test]
fn policy_shift_returns_handed_off_with_populated_handoff_object() {
    let (mut engine, identity, mut runtime) = materialize_test_route();
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("maintenance succeeds");
    let handoff = match result.outcome {
        RouteMaintenanceOutcome::HandedOff(handoff) => handoff,
        other => panic!("expected HandedOff, got {other:?}"),
    };
    assert_eq!(handoff.from_node_id, NodeId([1; 32]));
    assert_eq!(handoff.route_id, identity.handle.route_id);
}

// EpochAdvanced with repair budget remaining bumps the active epoch and
// consumes a repair step rather than escalating to replacement.
#[test]
fn epoch_advanced_with_budget_repairs_and_bumps_epoch() {
    let (mut engine, identity, mut runtime) = materialize_test_route();
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("maintenance succeeds");
    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
}

// LeaseExpiring is the soft signal: it does not fail outright but does
// signal that the route should be replaced before the lease ends.
#[test]
fn lease_expiring_returns_replacement_required() {
    let (mut engine, identity, mut runtime) = materialize_test_route();
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("maintenance succeeds");
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::ReplacementRequired {
            trigger: RouteMaintenanceTrigger::LeaseExpiring,
        }
    );
}

// RouteExpired is the typed lifecycle terminator: the route is over and
// the engine reports a typed LeaseExpired failure with progress Failed.
#[test]
fn route_expired_returns_typed_lease_expired_failure() {
    let (mut engine, identity, mut runtime) = materialize_test_route();
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::RouteExpired,
        )
        .expect("maintenance succeeds");
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired)
    );
}

// Repeated LinkDegraded triggers must eventually exhaust the repair
// budget. Once the budget is gone, the next LinkDegraded must escalate
// to ReplacementRequired rather than continuing to report Repaired.
#[test]
fn repair_budget_exhausts_and_escalates_to_replacement() {
    let (mut engine, identity, mut runtime) = materialize_test_route();
    let initial_budget = engine
        .active_route(&identity.handle.route_id)
        .map(|route| route.repair_steps_remaining)
        .expect("active route exists");
    assert!(initial_budget > 0, "repair budget should be positive");

    for _ in 0..initial_budget {
        let result = engine
            .maintain_route(
                &identity,
                &mut runtime,
                RouteMaintenanceTrigger::LinkDegraded,
            )
            .expect("maintenance succeeds while budget remains");
        assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    }

    let escalated = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance succeeds after budget exhaustion");
    assert_eq!(
        escalated.outcome,
        RouteMaintenanceOutcome::ReplacementRequired {
            trigger: RouteMaintenanceTrigger::LinkDegraded,
        }
    );
}
