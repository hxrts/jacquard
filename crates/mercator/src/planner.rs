//! `RoutingEnginePlanner` impl for `MercatorEngine`.

use jacquard_core::{
    Configuration, Observation, RouteAdmission, RouteAdmissionCheck, RouteAdmissionRejection,
    RouteCandidate, RouteError, RouteSelectionError, RoutingEngineCapabilities, RoutingEngineId,
    RoutingObjective, SelectedRoutingParameters,
};
use jacquard_traits::RoutingEnginePlanner;

use crate::{MercatorEngine, MERCATOR_CAPABILITIES, MERCATOR_ENGINE_ID};

impl RoutingEnginePlanner for MercatorEngine {
    fn engine_id(&self) -> RoutingEngineId {
        MERCATOR_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        MERCATOR_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        Vec::new()
    }

    fn check_candidate(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: &RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        Err(RouteSelectionError::Inadmissible(RouteAdmissionRejection::BackendUnavailable).into())
    }

    fn admit_route(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _candidate: RouteCandidate,
        _topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        Err(RouteSelectionError::Inadmissible(RouteAdmissionRejection::BackendUnavailable).into())
    }
}
