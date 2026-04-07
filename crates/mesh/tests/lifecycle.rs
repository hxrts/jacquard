//! End-to-end integration test for an active route's full lifecycle.
//!
//! This is the broad smoke test that drives a single route from
//! candidate production through admission, materialization, payload
//! forwarding, retention, repair, partition fallback, and lease
//! expiry. The narrow rejection-path, trigger-matrix, and lease-window
//! tests live in `admission.rs`, `maintenance.rs`, and `lease.rs`.

mod common;

use jacquard_traits::{
    jacquard_core::{
        NodeId, ReachabilityState, RouteMaintenanceFailure, RouteMaintenanceOutcome,
        RouteMaintenanceTrigger, Tick,
    },
    MeshRoutingEngine, RetentionStore, RoutingEngine,
};

use common::engine::{activate_route, build_engine, lease};
use common::fixtures::sample_configuration;

// One materialized route should support payload forwarding, retention
// store deposit and recovery, in-place repair on `LinkDegraded`, hold
// fallback on `PartitionDetected`, and a typed lease-expiry failure
// once the engine clock has moved past the lease window.
#[test]
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
            trigger: RouteMaintenanceTrigger::PartitionDetected,
        }
    );

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
    assert_eq!(engine.runtime_effects().events.len(), 4);
    assert!(matches!(
        runtime.health.reachability_state,
        ReachabilityState::Reachable
    ));
}
