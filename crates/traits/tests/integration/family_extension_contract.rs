//! Drive a stub RouteFamily through the full candidate-to-teardown lifecycle.

use std::collections::BTreeMap;

use contour_traits::{
    contour_core::{
        AdaptiveRoutingProfile, AdmissionDecision, AdversaryRegime, BackendRouteRef, ClaimStrength,
        ConnectivityRegime, DeliveryModelClass, DeploymentProfileId, FailureModelClass,
        FamilyFallbackPolicy, InstalledRoute, KnownValue, Limit, NodeDensityClass, Observed,
        PeerTrustClass, ReachabilityState, RouteAdmission, RouteAdmissionCheck, RouteBinding,
        RouteCandidate, RouteCommitment, RouteCommitmentId, RouteCommitmentResolution,
        RouteConnectivityClass, RouteCost, RouteDegradation, RouteEpoch, RouteEstimate,
        RouteHandle, RouteHealth, RouteId, RouteLease, RouteMaintenanceOutcome,
        RouteMaintenanceResult, RouteMaintenanceTrigger, RouteMaterializationProof,
        RoutePrivacyClass, RouteProgressContract, RouteProgressState, RouteReplacementPolicy,
        RouteSummary, RouteTransition, RouteWitness, RoutingAdmissionProfile, RoutingEvidenceClass,
        RoutingFact, RoutingFamilyCapabilities, RoutingFamilyId, RoutingObjective,
        RuntimeEnvelopeClass, ServiceFamily, Tick, TimeWindow, TopologySnapshot, TransportClass,
    },
    RouteFamily,
};

struct StubFamily {
    route: InstalledRoute,
}

impl RouteFamily for StubFamily {
    fn family_id(&self) -> RoutingFamilyId {
        RoutingFamilyId::Mesh
    }

    fn capabilities(&self) -> RoutingFamilyCapabilities {
        RoutingFamilyCapabilities {
            family: RoutingFamilyId::Mesh,
            max_privacy: RoutePrivacyClass::LinkConfidential,
            max_connectivity: RouteConnectivityClass::Repairable,
            repair_support: contour_traits::contour_core::RepairSupport::Supported,
            hold_support: contour_traits::contour_core::HoldSupport::Supported,
            decidable_admission: contour_traits::contour_core::DecidableSupport::Supported,
            quantitative_bounds:
                contour_traits::contour_core::QuantitativeBoundSupport::ProductiveOnly,
            reconfiguration_support:
                contour_traits::contour_core::ReconfigurationSupport::ReplaceOnly,
            route_shape_visibility: contour_traits::contour_core::RouteShapeVisibility::Explicit,
        }
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _topology: &Observed<TopologySnapshot>,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            summary: self.route.admission.summary.clone(),
            estimate: Observed {
                fact: RoutingFact {
                    value: RouteEstimate {
                        estimated_privacy: self.route.admission.summary.privacy,
                        estimated_connectivity: self.route.admission.summary.connectivity,
                        topology_epoch: self.route.admission.witness.topology_epoch,
                        degradation: self.route.admission.witness.degradation,
                    },
                    evidence_class: RoutingEvidenceClass::Observed,
                    trust_class: PeerTrustClass::ControllerBound,
                    observed_at_tick: Tick(1),
                },
            },
            backend_ref: BackendRouteRef {
                family: RoutingFamilyId::Mesh,
                opaque_id: vec![1],
            },
        }]
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _candidate: &RouteCandidate,
    ) -> Result<RouteAdmissionCheck, contour_traits::contour_core::RouteError> {
        Ok(self.route.admission.admission_check.clone())
    }

    fn admit_route(
        &mut self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _candidate: RouteCandidate,
    ) -> Result<RouteAdmission, contour_traits::contour_core::RouteError> {
        Ok(self.route.admission.clone())
    }

    fn install_route(
        &mut self,
        _admission: RouteAdmission,
    ) -> Result<InstalledRoute, contour_traits::contour_core::RouteError> {
        Ok(self.route.clone())
    }

    fn route_commitments(&self, route: &InstalledRoute) -> Vec<RouteCommitment> {
        vec![RouteCommitment {
            commitment_id: RouteCommitmentId([8; 16]),
            operation_id: contour_traits::contour_core::RouteOperationId([6; 16]),
            route_binding: RouteBinding::Bound(route.admission.route_id),
            owner_node_id: route.lease.owner_node_id,
            deadline_tick: Tick(10),
            retry_policy: contour_traits::contour_core::TimeoutPolicy {
                attempt_count_max: 1,
                initial_backoff_ms: contour_traits::contour_core::DurationMs(5),
                backoff_multiplier_permille: contour_traits::contour_core::RatioPermille(1000),
                backoff_ms_max: contour_traits::contour_core::DurationMs(5),
                overall_timeout_ms: contour_traits::contour_core::DurationMs(5),
            },
            resolution: RouteCommitmentResolution::Pending,
        }]
    }

    fn maintain_route(
        &mut self,
        route: &mut InstalledRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceResult, contour_traits::contour_core::RouteError> {
        route.current_transition = RouteTransition::Repaired;
        let result = match trigger {
            RouteMaintenanceTrigger::LinkDegraded => RouteMaintenanceResult {
                transition: RouteTransition::Repaired,
                outcome: RouteMaintenanceOutcome::Repaired,
            },
            _ => RouteMaintenanceResult {
                transition: route.current_transition,
                outcome: RouteMaintenanceOutcome::Continued,
            },
        };
        Ok(result)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        assert_eq!(*route_id, self.route.admission.route_id);
    }
}

fn sample_objective() -> RoutingObjective {
    RoutingObjective {
        destination: contour_traits::contour_core::DestinationId::Node(
            contour_traits::contour_core::NodeId([2; 32]),
        ),
        service_family: ServiceFamily::Move,
        target_privacy: RoutePrivacyClass::LinkConfidential,
        privacy_floor: RoutePrivacyClass::None,
        target_connectivity: RouteConnectivityClass::Repairable,
        hold_fallback_policy: contour_traits::contour_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Limited(contour_traits::contour_core::DurationMs(100)),
        privacy_priority: contour_traits::contour_core::PriorityPoints(1),
        connectivity_priority: contour_traits::contour_core::PriorityPoints(2),
    }
}

fn sample_profile() -> AdaptiveRoutingProfile {
    AdaptiveRoutingProfile {
        selected_privacy: RoutePrivacyClass::LinkConfidential,
        selected_connectivity: RouteConnectivityClass::Repairable,
        deployment_profile: DeploymentProfileId::SparseLowPower,
        diversity_floor: 1,
        family_fallback_policy: FamilyFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn sample_admission_profile() -> RoutingAdmissionProfile {
    RoutingAdmissionProfile {
        delivery_model: DeliveryModelClass::FifoPerLink,
        failure_model: FailureModelClass::CrashStop,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Sparse,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ExactUnderAssumptions,
    }
}

fn sample_route(objective: RoutingObjective, profile: AdaptiveRoutingProfile) -> InstalledRoute {
    InstalledRoute {
        handle: RouteHandle {
            route_id: RouteId([3; 16]),
            topology_epoch: RouteEpoch(1),
            materialized_at_tick: Tick(1),
            publication_id: [7; 16],
        },
        materialization_proof: RouteMaterializationProof {
            route_id: RouteId([3; 16]),
            topology_epoch: RouteEpoch(1),
            materialized_at_tick: Tick(1),
            publication_id: [7; 16],
            witness: contour_traits::contour_core::Authoritative {
                value: RouteWitness {
                    objective_privacy: RoutePrivacyClass::LinkConfidential,
                    delivered_privacy: RoutePrivacyClass::LinkConfidential,
                    objective_connectivity: RouteConnectivityClass::Repairable,
                    delivered_connectivity: RouteConnectivityClass::Repairable,
                    admission_profile: sample_admission_profile(),
                    topology_epoch: RouteEpoch(1),
                    degradation: RouteDegradation::None,
                },
                published_at_tick: Tick(1),
            },
        },
        admission: RouteAdmission {
            route_id: RouteId([3; 16]),
            objective,
            profile,
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: sample_admission_profile(),
                productive_step_bound: Limit::Limited(2),
                total_step_bound: Limit::Limited(4),
                route_cost: RouteCost {
                    message_count_max: Limit::Limited(4),
                    byte_count_max: Limit::Limited(1024),
                    hop_count: 2,
                    repair_attempt_count_max: Limit::Limited(1),
                    hold_bytes_reserved: Limit::Unlimited,
                    work_step_count_max: Limit::Limited(8),
                },
            },
            summary: RouteSummary {
                family: RoutingFamilyId::Mesh,
                privacy: RoutePrivacyClass::LinkConfidential,
                connectivity: RouteConnectivityClass::Repairable,
                transport_mix: vec![TransportClass::Proximity],
                hop_count_hint: KnownValue::Known(2),
                valid_for: TimeWindow {
                    start_tick: Tick(1),
                    end_tick: Tick(50),
                },
            },
            witness: RouteWitness {
                objective_privacy: RoutePrivacyClass::LinkConfidential,
                delivered_privacy: RoutePrivacyClass::LinkConfidential,
                objective_connectivity: RouteConnectivityClass::Repairable,
                delivered_connectivity: RouteConnectivityClass::Repairable,
                admission_profile: sample_admission_profile(),
                topology_epoch: RouteEpoch(1),
                degradation: RouteDegradation::None,
            },
        },
        lease: RouteLease {
            owner_node_id: contour_traits::contour_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(1),
            valid_for: TimeWindow {
                start_tick: Tick(1),
                end_tick: Tick(50),
            },
        },
        current_transition: RouteTransition::Established,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: contour_traits::contour_core::HealthScore(100),
            congestion_penalty_points: contour_traits::contour_core::PenaltyPoints(0),
            last_validated_at_tick: Tick(1),
        },
        progress: RouteProgressContract {
            productive_step_count_max: Limit::Limited(2),
            total_step_count_max: Limit::Limited(4),
            last_progress_at_tick: Tick(1),
            state: RouteProgressState::Pending,
        },
    }
}

fn empty_topology() -> TopologySnapshot {
    TopologySnapshot {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::new(),
        links: BTreeMap::new(),
        last_updated_at_tick: Tick(0),
    }
}

#[test]
fn route_family_extension_can_drive_candidate_to_installed_route() {
    let objective = sample_objective();
    let profile = sample_profile();
    let route = sample_route(objective.clone(), profile.clone());
    let mut family = StubFamily { route };
    let topology = Observed {
        fact: RoutingFact {
            value: empty_topology(),
            evidence_class: RoutingEvidenceClass::Observed,
            trust_class: PeerTrustClass::ControllerBound,
            observed_at_tick: Tick(0),
        },
    };
    let candidates = family.candidate_routes(&objective, &profile, &topology);
    let candidate = candidates.into_iter().next().expect("candidate");
    let check = family
        .check_candidate(&objective, &profile, &candidate)
        .expect("admission check");
    let admission = family
        .admit_route(&objective, &profile, candidate)
        .expect("admission");
    let mut installed = family.install_route(admission).expect("install");
    let commitments = family.route_commitments(&installed);
    let maintenance = family
        .maintain_route(&mut installed, RouteMaintenanceTrigger::LinkDegraded)
        .expect("maintenance");
    family.teardown(&installed.admission.route_id);

    assert_eq!(check.decision, AdmissionDecision::Admissible);
    assert_eq!(
        maintenance,
        RouteMaintenanceResult {
            transition: RouteTransition::Repaired,
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
    assert_eq!(installed.current_transition, RouteTransition::Repaired);
}
