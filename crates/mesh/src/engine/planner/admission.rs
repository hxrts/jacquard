//! Admission judgment for mesh candidates.

use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionAssumptions, AdmissionDecision, Limit,
    RouteAdmissionCheck, RouteAdmissionRejection, RouteCost, RoutePartitionClass,
    RouteRepairClass, RouteSummary, RoutingObjective,
};

pub(super) fn mesh_admission_check(
    objective: &RoutingObjective,
    profile: &AdaptiveRoutingProfile,
    summary: &RouteSummary,
    route_cost: &RouteCost,
    assumptions: &AdmissionAssumptions,
    committee_status: &super::super::support::CommitteeStatus,
) -> RouteAdmissionCheck {
    let backend_unavailable = profile.selected_connectivity.partition
        == RoutePartitionClass::PartitionTolerant
        && summary.connectivity.partition != RoutePartitionClass::PartitionTolerant
        || matches!(
            committee_status,
            super::super::support::CommitteeStatus::SelectorFailed
        );
    let decision = if summary.protection < objective.protection_floor {
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    } else if profile.selected_connectivity.repair == RouteRepairClass::Repairable
        && summary.connectivity.repair != RouteRepairClass::Repairable
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible)
    } else if backend_unavailable {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    } else {
        AdmissionDecision::Admissible
    };

    RouteAdmissionCheck {
        decision,
        profile: assumptions.clone(),
        productive_step_bound: Limit::Bounded(route_cost.hop_count.into()),
        total_step_bound: Limit::Bounded(route_cost.hop_count.into()),
        route_cost: route_cost.clone(),
    }
}
