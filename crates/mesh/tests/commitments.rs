//! Integration tests for v1 mesh route-commitment semantics.
//!
//! V1 mesh publishes one current commitment per route even when the route is in
//! a repair, handoff, or deferred-delivery posture. Those postures live in the
//! route's runtime state instead of becoming separate concurrent commitments.

mod common;

use common::{
    engine::{activate_route, build_engine, lease},
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{
        MaterializedRoute, NodeId, RouteCommitmentResolution, RouteMaintenanceTrigger,
        Tick,
    },
    RoutingEngine,
};

fn materialized_route_for(
    identity: jacquard_traits::jacquard_core::MaterializedRouteIdentity,
    runtime: jacquard_traits::jacquard_core::RouteRuntimeState,
) -> MaterializedRoute {
    MaterializedRoute { identity, runtime }
}

#[test]
// long-block-exception: steady/repair/partition/handoff posture in one block.
fn v1_mesh_exposes_one_commitment_per_route_across_runtime_postures() {
    let topology = sample_configuration();
    let mut engine = build_engine();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );

    let steady = materialized_route_for(identity.clone(), runtime.clone());
    let steady_commitments = engine.route_commitments(&steady);
    assert_eq!(steady_commitments.len(), 1);

    let repair_result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("repair maintenance");
    runtime.last_lifecycle_event = repair_result.event;
    let repaired = materialized_route_for(identity.clone(), runtime.clone());
    let repaired_commitments = engine.route_commitments(&repaired);
    assert_eq!(repaired_commitments.len(), 1);
    assert_eq!(
        repaired_commitments[0].commitment_id,
        steady_commitments[0].commitment_id
    );

    let partition_result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("partition maintenance");
    runtime.last_lifecycle_event = partition_result.event;
    engine
        .forward_payload(&identity.handle.route_id, b"retained")
        .expect("retention forwarding");
    let retained = materialized_route_for(identity.clone(), runtime.clone());
    let retained_commitments = engine.route_commitments(&retained);
    assert_eq!(retained_commitments.len(), 1);
    assert_eq!(
        retained_commitments[0].commitment_id,
        steady_commitments[0].commitment_id
    );

    let handoff_result = engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("handoff maintenance");
    runtime.last_lifecycle_event = handoff_result.event;
    let handed_off = materialized_route_for(identity.clone(), runtime);
    let handoff_commitments = engine.route_commitments(&handed_off);
    assert_eq!(handoff_commitments.len(), 1);
    assert_eq!(
        handoff_commitments[0].commitment_id,
        steady_commitments[0].commitment_id
    );
}

#[test]
fn expired_route_still_has_one_invalidated_commitment() {
    let topology = sample_configuration();
    let mut engine = build_engine();
    let (identity, runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(10)),
    );
    engine.runtime_effects_mut().now = Tick(11);

    let route = materialized_route_for(identity, runtime);
    let commitments = engine.route_commitments(&route);
    assert_eq!(commitments.len(), 1);
    assert!(matches!(
        commitments[0].resolution,
        RouteCommitmentResolution::Invalidated(_)
    ));
}
