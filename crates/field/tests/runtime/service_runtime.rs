use super::*;

#[test]
fn service_forward_evidence_uses_bounded_carry_forward() {
    let now_tick = Tick(5);
    let mut state = crate::state::DestinationFieldState::new(
        crate::state::DestinationKey::Service(vec![9, 9]),
        Tick(1),
    );
    state.frontier = state.frontier.insert(NeighborContinuation {
        neighbor_id: node(2),
        net_value: SupportBucket::new(420),
        downstream_support: SupportBucket::new(360),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.publication.record(
        FieldSummary {
            destination: SummaryDestinationKey::from(&DestinationId::Service(ServiceId(vec![
                9, 9,
            ]))),
            topology_epoch: RouteEpoch(4),
            freshness_tick: Tick(4),
            hop_band: HopBand::new(1, 2),
            delivery_support: SupportBucket::new(180),
            congestion_penalty: crate::state::EntropyBucket::new(50),
            retention_support: SupportBucket::new(220),
            uncertainty_penalty: crate::state::EntropyBucket::new(500),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Medium,
        },
        Tick(4),
    );

    let input = forward_evidence_for_observer(&state, now_tick);

    assert!(input.synthesized);
    assert!(input.service_carry_forward);
    assert_eq!(input.evidence.len(), 1);
    let carried = input.evidence.first().expect("service evidence");
    assert_eq!(carried.from_neighbor, node(2));
    assert!(carried.summary.delivery_support.value() >= 260);
    assert!(carried.summary.retention_support.value() >= 340);
}

#[test]
fn service_forward_evidence_keeps_multiple_branches_alive() {
    let now_tick = Tick(5);
    let mut state = crate::state::DestinationFieldState::new(
        crate::state::DestinationKey::Service(vec![7, 7]),
        Tick(1),
    );
    state.frontier = state.frontier.insert(NeighborContinuation {
        neighbor_id: node(2),
        net_value: SupportBucket::new(520),
        downstream_support: SupportBucket::new(420),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(3),
        net_value: SupportBucket::new(470),
        downstream_support: SupportBucket::new(360),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.publication.record(
        FieldSummary {
            destination: SummaryDestinationKey::from(&DestinationId::Service(ServiceId(vec![
                7, 7,
            ]))),
            topology_epoch: RouteEpoch(4),
            freshness_tick: Tick(4),
            hop_band: HopBand::new(1, 2),
            delivery_support: SupportBucket::new(220),
            congestion_penalty: crate::state::EntropyBucket::new(50),
            retention_support: SupportBucket::new(300),
            uncertainty_penalty: crate::state::EntropyBucket::new(460),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Medium,
        },
        Tick(4),
    );

    let input = forward_evidence_for_observer(&state, now_tick);

    assert!(input.synthesized);
    assert!(input.service_carry_forward);
    assert_eq!(input.evidence.len(), 2);
    assert_eq!(input.evidence[0].from_neighbor, node(2));
    assert_eq!(input.evidence[1].from_neighbor, node(3));
    assert!(
        input.evidence[1].summary.delivery_support.value() >= 300,
        "branch support: {:?}",
        input
            .evidence
            .iter()
            .map(|e| e.summary.delivery_support.value())
            .collect::<Vec<_>>()
    );
}

#[test]
// long-block-exception: regression keeps service maintenance shift
// selection within one corridor fixture and assertion block.
fn service_maintenance_prefers_shift_within_existing_corridor() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    let destination_id = DestinationId::Service(ServiceId(vec![8; 16]));
    let state = engine.state.upsert_destination_interest(
        &destination_id,
        DestinationInterestClass::Transit,
        Tick(4),
    );
    state.posterior.top_corridor_mass = SupportBucket::new(460);
    state.posterior.usability_entropy = crate::state::EntropyBucket::new(700);
    state.posterior.predicted_observation_class = crate::state::ObservationClass::ForwardPropagated;
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(320);
    state.corridor_belief.retention_affinity = SupportBucket::new(360);
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(2),
        net_value: SupportBucket::new(420),
        downstream_support: SupportBucket::new(320),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(3),
        net_value: SupportBucket::new(830),
        downstream_support: SupportBucket::new(700),
        expected_hop_band: HopBand::new(1, 2),
        freshness: Tick(4),
    });
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: node(4),
        net_value: SupportBucket::new(920),
        downstream_support: SupportBucket::new(760),
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
    let mut route = jacquard_core::MaterializedRoute::from_installation(input, installation);
    engine.state.note_tick(Tick(5));

    {
        let active = engine.active_routes.get_mut(&route_id).expect("active");
        active.selected_neighbor = node(2);
        active.continuation_neighbors = vec![node(2), node(3)];
        active.bootstrap_class = FieldBootstrapClass::Bootstrap;
        active.continuity_band = FieldContinuityBand::DegradedSteady;
    }

    let result = engine
        .maintain_route(
            &route.identity,
            &mut route.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");

    assert_eq!(result.outcome, RouteMaintenanceOutcome::Continued);
    let active = engine.active_routes.get(&route_id).expect("active");
    assert_eq!(active.selected_neighbor, node(3));
    assert!(
        active.continuation_neighbors.contains(&node(3)),
        "continuation envelope: {:?}",
        active.continuation_neighbors
    );
    assert_eq!(
        active.recovery.state.service_retention_carry_forward_count,
        1
    );
    let exported = engine
        .replay_snapshot(std::slice::from_ref(&route))
        .exported_bundle();
    assert!(exported.runtime_search.policy_events.iter().any(|event| {
        event.gate == "CarryForward" && event.reason == "EmittedByContinuityGate"
    }));
}

#[test]
// long-block-exception: regression keeps the narrowing-bias fixture and
// stale-branch assertions together.
fn service_runtime_narrows_away_from_weak_stale_branch() {
    let mut destination = crate::state::DestinationFieldState::new(
        crate::state::DestinationKey::Service(vec![9; 16]),
        Tick(8),
    );
    destination.pending_forward_evidence = vec![
        crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(2),
            summary: super::summary_for_destination(
                &destination,
                RouteEpoch(1),
                Tick(8),
                &DestinationId::Service(ServiceId(vec![9; 16])),
            ),
            observed_at_tick: Tick(8),
        },
        crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(4),
            summary: crate::summary::FieldSummary {
                destination: crate::summary::SummaryDestinationKey::from(&DestinationId::Service(
                    ServiceId(vec![9; 16]),
                )),
                topology_epoch: RouteEpoch(1),
                freshness_tick: Tick(3),
                hop_band: HopBand::new(1, 2),
                delivery_support: SupportBucket::new(80),
                congestion_penalty: crate::state::EntropyBucket::default(),
                retention_support: SupportBucket::new(90),
                uncertainty_penalty: crate::state::EntropyBucket::new(700),
                evidence_class: crate::summary::EvidenceContributionClass::ForwardPropagated,
                uncertainty_class: crate::summary::SummaryUncertaintyClass::High,
            },
            observed_at_tick: Tick(3),
        },
    ];
    let ranked = vec![
        (
            NeighborContinuation {
                neighbor_id: node(2),
                net_value: SupportBucket::new(860),
                downstream_support: SupportBucket::new(760),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(8),
            },
            SupportBucket::new(860),
        ),
        (
            NeighborContinuation {
                neighbor_id: node(3),
                net_value: SupportBucket::new(830),
                downstream_support: SupportBucket::new(700),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(8),
            },
            SupportBucket::new(830),
        ),
        (
            NeighborContinuation {
                neighbor_id: node(4),
                net_value: SupportBucket::new(260),
                downstream_support: SupportBucket::new(100),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(3),
            },
            SupportBucket::new(260),
        ),
    ];

    let narrowed = service_runtime_continuation_neighbors(
        &ranked,
        &destination,
        node(2),
        &crate::FieldSearchConfig::default(),
    );
    assert!(narrowed.contains(&node(2)));
    assert!(narrowed.contains(&node(3)));
    assert!(
        !narrowed.contains(&node(4)),
        "continuation envelope: {narrowed:?}"
    );
}

#[test]
// long-block-exception: regression compares wide and narrow continuation
// bias settings across one full service fixture.
fn stronger_narrowing_bias_keeps_fewer_service_continuations() {
    let destination_id =
        jacquard_core::DestinationId::Service(jacquard_core::ServiceId(vec![9; 16]));
    let mut destination = crate::state::DestinationFieldState::new(
        crate::state::DestinationKey::from(&destination_id),
        Tick(8),
    );
    destination.pending_forward_evidence = vec![
        crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(3),
            summary: crate::summary::FieldSummary {
                destination: crate::summary::SummaryDestinationKey::from(&destination_id),
                topology_epoch: jacquard_core::RouteEpoch(1),
                freshness_tick: Tick(8),
                hop_band: HopBand::new(1, 2),
                delivery_support: SupportBucket::new(300),
                congestion_penalty: crate::state::EntropyBucket::new(80),
                retention_support: SupportBucket::new(280),
                uncertainty_penalty: crate::state::EntropyBucket::new(220),
                evidence_class: crate::summary::EvidenceContributionClass::ForwardPropagated,
                uncertainty_class: crate::summary::SummaryUncertaintyClass::Low,
            },
            observed_at_tick: Tick(8),
        },
        crate::summary::ForwardPropagatedEvidence {
            from_neighbor: node(4),
            summary: crate::summary::FieldSummary {
                destination: crate::summary::SummaryDestinationKey::from(&destination_id),
                topology_epoch: jacquard_core::RouteEpoch(1),
                freshness_tick: Tick(8),
                hop_band: HopBand::new(1, 2),
                delivery_support: SupportBucket::new(260),
                congestion_penalty: crate::state::EntropyBucket::new(100),
                retention_support: SupportBucket::new(240),
                uncertainty_penalty: crate::state::EntropyBucket::new(260),
                evidence_class: crate::summary::EvidenceContributionClass::ForwardPropagated,
                uncertainty_class: crate::summary::SummaryUncertaintyClass::Low,
            },
            observed_at_tick: Tick(8),
        },
    ];
    let ranked = vec![
        (
            NeighborContinuation {
                neighbor_id: node(2),
                net_value: SupportBucket::new(860),
                downstream_support: SupportBucket::new(760),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(8),
            },
            SupportBucket::new(860),
        ),
        (
            NeighborContinuation {
                neighbor_id: node(3),
                net_value: SupportBucket::new(780),
                downstream_support: SupportBucket::new(680),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(8),
            },
            SupportBucket::new(780),
        ),
        (
            NeighborContinuation {
                neighbor_id: node(4),
                net_value: SupportBucket::new(700),
                downstream_support: SupportBucket::new(620),
                expected_hop_band: HopBand::new(1, 2),
                freshness: Tick(7),
            },
            SupportBucket::new(700),
        ),
    ];

    let broad = service_runtime_continuation_neighbors(
        &ranked,
        &destination,
        node(2),
        &crate::FieldSearchConfig::default().with_service_narrowing_bias(60),
    );
    let narrow = service_runtime_continuation_neighbors(
        &ranked,
        &destination,
        node(2),
        &crate::FieldSearchConfig::default().with_service_narrowing_bias(180),
    );

    assert!(broad.len() >= narrow.len());
    assert_eq!(narrow.len(), 2);
}

#[test]
fn retention_replay_is_observational_until_explicit_evidence_intake_changes_state() {
    let topology = supported_topology();
    let mut engine = seeded_engine();
    engine.state.posture.current = crate::state::RoutingPosture::RetentionBiased;
    let destination = engine
        .state
        .destinations
        .get_mut(&crate::state::DestinationKey::Node(node(2)))
        .expect("tracked destination");
    destination.corridor_belief.delivery_support = SupportBucket::new(350);
    destination.posterior.top_corridor_mass = SupportBucket::new(320);

    let before = destination.posterior.clone();
    assert!(engine.advance_protocol_sessions(RouteEpoch(4), Tick(4)));
    assert!(engine
        .protocol_artifacts()
        .iter()
        .any(|artifact| artifact.protocol == FieldProtocolKind::RetentionReplay));
    let after_protocol_only = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::Node(node(2)))
        .expect("destination after protocol-only replay")
        .posterior
        .clone();
    assert_eq!(after_protocol_only, before);

    let replay_summary = FieldSummary {
        destination: SummaryDestinationKey::from(&DestinationId::Node(node(2))),
        topology_epoch: RouteEpoch(4),
        freshness_tick: Tick(5),
        hop_band: HopBand::new(1, 2),
        delivery_support: SupportBucket::new(900),
        congestion_penalty: crate::state::EntropyBucket::default(),
        retention_support: SupportBucket::new(600),
        uncertainty_penalty: crate::state::EntropyBucket::default(),
        evidence_class: EvidenceContributionClass::ForwardPropagated,
        uncertainty_class: SummaryUncertaintyClass::Low,
    };
    engine
        .ingest_forward_summary(node(2), replay_summary.encode(), Tick(5))
        .expect("ingest replayed summary");
    assert!(engine.refresh_destination_observers(&topology.value, Tick(5)));
    let after_ingest = engine
        .state
        .destinations
        .get(&crate::state::DestinationKey::Node(node(2)))
        .expect("destination after replay intake")
        .posterior
        .clone();
    assert!(after_ingest.top_corridor_mass.value() > after_protocol_only.top_corridor_mass.value());
}
