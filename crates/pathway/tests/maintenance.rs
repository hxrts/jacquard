//! Integration tests for the pathway maintenance trigger matrix and the
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

use common::{
    engine::{
        activate_route, activate_route_with_profile, build_engine, lease, objective,
        profile_with_connectivity,
    },
    fixtures::sample_configuration,
    BRIDGE_NODE_ID, FAR_NODE_ID, LOCAL_NODE_ID, PEER_NODE_ID,
};
use jacquard_traits::{
    jacquard_core::{
        DestinationId, RouteError, RouteMaintenanceFailure, RouteMaintenanceOutcome,
        RouteMaintenanceResult, RouteMaintenanceTrigger, RoutePartitionClass, RouteRepairClass,
        RouteRuntimeError, Tick,
    },
    RouterManagedEngine, RoutingEngine,
};

/// Build a fresh engine, activate a route to [`FAR_NODE_ID`] (node `[3;32]`),
/// call `maintain_route` with the given trigger, and pass the result to the
/// provided closure for assertions.
///
/// This helper eliminates the build / tick / activate boilerplate that was
/// duplicated across the simple trigger matrix tests.
fn run_maintenance_trigger_test(
    trigger: RouteMaintenanceTrigger,
    check: impl FnOnce(RouteMaintenanceResult),
) {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
        lease(Tick(2), Tick(1000)),
    );
    let result = engine
        .maintain_route(&identity, &mut runtime, trigger)
        .expect("maintenance succeeds");
    check(result);
}

// CapacityExceeded is replacement pressure, not partition evidence. The
// route must stay out of partition mode and return a typed replacement
// requirement.
#[test]
fn capacity_exceeded_requires_replacement_without_entering_partition_mode() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
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
        RouteMaintenanceOutcome::ReplacementRequired {
            trigger: RouteMaintenanceTrigger::CapacityExceeded,
        }
    );
    let active_route = engine
        .active_route(&identity.stamp.route_id)
        .expect("active route remains installed");
    assert!(!active_route.retention.partition_mode);
}

// PolicyShift must produce a HandedOff outcome carrying a populated
// RouteSemanticHandoff and must rebase the runtime cursor to the next
// owner-relative hop.
#[test]
fn policy_shift_rebases_runtime_to_the_next_owner_relative_hop() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
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
    assert_eq!(handoff.from_node_id, LOCAL_NODE_ID);
    assert_eq!(handoff.to_node_id, PEER_NODE_ID);
    assert_eq!(handoff.route_id, identity.stamp.route_id);

    let active_route = engine
        .active_route(&identity.stamp.route_id)
        .expect("active route remains installed");
    assert_eq!(active_route.forwarding.current_owner_node_id, PEER_NODE_ID);
    assert_eq!(active_route.forwarding.next_hop_index, 1);

    let error = engine
        .forward_payload_for_router(&identity.stamp.route_id, b"payload")
        .expect_err("old owner must fail closed after handoff");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::StaleOwner)
    ));
}

// A single-hop path can still be handed off to the destination. The
// owner-relative cursor should advance to the end of the path rather
// than leaving a stale "first hop" behind.
#[test]
fn single_hop_policy_shift_advances_cursor_to_path_end() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route_with_profile(
        &mut engine,
        &topology,
        &objective(DestinationId::Node(BRIDGE_NODE_ID)),
        &profile_with_connectivity(
            RouteRepairClass::BestEffort,
            RoutePartitionClass::ConnectedOnly,
        ),
        lease(Tick(2), Tick(1000)),
    );

    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("single-hop handoff succeeds");
    let handoff = match result.outcome {
        RouteMaintenanceOutcome::HandedOff(handoff) => handoff,
        other => panic!("expected HandedOff, got {other:?}"),
    };
    assert_eq!(handoff.to_node_id, BRIDGE_NODE_ID);
    let active_route = engine
        .active_route(&identity.stamp.route_id)
        .expect("active route remains installed");
    assert_eq!(
        active_route.forwarding.current_owner_node_id,
        BRIDGE_NODE_ID
    );
    assert_eq!(
        usize::from(active_route.forwarding.next_hop_index),
        active_route.segment_count
    );
}

// Once the owner-relative cursor has reached the end of the path, a
// second PolicyShift has no valid next owner and must fail closed.
#[test]
fn repeated_policy_shift_after_full_handoff_fails_closed() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route_with_profile(
        &mut engine,
        &topology,
        &objective(DestinationId::Node(BRIDGE_NODE_ID)),
        &profile_with_connectivity(
            RouteRepairClass::BestEffort,
            RoutePartitionClass::ConnectedOnly,
        ),
        lease(Tick(2), Tick(1000)),
    );

    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("initial single-hop handoff succeeds");
    let error = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect_err("no further handoff should be possible");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

// EpochAdvanced with repair budget remaining bumps the active epoch and
// consumes a repair step rather than escalating to replacement.
#[test]
fn epoch_advanced_with_budget_repairs_and_bumps_epoch() {
    run_maintenance_trigger_test(RouteMaintenanceTrigger::EpochAdvanced, |result| {
        assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    });
}

// LeaseExpiring is the soft signal: it does not fail outright but does
// signal that the route should be replaced before the lease ends.
#[test]
fn lease_expiring_returns_replacement_required() {
    run_maintenance_trigger_test(RouteMaintenanceTrigger::LeaseExpiring, |result| {
        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::ReplacementRequired {
                trigger: RouteMaintenanceTrigger::LeaseExpiring,
            }
        );
    });
}

// RouteExpired is the typed lifecycle terminator: the route is over and
// the engine reports a typed LeaseExpired failure with progress Failed.
#[test]
fn route_expired_returns_typed_lease_expired_failure() {
    run_maintenance_trigger_test(RouteMaintenanceTrigger::RouteExpired, |result| {
        assert_eq!(
            result.outcome,
            RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired)
        );
    });
}

#[test]
fn maintenance_checkpoint_failure_leaves_runtime_and_active_route_unchanged() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
        lease(Tick(2), Tick(1000)),
    );
    let original_runtime = runtime.clone();
    let original_active_route = engine
        .active_route(&identity.stamp.route_id)
        .expect("active route present");
    let original_event_count = engine.effects.events().len();

    engine.effects.fail_store_bytes();
    let error = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::CapacityExceeded,
        )
        .expect_err("checkpoint failure must fail closed");

    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
    assert_eq!(runtime, original_runtime);
    assert_eq!(
        engine
            .active_route(&identity.stamp.route_id)
            .expect("active route remains installed"),
        original_active_route
    );
    assert_eq!(engine.effects.events().len(), original_event_count);
}

// In v1 pathway, AntiEntropyRequired is a typed progress refresh over the
// shared-world view. It does not yet perform route-export exchange or
// pathway-private reconciliation work.
#[test]
fn anti_entropy_required_is_a_progress_refresh_in_v1_pathway() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
        lease(Tick(2), Tick(1000)),
    );

    let before = runtime.progress.last_progress_at_tick;
    engine.effects.set_now(Tick(7));
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance succeeds");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
    assert!(runtime.progress.last_progress_at_tick > before);
}

#[test]
fn policy_shift_flushes_retained_payloads_before_handoff() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
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
        .forward_payload_for_router(&identity.stamp.route_id, b"held-before-handoff")
        .expect("partitioned forwarding retains payload");

    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("handoff succeeds");
    assert!(matches!(
        result.outcome,
        RouteMaintenanceOutcome::HandedOff(_)
    ));
    let active_route = engine
        .active_route(&identity.stamp.route_id)
        .expect("active route remains installed");
    assert_eq!(active_route.retention.retained_object_count, 0);
    assert!(engine
        .transport
        .sent_frames()
        .iter()
        .any(|(_endpoint, payload)| payload == b"held-before-handoff"));
}

// Repeated LinkDegraded triggers must eventually exhaust the repair
// budget. Once the budget is gone, the next LinkDegraded must escalate
// to ReplacementRequired rather than continuing to report Repaired.
#[test]
fn link_degraded_consumes_one_repair_budget_step_in_v1_pathway() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        FAR_NODE_ID,
        lease(Tick(2), Tick(1000)),
    );

    let before = engine
        .active_route(&identity.stamp.route_id)
        .map(|route| route.repair_steps_remaining)
        .expect("active route exists");
    let result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance succeeds");
    let after = engine
        .active_route(&identity.stamp.route_id)
        .map(|route| route.repair_steps_remaining)
        .expect("active route exists after repair");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Repaired);
    assert_eq!(after, before - 1);
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
        FAR_NODE_ID,
        lease(Tick(2), Tick(1000)),
    );

    let initial_budget = engine
        .active_route(&identity.stamp.route_id)
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
