//! Integration tests for mesh checkpoint storage and recovery.

mod common;

use common::{
    engine::{activate_route, build_engine, lease},
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{NodeId, Tick},
    RoutingEngine,
};

fn has_protocol_checkpoint(engine: &common::engine::TestEngine, prefix: &str) -> bool {
    engine.runtime_effects().storage.keys().any(|key| {
        std::str::from_utf8(key).is_ok_and(|value| value.starts_with(prefix))
    })
}

#[test]
fn checkpointed_active_route_round_trips_across_engine_restart() {
    let topology = sample_configuration();
    let mut original = build_engine();
    let (identity, _runtime) = activate_route(
        &mut original,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );
    let original_active_route = original
        .active_route(&identity.handle.route_id)
        .expect("active route present")
        .clone();
    let stored_bytes = original.runtime_effects().storage.clone();

    let mut recovered = build_engine();
    recovered.runtime_effects_mut().storage = stored_bytes;

    assert_eq!(
        recovered
            .checkpointed_topology_epoch()
            .expect("load topology epoch"),
        Some(topology.value.epoch)
    );

    let restored = recovered
        .restore_checkpointed_route(&identity.handle.route_id)
        .expect("restore checkpointed route")
        .expect("checkpointed route present");

    assert_eq!(restored, original_active_route);
    assert_eq!(
        recovered
            .active_route(&identity.handle.route_id)
            .expect("restored route present"),
        &original_active_route
    );
}

#[test]
fn checkpoint_round_trip_preserves_richer_runtime_substates() {
    let topology = sample_configuration();
    let mut original = build_engine();
    let (identity, mut runtime) = activate_route(
        &mut original,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );

    original
        .maintain_route(
            &identity,
            &mut runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("record repair state");
    original
        .maintain_route(
            &identity,
            &mut runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::CapacityExceeded,
        )
        .expect("enter partition mode");
    original
        .forward_payload(&identity.handle.route_id, b"checkpointed payload")
        .expect("buffer retained payload");
    original
        .maintain_route(
            &identity,
            &mut runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("record handoff state");

    let original_active_route = original
        .active_route(&identity.handle.route_id)
        .expect("active route present")
        .clone();
    let stored_bytes = original.runtime_effects().storage.clone();

    let mut recovered = build_engine();
    recovered.runtime_effects_mut().storage = stored_bytes;

    let restored = recovered
        .restore_checkpointed_route(&identity.handle.route_id)
        .expect("restore checkpointed route")
        .expect("checkpointed route present");

    assert_eq!(restored, original_active_route);
}

#[test]
fn protocol_checkpoints_round_trip_without_hidden_runtime_state() {
    let topology = sample_configuration();
    let mut original = build_engine();
    let (identity, mut runtime) = activate_route(
        &mut original,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(1000)),
    );

    original
        .maintain_route(
            &identity,
            &mut runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("record repair protocol checkpoint");
    original
        .engine_tick(&common::engine::tick_context(&topology))
        .expect("record tick protocol checkpoint");

    assert!(has_protocol_checkpoint(
        &original,
        "mesh/protocol/activation/"
    ));
    assert!(has_protocol_checkpoint(&original, "mesh/protocol/repair/"));
    assert!(has_protocol_checkpoint(
        &original,
        "mesh/protocol/forwarding/tick-epoch-"
    ));

    let stored_bytes = original.runtime_effects().storage.clone();
    let mut recovered = build_engine();
    recovered.runtime_effects_mut().storage = stored_bytes;

    let restored = recovered
        .restore_checkpointed_route(&identity.handle.route_id)
        .expect("restore checkpointed route")
        .expect("checkpointed route present");
    assert_eq!(
        restored.last_lifecycle_event,
        jacquard_traits::jacquard_core::RouteLifecycleEvent::Repaired
    );
    assert!(has_protocol_checkpoint(
        &recovered,
        "mesh/protocol/activation/"
    ));
    assert!(has_protocol_checkpoint(&recovered, "mesh/protocol/repair/"));
    assert!(has_protocol_checkpoint(
        &recovered,
        "mesh/protocol/forwarding/tick-epoch-"
    ));
}
