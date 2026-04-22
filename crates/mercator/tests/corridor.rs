use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Limit, NodeId, Observation, OperatingMode,
    OriginAuthenticationClass, PublicationId, RatioPermille, RouteEpoch, RouteHandle,
    RouteIdentityStamp, RouteLease, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteServiceKind, RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick,
    TimeWindow, TransportKind,
};
use jacquard_host_support::opaque_endpoint;
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_mercator::{
    corridor::{plan_corridor, MercatorPlanningOutcome},
    evidence::{MercatorEvidenceGraph, MercatorEvidenceMeta, MercatorLinkEvidence},
    MercatorEngine, MercatorEngineConfig, MercatorEvidenceBounds, MERCATOR_ENGINE_ID,
};
use jacquard_traits::{RouterManagedEngine, RoutingEngine, RoutingEnginePlanner};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(256))
}

fn mercator_node(byte: u8) -> jacquard_core::Node {
    NodePreset::route_capable(
        NodePresetOptions::new(
            NodeIdentity::new(node(byte), ControllerId([byte; 32])),
            endpoint(byte),
            Tick(1),
        ),
        &MERCATOR_ENGINE_ID,
    )
    .build()
}

fn topology(node_bytes: &[u8], link_bytes: &[(u8, u8)]) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: node_bytes
                .iter()
                .copied()
                .map(|byte| (node(byte), mercator_node(byte)))
                .collect(),
            links: link_bytes
                .iter()
                .copied()
                .map(|(from, to)| {
                    (
                        (node(from), node(to)),
                        LinkPreset::active(LinkPresetOptions::new(endpoint(to), Tick(1))).build(),
                    )
                })
                .collect(),
            environment: Environment {
                reachable_neighbor_count: u32::try_from(link_bytes.len()).unwrap_or(u32::MAX),
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::None,
        protection_floor: RouteProtectionClass::None,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
        latency_budget_ms: Limit::Bounded(DurationMs(100)),
        protection_priority: jacquard_core::PriorityPoints(1),
        connectivity_priority: jacquard_core::PriorityPoints(1),
    }
}

fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::None,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: OperatingMode::SparseLowPower,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

fn materialization_input(
    admission: jacquard_core::RouteAdmission,
    route_id: jacquard_core::RouteId,
) -> jacquard_core::RouteMaterializationInput {
    let lease = RouteLease {
        owner_node_id: node(1),
        lease_epoch: RouteEpoch(1),
        valid_for: TimeWindow::new(Tick(1), Tick(9)).expect("lease window"),
    };
    jacquard_core::RouteMaterializationInput {
        handle: RouteHandle {
            stamp: RouteIdentityStamp {
                route_id,
                topology_epoch: lease.lease_epoch,
                materialized_at_tick: lease.valid_for.start_tick(),
                publication_id: PublicationId([9; 16]),
            },
        },
        admission,
        lease,
    }
}

fn link_evidence(from: u8, to: u8, confidence: u16, order: u64) -> MercatorLinkEvidence {
    MercatorLinkEvidence {
        from: node(from),
        to: node(to),
        bidirectional_confidence: confidence,
        asymmetric_penalty: 0,
        meta: MercatorEvidenceMeta::new(
            RouteEpoch(1),
            Tick(1),
            DurationMs(1_000),
            jacquard_core::OrderStamp(order),
        ),
    }
}

#[test]
fn corridor_connected_low_loss_produces_one_router_candidate() {
    let topology = topology(&[1, 2, 3], &[(1, 2), (2, 3)]);
    let mut engine = MercatorEngine::new(node(1));
    let objective = objective(node(3));
    let profile = profile();

    let candidates = engine.candidate_routes(&objective, &profile, &topology);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].summary.engine, MERCATOR_ENGINE_ID);
    assert_eq!(
        candidates[0].summary.protection,
        RouteProtectionClass::LinkProtected
    );
    assert_eq!(candidates[0].summary.hop_count_hint.value_or(0), 2);
    let admission = engine
        .admit_route(&objective, &profile, candidates[0].clone(), &topology)
        .expect("admission");
    let installation = engine
        .materialize_route(materialization_input(admission, candidates[0].route_id))
        .expect("materialization");

    assert_eq!(
        installation.last_lifecycle_event,
        jacquard_core::RouteLifecycleEvent::Activated
    );
    assert_eq!(engine.active_route_count(), 1);
    engine
        .forward_payload_for_router(&candidates[0].route_id, b"payload")
        .expect("forwarding");
}

#[test]
fn corridor_preserves_alternates_privately() {
    let topology = topology(&[1, 2, 3, 4], &[(1, 2), (2, 4), (1, 3), (3, 4)]);
    let outcome = plan_corridor(
        node(1),
        &topology,
        &objective(node(4)),
        &MercatorEngineConfig::default(),
        &MercatorEvidenceGraph::new(MercatorEvidenceBounds::default()),
    );

    let MercatorPlanningOutcome::Selected(corridor) = outcome else {
        panic!("expected selected corridor");
    };
    assert_eq!(corridor.primary.path.first().copied(), Some(node(1)));
    assert_eq!(corridor.primary.path.last().copied(), Some(node(4)));
    assert_eq!(corridor.alternates.len(), 1);
}

#[test]
fn candidate_ordering_is_stable_under_input_permutation() {
    let topology = topology(&[1, 2, 3], &[]);
    let evidence = [link_evidence(1, 2, 850, 1), link_evidence(2, 3, 850, 2)];
    let mut forward = MercatorEngine::new(node(1));
    let mut reverse = MercatorEngine::new(node(1));
    for evidence in evidence {
        forward.evidence_mut().record_link_evidence(evidence);
    }
    for evidence in evidence.into_iter().rev() {
        reverse.evidence_mut().record_link_evidence(evidence);
    }

    let objective = objective(node(3));
    let profile = profile();
    let forward_candidate = forward
        .candidate_routes(&objective, &profile, &topology)
        .pop()
        .expect("forward candidate");
    let reverse_candidate = reverse
        .candidate_routes(&objective, &profile, &topology)
        .pop()
        .expect("reverse candidate");

    assert_eq!(forward_candidate.route_id, reverse_candidate.route_id);
    assert_eq!(forward_candidate.backend_ref, reverse_candidate.backend_ref);
}

#[test]
fn corridor_reports_no_candidate_and_inadmissible_outcomes() {
    let disconnected = topology(&[1, 2], &[]);
    let missing_destination = topology(&[1], &[]);
    let evidence = MercatorEvidenceGraph::new(MercatorEvidenceBounds::default());

    assert_eq!(
        plan_corridor(
            node(1),
            &disconnected,
            &objective(node(2)),
            &MercatorEngineConfig::default(),
            &evidence,
        ),
        MercatorPlanningOutcome::NoCandidate
    );
    assert_eq!(
        plan_corridor(
            node(1),
            &missing_destination,
            &objective(node(2)),
            &MercatorEngineConfig::default(),
            &evidence,
        ),
        MercatorPlanningOutcome::Inadmissible
    );
}

#[test]
fn corridor_planning_diagnostics_are_router_visible() {
    let engine = MercatorEngine::new(node(1));
    let connected = topology(&[1, 2], &[(1, 2)]);
    let disconnected = topology(&[1, 2], &[]);
    let missing_destination = topology(&[1], &[]);
    let objective = objective(node(2));
    let profile = profile();

    assert_eq!(
        engine
            .candidate_routes(&objective, &profile, &connected)
            .len(),
        1
    );
    assert!(engine
        .candidate_routes(&objective, &profile, &disconnected)
        .is_empty());
    assert!(engine
        .candidate_routes(&objective, &profile, &missing_destination)
        .is_empty());

    let snapshot = engine.router_analysis_snapshot();
    assert_eq!(snapshot.diagnostics.selected_result_rounds, 1);
    assert_eq!(snapshot.diagnostics.no_candidate_attempts, 1);
    assert_eq!(snapshot.diagnostics.inadmissible_candidate_attempts, 1);
}
