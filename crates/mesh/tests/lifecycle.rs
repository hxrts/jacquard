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
    MeshRoutingEngine, RetentionStore, RoutingEngine,
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
    let route_id = identity.handle.route_id;

    engine
        .forward_payload(&route_id, b"mesh-payload")
        .expect("forwarding");
    assert_eq!(engine.transport().sent_frames.len(), 1);

    let retained = engine
        .retain_for_route(&route_id, b"partition-buffer")
        .expect("retain payload");
    assert!(engine
        .retention_store()
        .contains_retained_payload(&retained)
        .expect("retention lookup"));
    assert_eq!(
        engine
            .recover_retained_payload(&route_id, &retained)
            .expect("recover payload"),
        Some(b"partition-buffer".to_vec())
    );

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
            trigger:               RouteMaintenanceTrigger::PartitionDetected,
            retained_object_count: 0,
        }
    );
    engine
        .forward_payload(&route_id, b"held-during-partition")
        .expect("partitioned forwarding retains payload");
    assert_eq!(
        engine.transport().sent_frames.len(),
        1,
        "partitioned forwarding should buffer rather than send immediately"
    );
    assert_eq!(
        engine
            .active_route(&route_id)
            .expect("active route present")
            .anti_entropy
            .retained_objects
            .len(),
        1
    );

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
        .transport()
        .sent_frames
        .iter()
        .any(|(_endpoint, payload)| payload == b"held-during-partition"));
    assert!(engine
        .active_route(&route_id)
        .expect("active route present")
        .anti_entropy
        .retained_objects
        .is_empty());

    engine.runtime_effects_mut().now = Tick(12);
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
    assert_eq!(engine.runtime_effects().events.len(), 5);
    assert!(matches!(
        runtime.health.reachability_state,
        ReachabilityState::Reachable
    ));
}
