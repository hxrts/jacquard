use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, Configuration, ConnectivityPosture, ControllerId, DestinationId, DurationMs,
    Environment, FactSourceClass, Limit, NodeId, Observation, OperatingMode,
    OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutePartitionClass,
    RouteProtectionClass, RouteRepairClass, RouteServiceKind, RoutingEvidenceClass,
    RoutingObjective, SelectedRoutingParameters, Tick, TransportKind,
};
use jacquard_mem_link_profile::{InMemoryRetentionStore, LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_mercator::{
    custody::{
        MercatorCustodyDecision, MercatorCustodyForwardingContext, MercatorCustodySuppressionReason,
    },
    evidence::{MercatorCustodyOpportunity, MercatorEvidenceMeta, MercatorObjectiveKey},
    MercatorEngine, MercatorEngineConfig, MercatorOperationalBounds, MERCATOR_ENGINE_ID,
};
use jacquard_traits::{RetentionStore, RoutingEnginePlanner};

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

fn opportunity(score: u16, carrier: u8) -> MercatorCustodyOpportunity {
    MercatorCustodyOpportunity {
        objective: MercatorObjectiveKey::destination(DestinationId::Node(node(9))),
        carrier: node(carrier),
        improvement_score: score,
        custody_pressure: 0,
        meta: MercatorEvidenceMeta::new(
            RouteEpoch(1),
            Tick(1),
            DurationMs(1_000),
            jacquard_core::OrderStamp(u64::from(carrier)),
        ),
    }
}

fn forwarding_context(
    bridge_opportunity: bool,
    same_cluster: bool,
    energy_pressure: u16,
    leakage: u16,
) -> MercatorCustodyForwardingContext {
    MercatorCustodyForwardingContext {
        receiver_same_cluster: same_cluster,
        receiver_cluster_has_holder: same_cluster,
        receiver_is_terminal_target: false,
        bridge_opportunity,
        energy_cost_units: 7,
        energy_pressure_permille: energy_pressure,
        observer_leakage_permille: leakage,
        decided_at_tick: Tick(2),
    }
}

#[test]
fn custody_retain_uses_shared_retention_boundary_and_bounded_records() {
    let mut engine = MercatorEngine::with_config(
        node(1),
        MercatorEngineConfig {
            evidence: jacquard_mercator::MercatorEvidenceBounds {
                custody_record_count_max: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut retention = InMemoryRetentionStore::default();

    let object_id = engine
        .retain_custody_payload(
            DestinationId::Node(node(9)),
            b"first".to_vec(),
            Tick(1),
            &mut retention,
        )
        .expect("retain first payload");

    assert!(retention
        .contains_retained_payload(&object_id)
        .expect("retention lookup"));
    assert_eq!(engine.custody_record_count(), 1);
    assert_eq!(engine.diagnostics().custody_storage_bytes, 5);
    assert_eq!(
        engine
            .retain_custody_payload(
                DestinationId::Node(node(9)),
                b"second".to_vec(),
                Tick(2),
                &mut retention
            )
            .expect_err("bounded record cap rejects second payload"),
        jacquard_core::RetentionError::Full
    );
}

#[test]
fn custody_strict_improvement_and_suppression_rules_are_deterministic() {
    let mut engine = MercatorEngine::new(node(1));
    let mut retention = InMemoryRetentionStore::default();
    let object_id = engine
        .retain_custody_payload(
            DestinationId::Node(node(9)),
            b"payload".to_vec(),
            Tick(1),
            &mut retention,
        )
        .expect("retain payload");

    let first = engine.plan_custody_forwarding(
        &object_id,
        &opportunity(200, 2),
        forwarding_context(false, false, 100, 0),
    );
    assert!(matches!(first, MercatorCustodyDecision::Forward(_)));

    let repeated = engine.plan_custody_forwarding(
        &object_id,
        &opportunity(200, 3),
        forwarding_context(false, false, 100, 0),
    );
    assert_eq!(
        repeated,
        MercatorCustodyDecision::Suppressed(MercatorCustodySuppressionReason::NoStrictImprovement)
    );

    let same_cluster = engine.plan_custody_forwarding(
        &object_id,
        &opportunity(260, 4),
        forwarding_context(false, true, 100, 0),
    );
    assert_eq!(
        same_cluster,
        MercatorCustodyDecision::Suppressed(MercatorCustodySuppressionReason::SameClusterRedundant)
    );
    let diagnostics = engine.diagnostics();
    assert_eq!(diagnostics.custody_transmission_count, 1);
    assert_eq!(diagnostics.custody_suppressed_forward_count, 2);
    assert_eq!(diagnostics.custody_same_cluster_suppression_count, 1);
}

#[test]
fn custody_protected_bridge_budget_is_reserved_for_rare_bridge_opportunity() {
    let mut engine = MercatorEngine::with_config(
        node(1),
        MercatorEngineConfig {
            bounds: MercatorOperationalBounds {
                custody_copy_budget_max: 0,
                custody_protected_bridge_budget: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut retention = InMemoryRetentionStore::default();
    let object_id = engine
        .retain_custody_payload(
            DestinationId::Node(node(9)),
            b"payload".to_vec(),
            Tick(1),
            &mut retention,
        )
        .expect("retain payload");

    let ordinary = engine.plan_custody_forwarding(
        &object_id,
        &opportunity(200, 2),
        forwarding_context(false, false, 100, 0),
    );
    assert_eq!(
        ordinary,
        MercatorCustodyDecision::Suppressed(MercatorCustodySuppressionReason::CopyBudgetExhausted)
    );

    let bridge = engine.plan_custody_forwarding(
        &object_id,
        &opportunity(240, 3),
        forwarding_context(true, false, 100, 0),
    );
    assert!(matches!(
        bridge,
        MercatorCustodyDecision::Forward(intent) if intent.protected_budget_used
    ));
    assert_eq!(engine.diagnostics().custody_protected_bridge_usage_count, 1);
}

#[test]
fn diffusion_boundedness_diagnostics_track_transmission_energy_storage_and_leakage() {
    let mut engine = MercatorEngine::new(node(1));
    let mut retention = InMemoryRetentionStore::default();
    let object_id = engine
        .retain_custody_payload(
            DestinationId::Node(node(9)),
            b"payload".to_vec(),
            Tick(1),
            &mut retention,
        )
        .expect("retain payload");

    let decision = engine.plan_custody_forwarding(
        &object_id,
        &opportunity(220, 2),
        forwarding_context(true, false, 100, 300),
    );

    assert!(matches!(decision, MercatorCustodyDecision::Forward(_)));
    let diagnostics = engine.diagnostics();
    assert_eq!(diagnostics.custody_reproduction_count, 1);
    assert_eq!(diagnostics.custody_copy_budget_spent, 1);
    assert_eq!(diagnostics.custody_transmission_count, 1);
    assert_eq!(diagnostics.custody_storage_bytes, 7);
    assert_eq!(diagnostics.custody_energy_spent_units, 7);
    assert_eq!(diagnostics.custody_leakage_risk_permille, 300);
    assert_eq!(diagnostics.custody_bridge_opportunity_count, 1);
}

#[test]
fn custody_mode_does_not_publish_connected_route_without_support() {
    let topology = topology(&[1, 9], &[]);
    let objective = objective(node(9));
    let profile = profile();
    let mut engine = MercatorEngine::new(node(1));
    let mut retention = InMemoryRetentionStore::default();
    engine
        .retain_custody_payload(
            DestinationId::Node(node(9)),
            b"payload".to_vec(),
            Tick(1),
            &mut retention,
        )
        .expect("retain payload");

    let candidates = engine.candidate_routes(&objective, &profile, &topology);

    assert!(candidates.is_empty());
    assert_eq!(engine.custody_record_count(), 1);
}
