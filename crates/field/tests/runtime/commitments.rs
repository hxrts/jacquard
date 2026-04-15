use super::*;

#[test]
fn route_commitments_follow_live_evidence_withdrawal() {
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
    let route = jacquard_core::MaterializedRoute::from_installation(input, installation);
    engine.state.note_tick(Tick(4));
    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        RouteCommitmentResolution::Pending,
    );

    let destination = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::Node(node(2)))
        .expect("tracked destination");
    destination.corridor_belief.delivery_support = SupportBucket::new(100);

    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        RouteCommitmentResolution::Invalidated(RouteInvalidationReason::EvidenceWithdrawn,),
    );
}

#[test]
// long-block-exception: regression keeps the service commitment floor and
// pending-resolution assertions in one route setup.
fn degraded_service_route_commitments_remain_pending_below_support_floor() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination_id = DestinationId::Service(ServiceId(vec![8; 16]));
    let state = engine.state.upsert_destination_interest(
        &destination_id,
        DestinationInterestClass::Transit,
        Tick(4),
    );
    state.posterior.top_corridor_mass = SupportBucket::new(420);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(720);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::ForwardPropagated;
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(260);
    state.corridor_belief.retention_affinity = SupportBucket::new(320);
    state.pending_forward_evidence = vec![crate::summary::ForwardPropagatedEvidence {
        from_neighbor: node(2),
        summary: super::summary_for_destination(
            state,
            topology.value.epoch,
            Tick(4),
            &destination_id,
        ),
        observed_at_tick: Tick(4),
    }];

    let objective = RoutingObjective {
        destination: destination_id.clone(),
        ..sample_objective(node(2))
    };
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
    let route = jacquard_core::MaterializedRoute::from_installation(input, installation);
    engine.state.note_tick(Tick(5));

    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.bootstrap_class = FieldBootstrapClass::Bootstrap;
        active.continuity_band = FieldContinuityBand::DegradedSteady;
    }
    let destination = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::Service(vec![8; 16]))
        .expect("tracked service destination");
    destination.corridor_belief.delivery_support = SupportBucket::new(145);

    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        RouteCommitmentResolution::Pending,
    );
}

#[test]
fn discovery_node_route_commitments_remain_pending_with_viable_continuation() {
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
    let installation = engine
        .materialize_route(input.clone())
        .expect("installation");
    let route = jacquard_core::MaterializedRoute::from_installation(input, installation);
    engine.state.note_tick(Tick(5));
    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.continuation_neighbors = vec![node(2), node(3)];
        active.bootstrap_class = FieldBootstrapClass::Bootstrap;
        active.continuity_band = FieldContinuityBand::DegradedSteady;
    }
    let destination = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::Node(node(2)))
        .expect("tracked destination");
    destination.corridor_belief.delivery_support = SupportBucket::new(130);
    destination.corridor_belief.retention_affinity = SupportBucket::new(220);
    destination.posterior.predicted_observation_class =
        crate::state::ObservationClass::ForwardPropagated;
    destination
        .pending_forward_evidence
        .push(crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(3),
            summary: super::summary_for_destination(
                destination,
                topology.value.epoch,
                Tick(5),
                &objective.destination,
            ),
            observed_at_tick: Tick(5),
        });

    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        RouteCommitmentResolution::Pending,
    );
}

#[test]
// long-block-exception: regression keeps the multi-branch service
// commitment viability setup and assertions together.
fn service_route_commitments_remain_pending_with_multiple_viable_branches() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination_id = DestinationId::Service(ServiceId(vec![6; 16]));
    let state = engine.state.upsert_destination_interest(
        &destination_id,
        DestinationInterestClass::Transit,
        Tick(4),
    );
    state.posterior.top_corridor_mass = SupportBucket::new(440);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(740);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::ForwardPropagated;
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(260);
    state.corridor_belief.retention_affinity = SupportBucket::new(320);
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(2),
        net_value: SupportBucket::new(260),
        downstream_support: SupportBucket::new(180),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(3),
        net_value: SupportBucket::new(250),
        downstream_support: SupportBucket::new(170),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.pending_forward_evidence = vec![
        crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(2),
            summary: super::summary_for_destination(
                state,
                topology.value.epoch,
                Tick(4),
                &destination_id,
            ),
            observed_at_tick: Tick(4),
        },
        crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(3),
            summary: super::summary_for_destination(
                state,
                topology.value.epoch,
                Tick(4),
                &destination_id,
            ),
            observed_at_tick: Tick(4),
        },
    ];

    let objective = RoutingObjective {
        destination: destination_id.clone(),
        ..sample_objective(node(2))
    };
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
    let route = jacquard_core::MaterializedRoute::from_installation(input, installation);
    engine.state.note_tick(Tick(5));
    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.continuation_neighbors = vec![node(2), node(3)];
        active.bootstrap_class = FieldBootstrapClass::Bootstrap;
        active.continuity_band = FieldContinuityBand::Bootstrap;
    }
    let destination = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::Service(vec![6; 16]))
        .expect("tracked service destination");
    destination.corridor_belief.delivery_support = SupportBucket::new(130);

    assert_eq!(
        engine.route_commitments(&route)[0].resolution,
        RouteCommitmentResolution::Pending,
    );
}
