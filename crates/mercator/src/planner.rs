//! `RoutingEnginePlanner` impl for `MercatorEngine`.

use jacquard_core::{
    Configuration, Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteError,
    RoutingEngineCapabilities, RoutingEngineId, RoutingObjective, SelectedRoutingParameters,
};
use jacquard_traits::RoutingEnginePlanner;

use crate::{
    corridor::{self, MercatorPlanningOutcome},
    MercatorEngine, MERCATOR_CAPABILITIES, MERCATOR_ENGINE_ID,
};

impl RoutingEnginePlanner for MercatorEngine {
    fn engine_id(&self) -> RoutingEngineId {
        MERCATOR_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        MERCATOR_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        let outcome = corridor::plan_corridor(
            self.local_node_id,
            topology,
            objective,
            &self.config,
            &self.evidence,
        );
        self.record_planning_outcome(&outcome);
        match outcome {
            MercatorPlanningOutcome::Selected(corridor) => corridor
                .candidate(objective, topology)
                .into_iter()
                .collect(),
            MercatorPlanningOutcome::NoCandidate | MercatorPlanningOutcome::Inadmissible => {
                Vec::new()
            }
        }
    }

    fn check_candidate(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        corridor::check_candidate(
            self.local_node_id,
            topology,
            objective,
            profile,
            candidate,
            &self.config,
            &self.evidence,
        )
        .map_err(RouteError::from)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        corridor::admit_candidate(
            self.local_node_id,
            topology,
            objective,
            profile,
            &candidate,
            &self.config,
            &self.evidence,
        )
        .map_err(RouteError::from)
    }
}
