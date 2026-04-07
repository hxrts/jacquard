//! Integration tests for mesh checkpoint storage and recovery.

mod common;

use jacquard_traits::jacquard_core::{NodeId, Tick};

use common::engine::{activate_route, build_engine, lease};
use common::fixtures::sample_configuration;

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
