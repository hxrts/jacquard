#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use jacquard_core::{
    Belief, BleDeviceId, BleProfileId, ByteCount, ClaimStrength, CommitteeId,
    CommitteeMember, CommitteeRole, CommitteeSelection, Configuration,
    ConnectivityPosture, ControllerId, DestinationId, DiscoveryScopeId, DiversityFloor,
    DurationMs, EndpointAddress, Environment, Estimate, FactBasis, FactSourceClass,
    HealthScore, HoldItemCount, IdentityAssuranceClass, InformationSetSummary,
    InformationSummaryEncoding, Link, LinkEndpoint, LinkRuntimeState, LinkState,
    MaintenanceWorkBudget, Node, NodeId, NodeProfile, NodeRelayBudget, NodeState,
    Observation, OriginAuthenticationClass, PriorityPoints, QuorumThreshold,
    RatioPermille, RelayWorkBudget, RepairCapacitySlots, RouteMaintenanceOutcome,
    RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy, RouteServiceKind,
    RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
    RoutingPolicyInputs, SelectedRoutingParameters, ServiceDescriptor, ServiceScope,
    Tick, TimeWindow, TransportProtocol,
};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine, MESH_ENGINE_ID};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{Blake3Hashing, CommitteeSelector};

pub(crate) type TestMeshEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    InMemoryTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
>;
pub(crate) type CommitteeMeshEngine = MeshEngine<
    DeterministicMeshTopologyModel,
    InMemoryTransport,
    InMemoryRetentionStore,
    InMemoryRuntimeEffects,
    Blake3Hashing,
    AdvisoryCommitteeSelector,
>;

pub(crate) const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);
pub(crate) const PEER_NODE_ID: NodeId = NodeId([2; 32]);
pub(crate) const FAR_NODE_ID: NodeId = NodeId([3; 32]);
pub(crate) const BRIDGE_NODE_ID: NodeId = NodeId([4; 32]);

pub(crate) fn build_router(
    now: Tick,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_effects(now, InMemoryRuntimeEffects { now, ..Default::default() })
}

pub(crate) fn build_router_with_selector(
    now: Tick,
    selector: AdvisoryCommitteeSelector,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine: CommitteeMeshEngine = MeshEngine::with_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now, ..Default::default() },
        Blake3Hashing,
        selector,
    );
    let policy_engine = FixedPolicyEngine::new(profile());
    let router_effects = InMemoryRuntimeEffects { now, ..Default::default() };

    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(engine))
        .expect("register committee mesh engine");
    router
}

pub(crate) fn build_router_with_effects(
    now: Tick,
    router_effects: InMemoryRuntimeEffects,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    build_router_with_runtime_pair(
        now,
        router_effects,
        InMemoryRuntimeEffects { now, ..Default::default() },
    )
}

pub(crate) fn build_router_with_runtime_pair(
    _now: Tick,
    router_effects: InMemoryRuntimeEffects,
    engine_effects: InMemoryRuntimeEffects,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let engine: TestMeshEngine = MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        engine_effects,
        Blake3Hashing,
    );
    let policy_engine = FixedPolicyEngine::new(profile());

    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(engine))
        .expect("register mesh engine");
    router
}

pub(crate) fn build_router_with_recoverable_engine(
    now: Tick,
    router_effects: InMemoryRuntimeEffects,
    shared_state: Arc<Mutex<std::collections::BTreeSet<jacquard_core::RouteId>>>,
) -> MultiEngineRouter<FixedPolicyEngine, InMemoryRuntimeEffects> {
    let topology = sample_configuration();
    let policy_inputs = sample_policy_inputs(&topology);
    let policy_engine = FixedPolicyEngine::new(profile());
    let mut router = MultiEngineRouter::new(
        LOCAL_NODE_ID,
        policy_engine,
        router_effects,
        topology,
        policy_inputs,
    );
    router
        .register_engine(Box::new(RecoverableTestEngine::new(
            LOCAL_NODE_ID,
            shared_state,
            now,
        )))
        .expect("register recoverable test engine");
    router
}

pub(crate) fn sample_policy_inputs(
    topology: &Observation<Configuration>,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&LOCAL_NODE_ID].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

pub(crate) fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: jacquard_core::RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

pub(crate) fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: jacquard_core::RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(crate) fn sample_configuration() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (LOCAL_NODE_ID, route_capable_node(1)),
                (PEER_NODE_ID, route_capable_node(2)),
                (FAR_NODE_ID, route_capable_node(3)),
                (BRIDGE_NODE_ID, route_capable_node(4)),
            ]),
            links: BTreeMap::from([
                ((LOCAL_NODE_ID, PEER_NODE_ID), link(2, 950)),
                ((PEER_NODE_ID, FAR_NODE_ID), link(3, 875)),
                ((LOCAL_NODE_ID, BRIDGE_NODE_ID), link(4, 925)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille: RatioPermille(150),
                contention_permille: RatioPermille(120),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
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

pub(crate) struct NullCandidateEngine {
    local_node_id: jacquard_core::NodeId,
    engine_id: jacquard_core::RoutingEngineId,
}

impl NullCandidateEngine {
    pub(crate) fn new(
        local_node_id: jacquard_core::NodeId,
        engine_id: jacquard_core::RoutingEngineId,
    ) -> Self {
        Self { local_node_id, engine_id }
    }
}

impl jacquard_traits::RoutingEnginePlanner for NullCandidateEngine {
    fn engine_id(&self) -> jacquard_core::RoutingEngineId {
        self.engine_id.clone()
    }

    fn capabilities(&self) -> jacquard_core::RoutingEngineCapabilities {
        jacquard_core::RoutingEngineCapabilities {
            engine: self.engine_id.clone(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_core::RouteShapeVisibility::Opaque,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<jacquard_core::RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmissionCheck, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: jacquard_core::RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmission, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }
}

impl jacquard_traits::RoutingEngine for NullCandidateEngine {
    fn materialize_route(
        &mut self,
        _input: jacquard_core::RouteMaterializationInput,
    ) -> Result<jacquard_core::RouteInstallation, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_core::MaterializedRouteIdentity,
        _runtime: &mut jacquard_core::RouteRuntimeState,
        _trigger: jacquard_core::RouteMaintenanceTrigger,
    ) -> Result<jacquard_core::RouteMaintenanceResult, jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn teardown(&mut self, _route_id: &jacquard_core::RouteId) {}
}

impl jacquard_traits::RouterManagedEngine for NullCandidateEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        _route_id: &jacquard_core::RouteId,
        _payload: &[u8],
    ) -> Result<(), jacquard_core::RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn restore_route_runtime_for_router(
        &mut self,
        _route_id: &jacquard_core::RouteId,
    ) -> Result<bool, jacquard_core::RouteError> {
        Ok(false)
    }
}

pub(crate) struct RecoverableTestEngine {
    local_node_id: jacquard_core::NodeId,
    shared_routes: Arc<Mutex<std::collections::BTreeSet<jacquard_core::RouteId>>>,
    now: Tick,
}

impl RecoverableTestEngine {
    pub(crate) fn new(
        local_node_id: jacquard_core::NodeId,
        shared_routes: Arc<Mutex<std::collections::BTreeSet<jacquard_core::RouteId>>>,
        now: Tick,
    ) -> Self {
        Self { local_node_id, shared_routes, now }
    }

    fn engine_id_value() -> jacquard_core::RoutingEngineId {
        jacquard_core::RoutingEngineId::External {
            name: "recoverable-test".to_string(),
            contract_id: jacquard_core::RoutingEngineContractId([8; 16]),
        }
    }

    fn route_summary(
        &self,
        objective: &RoutingObjective,
    ) -> jacquard_core::RouteSummary {
        jacquard_core::RouteSummary {
            engine: Self::engine_id_value(),
            protection: objective.target_protection,
            connectivity: objective.target_connectivity,
            protocol_mix: vec![TransportProtocol::BleGatt],
            hop_count_hint: Belief::Estimated(jacquard_core::Estimate {
                value: 1,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.now,
            }),
            valid_for: TimeWindow::new(self.now, Tick(self.now.0.saturating_add(8)))
                .expect("valid candidate window"),
        }
    }

    fn route_id() -> jacquard_core::RouteId {
        jacquard_core::RouteId([7; 16])
    }
}

impl jacquard_traits::RoutingEnginePlanner for RecoverableTestEngine {
    fn engine_id(&self) -> jacquard_core::RoutingEngineId {
        Self::engine_id_value()
    }

    fn capabilities(&self) -> jacquard_core::RoutingEngineCapabilities {
        jacquard_core::RoutingEngineCapabilities {
            engine: Self::engine_id_value(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::BestEffort,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            repair_support: jacquard_core::RepairSupport::Unsupported,
            hold_support: jacquard_core::HoldSupport::Unsupported,
            decidable_admission: jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support: jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_core::RouteShapeVisibility::Opaque,
        }
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<jacquard_core::RouteCandidate> {
        vec![jacquard_core::RouteCandidate {
            summary: self.route_summary(objective),
            estimate: jacquard_core::Estimate {
                value: jacquard_core::RouteEstimate {
                    estimated_protection: objective.target_protection,
                    estimated_connectivity: objective.target_connectivity,
                    topology_epoch: topology.value.epoch,
                    degradation: jacquard_core::RouteDegradation::None,
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: self.now,
            },
            backend_ref: jacquard_core::BackendRouteRef {
                engine: Self::engine_id_value(),
                backend_route_id: jacquard_core::BackendRouteId(vec![7]),
            },
        }]
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &jacquard_core::RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmissionCheck, jacquard_core::RouteError> {
        self.admit_route(
            objective,
            &profile(),
            self.candidate_routes(objective, &profile(), topology)[0].clone(),
            topology,
        )
        .map(|admission| admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: jacquard_core::RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<jacquard_core::RouteAdmission, jacquard_core::RouteError> {
        Ok(jacquard_core::RouteAdmission {
            route_id: Self::route_id(),
            backend_ref: candidate.backend_ref,
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: jacquard_core::RouteAdmissionCheck {
                decision: jacquard_core::AdmissionDecision::Admissible,
                profile: jacquard_core::AdmissionAssumptions {
                    message_flow_assumption:
                        jacquard_core::MessageFlowAssumptionClass::BestEffort,
                    failure_model: jacquard_core::FailureModelClass::Benign,
                    runtime_envelope: jacquard_core::RuntimeEnvelopeClass::Canonical,
                    node_density_class: jacquard_core::NodeDensityClass::Sparse,
                    connectivity_regime: jacquard_core::ConnectivityRegime::Stable,
                    adversary_regime: jacquard_core::AdversaryRegime::Cooperative,
                    claim_strength: jacquard_core::ClaimStrength::ExactUnderAssumptions,
                },
                productive_step_bound: jacquard_core::Limit::Bounded(1),
                total_step_bound: jacquard_core::Limit::Bounded(1),
                route_cost: jacquard_core::RouteCost {
                    message_count_max: jacquard_core::Limit::Bounded(1),
                    byte_count_max: jacquard_core::Limit::Bounded(ByteCount(256)),
                    hop_count: 1,
                    repair_attempt_count_max: jacquard_core::Limit::Bounded(0),
                    hold_bytes_reserved: jacquard_core::Limit::Bounded(ByteCount(0)),
                    work_step_count_max: jacquard_core::Limit::Bounded(1),
                },
            },
            summary: self.route_summary(objective),
            witness: jacquard_core::RouteWitness {
                objective_protection: objective.target_protection,
                delivered_protection: objective.target_protection,
                objective_connectivity: objective.target_connectivity,
                delivered_connectivity: objective.target_connectivity,
                admission_profile: jacquard_core::AdmissionAssumptions {
                    message_flow_assumption:
                        jacquard_core::MessageFlowAssumptionClass::BestEffort,
                    failure_model: jacquard_core::FailureModelClass::Benign,
                    runtime_envelope: jacquard_core::RuntimeEnvelopeClass::Canonical,
                    node_density_class: jacquard_core::NodeDensityClass::Sparse,
                    connectivity_regime: jacquard_core::ConnectivityRegime::Stable,
                    adversary_regime: jacquard_core::AdversaryRegime::Cooperative,
                    claim_strength: jacquard_core::ClaimStrength::ExactUnderAssumptions,
                },
                topology_epoch: topology.value.epoch,
                degradation: jacquard_core::RouteDegradation::None,
            },
        })
    }
}

impl jacquard_traits::RoutingEngine for RecoverableTestEngine {
    fn materialize_route(
        &mut self,
        input: jacquard_core::RouteMaterializationInput,
    ) -> Result<jacquard_core::RouteInstallation, jacquard_core::RouteError> {
        self.shared_routes
            .lock()
            .expect("recoverable engine state")
            .insert(input.handle.route_id);
        Ok(jacquard_core::RouteInstallation {
            materialization_proof: jacquard_core::RouteMaterializationProof {
                route_id: input.handle.route_id,
                topology_epoch: input.handle.topology_epoch,
                materialized_at_tick: input.handle.materialized_at_tick,
                publication_id: input.handle.publication_id,
                witness: jacquard_core::Fact {
                    basis: FactBasis::Admitted,
                    value: input.admission.witness.clone(),
                    established_at_tick: self.now,
                },
            },
            last_lifecycle_event: jacquard_core::RouteLifecycleEvent::Activated,
            health: jacquard_core::RouteHealth {
                reachability_state: jacquard_core::ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: jacquard_core::PenaltyPoints(0),
                last_validated_at_tick: self.now,
            },
            progress: jacquard_core::RouteProgressContract {
                productive_step_count_max: jacquard_core::Limit::Bounded(1),
                total_step_count_max: jacquard_core::Limit::Bounded(1),
                last_progress_at_tick: self.now,
                state: jacquard_core::RouteProgressState::Pending,
            },
        })
    }

    fn route_commitments(
        &self,
        _route: &jacquard_core::MaterializedRoute,
    ) -> Vec<jacquard_core::RouteCommitment> {
        Vec::new()
    }

    fn maintain_route(
        &mut self,
        _identity: &jacquard_core::MaterializedRouteIdentity,
        _runtime: &mut jacquard_core::RouteRuntimeState,
        _trigger: jacquard_core::RouteMaintenanceTrigger,
    ) -> Result<jacquard_core::RouteMaintenanceResult, jacquard_core::RouteError> {
        Ok(jacquard_core::RouteMaintenanceResult {
            event: jacquard_core::RouteLifecycleEvent::Activated,
            outcome: RouteMaintenanceOutcome::Continued,
        })
    }

    fn teardown(&mut self, route_id: &jacquard_core::RouteId) {
        self.shared_routes
            .lock()
            .expect("recoverable engine state")
            .remove(route_id);
    }
}

impl jacquard_traits::RouterManagedEngine for RecoverableTestEngine {
    fn local_node_id_for_router(&self) -> jacquard_core::NodeId {
        self.local_node_id
    }

    fn forward_payload_for_router(
        &mut self,
        route_id: &jacquard_core::RouteId,
        _payload: &[u8],
    ) -> Result<(), jacquard_core::RouteError> {
        if self
            .shared_routes
            .lock()
            .expect("recoverable engine state")
            .contains(route_id)
        {
            Ok(())
        } else {
            Err(jacquard_core::RouteSelectionError::NoCandidate.into())
        }
    }

    fn restore_route_runtime_for_router(
        &mut self,
        route_id: &jacquard_core::RouteId,
    ) -> Result<bool, jacquard_core::RouteError> {
        Ok(self
            .shared_routes
            .lock()
            .expect("recoverable engine state")
            .contains(route_id))
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
        relay_work_budget_max: RelayWorkBudget(10),
        maintenance_work_budget_max: MaintenanceWorkBudget(10),
        hold_item_count_max: HoldItemCount(8),
        hold_capacity_bytes_max: ByteCount(8192),
    }
}

fn node_state() -> NodeState {
    NodeState {
        relay_budget: relay_budget(),
        available_connection_count: estimate(4),
        hold_capacity_available_bytes: estimate(ByteCount(4096)),
        information_summary: estimate(InformationSetSummary {
            summary_encoding: InformationSummaryEncoding::BloomFilter,
            item_count: estimate(HoldItemCount(4)),
            byte_count: estimate(ByteCount(2048)),
            false_positive_permille: estimate(RatioPermille(10)),
        }),
    }
}

fn relay_budget() -> Belief<NodeRelayBudget> {
    Belief::Estimated(Estimate {
        value: NodeRelayBudget {
            relay_work_budget: estimate(RelayWorkBudget(8)),
            utilization_permille: RatioPermille(100),
            retention_horizon_ms: estimate(DurationMs(500)),
        },
        confidence_permille: RatioPermille(1000),
        updated_at_tick: Tick(1),
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
pub(crate) struct AdvisoryCommitteeSelector {
    pub(crate) fail: bool,
}

impl CommitteeSelector for AdvisoryCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, jacquard_core::RouteError> {
        if self.fail {
            return Err(jacquard_core::RouteSelectionError::Inadmissible(
                jacquard_core::RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        Ok(Some(CommitteeSelection {
            committee_id: CommitteeId([4; 16]),
            topology_epoch: topology.value.epoch,
            selected_at_tick: topology.observed_at_tick,
            valid_for: TimeWindow::new(
                topology.observed_at_tick,
                Tick(topology.observed_at_tick.0.saturating_add(8)),
            )
            .expect("committee window"),
            evidence_basis: FactBasis::Observed,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold: QuorumThreshold(1),
            members: vec![CommitteeMember {
                node_id: LOCAL_NODE_ID,
                controller_id: ControllerId([1; 32]),
                role: CommitteeRole::Participant,
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
                value: jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity_slots: Belief::Estimated(Estimate {
                        value: RepairCapacitySlots(4),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                    hold_capacity_bytes: Belief::Estimated(Estimate {
                        value: ByteCount(4096),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
        })
        .collect()
}

fn ble_endpoint(device_byte: u8) -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: EndpointAddress::Ble {
            device_id: BleDeviceId(vec![device_byte]),
            profile_id: BleProfileId([device_byte; 16]),
        },
        mtu_bytes: ByteCount(256),
    }
}

fn link(device_byte: u8, confidence: u16) -> Link {
    Link {
        endpoint: ble_endpoint(device_byte),
        state: LinkState {
            state: LinkRuntimeState::Active,
            median_rtt_ms: DurationMs(40),
            transfer_rate_bytes_per_sec: Belief::Estimated(Estimate {
                value: 2048,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            stability_horizon_ms: Belief::Estimated(Estimate {
                value: DurationMs(500),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            loss_permille: RatioPermille(50),
            delivery_confidence_permille: Belief::Estimated(Estimate {
                value: RatioPermille(confidence),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
            symmetry_permille: Belief::Estimated(Estimate {
                value: RatioPermille(900),
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(1),
            }),
        },
    }
}
