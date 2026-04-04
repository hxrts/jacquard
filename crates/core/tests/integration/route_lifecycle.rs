//! Build an InstalledRoute from shared lifecycle types to verify structural coherence.

use contour_core::{
    AdaptiveRoutingProfile, AdversaryRegime, BackendRouteRef, ClaimStrength, ConnectivityRegime,
    DeliveryModelClass, DeploymentProfileId, FailureModelClass, FamilyFallbackPolicy,
    HoldFallbackPolicy, InstalledRoute, NodeDensityClass, ReachabilityState, RouteAdmission,
    RouteAdmissionCheck, RouteCandidate, RouteConnectivityClass, RouteCost, RouteEpoch,
    RouteHealth, RouteId, RouteLease, RoutePrivacyClass, RouteProgressContract, RouteProgressState,
    RouteReplacementPolicy, RouteSummary, RouteTransition, RouteWitness, RoutingAdmissionProfile,
    RoutingFamilyId, RoutingObjective, RuntimeEnvelopeClass, ServiceFamily, Tick, TransportClass,
};

#[test]
fn installed_route_can_be_built_from_shared_lifecycle_types() {
    let objective = RoutingObjective {
        destination: contour_core::DestinationId::Node(contour_core::NodeId([7; 32])),
        service_family: ServiceFamily::Move,
        target_privacy: RoutePrivacyClass::LinkConfidential,
        privacy_floor: RoutePrivacyClass::None,
        target_connectivity: RouteConnectivityClass::Repairable,
        hold_fallback_policy: HoldFallbackPolicy::Allowed,
        latency_budget: Some(contour_core::DurationMs(250)),
        privacy_priority: contour_core::PriorityPoints(10),
        connectivity_priority: contour_core::PriorityPoints(20),
    };
    let admission_profile = RoutingAdmissionProfile {
        delivery_model: DeliveryModelClass::FifoPerLink,
        failure_model: FailureModelClass::CrashStop,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: NodeDensityClass::Moderate,
        connectivity_regime: ConnectivityRegime::Stable,
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ExactUnderAssumptions,
    };
    let summary = RouteSummary {
        family: RoutingFamilyId::Mesh,
        privacy: RoutePrivacyClass::LinkConfidential,
        connectivity: RouteConnectivityClass::Repairable,
        transport_mix: vec![TransportClass::Proximity, TransportClass::LocalArea],
        hop_count_hint: Some(3),
        expires_at: Tick(500),
    };
    let witness = RouteWitness {
        objective_privacy: RoutePrivacyClass::LinkConfidential,
        delivered_privacy: RoutePrivacyClass::LinkConfidential,
        objective_connectivity: RouteConnectivityClass::Repairable,
        delivered_connectivity: RouteConnectivityClass::Repairable,
        admission_profile: admission_profile.clone(),
        topology_epoch: RouteEpoch(4),
        degradation_reason: None,
    };
    let candidate = RouteCandidate {
        summary: summary.clone(),
        witness: witness.clone(),
        backend_ref: BackendRouteRef {
            family: RoutingFamilyId::Mesh,
            opaque_id: vec![1, 2, 3],
        },
    };
    let admission = RouteAdmission {
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
            admissible: true,
            profile: admission_profile,
            productive_step_bound: Some(4),
            total_step_bound: Some(8),
            route_cost: RouteCost {
                message_count_max: Some(12),
                byte_count_max: Some(2048),
                hop_count: 3,
                repair_attempt_count_max: Some(2),
                hold_bytes_reserved: Some(512),
                cpu_work_units_max: Some(40),
            },
            rejection_reason: None,
        },
        summary,
        witness,
    };
    let route = InstalledRoute {
        admission,
        lease: RouteLease {
            owner_node_id: contour_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(4),
            leased_at: Tick(100),
            expires_at: Tick(500),
        },
        current_transition: RouteTransition::Established,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: contour_core::HealthScore(900),
            congestion_penalty_points: contour_core::PenaltyPoints(12),
            last_validated_at: Tick(110),
        },
        progress: RouteProgressContract {
            productive_step_count_max: Some(6),
            total_step_count_max: Some(12),
            last_progress_at: Tick(110),
            state: RouteProgressState::Satisfied,
        },
    };

    assert_eq!(candidate.summary.family, RoutingFamilyId::Mesh);
    assert_eq!(route.admission.summary.transport_mix.len(), 2);
    assert_eq!(route.lease.owner_node_id, contour_core::NodeId([9; 32]));
    assert_eq!(route.current_transition, RouteTransition::Established);
}
