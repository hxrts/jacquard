use std::collections::BTreeMap;

use jacquard_core::{
    AdaptiveRoutingProfile, Belief, BleDeviceId, BleProfileId, ByteCount,
    CapabilityError, ClaimStrength, CommitteeId, CommitteeMember, CommitteeRole,
    CommitteeSelection, Configuration, ControllerId, DestinationId, DiscoveryScopeId,
    DurationMs, EndpointAddress, Environment, Estimate, FactBasis, FactSourceClass,
    HealthScore, IdentityAssuranceClass, InformationSetSummary,
    InformationSummaryEncoding, Link, LinkEndpoint, LinkRuntimeState, LinkState, Node,
    NodeId, NodeProfile, NodeRelayBudget, NodeState, Observation,
    OriginAuthenticationClass, PriorityPoints, RatioPermille, ReceiptId,
    RouteConnectivityProfile, RouteMaintenanceOutcome, RouteMaintenanceTrigger,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteReplacementPolicy, RouteSemanticHandoff, RouteServiceKind,
    RouterCanonicalMutation, RoutingEngineFallbackPolicy, RoutingEvidenceClass,
    RoutingObjective, RoutingPolicyInputs, ServiceDescriptor, ServiceScope, Tick,
    TimeWindow, TransportProtocol,
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine, MESH_ENGINE_ID};
use jacquard_mock_transport::{
    InMemoryMeshTransport, InMemoryRetentionStore, InMemoryRuntimeEffects,
};
use jacquard_router::{FixedPolicyEngine, SingleEngineRouter};
use jacquard_traits::{
    Blake3Hashing, CommitteeSelector, Router, RoutingControlPlane, RoutingDataPlane,
};

type TestMeshEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    InMemoryMeshTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
>;
type CommitteeMeshEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    InMemoryMeshTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
    AdvisoryCommitteeSelector,
>;

const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);
const PEER_NODE_ID: NodeId = NodeId([2; 32]);
const FAR_NODE_ID: NodeId = NodeId([3; 32]);
const BRIDGE_NODE_ID: NodeId = NodeId([4; 32]);

#[test]
fn activate_route_publishes_router_owned_materialized_route() {
    let mut router = build_router(Tick(2));

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("router activation");

    let stored = router
        .active_route(&route.identity.handle.route_id)
        .expect("router stores active route");
    assert_eq!(stored.identity.handle, route.identity.handle);
    assert_eq!(
        stored.identity.materialization_proof.publication_id,
        route.identity.handle.publication_id,
    );
}

#[test]
fn route_commitments_use_router_published_route_identity() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("router activation");

    let commitments = router
        .route_commitments(&route.identity.handle.route_id)
        .expect("route commitments");

    assert!(!commitments.is_empty());
    assert!(commitments.iter().all(|commitment| commitment.route_binding
        == jacquard_core::RouteBinding::Bound(route.identity.handle.route_id)));
}

#[test]
fn reselect_route_replaces_router_published_route() {
    let mut router = build_router(Tick(2));
    let first = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("first activation");

    let replacement = router
        .reselect_route(
            &first.identity.handle.route_id,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("reselection");

    assert_ne!(
        first.identity.handle.publication_id,
        replacement.identity.handle.publication_id,
    );
    assert!(router
        .active_route(&replacement.identity.handle.route_id)
        .is_some());
}

#[test]
fn maintain_route_dispatches_to_engine_via_control_plane() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");

    let result = router
        .maintain_route(
            &route.identity.handle.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("maintenance result");

    assert_eq!(
        result.engine_result.event,
        jacquard_core::RouteLifecycleEvent::Activated,
    );
    assert_eq!(
        result.engine_result.outcome,
        RouteMaintenanceOutcome::Continued,
    );
    assert_eq!(result.canonical_mutation, RouterCanonicalMutation::None);
}

#[test]
fn anti_entropy_tick_drives_engine_tick_without_exposing_private_state() {
    let mut router = build_router(Tick(2));

    let outcome = router.anti_entropy_tick().expect("anti-entropy tick");

    let latest_topology = router
        .engine()
        .latest_topology()
        .expect("engine should receive router-owned tick topology");
    assert_eq!(
        latest_topology.value.epoch,
        sample_configuration().value.epoch
    );
    assert_eq!(router.registered_engine_id(), MESH_ENGINE_ID);
    assert_eq!(router.registered_capabilities().engine, MESH_ENGINE_ID);
    assert_eq!(outcome.canonical_mutation, RouterCanonicalMutation::None);
}

#[test]
fn anti_entropy_tick_drives_mesh_cooperative_choreographies_through_router_cadence() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");
    let _ = router
        .maintain_route(
            &route.identity.handle.route_id,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("enter partition mode");

    let outcome = router.anti_entropy_tick().expect("anti-entropy tick");
    let stored_keys = router
        .engine()
        .runtime_effects()
        .storage
        .keys()
        .map(|key| String::from_utf8_lossy(key).into_owned())
        .collect::<Vec<_>>();

    assert_eq!(outcome.topology_epoch, sample_configuration().value.epoch);
    assert!(stored_keys
        .iter()
        .any(|key| key.starts_with("mesh/protocol/route-export/")));
    assert!(stored_keys
        .iter()
        .any(|key| key.starts_with("mesh/protocol/neighbor-advertisement/")));
    assert!(
        stored_keys.iter().any(|key| {
            key.starts_with("mesh/protocol/anti-entropy/")
                && key.contains(&format!("{:?}", route.identity.handle.route_id))
        }) || stored_keys
            .iter()
            .any(|key| key.starts_with("mesh/protocol/anti-entropy/"))
    );
}

#[test]
fn observe_route_health_reports_router_owned_observation() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");

    let observed = router
        .observe_route_health(&route.identity.handle.route_id)
        .expect("health observation");

    assert_eq!(observed.value, route.runtime.health);
    assert_eq!(observed.source_class, FactSourceClass::Local);
    assert_eq!(
        observed.evidence_class,
        RoutingEvidenceClass::AdmissionWitnessed,
    );
}

#[test]
fn mesh_only_router_rejects_duplicate_mesh_registration() {
    let mut router = build_router(Tick(2));
    let duplicate_engine = MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryMeshTransport::new(TransportProtocol::BleGatt),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now: Tick(2), ..Default::default() },
        Blake3Hashing,
    );

    let error = router
        .register_engine(Box::new(duplicate_engine))
        .expect_err("duplicate mesh engine should be rejected");

    assert_eq!(error, CapabilityError::Rejected.into());
}

#[test]
fn transfer_route_lease_updates_router_owned_lease() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");

    let handoff = RouteSemanticHandoff {
        route_id:      route.identity.handle.route_id,
        from_node_id:  LOCAL_NODE_ID,
        to_node_id:    PEER_NODE_ID,
        handoff_epoch: jacquard_core::RouteEpoch(3),
        receipt_id:    ReceiptId([9; 16]),
    };

    let transferred = router
        .transfer_route_lease(&route.identity.handle.route_id, handoff.clone())
        .expect("lease transfer");

    assert_eq!(transferred.identity.lease.owner_node_id, PEER_NODE_ID);
    assert_eq!(
        transferred.identity.lease.lease_epoch,
        handoff.handoff_epoch
    );
}

#[test]
fn failing_committee_selector_cannot_publish_canonical_route_truth() {
    let mut router =
        build_router_with_selector(Tick(2), AdvisoryCommitteeSelector { fail: true });

    let error = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect_err("selector failure must block proof-bearing activation");

    assert!(matches!(
        error,
        jacquard_core::RouteError::Selection(
            jacquard_core::RouteSelectionError::Inadmissible(_)
        )
    ));
    assert_eq!(router.active_route_count(), 0);
}

#[test]
fn activation_fails_closed_when_router_event_logging_fails() {
    let mut router = build_router_with_effects(
        Tick(2),
        InMemoryRuntimeEffects {
            now: Tick(2),
            fail_record_route_event: true,
            ..Default::default()
        },
    );

    let error = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect_err("router must fail closed when canonical event logging fails");

    assert!(matches!(
        error,
        jacquard_core::RouteError::Runtime(
            jacquard_core::RouteRuntimeError::Invalidated
        )
    ));
    assert_eq!(router.engine().active_route_count(), 0);
    assert!(router.effects().events.is_empty());
}

#[test]
fn activation_reselection_and_maintenance_are_deterministic_for_equal_inputs() {
    let mut left = build_router(Tick(2));
    let mut right = build_router(Tick(2));

    let left_route =
        Router::activate_route(&mut left, objective(DestinationId::Node(FAR_NODE_ID)))
            .expect("left activation");
    let right_route =
        Router::activate_route(&mut right, objective(DestinationId::Node(FAR_NODE_ID)))
            .expect("right activation");
    assert_eq!(left_route.identity, right_route.identity);

    let left_maintenance = left
        .maintain_route(
            &left_route.identity.handle.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("left maintenance");
    let right_maintenance = right
        .maintain_route(
            &right_route.identity.handle.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("right maintenance");
    assert_eq!(left_maintenance, right_maintenance);

    let left_reselected = left
        .reselect_route(
            &left_route.identity.handle.route_id,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("left reselection");
    let right_reselected = right
        .reselect_route(
            &right_route.identity.handle.route_id,
            RouteMaintenanceTrigger::LeaseExpiring,
        )
        .expect("right reselection");
    assert_eq!(left_reselected.identity, right_reselected.identity);
}

#[test]
fn recovery_restores_router_and_mesh_state_from_router_owned_registry() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");
    let persisted_router_effects = router.effects().clone();
    let persisted_engine_effects = router.engine().runtime_effects().clone();

    let mut recovered = build_router_with_runtime_pair(
        Tick(2),
        persisted_router_effects,
        persisted_engine_effects,
    );
    let restored_count = recovered
        .recover_checkpointed_routes()
        .expect("recover router and engine state");

    assert_eq!(restored_count, 1);
    assert!(recovered
        .active_route(&route.identity.handle.route_id)
        .is_some());
    assert!(recovered
        .engine()
        .active_route(&route.identity.handle.route_id)
        .is_some());
}

fn build_router(
    now: Tick,
) -> SingleEngineRouter<TestMeshEngine, FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_effects(now, InMemoryRuntimeEffects { now, ..Default::default() })
}

fn build_router_with_selector(
    now: Tick,
    selector: AdvisoryCommitteeSelector,
) -> SingleEngineRouter<CommitteeMeshEngine, FixedPolicyEngine, InMemoryRuntimeEffects>
{
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine = MeshEngine::with_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryMeshTransport::new(TransportProtocol::BleGatt),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
        selector,
    );
    let policy_engine = FixedPolicyEngine::new(profile());
    let router_effects = InMemoryRuntimeEffects { now, ..Default::default() };

    SingleEngineRouter::new(
        engine,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    )
}

fn build_router_with_effects(
    now: Tick,
    router_effects: InMemoryRuntimeEffects,
) -> SingleEngineRouter<TestMeshEngine, FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_runtime_pair(
        now,
        router_effects,
        InMemoryRuntimeEffects { now, ..Default::default() },
    )
}

fn build_router_with_runtime_pair(
    _now: Tick,
    router_effects: InMemoryRuntimeEffects,
    engine_effects: InMemoryRuntimeEffects,
) -> SingleEngineRouter<TestMeshEngine, FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine = MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryMeshTransport::new(TransportProtocol::BleGatt),
        InMemoryRetentionStore::default(),
        engine_effects,
        Blake3Hashing,
    );
    let policy_engine = FixedPolicyEngine::new(profile());

    SingleEngineRouter::new(
        engine,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    )
}

fn sample_policy_inputs(topology: &Observation<Configuration>) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node:                  Observation {
            value:                 topology.value.nodes[&LOCAL_NODE_ID].clone(),
            source_class:          topology.source_class,
            evidence_class:        topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick:      topology.observed_at_tick,
        },
        local_environment:           Observation {
            value:                 topology.value.environment.clone(),
            source_class:          topology.source_class,
            evidence_class:        topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick:      topology.observed_at_tick,
        },
        routing_engine_count:        1,
        median_rtt_ms:               DurationMs(40),
        loss_permille:               RatioPermille(50),
        partition_risk_permille:     RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance:          IdentityAssuranceClass::ControllerBound,
        direct_reachability_score:   HealthScore(900),
    }
}

fn profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection:            RouteProtectionClass::LinkProtected,
        selected_connectivity:          RouteConnectivityProfile {
            repair:    RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile:
            jacquard_core::DeploymentProfile::FieldPartitionTolerant,
        diversity_floor:                1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy:       RouteReplacementPolicy::Allowed,
    }
}

fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: RouteConnectivityProfile {
            repair:    RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn sample_configuration() -> Observation<Configuration> {
    Observation {
        value:                 Configuration {
            epoch:       jacquard_core::RouteEpoch(2),
            nodes:       BTreeMap::from([
                (LOCAL_NODE_ID, route_capable_node(1)),
                (PEER_NODE_ID, route_capable_node(2)),
                (FAR_NODE_ID, route_capable_node(3)),
                (BRIDGE_NODE_ID, route_capable_node(4)),
            ]),
            links:       BTreeMap::from([
                ((LOCAL_NODE_ID, PEER_NODE_ID), link(2, 950)),
                ((PEER_NODE_ID, FAR_NODE_ID), link(3, 875)),
                ((LOCAL_NODE_ID, BRIDGE_NODE_ID), link(4, 925)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille:           RatioPermille(150),
                contention_permille:      RatioPermille(120),
            },
        },
        source_class:          FactSourceClass::Local,
        evidence_class:        RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick:      Tick(2),
    }
}

fn route_capable_node(node_byte: u8) -> Node {
    let node_id = NodeId([node_byte; 32]);
    let controller_id = ControllerId([node_byte; 32]);

    Node {
        controller_id,
        profile: route_capable_profile(node_byte, node_id, controller_id),
        state: node_state(),
    }
}

fn route_capable_profile(
    node_byte: u8,
    node_id: NodeId,
    controller_id: ControllerId,
) -> NodeProfile {
    NodeProfile {
        services: route_capable_services(node_id, controller_id),
        endpoints: vec![ble_endpoint(node_byte)],
        connection_count_max: 8,
        neighbor_state_count_max: 8,
        simultaneous_transfer_count_max: 4,
        active_route_count_max: 4,
        relay_work_budget_max: 10,
        maintenance_work_budget_max: 10,
        hold_item_count_max: 8,
        hold_capacity_bytes_max: ByteCount(8192),
    }
}

fn node_state() -> NodeState {
    NodeState {
        relay_budget:                  relay_budget(),
        available_connection_count:    estimate(4),
        hold_capacity_available_bytes: estimate(ByteCount(4096)),
        information_summary:           estimate(InformationSetSummary {
            summary_encoding:        InformationSummaryEncoding::BloomFilter,
            item_count:              estimate(4),
            byte_count:              estimate(ByteCount(2048)),
            false_positive_permille: estimate(RatioPermille(10)),
        }),
    }
}

fn relay_budget() -> Belief<NodeRelayBudget> {
    Belief::Estimated(Estimate {
        value:               NodeRelayBudget {
            relay_work_budget:    estimate(8),
            utilization_permille: RatioPermille(100),
            retention_horizon_ms: estimate(DurationMs(500)),
        },
        confidence_permille: RatioPermille(1000),
        updated_at_tick:     Tick(1),
    })
}

fn estimate<T>(value: T) -> Belief<T> {
    Belief::Estimated(Estimate {
        value,
        confidence_permille: RatioPermille(1000),
        updated_at_tick: Tick(1),
    })
}

#[derive(Clone, Copy)]
struct AdvisoryCommitteeSelector {
    fail: bool,
}

impl CommitteeSelector for AdvisoryCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, jacquard_core::RouteError> {
        if self.fail {
            return Err(jacquard_core::RouteSelectionError::Inadmissible(
                jacquard_core::RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        Ok(Some(CommitteeSelection {
            committee_id:       CommitteeId([4; 16]),
            topology_epoch:     topology.value.epoch,
            selected_at_tick:   topology.observed_at_tick,
            valid_for:          TimeWindow::new(
                topology.observed_at_tick,
                Tick(topology.observed_at_tick.0.saturating_add(8)),
            )
            .expect("committee window"),
            evidence_basis:     FactBasis::Observed,
            claim_strength:     ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold:   1,
            members:            vec![CommitteeMember {
                node_id:       LOCAL_NODE_ID,
                controller_id: ControllerId([1; 32]),
                role:          CommitteeRole::Participant,
            }],
        }))
    }
}

fn route_capable_services(
    node_id: NodeId,
    controller_id: ControllerId,
) -> Vec<ServiceDescriptor> {
    let valid_for = TimeWindow::new(Tick(1), Tick(20)).expect("valid service window");
    [RouteServiceKind::Discover, RouteServiceKind::Move, RouteServiceKind::Hold]
        .into_iter()
        .map(|service_kind| ServiceDescriptor {
            provider_node_id: node_id,
            controller_id,
            service_kind,
            endpoints: vec![ble_endpoint(node_id.0[0])],
            routing_engines: vec![MESH_ENGINE_ID],
            scope: ServiceScope::Discovery(DiscoveryScopeId([7; 16])),
            valid_for,
            capacity: Belief::Estimated(Estimate {
                value:               jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity:     Belief::Estimated(Estimate {
                        value:               4,
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick:     Tick(1),
                    }),
                    hold_capacity_bytes: Belief::Estimated(Estimate {
                        value:               ByteCount(4096),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick:     Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
        })
        .collect()
}

fn ble_endpoint(device_byte: u8) -> LinkEndpoint {
    LinkEndpoint {
        protocol:  TransportProtocol::BleGatt,
        address:   EndpointAddress::Ble {
            device_id:  BleDeviceId(vec![device_byte]),
            profile_id: BleProfileId([device_byte; 16]),
        },
        mtu_bytes: ByteCount(256),
    }
}

fn link(device_byte: u8, confidence: u16) -> Link {
    Link {
        endpoint: ble_endpoint(device_byte),
        state:    LinkState {
            state: LinkRuntimeState::Active,
            median_rtt_ms: DurationMs(40),
            transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
                value:               2048,
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
            stability_horizon_ms: Belief::Estimated(Estimate {
                value:               DurationMs(500),
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
            loss_permille: RatioPermille(50),
            delivery_confidence_permille: Belief::Estimated(Estimate {
                value:               RatioPermille(confidence),
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
            symmetry_permille: Belief::Estimated(Estimate {
                value:               RatioPermille(900),
                confidence_permille: RatioPermille(1000),
                updated_at_tick:     Tick(1),
            }),
        },
    }
}
