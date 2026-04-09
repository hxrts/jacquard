//! Integration tests for the choreography-backed pathway runtime entry points.
//!
//! These tests verify that the router-facing runtime API still behaves the same
//! while materialization, maintenance, forwarding, and tick progress now route
//! protocol-side sequencing through the pathway choreography guest runtime.

mod common;

use common::{
    engine::{activate_route, build_engine, lease, tick_context},
    fixtures::sample_configuration,
};
use jacquard_traits::{
    jacquard_core::{NodeId, RouteMaintenanceTrigger, Tick},
    RouterManagedEngine, RoutingEngine,
};

fn has_protocol_checkpoint(engine: &common::engine::TestEngine, prefix: &str) -> bool {
    engine.effects.storage_keys().iter().any(|key| {
        std::str::from_utf8(key).is_ok_and(|value| value.starts_with(prefix))
    })
}

#[test]
fn materialization_records_activation_protocol_checkpoint() {
    let mut engine = build_engine();
    let topology = sample_configuration();

    let _ = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );

    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/activation/"
    ));
}

#[test]
fn maintenance_records_repair_and_handoff_protocol_checkpoints() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );

    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("repair maintenance");
    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PolicyShift,
        )
        .expect("handoff maintenance");

    assert!(has_protocol_checkpoint(&engine, "pathway/protocol/repair/"));
    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/handoff/"
    ));
}

#[test]
fn forwarding_and_partition_hold_use_protocol_runtime_paths() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );

    engine
        .forward_payload_for_router(&identity.stamp.route_id, b"frame")
        .expect("forward payload");
    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("enter partition mode");
    engine
        .forward_payload_for_router(&identity.stamp.route_id, b"held")
        .expect("retain payload while partitioned");

    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/forwarding/"
    ));
    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/hold-replay/"
    ));
}

#[test]
fn engine_tick_records_tick_and_cooperative_protocol_checkpoints() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, mut runtime) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );

    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("enter partition mode");

    engine
        .engine_tick(&tick_context(&topology))
        .expect("engine tick");

    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/forwarding/tick-epoch-"
    ));
    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/neighbor-advertisement/tick-epoch-"
    ));
    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/route-export/"
    ));
    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/anti-entropy/"
    ));
}

#[test]
fn teardown_clears_route_scoped_protocol_checkpoints_but_keeps_tick_state() {
    let mut engine = build_engine();
    let topology = sample_configuration();
    let (identity, _) = activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );

    engine.teardown(&identity.stamp.route_id);

    assert!(!has_protocol_checkpoint(
        &engine,
        "pathway/protocol/activation/activation-"
    ));
    assert!(!has_protocol_checkpoint(
        &engine,
        "pathway/protocol/route-export/"
    ));
    assert!(!has_protocol_checkpoint(
        &engine,
        "pathway/protocol/anti-entropy/"
    ));
    assert!(has_protocol_checkpoint(
        &engine,
        "pathway/protocol/forwarding/tick-epoch-"
    ));
}
