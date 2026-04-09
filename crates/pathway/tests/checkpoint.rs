//! Integration tests for pathway checkpoint storage and recovery.
//!
//! The pathway engine persists route runtime state and protocol checkpoints
//! through the `StorageEffects` surface so that a restarted engine can
//! restore active routes without losing buffered payloads, repair counters,
//! or handoff state. These tests verify three invariants:
//!
//! 1. A basic active route round-trips across an engine restart with the same
//!    `ActiveRoute` value visible through `active_route`.
//! 2. Richer runtime sub-states (partition buffering, repair, handoff) are
//!    faithfully preserved through checkpoint storage and restored on the
//!    recovered engine.
//! 3. Protocol-layer checkpoints (activation, repair, forwarding tick) are
//!    written under their canonical `pathway/protocol/` key prefixes and
//!    survive a storage round-trip without hidden runtime state leaking across
//!    the restart boundary.

mod common;

use common::{
    engine::{activate_route, build_engine, lease},
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{NodeId, Tick},
    RouterManagedEngine, RoutingEngine,
};

fn has_protocol_checkpoint(engine: &common::engine::TestEngine, prefix: &str) -> bool {
    engine.effects.storage_keys().iter().any(|key| {
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
        .active_route(&identity.stamp.route_id)
        .expect("active route present");
    let stored_bytes = original.effects.storage_clone();

    let mut recovered = build_engine();
    recovered.effects.replace_storage(stored_bytes);

    assert_eq!(
        recovered
            .checkpointed_topology_epoch()
            .expect("load topology epoch"),
        Some(topology.value.epoch)
    );

    assert!(recovered
        .restore_route_runtime_for_router(&identity.stamp.route_id)
        .expect("restore checkpointed route"));
    assert_eq!(
        recovered
            .active_route(&identity.stamp.route_id)
            .expect("restored route present"),
        original_active_route
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
        .forward_payload_for_router(&identity.stamp.route_id, b"checkpointed payload")
        .expect("buffer retained payload");
    original
        .maintain_route(
            &identity,
            &mut runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("record handoff state");

    let original_active_route = original
        .active_route(&identity.stamp.route_id)
        .expect("active route present");
    let stored_bytes = original.effects.storage_clone();

    let mut recovered = build_engine();
    recovered.effects.replace_storage(stored_bytes);

    assert!(recovered
        .restore_route_runtime_for_router(&identity.stamp.route_id)
        .expect("restore checkpointed route"));
    assert_eq!(
        recovered
            .active_route(&identity.stamp.route_id)
            .expect("checkpointed route present"),
        original_active_route
    );
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
        "pathway/protocol/activation/"
    ));
    assert!(has_protocol_checkpoint(
        &original,
        "pathway/protocol/repair/"
    ));
    assert!(has_protocol_checkpoint(
        &original,
        "pathway/protocol/forwarding/tick-epoch-"
    ));

    let stored_bytes = original.effects.storage_clone();
    let mut recovered = build_engine();
    recovered.effects.replace_storage(stored_bytes);

    assert!(recovered
        .restore_route_runtime_for_router(&identity.stamp.route_id)
        .expect("restore checkpointed route"));
    assert!(has_protocol_checkpoint(
        &recovered,
        "pathway/protocol/activation/"
    ));
    assert!(has_protocol_checkpoint(
        &recovered,
        "pathway/protocol/repair/"
    ));
    assert!(has_protocol_checkpoint(
        &recovered,
        "pathway/protocol/forwarding/tick-epoch-"
    ));
}
