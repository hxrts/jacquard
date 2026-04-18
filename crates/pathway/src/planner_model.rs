//! Engine-owned pure planner model surface for Pathway.

use jacquard_core::{
    Configuration, NodeId, Observation, RouteAdmission, RouteCandidate, RouteError,
    SelectedRoutingParameters,
};
use jacquard_traits::{Blake3Hashing, RoutingEnginePlanner, RoutingEnginePlannerModel};

use crate::{DeterministicPathwayTopologyModel, PathwayEngine};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathwayPlannerSeed {
    pub local_node_id: NodeId,
}

pub struct PathwayPlannerModel;

impl RoutingEnginePlannerModel for PathwayPlannerModel {
    type PlannerSnapshot = PathwayPlannerSeed;
    type PlannerCandidate = RouteCandidate;
    type PlannerAdmission = RouteAdmission;

    fn candidate_routes_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate> {
        let engine = PathwayEngine::without_committee_selector(
            snapshot.local_node_id,
            DeterministicPathwayTopologyModel::new(),
            (),
            (),
            (),
            Blake3Hashing,
        );
        engine.candidate_routes(objective, profile, topology)
    }

    fn admit_route_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &Self::PlannerCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<Self::PlannerAdmission, RouteError> {
        let engine = PathwayEngine::without_committee_selector(
            snapshot.local_node_id,
            DeterministicPathwayTopologyModel::new(),
            (),
            (),
            (),
            Blake3Hashing,
        );
        engine.admit_route(objective, profile, candidate.clone(), topology)
    }
}
