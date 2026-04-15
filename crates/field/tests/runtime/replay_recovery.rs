use super::*;

// long-block-exception: this test keeps materialization, frontier shift,
// maintenance, and reduced protocol replay assertions together so the
// continuation-shift reconfiguration path stays readable as one scenario.
#[test]
fn maintenance_switches_realization_inside_corridor_envelope() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let objective = sample_objective(node(2));
    let candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&objective, &sample_profile(), candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission);
    let installation = engine
        .materialize_route(input.clone())
        .expect("installation");
    let mut materialized = jacquard_core::MaterializedRoute::from_installation(input, installation);
    let active = engine
        .active_routes
        .get_mut(&route_id)
        .expect("active route");
    active.continuation_neighbors = vec![node(2), node(3)];
    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::from(&DestinationId::Node(
            node(2),
        )))
        .expect("destination");
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(3),
        net_value: SupportBucket::new(950),
        downstream_support: SupportBucket::new(900),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(5),
    });
    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");
    assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
    assert_eq!(
        engine
            .active_routes
            .get(&route_id)
            .expect("active route")
            .selected_neighbor,
        node(3)
    );
    let recovery = &engine
        .active_routes
        .get(&route_id)
        .expect("active route")
        .recovery
        .state;
    assert_eq!(recovery.continuation_shift_count, 1);
    assert_eq!(
        recovery.last_outcome,
        Some(crate::FieldRouteRecoveryOutcome::ContinuationRetained)
    );
    let protocol_replay = engine
        .replay_snapshot(std::slice::from_ref(&materialized))
        .reduced_protocol_replay();
    assert!(protocol_replay
        .reconfigurations
        .iter()
        .any(|reconfiguration| {
            reconfiguration.prior_session.route_id == Some(route_id)
                && reconfiguration.next_session.route_id == Some(route_id)
                && reconfiguration.cause
                    == crate::choreography::FieldProtocolReconfigurationCause::ContinuationShift
                && reconfiguration.prior_owner_tag != reconfiguration.next_owner_tag
        }));
}

#[test]
// long-block-exception: replay export regression keeps the full fixture
// assembly and golden comparison in one test.
fn exported_replay_bundle_captures_continuation_shift_fixture() {
    use std::path::PathBuf;

    let topology = supported_topology();
    let mut engine = seeded_engine();
    let objective = sample_objective(node(2));
    let candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&objective, &sample_profile(), candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission);
    let installation = engine
        .materialize_route(input.clone())
        .expect("installation");
    let mut materialized = jacquard_core::MaterializedRoute::from_installation(input, installation);
    let active = engine
        .active_routes
        .get_mut(&route_id)
        .expect("active route");
    active.continuation_neighbors = vec![node(2), node(3)];
    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::from(&DestinationId::Node(
            node(2),
        )))
        .expect("destination");
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(3),
        net_value: SupportBucket::new(950),
        downstream_support: SupportBucket::new(900),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(5),
    });
    // allow-ignored-result: this fixture needs only the route-local shift side effects before exporting the replay bundle.
    let _ = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");

    let actual = engine
        .replay_snapshot(std::slice::from_ref(&materialized))
        .exported_bundle_json()
        .expect("export replay json");
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("../fixtures/replay/continuation-shift.json");
    if std::env::var_os("JACQUARD_UPDATE_FIELD_REPLAY_FIXTURES").is_some() {
        std::fs::write(&fixture_path, format!("{actual}\n"))
            .expect("write continuation shift replay fixture");
    }
    let expected =
        std::fs::read_to_string(&fixture_path).expect("read continuation shift replay fixture");
    assert_eq!(actual, expected.trim_end());
}

#[test]
fn suspend_route_runtime_captures_checkpoint_and_marks_recovery_surface() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let objective = sample_objective(node(2));
    let candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&objective, &sample_profile(), candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission);
    let installation = engine
        .materialize_route(input.clone())
        .expect("installation");
    let materialized = jacquard_core::MaterializedRoute::from_installation(input, installation);

    assert!(engine
        .suspend_route_runtime_for_recovery(&route_id)
        .expect("suspend"));

    let active = engine.active_routes.get(&route_id).expect("active route");
    assert!(active.coordination_capability.is_none());
    assert!(active.recovery.checkpoint.is_some());

    let replay = engine.replay_snapshot(std::slice::from_ref(&materialized));
    let entry = replay
        .recovery
        .entries
        .into_iter()
        .find(|entry| entry.route_id == route_id)
        .expect("recovery entry");
    assert!(entry.state.checkpoint_available);
    assert_eq!(entry.state.checkpoint_capture_count, 1);
    assert_eq!(
        entry.state.last_outcome,
        Some(crate::FieldRouteRecoveryOutcome::CheckpointStored)
    );
}

#[test]
fn restore_route_runtime_prefers_checkpoint_restore() {
    let topology = supported_topology();
    let mut engine = seeded_transport_engine();
    let objective = sample_objective(node(2));
    let candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&objective, &sample_profile(), candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission);
    let installation = engine
        .materialize_route(input.clone())
        .expect("installation");
    let materialized = jacquard_core::MaterializedRoute::from_installation(input, installation);

    engine
        .suspend_route_runtime_for_recovery(&route_id)
        .expect("suspend");
    assert!(engine
        .restore_route_runtime_for_router(&route_id)
        .expect("restore"));

    let active = engine.active_routes.get(&route_id).expect("active route");
    assert!(active.coordination_capability.is_some());
    assert!(!active.recovery.state.checkpoint_available);
    assert_eq!(active.recovery.state.checkpoint_capture_count, 1);
    assert_eq!(active.recovery.state.checkpoint_restore_count, 1);
    assert_eq!(
        active.recovery.state.last_outcome,
        Some(crate::FieldRouteRecoveryOutcome::CheckpointRestored)
    );

    let protocol_replay = engine
        .replay_snapshot(std::slice::from_ref(&materialized))
        .reduced_protocol_replay();
    assert!(protocol_replay
        .reconfigurations
        .iter()
        .any(|reconfiguration| {
            reconfiguration.prior_session.route_id == Some(route_id)
                && reconfiguration.cause
                    == crate::choreography::FieldProtocolReconfigurationCause::CheckpointRestore
        }));
}

#[test]
fn restore_route_runtime_fails_closed_for_stale_checkpoint_owner() {
    let topology = supported_topology();
    let mut engine = seeded_transport_engine();
    let objective = sample_objective(node(2));
    let candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&objective, &sample_profile(), candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission);
    engine.materialize_route(input).expect("installation");

    engine
        .suspend_route_runtime_for_recovery(&route_id)
        .expect("suspend");
    let active = engine
        .active_routes
        .get_mut(&route_id)
        .expect("active route");
    active.selected_neighbor = node(9);

    let error = engine
        .restore_route_runtime_for_router(&route_id)
        .expect_err("stale restore must fail closed");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
    let recovery = &engine
        .active_routes
        .get(&route_id)
        .expect("active route")
        .recovery
        .state;
    assert_eq!(
        recovery.last_trigger,
        Some(crate::FieldRouteRecoveryTrigger::RestoreRuntime)
    );
    assert_eq!(
        recovery.last_outcome,
        Some(crate::FieldRouteRecoveryOutcome::RecoveryFailed)
    );
}
