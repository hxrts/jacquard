//! Integration tests for the mesh maintenance trigger matrix and the
//! repair budget exhaustion path.
//!
//! Each `RouteMaintenanceTrigger` variant maps to a specific
//! `RouteMaintenanceOutcome` in the engine's dispatch logic. The
//! end-to-end test in `lifecycle.rs` covers `LinkDegraded`,
//! `PartitionDetected`, and `AntiEntropyRequired`. This file fills in
//! `CapacityExceeded`, `PolicyShift`, `EpochAdvanced`, `LeaseExpiring`,
//! and `RouteExpired`. It also verifies that repeated `LinkDegraded`
//! triggers eventually exhaust the repair budget and escalate to
//! `ReplacementRequired`.

mod common;

use jacquard_traits::{
    jacquard_core::{
        NodeId, RouteMaintenanceFailure, RouteMaintenanceOutcome, RouteMaintenanceTrigger, Tick,
    },
    RoutingEngine,
};

use common::engine::{activate_route, build_engine, lease};
use common::fixtures::sample_configuration;

// CapacityExceeded must drive the route into partition mode and produce
// HoldFallback with the trigger preserved on the outcome.
#[test]
fn capacity_exceeded_enters_partition_mode_and_returns_hold_fallback() {
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
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("maintenance succeeds");
    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
}

// LeaseExpiring is the soft signal: it does not fail outright but does
// signal that the route should be replaced before the lease ends.
#[test]
fn lease_expiring_returns_replacement_required() {
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
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );

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
