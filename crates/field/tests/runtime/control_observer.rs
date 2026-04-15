use super::*;

#[test]
fn maintenance_enters_hold_fallback_under_retention_pressure() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    engine.state.posture.current = crate::state::RoutingPosture::RetentionBiased;
    engine.state.controller.congestion_price = crate::state::EntropyBucket::new(900);
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
    if let Some(active) = engine.active_routes.get_mut(&route_id) {
        active.witness_detail.posture = crate::state::RoutingPosture::RetentionBiased;
    }
    let result = engine
        .maintain_route(
            &materialized.identity,
            &mut materialized.runtime,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("maintenance");
    assert_eq!(
        result.outcome,
        RouteMaintenanceOutcome::HoldFallback {
            trigger: RouteMaintenanceTrigger::CapacityExceeded,
            retained_object_count: jacquard_core::HoldItemCount(1),
        }
    );
}

#[test]
fn weak_corridor_support_prefers_hold_fallback_before_failure() {
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
fn summary_transmission_heartbeats_each_tick_for_active_destinations() {
    let mut engine = seeded_engine();
    assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(4)));
    assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(5)));
    assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(6)));
}

#[test]
fn observer_refresh_is_incremental_and_sparse_under_load() {
    let topology = supported_topology();
    let mut engine = FieldEngine::new(node(1), NoopTransport, ());
    for index in 0..MAX_ACTIVE_DESTINATIONS {
        let destination = DestinationId::Node(node(u8::try_from(index + 2).unwrap()));
        let state = engine.state.upsert_destination_interest(
            &destination,
            DestinationInterestClass::Transit,
            Tick(u64::try_from(index + 1).unwrap()),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(u16::try_from(900 - index).unwrap());
        state.corridor_belief.delivery_support =
            SupportBucket::new(u16::try_from(800 - index).unwrap());
    }

    assert!(engine.refresh_destination_observers(&topology.value, Tick(10)));
    let refreshed = engine
        .state
        .destinations
        .values()
        .filter(|state| state.observer_cache.last_updated_at == Some(Tick(10)))
        .count();
    assert_eq!(refreshed, MAX_ACTIVE_DESTINATIONS);

    assert!(!engine.refresh_destination_observers(&topology.value, Tick(11)));
    assert!(engine.refresh_destination_observers(&topology.value, Tick(12)));
}

#[test]
fn observer_refresh_consumes_pending_evidence_once() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    engine.record_forward_summary(
        &DestinationId::Node(node(2)),
        node(2),
        crate::engine::FieldForwardSummaryObservation::new(RouteEpoch(4), Tick(4), 850, 1, 2),
    );
    engine.record_reverse_feedback(&DestinationId::Node(node(2)), node(2), 900, Tick(4));

    let destination = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::Node(node(2)))
        .expect("destination before refresh");
    assert_eq!(destination.pending_forward_evidence.len(), 1);
    assert_eq!(destination.pending_reverse_feedback.len(), 1);

    assert!(engine.refresh_destination_observers(&topology.value, Tick(4)));
    let destination = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::Node(node(2)))
        .expect("destination after refresh");
    assert!(destination.pending_forward_evidence.is_empty());
    assert!(destination.pending_reverse_feedback.is_empty());

    assert!(engine.refresh_destination_observers(&topology.value, Tick(5)));
    assert!(!engine.refresh_destination_observers(&topology.value, Tick(6)));
}

#[test]
fn evidence_accumulates_across_ticks_and_changes_observer_state() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    engine.record_forward_summary(
        &DestinationId::Node(node(2)),
        node(2),
        crate::engine::FieldForwardSummaryObservation::new(RouteEpoch(4), Tick(4), 650, 1, 2),
    );
    assert!(engine.refresh_destination_observers(&topology.value, Tick(4)));
    let after_forward = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::Node(node(2)))
        .expect("destination after forward evidence")
        .posterior
        .clone();
    assert_eq!(
        after_forward.predicted_observation_class,
        crate::state::ObservationClass::ForwardPropagated,
    );

    engine.record_reverse_feedback(&DestinationId::Node(node(2)), node(2), 900, Tick(5));
    assert!(engine.refresh_destination_observers(&topology.value, Tick(5)));
    let after_reverse = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::Node(node(2)))
        .expect("destination after reverse feedback")
        .posterior
        .clone();
    assert_eq!(
        after_reverse.predicted_observation_class,
        crate::state::ObservationClass::ReverseValidated,
    );
    assert!(after_reverse.top_corridor_mass.value() >= after_forward.top_corridor_mass.value());
}
