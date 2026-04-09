use jacquard_core::{
    Configuration, Observation, RouteAdmission, RouteAdmissionCheck, RouteCandidate,
    RouteError, RoutingEngineCapabilities, RoutingEngineId, SelectedRoutingParameters,
};
use jacquard_traits::RoutingEnginePlanner;

use crate::{FieldEngine, FIELD_CAPABILITIES, FIELD_ENGINE_ID};

impl<Transport, Effects> RoutingEnginePlanner for FieldEngine<Transport, Effects> {
    fn engine_id(&self) -> RoutingEngineId {
        FIELD_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        FIELD_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        _objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }

    fn admit_route(
        &self,
        _objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        Err(jacquard_core::RouteSelectionError::NoCandidate.into())
    }
}
