//! Final admission judgment for an already-derived pathway candidate.
//!
//! Control flow: candidate assembly produces a shared `RouteSummary`, a
//! computed `RouteCost`, and any committee outcome. This module folds those
//! pieces together with the requested objective/profile and returns the
//! typed `RouteAdmissionCheck` that `check_candidate` and `admit_route` both
//! expose. The single exported function `pathway_admission_check` is pure:
//! it has no side effects and makes no routing decisions beyond the
//! admission verdict. Rejection reasons are typed
//! (`ProtectionFloorUnsatisfied`, `BranchingInfeasible`, `BackendUnavailable`)
//! so the router can surface structured diagnostics rather than opaque
//! failures.

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, Limit, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteCost, RoutePartitionClass, RouteRepairClass,
    RouteSummary, RoutingObjective, SelectedRoutingParameters,
};

pub(super) fn pathway_admission_check(
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    summary: &RouteSummary,
    route_cost: &RouteCost,
    assumptions: &AdmissionAssumptions,
    committee_status: &super::super::support::CommitteeStatus,
) -> RouteAdmissionCheck {
    // Two distinct cases share this rejection: (1) profile requires partition
    // tolerance but this route cannot provide it; (2) the committee selector
    // failed, making local coordination impossible.
    let backend_unavailable = (profile.selected_connectivity.partition
        == RoutePartitionClass::PartitionTolerant
        && summary.connectivity.partition != RoutePartitionClass::PartitionTolerant)
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
