//! Drive a stub RoutingEngine through router-owned materialization and
//! teardown.

use std::collections::BTreeMap;

use jacquard_traits::{
    jacquard_core::{
        AdmissionAssumptions, AdmissionDecision, AdversaryRegime, BackendRouteRef,
        ByteCount, ClaimStrength, CommitteeId, CommitteeMember, CommitteeRole,
        CommitteeSelection, Configuration, ConnectivityPosture, ConnectivityRegime,
        DiversityFloor, Environment, Estimate, Fact, FactBasis, FailureModelClass,
        IdentityAssuranceClass, LayerParameter, LayerParameters, Limit,
        MaterializedRoute, MaterializedRouteIdentity, MessageFlowAssumptionClass,
        NodeDensityClass, ObjectiveVsDelivered, Observation, OperatingMode,
        PublicationId, QuorumThreshold, ReachabilityState, RouteAdmission,
        RouteAdmissionCheck, RouteBinding, RouteCandidate, RouteCommitment,
        RouteCommitmentId, RouteCommitmentResolution, RouteCost, RouteDegradation,
        RouteEpoch, RouteEstimate, RouteHandle, RouteHealth, RouteId,
        RouteInstallation, RouteLease, RouteLifecycleEvent, RouteMaintenanceOutcome,
        RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationInput,
        RouteMaterializationProof, RoutePartitionClass, RouteProgressContract,
        RouteProgressState, RouteProtectionClass, RouteRepairClass,
        RouteReplacementPolicy, RouteRuntimeState, RouteServiceKind, RouteSummary,
        RouteWitness, RoutingEngineCapabilities, RoutingEngineFallbackPolicy,
        RoutingEngineId, RoutingEvidenceClass, RoutingObjective, RoutingTickChange,
        RoutingTickContext, RuntimeEnvelopeClass, SelectedRoutingParameters,
        SubstrateCandidate, SubstrateCapabilities, SubstrateLease,
        SubstrateRequirements, Tick, TimeWindow, TransportProtocol,
    },
    CommitteeCoordinatedEngine, CommitteeSelector, LayeredRoutingEngine,
    LayeredRoutingEnginePlanner, LayeringPolicyEngine, RoutingEngine,
    RoutingEnginePlanner, SubstratePlanner, SubstrateRuntime,
};

use super::common;

fn repairable_connected() -> ConnectivityPosture {
    ConnectivityPosture {
        repair: RouteRepairClass::Repairable,
        partition: RoutePartitionClass::ConnectedOnly,
    }
}

fn stub_engine_id() -> RoutingEngineId {
    RoutingEngineId::from_contract_bytes([1; 16])
}

struct StubEngine {
    route: MaterializedRoute,
    selector: Option<StubCommitteeSelector>,
}

struct StubCommitteeSelector;
struct StubSubstrateProvider {
    route: MaterializedRoute,
}

struct StubLayeringPolicyEngine {
    route: MaterializedRoute,
}

impl CommitteeSelector for StubCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, jacquard_traits::jacquard_core::RouteError>
    {
        Ok(Some(CommitteeSelection {
            committee_id: CommitteeId([5; 16]),
            topology_epoch: RouteEpoch(1),
            selected_at_tick: Tick(1),
            valid_for: TimeWindow::new(Tick(1), Tick(10))
                .expect("valid committee window"),
            evidence_basis: FactBasis::Observed,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold: QuorumThreshold(2),
            members: vec![
                CommitteeMember {
                    node_id: jacquard_traits::jacquard_core::NodeId([1; 32]),
                    controller_id: jacquard_traits::jacquard_core::ControllerId(
                        [1; 32],
                    ),
                    role: CommitteeRole::Participant,
                },
                CommitteeMember {
                    node_id: jacquard_traits::jacquard_core::NodeId([2; 32]),
                    controller_id: jacquard_traits::jacquard_core::ControllerId(
                        [2; 32],
                    ),
                    role: CommitteeRole::Witness,
                },
            ],
        }))
    }
}

impl CommitteeCoordinatedEngine for StubEngine {
    type Selector = StubCommitteeSelector;

    fn committee_selector(&self) -> Option<&Self::Selector> {
        self.selector.as_ref()
    }
}

impl RoutingEnginePlanner for StubEngine {
    fn engine_id(&self) -> RoutingEngineId {
        stub_engine_id()
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: stub_engine_id(),
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: repairable_connected(),
            repair_support: jacquard_traits::jacquard_core::RepairSupport::Supported,
            hold_support: jacquard_traits::jacquard_core::HoldSupport::Supported,
            decidable_admission:
                jacquard_traits::jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_traits::jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support:
                jacquard_traits::jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility:
                jacquard_traits::jacquard_core::RouteShapeVisibility::Explicit,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            summary: self.route.identity.admission.summary.clone(),
            estimate: Estimate {
                value: RouteEstimate {
                    estimated_protection: self
                        .route
                        .identity
                        .admission
                        .summary
                        .protection,
                    estimated_connectivity: self
                        .route
                        .identity
                        .admission
                        .summary
                        .connectivity,
                    topology_epoch: self
                        .route
                        .identity
                        .admission
                        .witness
                        .topology_epoch,
                    degradation: self.route.identity.admission.witness.degradation,
                },
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(
                    1000,
                ),
                updated_at_tick: Tick(1),
            },
            backend_ref: BackendRouteRef {
                engine: stub_engine_id(),
                backend_route_id: jacquard_traits::jacquard_core::BackendRouteId(vec![
                    1,
                ]),
            },
        }]
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.identity.admission.admission_check.clone())
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.identity.admission.clone())
    }
}

impl RoutingEngine for StubEngine {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, jacquard_traits::jacquard_core::RouteError> {
        assert_eq!(input.handle.stamp, self.route.identity.stamp);
        assert_eq!(input.lease, self.route.identity.lease);
        assert_eq!(input.admission, self.route.identity.admission);
        Ok(RouteInstallation {
            materialization_proof: self.route.identity.proof.clone(),
            last_lifecycle_event: self.route.runtime.last_lifecycle_event,
            health: self.route.runtime.health.clone(),
            progress: self.route.runtime.progress.clone(),
        })
    }

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment> {
        vec![RouteCommitment {
            commitment_id: RouteCommitmentId([8; 16]),
            operation_id: jacquard_traits::jacquard_core::RouteOperationId([6; 16]),
            route_binding: RouteBinding::Bound(route.identity.stamp.route_id),
            owner_node_id: route.identity.lease.owner_node_id,
            deadline_tick: Tick(10),
            retry_policy: jacquard_traits::jacquard_core::TimeoutPolicy {
                attempt_count_max: 1,
                initial_backoff_ms: jacquard_traits::jacquard_core::DurationMs(5),
                backoff_multiplier_permille:
                    jacquard_traits::jacquard_core::RatioPermille(1000),
                backoff_ms_max: jacquard_traits::jacquard_core::DurationMs(5),
                overall_timeout_ms: jacquard_traits::jacquard_core::DurationMs(5),
            },
            resolution: RouteCommitmentResolution::Pending,
        }]
    }

    fn maintain_route(
        &mut self,
        _identity: &MaterializedRouteIdentity,
        runtime: &mut RouteRuntimeState,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, jacquard_traits::jacquard_core::RouteError>
    {
        runtime.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        let result = match trigger {
            | RouteMaintenanceTrigger::LinkDegraded => RouteMaintenanceResult {
                event: RouteLifecycleEvent::Repaired,
                outcome: RouteMaintenanceOutcome::Repaired,
            },
            | _ => RouteMaintenanceResult {
                event: runtime.last_lifecycle_event,
                outcome: RouteMaintenanceOutcome::Continued,
            },
        };
        Ok(result)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        assert_eq!(*route_id, self.route.identity.stamp.route_id);
    }
}

impl LayeredRoutingEnginePlanner for StubEngine {
    fn candidate_routes_on_substrate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _substrate: &SubstrateLease,
        _parameters: &LayerParameters,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            summary: self.route.identity.admission.summary.clone(),
            estimate: Estimate {
                value: RouteEstimate {
                    estimated_protection: self
                        .route
                        .identity
                        .admission
                        .summary
                        .protection,
                    estimated_connectivity: self
                        .route
                        .identity
                        .admission
                        .summary
                        .connectivity,
                    topology_epoch: self
                        .route
                        .identity
                        .admission
                        .witness
                        .topology_epoch,
                    degradation: self.route.identity.admission.witness.degradation,
                },
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(
                    1000,
                ),
                updated_at_tick: Tick(1),
            },
            backend_ref: BackendRouteRef {
                engine: stub_engine_id(),
                backend_route_id: jacquard_traits::jacquard_core::BackendRouteId(vec![
                    9,
                ]),
            },
        }]
    }

    fn admit_route_on_substrate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _substrate: &SubstrateLease,
        _parameters: &LayerParameters,
        _candidate: RouteCandidate,
    ) -> Result<RouteAdmission, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.identity.admission.clone())
    }
}

impl LayeredRoutingEngine for StubEngine {
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        _substrate: SubstrateLease,
        _parameters: LayerParameters,
    ) -> Result<RouteInstallation, jacquard_traits::jacquard_core::RouteError> {
        assert_eq!(input.handle.stamp, self.route.identity.stamp);
        Ok(RouteInstallation {
            materialization_proof: self.route.identity.proof.clone(),
            last_lifecycle_event: self.route.runtime.last_lifecycle_event,
            health: self.route.runtime.health.clone(),
            progress: self.route.runtime.progress.clone(),
        })
    }
}

impl SubstratePlanner for StubSubstrateProvider {
    fn candidate_substrates(
        &self,
        _requirements: &SubstrateRequirements,
        _topology: &Observation<Configuration>,
    ) -> Vec<SubstrateCandidate> {
        vec![SubstrateCandidate {
            capabilities: SubstrateCapabilities {
                engine: stub_engine_id(),
                protection: RouteProtectionClass::LinkProtected,
                connectivity: repairable_connected(),
                mtu_bytes: ByteCount(1200),
            },
            expected_health: Some(self.route.runtime.health.clone()),
        }]
    }
}

impl SubstrateRuntime for StubSubstrateProvider {
    fn acquire_substrate(
        &mut self,
        candidate: SubstrateCandidate,
    ) -> Result<SubstrateLease, jacquard_traits::jacquard_core::RouteError> {
        Ok(SubstrateLease {
            capabilities: candidate.capabilities,
            handle: RouteHandle {
                stamp: self.route.identity.stamp.clone(),
            },
            lease: self.route.identity.lease.clone(),
        })
    }

    fn release_substrate(
        &mut self,
        lease: &SubstrateLease,
    ) -> Result<(), jacquard_traits::jacquard_core::RouteError> {
        assert_eq!(lease.handle.route_id(), self.route.identity.route_id());
        Ok(())
    }

    fn observe_substrate_health(
        &self,
        _lease: &SubstrateLease,
    ) -> Result<Observation<RouteHealth>, jacquard_traits::jacquard_core::RouteError>
    {
        Ok(common::local_observation(
            self.route.runtime.health.clone(),
            Tick(1),
        ))
    }
}

impl LayeringPolicyEngine for StubLayeringPolicyEngine {
    fn activate_layered_route(
        &mut self,
        _objective: RoutingObjective,
        _outer_engine: RoutingEngineId,
        _substrate_requirements: SubstrateRequirements,
        _parameters: LayerParameters,
    ) -> Result<MaterializedRoute, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.clone())
    }
}

fn sample_objective() -> RoutingObjective {
    RoutingObjective {
        destination: jacquard_traits::jacquard_core::DestinationId::Node(
            jacquard_traits::jacquard_core::NodeId([2; 32]),
        ),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::None,
        target_connectivity: repairable_connected(),
        hold_fallback_policy:
            jacquard_traits::jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(jacquard_traits::jacquard_core::DurationMs(
            100,
        )),
        protection_priority: jacquard_traits::jacquard_core::PriorityPoints(1),
        connectivity_priority: jacquard_traits::jacquard_core::PriorityPoints(2),
    }
}

fn sample_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: repairable_connected(),
        deployment_profile: OperatingMode::SparseLowPower,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn sample_admission_assumptions() -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::PerRouteSequenced,
        failure_model: FailureModelClass::CrashStop,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Sparse,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ExactUnderAssumptions,
    }
}

// long-block-exception: canonical route assembly fixture for contract tests.
fn sample_route(
    objective: RoutingObjective,
    profile: SelectedRoutingParameters,
) -> MaterializedRoute {
    let stamp = jacquard_traits::jacquard_core::RouteIdentityStamp {
        route_id: RouteId([3; 16]),
        topology_epoch: RouteEpoch(1),
        materialized_at_tick: Tick(1),
        publication_id: PublicationId([7; 16]),
    };
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            stamp: stamp.clone(),
        },
        admission: RouteAdmission {
            route_id: RouteId([3; 16]),
            backend_ref: BackendRouteRef {
                engine: stub_engine_id(),
                backend_route_id: jacquard_traits::jacquard_core::BackendRouteId(vec![
                    1, 2, 3,
                ]),
            },
            objective,
            profile,
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: sample_admission_assumptions(),
                productive_step_bound: Limit::Bounded(2),
                total_step_bound: Limit::Bounded(4),
                route_cost: RouteCost {
                    message_count_max: Limit::Bounded(4),
                    byte_count_max: Limit::Bounded(ByteCount(1024)),
                    hop_count: 2,
                    repair_attempt_count_max: Limit::Bounded(1),
                    hold_bytes_reserved: Limit::Unbounded,
                    work_step_count_max: Limit::Bounded(8),
                },
            },
            summary: RouteSummary {
                engine: stub_engine_id(),
                protection: RouteProtectionClass::LinkProtected,
                connectivity: repairable_connected(),
                protocol_mix: vec![TransportProtocol::BleGatt],
                hop_count_hint: common::estimated(2, 1000, Tick(1)),
                valid_for: TimeWindow::new(Tick(1), Tick(50))
                    .expect("valid route summary window"),
            },
            witness: RouteWitness {
                protection: ObjectiveVsDelivered {
                    objective: RouteProtectionClass::LinkProtected,
                    delivered: RouteProtectionClass::LinkProtected,
                },
                connectivity: ObjectiveVsDelivered {
                    objective: repairable_connected(),
                    delivered: repairable_connected(),
                },
                admission_profile: sample_admission_assumptions(),
                topology_epoch: RouteEpoch(1),
                degradation: RouteDegradation::None,
            },
        },
        lease: RouteLease {
            owner_node_id: jacquard_traits::jacquard_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(1),
            valid_for: TimeWindow::new(Tick(1), Tick(50))
                .expect("valid route lease window"),
        },
    };
    let installation = RouteInstallation {
        materialization_proof: RouteMaterializationProof {
            stamp,
            witness: Fact {
                value: RouteWitness {
                    protection: ObjectiveVsDelivered {
                        objective: RouteProtectionClass::LinkProtected,
                        delivered: RouteProtectionClass::LinkProtected,
                    },
                    connectivity: ObjectiveVsDelivered {
                        objective: repairable_connected(),
                        delivered: repairable_connected(),
                    },
                    admission_profile: sample_admission_assumptions(),
                    topology_epoch: RouteEpoch(1),
                    degradation: RouteDegradation::None,
                },
                basis: FactBasis::Published,
                established_at_tick: Tick(1),
            },
        },
        last_lifecycle_event: RouteLifecycleEvent::Activated,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: jacquard_traits::jacquard_core::HealthScore(100),
            congestion_penalty_points: jacquard_traits::jacquard_core::PenaltyPoints(0),
            last_validated_at_tick: Tick(1),
        },
        progress: RouteProgressContract {
            productive_step_count_max: Limit::Bounded(2),
            total_step_count_max: Limit::Bounded(4),
            last_progress_at_tick: Tick(1),
            state: RouteProgressState::Pending,
        },
    };
    MaterializedRoute::from_installation(input, installation)
}

fn materialization_input(route: &MaterializedRoute) -> RouteMaterializationInput {
    RouteMaterializationInput {
        handle: RouteHandle {
            stamp: route.identity.stamp.clone(),
        },
        admission: route.identity.admission.clone(),
        lease: route.identity.lease.clone(),
    }
}

fn empty_configuration() -> Configuration {
    Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::new(),
        links: BTreeMap::new(),
        environment: Environment {
            reachable_neighbor_count: 0,
            churn_permille: jacquard_traits::jacquard_core::RatioPermille(0),
            contention_permille: jacquard_traits::jacquard_core::RatioPermille(0),
        },
    }
}

fn sample_substrate_requirements() -> SubstrateRequirements {
    SubstrateRequirements {
        min_protection: RouteProtectionClass::LinkProtected,
        min_connectivity: repairable_connected(),
        latency_budget_ms: Limit::Bounded(jacquard_traits::jacquard_core::DurationMs(
            50,
        )),
        mtu_floor_bytes: ByteCount(512),
        identity_assurance_floor: IdentityAssuranceClass::WeakObserved,
    }
}

#[test]
// long-block-exception: full candidate-to-materialized route flow.
fn routing_engine_contract_can_drive_candidate_to_materialized_route() {
    let objective = sample_objective();
    let profile = sample_profile();
    let route = sample_route(objective.clone(), profile.clone());
    let mut engine = StubEngine { route: route.clone(), selector: None };
    let topology = Observation {
        value: empty_configuration(),
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Remote,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Authenticated,
        observed_at_tick: Tick(0),
    };
    let candidates = engine.candidate_routes(&objective, &profile, &topology);
    let candidate = candidates.into_iter().next().expect("candidate");
    let check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("admission check");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admission");
    assert_eq!(admission, route.identity.admission);
    let installation = engine
        .materialize_route(materialization_input(&route))
        .expect("materialize");
    let mut installed = MaterializedRoute::from_installation(
        materialization_input(&route),
        installation,
    );
    let commitments = engine.route_commitments(&installed);
    let maintenance = engine
        .maintain_route(
            &installed.identity,
            &mut installed.runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("maintenance");
    engine.teardown(&installed.identity.stamp.route_id);

    assert_eq!(check.decision, AdmissionDecision::Admissible);
    assert_eq!(
        maintenance,
        RouteMaintenanceResult {
            event: RouteLifecycleEvent::Repaired,
            outcome: RouteMaintenanceOutcome::Repaired,
        },
    );
    assert_eq!(commitments.len(), 1);
    assert_eq!(commitments[0].commitment_id, RouteCommitmentId([8; 16]));
    assert_eq!(
        commitments[0].route_binding,
        RouteBinding::Bound(installed.identity.stamp.route_id),
    );
    assert_eq!(installed.identity.stamp.route_id, RouteId([3; 16]));
    assert_eq!(
        installed.identity.proof.witness.value.topology_epoch,
        RouteEpoch(1),
    );
    assert_eq!(
        installed.runtime.last_lifecycle_event,
        RouteLifecycleEvent::Repaired
    );
}

#[test]
fn routing_engine_supports_engine_wide_periodic_progress() {
    let route = sample_route(sample_objective(), sample_profile());
    let mut engine = StubEngine { route, selector: None };

    let topology = common::local_observation(empty_configuration(), Tick(1));

    let outcome = engine
        .engine_tick(&RoutingTickContext::new(topology))
        .expect("default engine tick should succeed");
    assert_eq!(outcome.topology_epoch, RouteEpoch(1));
    assert_eq!(outcome.change, RoutingTickChange::NoChange);
}

#[test]
fn committee_selector_trait_supports_shared_result_shape() {
    let selector = StubCommitteeSelector;
    let committee = selector
        .select_committee(
            &sample_objective(),
            &sample_profile(),
            &common::local_observation(empty_configuration(), Tick(1)),
        )
        .expect("committee selection should succeed")
        .expect("committee should be present");

    assert_eq!(committee.quorum_threshold, QuorumThreshold(2));
    assert_eq!(committee.members.len(), 2);
    assert_eq!(committee.members[0].role, CommitteeRole::Participant);
}

#[test]
fn committee_coordinated_engine_exposes_optional_swappable_selector() {
    let route = sample_route(sample_objective(), sample_profile());
    let engine_with_selector = StubEngine {
        route: route.clone(),
        selector: Some(StubCommitteeSelector),
    };
    let engine_without_selector = StubEngine { route, selector: None };

    let selector = engine_with_selector
        .committee_selector()
        .expect("selector should be present");
    let committee = selector
        .select_committee(
            &sample_objective(),
            &sample_profile(),
            &common::local_observation(empty_configuration(), Tick(1)),
        )
        .expect("committee selection should succeed")
        .expect("committee should be present");

    assert_eq!(committee.committee_id, CommitteeId([5; 16]));
    assert!(engine_without_selector.committee_selector().is_none());
}

#[test]
// long-block-exception: full layering composition chain in one test body.
fn substrate_and_layering_traits_support_policy_driven_composition() {
    let objective = sample_objective();
    let profile = sample_profile();
    let route = sample_route(objective.clone(), profile.clone());
    let topology = common::local_observation(empty_configuration(), Tick(1));

    let mut provider = StubSubstrateProvider { route: route.clone() };
    let substrate_candidate = provider
        .candidate_substrates(&sample_substrate_requirements(), &topology)
        .into_iter()
        .next()
        .expect("substrate candidate");
    let substrate = provider
        .acquire_substrate(substrate_candidate)
        .expect("substrate lease");

    let parameters = LayerParameters {
        items: vec![LayerParameter::PathLengthHint(2)],
    };
    let mut layered_engine = StubEngine { route: route.clone(), selector: None };
    let candidate = layered_engine
        .candidate_routes_on_substrate(&objective, &profile, &substrate, &parameters)
        .into_iter()
        .next()
        .expect("layered candidate");
    let admission = layered_engine
        .admit_route_on_substrate(
            &objective,
            &profile,
            &substrate,
            &parameters,
            candidate,
        )
        .expect("layered admission");
    assert_eq!(admission, route.identity.admission);
    let layered_installation = layered_engine
        .materialize_route_on_substrate(
            materialization_input(&route),
            substrate.clone(),
            parameters.clone(),
        )
        .expect("layered route");
    let layered_route = MaterializedRoute::from_installation(
        materialization_input(&route),
        layered_installation,
    );
    let substrate_health = provider
        .observe_substrate_health(&substrate)
        .expect("substrate health");
    provider
        .release_substrate(&substrate)
        .expect("release substrate");

    let mut coordinator = StubLayeringPolicyEngine { route };
    let coordinated = coordinator
        .activate_layered_route(
            sample_objective(),
            RoutingEngineId::from_contract_bytes([9; 16]),
            sample_substrate_requirements(),
            parameters,
        )
        .expect("coordinated layered route");

    assert_eq!(substrate.capabilities.engine, stub_engine_id());
    assert_eq!(
        substrate_health.value.reachability_state,
        ReachabilityState::Reachable
    );
    assert_eq!(layered_route.identity.stamp.route_id, RouteId([3; 16]));
    assert_eq!(coordinated.identity.stamp.route_id, RouteId([3; 16]));
}
