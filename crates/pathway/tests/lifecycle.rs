//! End-to-end integration test for an active route's full lifecycle.
//!
//! This is the broad smoke test that drives a single route from
//! candidate production through admission, materialization, payload
//! forwarding, retention, repair, partition fallback, and lease
//! expiry. The narrow rejection-path, trigger-matrix, and lease-window
//! tests live in `admission.rs`, `maintenance.rs`, and `lease.rs`.

mod common;

use common::{
    engine::{activate_route, build_engine, lease},
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{
        NodeId, ReachabilityState, RouteMaintenanceFailure, RouteMaintenanceOutcome,
        RouteMaintenanceTrigger, Tick,
    },
    RouterManagedEngine, RoutingEngine,
};

// One materialized route should support payload forwarding, retention
// store deposit and recovery, in-place repair on `LinkDegraded`, hold
// fallback on `PartitionDetected`, automatic buffering while
// partitioned, replay of retained payloads on recovery, and a typed
// lease-expiry failure once the engine clock has moved past the lease
// window.
#[test]
// long-block-exception: end-to-end repair/partition/retention state machine.
fn active_routes_respect_repairs_partitions_and_retention_boundaries() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(10)),
    );
    let route_id = identity.stamp.route_id;

    engine
        .forward_payload_for_router(&route_id, b"test-payload")
        .expect("forwarding");
    assert_eq!(engine.transport.sent_frames().len(), 1);

    let repaired = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("repair");
    assert_eq!(repaired.outcome, RouteMaintenanceOutcome::Repaired);

    let hold_fallback = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("partition maintenance");
    assert_eq!(
        hold_fallback.outcome,
        RouteMaintenanceOutcome::HoldFallback {
            trigger: RouteMaintenanceTrigger::PartitionDetected,
            retained_object_count: jacquard_traits::jacquard_core::HoldItemCount(0),
        }
    );
    engine
        .forward_payload_for_router(&route_id, b"held-during-partition")
        .expect("partitioned forwarding retains payload");
    assert_eq!(
        engine.transport.sent_frames().len(),
        1,
        "partitioned forwarding should buffer rather than send immediately"
    );
    assert_eq!(
        engine
            .active_route(&route_id)
            .expect("active route present")
            .retention
            .retained_object_count,
        1
    );
    assert_eq!(engine.retention.payload_count(), 1);

    let recovered = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("anti-entropy recovery");
    assert_eq!(recovered.outcome, RouteMaintenanceOutcome::Continued);
    assert_eq!(
        runtime.last_lifecycle_event,
        jacquard_traits::jacquard_core::RouteLifecycleEvent::RecoveredFromPartition
    );
    assert!(engine
        .transport
        .sent_frames()
        .iter()
        .any(|(_endpoint, payload)| payload == b"held-during-partition"));
    assert!(
        engine
            .active_route(&route_id)
            .expect("active route present")
            .retention
            .retained_object_count
            == 0
    );
    assert_eq!(engine.retention.payload_count(), 0);

    engine.effects.set_now(Tick(12));
    let expired = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("lease expiry maintenance");
    assert_eq!(
        expired.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::LeaseExpired)
    );
    assert!(
        engine.effects.events().is_empty(),
        "canonical lifecycle events are now router-owned rather than engine-owned"
    );
    assert!(matches!(
        runtime.health.reachability_state,
        ReachabilityState::Reachable
    ));
}

// The same active route should survive the common repair → partition →
// recovery transition sequence without drifting to a new canonical route id or
// losing its owner-relative runtime progress record.
#[test]
fn transition_sequence_preserves_active_route_identity() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(100)),
    );
    let route_id = identity.stamp.route_id;

    for trigger in [
        RouteMaintenanceTrigger::LinkDegraded,
        RouteMaintenanceTrigger::PartitionDetected,
        RouteMaintenanceTrigger::AntiEntropyRequired,
    ] {
        let result = engine
            .maintain_route(&identity, &mut runtime, trigger)
            .expect("maintenance trigger should succeed");
        runtime.last_lifecycle_event = result.event;
        assert!(engine.active_route(&route_id).is_some());
        assert_eq!(identity.stamp.route_id, route_id);
    }

    let active = engine
        .active_route(&route_id)
        .expect("active route remains");
    assert_eq!(active.segment_count, 2);
    assert!(usize::from(active.forwarding.next_hop_index) <= active.segment_count);
}
