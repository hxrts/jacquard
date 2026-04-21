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
        let context = self.planning_context_for(objective);
        if context.reserve_for_underserved_objective {
            self.record_weakest_flow_search_reservation();
        }
        let outcome = corridor::plan_corridor_with_context(
            self.local_node_id,
            topology,
            objective,
            &self.config,
            &self.evidence,
            context,
        );
        self.record_planning_outcome(&outcome);
        match outcome {
            MercatorPlanningOutcome::Selected(corridor) => {
                if corridor.avoided_overloaded_broker(
                    self.config.bounds.broker_overload_pressure_threshold,
                ) {
                    self.record_overloaded_broker_penalty();
                }
                corridor
                    .candidate(objective, topology)
                    .into_iter()
                    .collect()
            }
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
        let context = self.planning_context_for(objective);
        let expected = corridor::candidate_for_with_context(
            self.local_node_id,
            topology,
            objective,
            &self.config,
            &self.evidence,
            context,
        )?;
        if expected.backend_ref != candidate.backend_ref || expected.route_id != candidate.route_id
        {
            return corridor::check_candidate(
                self.local_node_id,
                topology,
                objective,
                profile,
                candidate,
                &self.config,
                &self.evidence,
            )
            .map_err(RouteError::from);
        }
        Ok(corridor::admit_candidate(
            self.local_node_id,
            topology,
            objective,
            profile,
            &expected,
            &self.config,
            &self.evidence,
        )?
        .admission_check)
    }

    fn admit_route(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let context = self.planning_context_for(objective);
        let expected = corridor::candidate_for_with_context(
            self.local_node_id,
            topology,
            objective,
            &self.config,
            &self.evidence,
            context,
        )?;
        if expected.backend_ref != candidate.backend_ref || expected.route_id != candidate.route_id
        {
            return corridor::admit_candidate(
                self.local_node_id,
                topology,
                objective,
                profile,
                &candidate,
                &self.config,
                &self.evidence,
            )
            .map_err(RouteError::from);
        }
        corridor::admit_candidate(
            self.local_node_id,
            topology,
            objective,
            profile,
            &expected,
            &self.config,
            &self.evidence,
        )
        .map_err(RouteError::from)
    }
}
