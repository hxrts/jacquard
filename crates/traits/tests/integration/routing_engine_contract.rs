//! Drive a stub RoutingEngine through router-owned materialization and teardown.

use std::collections::BTreeMap;

use jacquard_traits::{
    jacquard_core::{
        AdaptiveRoutingProfile, AdmissionDecision, AdversaryRegime, BackendRouteRef, Belief,
        ByteCount, ClaimStrength, CommitteeId, CommitteeMember, CommitteeRole, CommitteeSelection,
        Configuration, ConnectivityRegime, DeploymentProfile, Environment, Estimate, Fact,
        FactBasis, FailureModelClass, IdentityAssuranceClass, LayerParameter, LayerParameters,
        Limit, MaterializedRoute, MessageFlowAssumptionClass, NodeDensityClass, Observation,
        PublicationId, ReachabilityState, RouteAdmission, RouteAdmissionCheck, RouteBinding,
        RouteCandidate, RouteCommitment, RouteCommitmentId, RouteCommitmentResolution,
        RouteConnectivityProfile, RouteCost, RouteDegradation, RouteEpoch, RouteEstimate,
        RouteHandle, RouteHealth, RouteId, RouteInstallation, RouteLease, RouteLifecycleEvent,
        RouteMaintenanceOutcome, RouteMaintenanceResult, RouteMaintenanceTrigger,
        RouteMaterializationInput, RouteMaterializationProof, RoutePartitionClass,
        RouteProgressContract, RouteProgressState, RouteProtectionClass, RouteRepairClass,
        RouteReplacementPolicy, RouteServiceKind, RouteSummary, RouteWitness,
        RoutingAdmissionProfile, RoutingEngineCapabilities, RoutingEngineFallbackPolicy,
        RoutingEngineId, RoutingEvidenceClass, RoutingObjective, RuntimeEnvelopeClass,
        SubstrateCandidate, SubstrateCapabilities, SubstrateLease, SubstrateRequirements, Tick,
        TimeWindow, TransportProtocol,
    },
    CommitteeSelector, LayeredRoutingEngine, LayeredRoutingEnginePlanner, LayeringPolicyEngine,
    RoutingEngine, RoutingEnginePlanner, SubstratePlanner, SubstrateRuntime,
};

fn repairable_connected() -> RouteConnectivityProfile {
    RouteConnectivityProfile {
        repair: RouteRepairClass::Repairable,
        partition: RoutePartitionClass::ConnectedOnly,
    }
}

struct StubFamily {
    route: MaterializedRoute,
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
        _profile: &AdaptiveRoutingProfile,
        _topology: &Observation<Self::TopologyView>,
    ) -> Result<CommitteeSelection, jacquard_traits::jacquard_core::RouteError> {
        Ok(CommitteeSelection {
            committee_id: CommitteeId([5; 16]),
            topology_epoch: RouteEpoch(1),
            selected_at_tick: Tick(1),
            valid_for: TimeWindow {
                start_tick: Tick(1),
                end_tick: Tick(10),
            },
            evidence_basis: FactBasis::Observed,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold: 2,
            members: vec![
                CommitteeMember {
                    node_id: jacquard_traits::jacquard_core::NodeId([1; 32]),
                    controller_id: jacquard_traits::jacquard_core::ControllerId([1; 32]),
                    role: CommitteeRole::Participant,
                    trust_score: Belief::Estimated(Estimate {
                        value: jacquard_traits::jacquard_core::HealthScore(900),
                        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                        updated_at_tick: Tick(1),
                    }),
                },
                CommitteeMember {
                    node_id: jacquard_traits::jacquard_core::NodeId([2; 32]),
                    controller_id: jacquard_traits::jacquard_core::ControllerId([2; 32]),
                    role: CommitteeRole::Witness,
                    trust_score: Belief::Absent,
                },
            ],
        })
    }
}

impl RoutingEnginePlanner for StubFamily {
    fn engine_id(&self) -> RoutingEngineId {
        RoutingEngineId::Mesh
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        RoutingEngineCapabilities {
            engine: RoutingEngineId::Mesh,
            max_protection: RouteProtectionClass::LinkProtected,
            max_connectivity: repairable_connected(),
            repair_support: jacquard_traits::jacquard_core::RepairSupport::Supported,
            hold_support: jacquard_traits::jacquard_core::HoldSupport::Supported,
            decidable_admission: jacquard_traits::jacquard_core::DecidableSupport::Supported,
            quantitative_bounds:
                jacquard_traits::jacquard_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support:
                jacquard_traits::jacquard_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: jacquard_traits::jacquard_core::RouteShapeVisibility::Explicit,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            summary: self.route.admission.summary.clone(),
            estimate: Estimate {
                value: RouteEstimate {
                    estimated_protection: self.route.admission.summary.protection,
                    estimated_connectivity: self.route.admission.summary.connectivity,
                    topology_epoch: self.route.admission.witness.topology_epoch,
                    degradation: self.route.admission.witness.degradation,
                },
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            },
            backend_ref: BackendRouteRef {
                engine: RoutingEngineId::Mesh,
                backend_route_id: jacquard_traits::jacquard_core::BackendRouteId(vec![1]),
            },
        }]
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.admission.admission_check.clone())
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _candidate: RouteCandidate,
    ) -> Result<RouteAdmission, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.admission.clone())
    }
}

impl RoutingEngine for StubFamily {
    fn materialize_route(
        &mut self,
        input: RouteMaterializationInput,
    ) -> Result<RouteInstallation, jacquard_traits::jacquard_core::RouteError> {
        assert_eq!(input.handle, self.route.handle);
        assert_eq!(input.lease, self.route.lease);
        assert_eq!(input.admission, self.route.admission);
        Ok(RouteInstallation {
            materialization_proof: self.route.materialization_proof.clone(),
            last_lifecycle_event: self.route.last_lifecycle_event,
            health: self.route.health.clone(),
            progress: self.route.progress.clone(),
        })
    }

    fn route_commitments(&self, route: &MaterializedRoute) -> Vec<RouteCommitment> {
        vec![RouteCommitment {
            commitment_id: RouteCommitmentId([8; 16]),
            operation_id: jacquard_traits::jacquard_core::RouteOperationId([6; 16]),
            route_binding: RouteBinding::Bound(route.admission.route_id),
            owner_node_id: route.lease.owner_node_id,
            deadline_tick: Tick(10),
            retry_policy: jacquard_traits::jacquard_core::TimeoutPolicy {
                attempt_count_max: 1,
                initial_backoff_ms: jacquard_traits::jacquard_core::DurationMs(5),
                backoff_multiplier_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                backoff_ms_max: jacquard_traits::jacquard_core::DurationMs(5),
                overall_timeout_ms: jacquard_traits::jacquard_core::DurationMs(5),
            },
            resolution: RouteCommitmentResolution::Pending,
        }]
    }

    fn maintain_route(
        &mut self,
        route: &mut MaterializedRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, jacquard_traits::jacquard_core::RouteError> {
        route.last_lifecycle_event = RouteLifecycleEvent::Repaired;
        let result = match trigger {
            RouteMaintenanceTrigger::LinkDegraded => RouteMaintenanceResult {
                event: RouteLifecycleEvent::Repaired,
                outcome: RouteMaintenanceOutcome::Repaired,
            },
            _ => RouteMaintenanceResult {
                event: route.last_lifecycle_event,
                outcome: RouteMaintenanceOutcome::Continued,
            },
        };
        Ok(result)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        assert_eq!(*route_id, self.route.admission.route_id);
    }
}

impl LayeredRoutingEnginePlanner for StubFamily {
    fn candidate_routes_on_substrate(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _substrate: &SubstrateLease,
        _parameters: &LayerParameters,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            summary: self.route.admission.summary.clone(),
            estimate: Estimate {
                value: RouteEstimate {
                    estimated_protection: self.route.admission.summary.protection,
                    estimated_connectivity: self.route.admission.summary.connectivity,
                    topology_epoch: self.route.admission.witness.topology_epoch,
                    degradation: self.route.admission.witness.degradation,
                },
                confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                updated_at_tick: Tick(1),
            },
            backend_ref: BackendRouteRef {
                engine: RoutingEngineId::Mesh,
                backend_route_id: jacquard_traits::jacquard_core::BackendRouteId(vec![9]),
            },
        }]
    }

    fn admit_route_on_substrate(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _substrate: &SubstrateLease,
        _parameters: &LayerParameters,
        _candidate: RouteCandidate,
    ) -> Result<RouteAdmission, jacquard_traits::jacquard_core::RouteError> {
        Ok(self.route.admission.clone())
    }
}

impl LayeredRoutingEngine for StubFamily {
    fn materialize_route_on_substrate(
        &mut self,
        input: RouteMaterializationInput,
        _substrate: SubstrateLease,
        _parameters: LayerParameters,
    ) -> Result<RouteInstallation, jacquard_traits::jacquard_core::RouteError> {
        assert_eq!(input.handle, self.route.handle);
        Ok(RouteInstallation {
            materialization_proof: self.route.materialization_proof.clone(),
            last_lifecycle_event: self.route.last_lifecycle_event,
            health: self.route.health.clone(),
            progress: self.route.progress.clone(),
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
                engine: RoutingEngineId::Mesh,
                protection: RouteProtectionClass::LinkProtected,
                connectivity: repairable_connected(),
                mtu_bytes: ByteCount(1200),
            },
            expected_health: Some(self.route.health.clone()),
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
            handle: self.route.handle.clone(),
            lease: self.route.lease.clone(),
        })
    }

    fn release_substrate(
        &mut self,
        lease: &SubstrateLease,
    ) -> Result<(), jacquard_traits::jacquard_core::RouteError> {
        assert_eq!(lease.handle.route_id, self.route.handle.route_id);
        Ok(())
    }

    fn observe_substrate_health(
        &self,
        _lease: &SubstrateLease,
    ) -> Result<Observation<RouteHealth>, jacquard_traits::jacquard_core::RouteError> {
        Ok(Observation {
            value: self.route.health.clone(),
            source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication:
                jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        })
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
        hold_fallback_policy: jacquard_traits::jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(jacquard_traits::jacquard_core::DurationMs(100)),
        protection_priority: jacquard_traits::jacquard_core::PriorityPoints(1),
        connectivity_priority: jacquard_traits::jacquard_core::PriorityPoints(2),
    }
}

fn sample_profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: repairable_connected(),
        deployment_profile: DeploymentProfile::SparseLowPower,
        diversity_floor: 1,
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn sample_admission_profile() -> RoutingAdmissionProfile {
    RoutingAdmissionProfile {
        message_flow_assumption: MessageFlowAssumptionClass::PerRouteSequenced,
        failure_model: FailureModelClass::CrashStop,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Sparse,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ExactUnderAssumptions,
    }
}

fn sample_route(objective: RoutingObjective, profile: AdaptiveRoutingProfile) -> MaterializedRoute {
    let input = RouteMaterializationInput {
        handle: RouteHandle {
            route_id: RouteId([3; 16]),
            topology_epoch: RouteEpoch(1),
            materialized_at_tick: Tick(1),
            publication_id: PublicationId([7; 16]),
        },
        admission: RouteAdmission {
            route_id: RouteId([3; 16]),
            objective,
            profile,
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: sample_admission_profile(),
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
                engine: RoutingEngineId::Mesh,
                protection: RouteProtectionClass::LinkProtected,
                connectivity: repairable_connected(),
                protocol_mix: vec![TransportProtocol::BleGatt],
                hop_count_hint: Belief::Estimated(Estimate {
                    value: 2,
                    confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
                    updated_at_tick: Tick(1),
                }),
                valid_for: TimeWindow {
                    start_tick: Tick(1),
                    end_tick: Tick(50),
                },
            },
            witness: RouteWitness {
                objective_protection: RouteProtectionClass::LinkProtected,
                delivered_protection: RouteProtectionClass::LinkProtected,
                objective_connectivity: repairable_connected(),
                delivered_connectivity: repairable_connected(),
                admission_profile: sample_admission_profile(),
                topology_epoch: RouteEpoch(1),
                degradation: RouteDegradation::None,
            },
        },
        lease: RouteLease {
            owner_node_id: jacquard_traits::jacquard_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(1),
            valid_for: TimeWindow {
                start_tick: Tick(1),
                end_tick: Tick(50),
            },
        },
    };
    let installation = RouteInstallation {
        materialization_proof: RouteMaterializationProof {
            route_id: RouteId([3; 16]),
            topology_epoch: RouteEpoch(1),
            materialized_at_tick: Tick(1),
            publication_id: PublicationId([7; 16]),
            witness: Fact {
                value: RouteWitness {
                    objective_protection: RouteProtectionClass::LinkProtected,
                    delivered_protection: RouteProtectionClass::LinkProtected,
                    objective_connectivity: repairable_connected(),
                    delivered_connectivity: repairable_connected(),
                    admission_profile: sample_admission_profile(),
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
        handle: route.handle.clone(),
        admission: route.admission.clone(),
        lease: route.lease.clone(),
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
        latency_budget_ms: Limit::Bounded(jacquard_traits::jacquard_core::DurationMs(50)),
        mtu_floor_bytes: ByteCount(512),
        identity_assurance_floor: IdentityAssuranceClass::WeakObserved,
    }
}

#[test]
fn routing_engine_contract_can_drive_candidate_to_materialized_route() {
    let objective = sample_objective();
    let profile = sample_profile();
    let route = sample_route(objective.clone(), profile.clone());
    let mut family = StubFamily {
        route: route.clone(),
    };
    let topology = Observation {
        value: empty_configuration(),
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Remote,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Authenticated,
        observed_at_tick: Tick(0),
    };
    let candidates = family.candidate_routes(&objective, &profile, &topology);
    let candidate = candidates.into_iter().next().expect("candidate");
    let check = family
        .check_candidate(&objective, &profile, &candidate)
        .expect("admission check");
    let admission = family
        .admit_route(&objective, &profile, candidate)
        .expect("admission");
    assert_eq!(admission, route.admission);
    let installation = family
        .materialize_route(materialization_input(&route))
        .expect("materialize");
    let mut installed =
        MaterializedRoute::from_installation(materialization_input(&route), installation);
    let commitments = family.route_commitments(&installed);
    let maintenance = family
        .maintain_route(&mut installed, RouteMaintenanceTrigger::LinkDegraded)
        .expect("maintenance");
    family.teardown(&installed.admission.route_id);

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
        RouteBinding::Bound(installed.admission.route_id),
    );
    assert_eq!(installed.handle.route_id, RouteId([3; 16]));
    assert_eq!(
        installed.materialization_proof.witness.value.topology_epoch,
        RouteEpoch(1),
    );
    assert_eq!(
        installed.last_lifecycle_event,
        RouteLifecycleEvent::Repaired
    );
}

#[test]
fn committee_selector_trait_supports_shared_result_shape() {
    let selector = StubCommitteeSelector;
    let committee = selector
        .select_committee(
            &sample_objective(),
            &sample_profile(),
            &Observation {
                value: empty_configuration(),
                source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
                evidence_class: RoutingEvidenceClass::DirectObservation,
                origin_authentication:
                    jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
                observed_at_tick: Tick(1),
            },
        )
        .expect("committee selection should succeed");

    assert_eq!(committee.quorum_threshold, 2);
    assert_eq!(committee.members.len(), 2);
    assert_eq!(committee.members[0].role, CommitteeRole::Participant);
}

#[test]
fn substrate_and_layering_traits_support_policy_driven_composition() {
    let objective = sample_objective();
    let profile = sample_profile();
    let route = sample_route(objective.clone(), profile.clone());
    let topology = Observation {
        value: empty_configuration(),
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    };

    let mut provider = StubSubstrateProvider {
        route: route.clone(),
    };
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
    let mut layered_family = StubFamily {
        route: route.clone(),
    };
    let candidate = layered_family
        .candidate_routes_on_substrate(&objective, &profile, &substrate, &parameters)
        .into_iter()
        .next()
        .expect("layered candidate");
    let admission = layered_family
        .admit_route_on_substrate(&objective, &profile, &substrate, &parameters, candidate)
        .expect("layered admission");
    assert_eq!(admission, route.admission);
    let layered_installation = layered_family
        .materialize_route_on_substrate(
            materialization_input(&route),
            substrate.clone(),
            parameters.clone(),
        )
        .expect("layered route");
    let layered_route =
        MaterializedRoute::from_installation(materialization_input(&route), layered_installation);
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
            RoutingEngineId::External {
                name: "onion".into(),
                contract_id: jacquard_traits::jacquard_core::RoutingEngineContractId([1; 16]),
            },
            sample_substrate_requirements(),
            parameters,
        )
        .expect("coordinated layered route");

    assert_eq!(substrate.capabilities.engine, RoutingEngineId::Mesh);
    assert_eq!(
        substrate_health.value.reachability_state,
        ReachabilityState::Reachable
    );
    assert_eq!(layered_route.handle.route_id, RouteId([3; 16]));
    assert_eq!(coordinated.handle.route_id, RouteId([3; 16]));
}
