//! Drive a stub RouteFamilyExtension through the full candidate-to-teardown lifecycle.

use contour_traits::{
    contour_core::{
        AdaptiveRoutingProfile, AdversaryRegime, BackendRouteRef, ClaimStrength,
        ConnectivityRegime, DeliveryModelClass, DeploymentProfileId, FailureModelClass,
        FamilyFallbackPolicy, InstalledRoute, NodeDensityClass, ReachabilityState, RouteAdmission,
        RouteAdmissionCheck, RouteCandidate, RouteConnectivityClass, RouteCost, RouteEpoch,
        RouteHealth, RouteId, RouteLease, RouteMaintenanceDisposition, RouteMaintenanceTrigger,
        RoutePrivacyClass, RouteProgressContract, RouteProgressState, RouteReplacementPolicy,
        RouteSummary, RouteTransition, RouteWitness, RoutingAdmissionProfile,
        RoutingFamilyCapabilities, RoutingFamilyId, RoutingObjective, RuntimeEnvelopeClass,
        ServiceFamily, Tick, TopologySnapshot, TransportClass,
    },
    RouteFamilyExtension,
};
use std::collections::BTreeMap;

struct StubFamily {
    route: InstalledRoute,
}

impl RouteFamilyExtension for StubFamily {
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
        _topology: &TopologySnapshot,
    ) -> Vec<RouteCandidate> {
        vec![RouteCandidate {
            summary: self.route.admission.summary.clone(),
            witness: self.route.admission.witness.clone(),
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

    fn maintain_route(
        &mut self,
        route: &mut InstalledRoute,
        trigger: RouteMaintenanceTrigger,
    ) -> Result<RouteMaintenanceDisposition, contour_traits::contour_core::RouteError> {
        route.current_transition = RouteTransition::Repaired;
        let disposition = match trigger {
            RouteMaintenanceTrigger::LinkDegraded => RouteMaintenanceDisposition::Repaired,
            _ => RouteMaintenanceDisposition::Continue,
        };
        Ok(disposition)
    }

    fn teardown(&mut self, route_id: &RouteId) {
        assert_eq!(*route_id, self.route.admission.route_id);
    }
}

#[test]
fn route_family_extension_can_drive_candidate_to_installed_route() {
    let objective = RoutingObjective {
        destination: contour_traits::contour_core::DestinationId::Node(
            contour_traits::contour_core::NodeId([2; 32]),
        ),
        service_family: ServiceFamily::Move,
        target_privacy: RoutePrivacyClass::LinkConfidential,
        privacy_floor: RoutePrivacyClass::None,
        target_connectivity: RouteConnectivityClass::Repairable,
        hold_fallback_policy: contour_traits::contour_core::HoldFallbackPolicy::Allowed,
        latency_budget: Some(contour_traits::contour_core::DurationMs(100)),
        privacy_priority: contour_traits::contour_core::PriorityPoints(1),
        connectivity_priority: contour_traits::contour_core::PriorityPoints(2),
    };
    let profile = AdaptiveRoutingProfile {
        selected_privacy: RoutePrivacyClass::LinkConfidential,
        selected_connectivity: RouteConnectivityClass::Repairable,
        deployment_profile: DeploymentProfileId::SparseLowPower,
        diversity_floor: 1,
        family_fallback_policy: FamilyFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    };
    let route = InstalledRoute {
        admission: RouteAdmission {
            route_id: RouteId([3; 16]),
            objective: objective.clone(),
            profile: profile.clone(),
            admission_check: RouteAdmissionCheck {
                admissible: true,
                profile: RoutingAdmissionProfile {
                    delivery_model: DeliveryModelClass::FifoPerLink,
                    failure_model: FailureModelClass::CrashStop,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Sparse,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: ClaimStrength::ExactUnderAssumptions,
                },
                productive_step_bound: Some(2),
                total_step_bound: Some(4),
                route_cost: RouteCost {
                    message_count_max: Some(4),
                    byte_count_max: Some(1024),
                    hop_count: 2,
                    repair_attempt_count_max: Some(1),
                    hold_bytes_reserved: None,
                    cpu_work_units_max: Some(8),
                },
                rejection_reason: None,
            },
            summary: RouteSummary {
                family: RoutingFamilyId::Mesh,
                privacy: RoutePrivacyClass::LinkConfidential,
                connectivity: RouteConnectivityClass::Repairable,
                transport_mix: vec![TransportClass::Proximity],
                hop_count_hint: Some(2),
                expires_at: Tick(50),
            },
            witness: RouteWitness {
                objective_privacy: RoutePrivacyClass::LinkConfidential,
                delivered_privacy: RoutePrivacyClass::LinkConfidential,
                objective_connectivity: RouteConnectivityClass::Repairable,
                delivered_connectivity: RouteConnectivityClass::Repairable,
                admission_profile: RoutingAdmissionProfile {
                    delivery_model: DeliveryModelClass::FifoPerLink,
                    failure_model: FailureModelClass::CrashStop,
                    runtime_envelope: RuntimeEnvelopeClass::Canonical,
                    node_density_class: NodeDensityClass::Sparse,
                    connectivity_regime: ConnectivityRegime::Stable,
                    adversary_regime: AdversaryRegime::Cooperative,
                    claim_strength: ClaimStrength::ExactUnderAssumptions,
                },
                topology_epoch: RouteEpoch(1),
                degradation_reason: None,
            },
        },
        lease: RouteLease {
            owner_node_id: contour_traits::contour_core::NodeId([9; 32]),
            lease_epoch: RouteEpoch(1),
            leased_at: Tick(1),
            expires_at: Tick(50),
        },
        current_transition: RouteTransition::Established,
        health: RouteHealth {
            reachability_state: ReachabilityState::Reachable,
            stability_score: contour_traits::contour_core::HealthScore(100),
            congestion_penalty_points: contour_traits::contour_core::PenaltyPoints(0),
            last_validated_at: Tick(1),
        },
        progress: RouteProgressContract {
            productive_step_count_max: Some(2),
            total_step_count_max: Some(4),
            last_progress_at: Tick(1),
            state: RouteProgressState::Pending,
        },
    };
    let mut family = StubFamily { route };
    let topology = TopologySnapshot {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::new(),
        links: BTreeMap::new(),
        last_updated_at: Tick(0),
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
    let maintenance = family
        .maintain_route(&mut installed, RouteMaintenanceTrigger::LinkDegraded)
        .expect("maintenance");
    family.teardown(&installed.admission.route_id);

    assert!(check.admissible);
    assert_eq!(maintenance, RouteMaintenanceDisposition::Repaired);
    assert_eq!(installed.current_transition, RouteTransition::Repaired);
}
