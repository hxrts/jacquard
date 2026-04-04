//! Build an InstalledRoute from shared lifecycle types to verify structural coherence.

use contour_core::{
    AdaptiveRoutingProfile, AdmissionDecision, AdversaryRegime, BackendRouteRef, ClaimStrength,
    ConnectivityRegime, DeliveryModelClass, DeploymentProfileId, FailureModelClass,
    FamilyFallbackPolicy, HoldFallbackPolicy, InstalledRoute, KnownValue, Limit, NodeDensityClass,
    Observed, PeerTrustClass, ReachabilityState, RouteAdmission, RouteAdmissionCheck,
    RouteCandidate, RouteConnectivityClass, RouteCost, RouteDegradation, RouteEpoch, RouteEstimate,
    RouteHandle, RouteHealth, RouteId, RouteLease, RouteMaterializationProof, RoutePrivacyClass,
    RouteProgressContract, RouteProgressState, RouteReplacementPolicy, RouteSummary,
    RouteTransition, RouteWitness, RoutingAdmissionProfile, RoutingEvidenceClass, RoutingFact,
    RoutingFamilyId, RoutingObjective, RuntimeEnvelopeClass, ServiceFamily, Tick, TimeWindow,
    TransportClass,
};

fn sample_objective() -> RoutingObjective {
    RoutingObjective {
        destination: contour_core::DestinationId::Node(contour_core::NodeId([7; 32])),
        service_family: ServiceFamily::Move,
        target_privacy: RoutePrivacyClass::LinkConfidential,
        privacy_floor: RoutePrivacyClass::None,
        target_connectivity: RouteConnectivityClass::Repairable,
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Limited(contour_core::DurationMs(250)),
        privacy_priority: contour_core::PriorityPoints(10),
        connectivity_priority: contour_core::PriorityPoints(20),
    }
}

fn sample_admission_profile() -> RoutingAdmissionProfile {
    RoutingAdmissionProfile {
        delivery_model: DeliveryModelClass::FifoPerLink,
        failure_model: FailureModelClass::CrashStop,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Moderate,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ExactUnderAssumptions,
    }
}

fn sample_summary() -> RouteSummary {
    RouteSummary {
        family: RoutingFamilyId::Mesh,
        privacy: RoutePrivacyClass::LinkConfidential,
        connectivity: RouteConnectivityClass::Repairable,
        transport_mix: vec![TransportClass::Proximity, TransportClass::LocalArea],
        hop_count_hint: KnownValue::Known(3),
        valid_for: TimeWindow {
            start_tick: Tick(100),
            end_tick: Tick(500),
        },
    }
}

fn sample_witness(admission_profile: RoutingAdmissionProfile) -> RouteWitness {
    RouteWitness {
        objective_privacy: RoutePrivacyClass::LinkConfidential,
        delivered_privacy: RoutePrivacyClass::LinkConfidential,
        objective_connectivity: RouteConnectivityClass::Repairable,
        delivered_connectivity: RouteConnectivityClass::Repairable,
        admission_profile,
        topology_epoch: RouteEpoch(4),
        degradation: RouteDegradation::None,
    }
}

fn sample_route_cost() -> RouteCost {
    RouteCost {
        message_count_max: Limit::Limited(12),
        byte_count_max: Limit::Limited(2048),
        hop_count: 3,
        repair_attempt_count_max: Limit::Limited(2),
        hold_bytes_reserved: Limit::Limited(512),
        work_step_count_max: Limit::Limited(40),
    }
}

fn sample_route() -> (RouteCandidate, InstalledRoute) {
    let objective = sample_objective();
    let admission_profile = sample_admission_profile();
    let summary = sample_summary();
    let witness = sample_witness(admission_profile.clone());
    let candidate = RouteCandidate {
        summary: summary.clone(),
        estimate: Observed {
            fact: RoutingFact {
                value: RouteEstimate {
                    estimated_privacy: summary.privacy,
                    estimated_connectivity: summary.connectivity,
                    topology_epoch: RouteEpoch(4),
                    degradation: RouteDegradation::None,
                },
                evidence_class: RoutingEvidenceClass::Observed,
                trust_class: PeerTrustClass::ControllerBound,
                observed_at_tick: Tick(100),
            },
        },
        backend_ref: BackendRouteRef {
            family: RoutingFamilyId::Mesh,
            opaque_id: vec![1, 2, 3],
        },
    };
    let route = InstalledRoute {
        handle: RouteHandle {
            route_id: RouteId([5; 16]),
            topology_epoch: RouteEpoch(4),
            materialized_at_tick: Tick(101),
            publication_id: [4; 16],
        },
        materialization_proof: RouteMaterializationProof {
            route_id: RouteId([5; 16]),
            topology_epoch: RouteEpoch(4),
            materialized_at_tick: Tick(101),
            publication_id: [4; 16],
            witness: contour_core::Authoritative {
                value: witness.clone(),
                published_at_tick: Tick(101),
            },
        },
        admission: RouteAdmission {
            route_id: RouteId([5; 16]),
            objective,
            profile: AdaptiveRoutingProfile {
                selected_privacy: RoutePrivacyClass::LinkConfidential,
                selected_connectivity: RouteConnectivityClass::Repairable,
                deployment_profile: DeploymentProfileId::DenseInteractive,
                diversity_floor: 1,
                family_fallback_policy: FamilyFallbackPolicy::Allowed,
                route_replacement_policy: RouteReplacementPolicy::Allowed,
            },
            admission_check: RouteAdmissionCheck {
                decision: AdmissionDecision::Admissible,
                profile: admission_profile,
                productive_step_bound: Limit::Limited(4),
                total_step_bound: Limit::Limited(8),
                route_cost: sample_route_cost(),
            },
            summary,
            witness,
        },
        lease: RouteLease {
            owner_node_id: contour_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(4),
            valid_for: TimeWindow {
                start_tick: Tick(100),
                end_tick: Tick(500),
            },
        },
        current_transition: RouteTransition::Established,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: contour_core::HealthScore(900),
            congestion_penalty_points: contour_core::PenaltyPoints(12),
            last_validated_at_tick: Tick(110),
        },
        progress: RouteProgressContract {
            productive_step_count_max: Limit::Limited(6),
            total_step_count_max: Limit::Limited(12),
            last_progress_at_tick: Tick(110),
            state: RouteProgressState::Satisfied,
        },
    };

    (candidate, route)
}

#[test]
fn installed_route_can_be_built_from_shared_lifecycle_types() {
    let (candidate, route) = sample_route();

    assert_eq!(candidate.summary.family, RoutingFamilyId::Mesh);
    assert_eq!(
        candidate.estimate.fact.value.estimated_connectivity,
        RouteConnectivityClass::Repairable,
    );
    assert_eq!(route.admission.summary.transport_mix.len(), 2);
    assert_eq!(route.handle.route_id, RouteId([5; 16]));
    assert_eq!(
        route.materialization_proof.witness.value.topology_epoch,
        RouteEpoch(4),
    );
    assert_eq!(route.lease.owner_node_id, contour_core::NodeId([9; 32]));
    assert_eq!(route.current_transition, RouteTransition::Established);
}
