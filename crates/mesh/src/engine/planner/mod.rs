//! `RoutingEnginePlanner` implementation for `MeshEngine`.
//!
//! Candidate production runs a five-step deterministic pipeline: metric-aware
//! path search from the local node, filter by engine capability and objective
//! match, derive a self-contained `BackendRouteId` plan token plus admission
//! check, sort by path metric, mesh-private topology-model preference, and
//! deterministic order key, then truncate to
//! `MESH_CANDIDATE_COUNT_MAX`. `check_candidate` and `admit_route` take
//! topology explicitly and re-derive from the plan token on cache miss,
//! so the candidate cache is an optimization rather than a required
//! piece of engine state.

mod admission;
mod candidates;
mod pathing;
mod publishing;
mod scoring;

use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionDecision, Configuration, Observation,
    RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteError,
    RouteSelectionError, RoutingObjective,
};
use jacquard_traits::{
    MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess, RoutingEnginePlanner,
};

use super::{
    MeshEngine, MeshHasherBounds, MeshSelectorBounds, MESH_CAPABILITIES, MESH_ENGINE_ID,
};
use crate::topology::objective_matches_node;

const PATH_METRIC_BASE_HOP_COST: u32 = 1_000;
const PATH_METRIC_DELIVERY_PENALTY_WEIGHT: u32 = 2;
const PATH_METRIC_LOSS_PENALTY_WEIGHT: u32 = 2;
const PATH_METRIC_SYMMETRY_PENALTY_WEIGHT: u32 = 1;
const PATH_METRIC_PROTOCOL_REPEAT_PENALTY: u32 = 125;
const PATH_METRIC_DIVERSITY_BONUS: u32 = 75;
const PATH_METRIC_DEFERRED_DELIVERY_BONUS: u32 = 150;

impl<Topology, Transport, Retention, Effects, Hasher, Selector> RoutingEnginePlanner
    for MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::MeshTopologyBounds,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Hasher: MeshHasherBounds,
    Selector: MeshSelectorBounds,
{
    fn engine_id(&self) -> jacquard_core::RoutingEngineId {
        MESH_ENGINE_ID
    }

    fn capabilities(&self) -> jacquard_core::RoutingEngineCapabilities {
        MESH_CAPABILITIES.clone()
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        let mut cached = self.collect_candidates(objective, profile, topology);
        self.sort_candidates(objective, topology, &mut cached);
        self.cache_and_publish_candidates(cached)
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        // Cache hit is the fast path. On cache miss (e.g. after an
        // engine_tick cleared the cache) we re-derive from the plan
        // token against the supplied topology. Same inputs produce the
        // same admission check either way.
        if let Some(cached) = self
            .candidate_cache
            .borrow()
            .get(&candidate.backend_ref.backend_route_id)
        {
            return Ok(cached.admission_check.clone());
        }
        let derived = self.derive_candidate_from_backend_ref(
            objective,
            profile,
            topology,
            &candidate.backend_ref.backend_route_id,
        )?;
        Ok(derived.admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let cached = self
            .candidate_cache
            .borrow()
            .get(&candidate.backend_ref.backend_route_id)
            .cloned()
            .map_or_else(
                || {
                    self.derive_candidate_from_backend_ref(
                        objective,
                        profile,
                        topology,
                        &candidate.backend_ref.backend_route_id,
                    )
                },
                Ok,
            )?;

        match cached.admission_check.decision {
            | AdmissionDecision::Admissible => Ok(RouteAdmission {
                route_id: cached.route_id,
                backend_ref: candidate.backend_ref,
                objective: objective.clone(),
                profile: profile.clone(),
                admission_check: cached.admission_check,
                summary: cached.summary,
                witness: cached.witness,
            }),
            | AdmissionDecision::Rejected(rejection) => {
                Err(RouteSelectionError::Inadmissible(rejection).into())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        RouteAdmissionRejection, RoutePartitionClass, RouteProtectionClass,
        RouteRepairClass,
    };

    use super::{admission::mesh_admission_check, *};
    use crate::engine::{support::CommitteeStatus, test_helpers::*};

    #[test]
    fn admission_check_rejects_protection_floor_regression() {
        let objective = objective_with_floor(RouteProtectionClass::TopologyProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
            &CommitteeStatus::NotApplicable,
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(
                RouteAdmissionRejection::ProtectionFloorUnsatisfied
            ),
        );
    }

    #[test]
    fn admission_check_rejects_repair_mismatch() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::BestEffort,
            RoutePartitionClass::ConnectedOnly,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
            &CommitteeStatus::NotApplicable,
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible),
        );
    }

    #[test]
    fn admission_check_rejects_partition_mismatch() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
            &CommitteeStatus::NotApplicable,
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable),
        );
    }

    #[test]
    fn admission_check_admits_matching_profile_and_summary() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
            &CommitteeStatus::NotApplicable,
        );
        assert_eq!(check.decision, AdmissionDecision::Admissible);
    }

    #[test]
    fn admission_check_preserves_protection_failure_over_committee_failure() {
        let objective = objective_with_floor(RouteProtectionClass::TopologyProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
            &CommitteeStatus::SelectorFailed,
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(
                RouteAdmissionRejection::ProtectionFloorUnsatisfied
            ),
        );
    }

    #[test]
    fn admission_check_rejects_committee_selector_failure_after_hard_invariants_pass() {
        let objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let check = mesh_admission_check(
            &objective,
            &profile,
            &summary,
            &unit_route_cost(),
            &neutral_assumptions(),
            &CommitteeStatus::SelectorFailed,
        );
        assert_eq!(
            check.decision,
            AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable),
        );
    }
}
