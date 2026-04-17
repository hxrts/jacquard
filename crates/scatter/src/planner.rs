//! `RoutingEnginePlanner` impl for `ScatterEngine`.

use jacquard_core::{
    AdmissionDecision, Configuration, Observation, RouteAdmission, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteCandidate, RouteError, RouteSelectionError,
    RoutingEngineCapabilities, RoutingEngineId, SelectedRoutingParameters,
};
use jacquard_traits::{RoutingEnginePlanner, TimeEffects, TransportSenderEffects};

use crate::{
    support::{
        admission_for_snapshot, candidate_for_snapshot, decode_backend_token, objective_supported,
    },
    ScatterEngine, SCATTER_CAPABILITIES, SCATTER_ENGINE_ID,
};

impl<Transport, Effects> RoutingEnginePlanner for ScatterEngine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn engine_id(&self) -> RoutingEngineId {
        SCATTER_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        SCATTER_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        if !objective_supported(topology, objective, topology.observed_at_tick) {
            return Vec::new();
        }
        let snapshot = self.planner_snapshot();
        candidate_for_snapshot(topology, objective, &snapshot)
            .map(|candidate| vec![candidate])
            .unwrap_or_default()
    }

    fn check_candidate(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        if !objective_supported(topology, objective, topology.observed_at_tick) {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let Some(token) = decode_backend_token(&candidate.backend_ref.backend_route_id) else {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        };
        if candidate.backend_ref.engine != SCATTER_ENGINE_ID
            || token.destination != objective.destination
            || token.service_kind != objective.service_kind
            || token.topology_epoch != topology.value.epoch
        {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let snapshot = self.planner_snapshot();
        let expected =
            candidate_for_snapshot(topology, objective, &snapshot).map_err(RouteError::from)?;
        if expected.backend_ref != candidate.backend_ref {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let admission = admission_for_snapshot(topology, objective, profile, expected, &snapshot);
        if let AdmissionDecision::Rejected(reason) = admission.admission_check.decision {
            return Err(RouteSelectionError::Inadmissible(reason).into());
        }
        Ok(admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        if !objective_supported(topology, objective, topology.observed_at_tick) {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let Some(token) = decode_backend_token(&candidate.backend_ref.backend_route_id) else {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        };
        if candidate.backend_ref.engine != SCATTER_ENGINE_ID
            || token.destination != objective.destination
            || token.service_kind != objective.service_kind
            || token.topology_epoch != topology.value.epoch
        {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let snapshot = self.planner_snapshot();
        let expected =
            candidate_for_snapshot(topology, objective, &snapshot).map_err(RouteError::from)?;
        if expected.backend_ref != candidate.backend_ref {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        Ok(admission_for_snapshot(
            topology, objective, profile, expected, &snapshot,
        ))
    }
}
