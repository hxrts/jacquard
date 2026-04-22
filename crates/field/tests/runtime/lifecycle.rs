use super::*;

#[test]
fn materialize_route_installs_private_corridor_runtime_record() {
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
    let installation = engine
        .materialize_route(materialization_input(route_id, admission))
        .expect("installation");
    assert_eq!(
        installation.last_lifecycle_event,
        RouteLifecycleEvent::Activated
    );
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.selected_neighbor, node(2));
    assert_eq!(active.continuation_neighbors, vec![node(2)]);
    assert_eq!(
        active.destination,
        crate::state::DestinationKey::Node(node(2))
    );
}

#[test]
fn materialize_route_fails_closed_for_invalid_backend_token() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let objective = sample_objective(node(2));
    let mut candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let mut admission = engine
        .admit_route(&objective, &sample_profile(), candidate.clone(), &topology)
        .expect("admission");
    admission.backend_ref.backend_route_id.0 = vec![0xff, 0x00, 0xaa];
    candidate.backend_ref.backend_route_id = admission.backend_ref.backend_route_id.clone();
    let error = engine
        .materialize_route(materialization_input(route_id, admission))
        .expect_err("invalid backend must fail");
    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
            | RouteError::Selection(RouteSelectionError::NoCandidate)
    ));
}

#[test]
fn forward_payload_uses_selected_corridor_realization() {
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
    engine
        .materialize_route(materialization_input(route_id, admission))
        .expect("installation");
    engine
        .forward_payload_for_router(&route_id, b"payload")
        .expect("forward");
    assert_eq!(engine.transport.sent_frames.len(), 1);
    assert_eq!(engine.transport.sent_frames[0].1, b"payload".to_vec());
}

#[test]
fn forward_payload_switches_realization_inside_continuation_envelope() {
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
    engine
        .materialize_route(materialization_input(route_id, admission))
        .expect("installation");
    engine.state.neighbor_endpoints.remove(&node(2));
    engine.state.neighbor_endpoints.insert(
        node(3),
        jacquard_host_support::opaque_endpoint(
            jacquard_core::TransportKind::WifiAware,
            vec![3],
            ByteCount(128),
        ),
    );
    let active = engine
        .active_routes
        .get_mut(&route_id)
        .expect("active route");
    active.continuation_neighbors = vec![node(2), node(3)];
    engine
        .forward_payload_for_router(&route_id, b"fallback")
        .expect("forward");
    assert_eq!(engine.transport.sent_frames.len(), 1);
    assert_eq!(engine.transport.sent_frames[0].1, b"fallback".to_vec());
    assert_eq!(
        engine
            .active_routes
            .get(&route_id)
            .expect("active")
            .selected_neighbor,
        node(3)
    );
}

#[test]
fn maintenance_requires_replacement_when_best_neighbor_leaves_corridor_envelope() {
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
    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::from(&DestinationId::Node(
            node(2),
        )))
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(210);
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
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::HoldFallback {
            trigger: RouteMaintenanceTrigger::LinkDegraded,
            retained_object_count: jacquard_core::HoldItemCount(1),
        }
    );
}

#[test]
fn maintenance_expands_corridor_envelope_for_close_stronger_neighbor() {
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
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.selected_neighbor, node(3));
    assert!(active.continuation_neighbors.contains(&node(3)));
    assert!(active.continuation_neighbors.contains(&node(2)));
}

#[test]
fn bootstrap_route_materialization_records_activation() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(240);
    state.corridor_belief.retention_affinity = SupportBucket::new(320);

    let objective = sample_objective(node(2));
    let candidate = engine
        .candidate_routes(&objective, &sample_profile(), &topology)
        .pop()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&objective, &sample_profile(), candidate, &topology)
        .expect("admission");
    engine
        .materialize_route(materialization_input(route_id, admission))
        .expect("installation");

    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.bootstrap_class, FieldBootstrapClass::Bootstrap);
    assert!(active.recovery.state.bootstrap_active);
    assert_eq!(
        active.recovery.state.last_bootstrap_transition,
        Some(crate::FieldBootstrapTransition::Activated)
    );
    assert_eq!(active.recovery.state.bootstrap_activation_count, 1);
}

#[test]
fn bootstrap_route_upgrades_to_steady_without_replacement() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    {
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(240);
        state.corridor_belief.retention_affinity = SupportBucket::new(320);
    }

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

    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(360);
    state.corridor_belief.retention_affinity = SupportBucket::new(360);
    state.posterior.top_corridor_mass = SupportBucket::new(340);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(720);

    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.bootstrap_class, FieldBootstrapClass::Steady);
    assert!(!active.recovery.state.bootstrap_active);
    assert_eq!(
        active.recovery.state.last_bootstrap_transition,
        Some(crate::FieldBootstrapTransition::Upgraded)
    );
    assert_eq!(active.recovery.state.bootstrap_upgrade_count, 1);
}

#[test]
// long-block-exception: regression exercises the full bootstrap upgrade
// path across repeated bridge confirmation rounds.
fn bootstrap_route_promotes_after_confirmed_bridge_streak() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    {
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(240);
        state.corridor_belief.retention_affinity = SupportBucket::new(340);
        state.posterior.top_corridor_mass = SupportBucket::new(320);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(900);
    }

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

    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.bootstrap_confirmation_streak = 1;
    }
    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.posterior.predicted_observation_class = crate::state::ObservationClass::ForwardPropagated;
    state.posterior.top_corridor_mass = SupportBucket::new(340);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(835);
    state.corridor_belief.delivery_support = SupportBucket::new(275);
    state.corridor_belief.retention_affinity = SupportBucket::new(360);
    state.publication.last_summary = Some(super::summary_for_destination(
        state,
        topology.value.epoch,
        Tick(4),
        &objective.destination,
    ));
    state.publication.last_sent_at = Some(Tick(4));

    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.bootstrap_class, FieldBootstrapClass::Steady);
    assert_eq!(
        active.recovery.state.last_bootstrap_transition,
        Some(crate::FieldBootstrapTransition::Upgraded)
    );
    assert_eq!(active.recovery.state.bootstrap_upgrade_count, 1);
}

#[test]
fn bootstrap_route_withdrawal_is_recorded_before_failure() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    {
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(240);
        state.corridor_belief.retention_affinity = SupportBucket::new(320);
    }

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

    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(120);

    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");

    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded)
    );
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert!(!active.recovery.state.bootstrap_active);
    assert_eq!(
        active.recovery.state.last_bootstrap_transition,
        Some(crate::FieldBootstrapTransition::Withdrawn)
    );
    assert_eq!(active.recovery.state.bootstrap_withdraw_count, 1);
}

#[test]
// long-block-exception: regression keeps the degraded discovery bootstrap
// transition sequence explicit and end-to-end.
fn discovery_bootstrap_route_enters_degraded_steady_before_withdrawal() {
    let topology = supported_topology();
    let mut engine = seeded_engine().with_search_config(
        crate::FieldSearchConfig::default()
            .with_node_bootstrap_support_floor(180)
            .with_node_bootstrap_top_mass_floor(180)
            .with_node_bootstrap_entropy_ceiling(970)
            .enable_node_discovery(),
    );
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    {
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(230);
        state.corridor_belief.retention_affinity = SupportBucket::new(280);
        state.posterior.top_corridor_mass = SupportBucket::new(220);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(900);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
    }

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

    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(200);
    state.corridor_belief.retention_affinity = SupportBucket::new(270);
    state.posterior.top_corridor_mass = SupportBucket::new(190);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(920);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::ForwardPropagated;

    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");

    assert_ne!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded)
    );
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.bootstrap_class, FieldBootstrapClass::Bootstrap);
    assert_eq!(active.continuity_band, FieldContinuityBand::DegradedSteady);
    assert_ne!(
        active.recovery.state.last_bootstrap_transition,
        Some(crate::FieldBootstrapTransition::Withdrawn)
    );
}

#[test]
// long-block-exception: regression keeps the degraded steady-state node
// retention path explicit across observer and maintenance updates.
fn discovery_node_route_stays_degraded_steady_instead_of_withdrawing() {
    let topology = supported_topology();
    let mut engine = seeded_engine().with_search_config(
        crate::FieldSearchConfig::default()
            .with_node_bootstrap_support_floor(180)
            .with_node_bootstrap_top_mass_floor(180)
            .with_node_bootstrap_entropy_ceiling(970)
            .enable_node_discovery(),
    );
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    {
        let state = engine
            .state
            .destinations
            .get_mut(&destination)
            .expect("destination");
        state.corridor_belief.delivery_support = SupportBucket::new(230);
        state.corridor_belief.retention_affinity = SupportBucket::new(280);
        state.posterior.top_corridor_mass = SupportBucket::new(220);
        state.posterior.usability_entropy = crate::state::EntropyBucket::new(900);
        state.posterior.predicted_observation_class =
            crate::state::ObservationClass::ForwardPropagated;
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(3),
            net_value: SupportBucket::new(240),
            downstream_support: SupportBucket::new(170),
            expected_hop_band: HopBand::new(2, 4),
            freshness: Tick(4),
        });
    }

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

    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.bootstrap_class = FieldBootstrapClass::Bootstrap;
        active.continuity_band = FieldContinuityBand::DegradedSteady;
        active.continuation_neighbors = vec![node(2), node(3)];
    }
    engine.state.note_tick(Tick(6));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(150);
    state.corridor_belief.retention_affinity = SupportBucket::new(190);
    state.posterior.top_corridor_mass = SupportBucket::new(170);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(940);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::ForwardPropagated;
    state
        .pending_forward_evidence
        .push(crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(3),
            summary: super::summary_for_destination(
                state,
                topology.value.epoch,
                Tick(6),
                &objective.destination,
            ),
            observed_at_tick: Tick(6),
        });

    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance");

    assert_ne!(
        result.outcome,
        RouteMaintenanceOutcome::Failed(RouteMaintenanceFailure::CapacityExceeded)
    );
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.continuity_band, FieldContinuityBand::DegradedSteady);
    assert_ne!(
        active.recovery.state.last_bootstrap_transition,
        Some(crate::FieldBootstrapTransition::Withdrawn)
    );
}

#[test]
// long-block-exception: regression keeps observer refresh synthesis and
// continuation carry-forward assertions in one fixture.
fn discovery_node_observer_refresh_synthesizes_carry_forward_from_active_route() {
    let topology = supported_topology();
    let mut engine = seeded_engine().with_search_config(
        crate::FieldSearchConfig::default()
            .with_node_bootstrap_support_floor(180)
            .with_node_bootstrap_top_mass_floor(180)
            .with_node_bootstrap_entropy_ceiling(970)
            .enable_node_discovery(),
    );
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

    engine.state.neighbor_endpoints.insert(
        node(3),
        jacquard_host_support::opaque_endpoint(
            jacquard_core::TransportKind::WifiAware,
            vec![3],
            ByteCount(128),
        ),
    );
    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.bootstrap_class = FieldBootstrapClass::Bootstrap;
        active.continuity_band = FieldContinuityBand::DegradedSteady;
        active.continuation_neighbors = vec![node(2), node(3)];
        active.corridor_envelope.delivery_support = SupportBucket::new(260);
        active.corridor_envelope.retention_affinity = SupportBucket::new(240);
    }
    {
        let destination = engine
            .state
            .destinations
            .get_mut(&crate::state::DestinationKey::from(&DestinationId::Node(
                node(2),
            )))
            .expect("destination");
        destination.frontier = crate::state::ContinuationFrontier::default();
        destination.pending_forward_evidence.clear();
        destination.publication.last_summary = Some(super::summary_for_destination(
            destination,
            topology.value.epoch,
            Tick(6),
            &objective.destination,
        ));
        destination.publication.last_sent_at = Some(Tick(6));
        destination.corridor_belief.delivery_support = SupportBucket::new(230);
        destination.corridor_belief.retention_affinity = SupportBucket::new(220);
    }

    let changed = engine.refresh_destination_observers(&topology.value, Tick(7));
    assert!(
        changed,
        "expected observer refresh to synthesize node carry-forward evidence"
    );
    let destination = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::from(&DestinationId::Node(
            node(2),
        )))
        .expect("destination");
    let frontier_neighbors = destination
        .frontier
        .as_slice()
        .iter()
        .map(|entry| entry.neighbor_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        frontier_neighbors.contains(&node(2)) || frontier_neighbors.contains(&node(3)),
        "expected synthesized node carry-forward frontier entries, got {:?}",
        frontier_neighbors
    );
}

#[test]
fn steady_route_enters_degraded_band_before_bootstrap_collapse() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination = crate::state::DestinationKey::from(&DestinationId::Node(node(2)));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(420);
    state.corridor_belief.retention_affinity = SupportBucket::new(380);
    state.posterior.top_corridor_mass = SupportBucket::new(360);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(680);
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

    engine.state.note_tick(Tick(5));
    let state = engine
        .state
        .destinations
        .get_mut(&destination)
        .expect("destination");
    state.corridor_belief.delivery_support = SupportBucket::new(330);
    state.corridor_belief.retention_affinity = SupportBucket::new(300);
    state.posterior.top_corridor_mass = SupportBucket::new(260);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(810);

    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
    let active = engine.active_routes.get(&route_id).expect("active route");
    assert_eq!(active.bootstrap_class, FieldBootstrapClass::Steady);
    assert_eq!(active.continuity_band, FieldContinuityBand::DegradedSteady);
    assert_eq!(
        active.recovery.state.last_continuity_transition,
        Some(crate::recovery::FieldContinuityTransition::EnteredDegradedSteady)
    );
    assert_eq!(active.recovery.state.degraded_steady_entry_count, 1);
}
