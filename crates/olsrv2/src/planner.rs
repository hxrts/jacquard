//! `RoutingEnginePlanner` impl for `OlsrV2Engine`.

use jacquard_core::{
    AdmissionDecision, Configuration, DestinationId, Observation, RouteAdmission,
    RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate, RouteError, RouteSelectionError,
    RoutingEngineCapabilities, RoutingEngineId, SelectedRoutingParameters,
};
use jacquard_traits::{RoutingEnginePlanner, TimeEffects, TransportSenderEffects};

use crate::{OlsrV2Engine, OLSRV2_CAPABILITIES, OLSRV2_ENGINE_ID};

impl<Transport, Effects> RoutingEnginePlanner for OlsrV2Engine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn engine_id(&self) -> RoutingEngineId {
        OLSRV2_ENGINE_ID
    }

    fn capabilities(&self) -> RoutingEngineCapabilities {
        OLSRV2_CAPABILITIES
    }

    fn candidate_routes(
        &self,
        objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<RouteCandidate> {
        let DestinationId::Node(destination) = objective.destination else {
            return Vec::new();
        };
        if !destination_supports_objective(topology, destination, objective.service_kind) {
            return Vec::new();
        }
        self.best_next_hops
            .get(&destination)
            .map(|best| vec![self.candidate_for(objective, best)])
            .unwrap_or_default()
    }

    fn check_candidate(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        self.admit_route(objective, profile, candidate.clone(), topology)
            .map(|admission| admission.admission_check)
    }

    fn admit_route(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmission, RouteError> {
        let DestinationId::Node(destination) = objective.destination else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        if !destination_supports_objective(topology, destination, objective.service_kind) {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let Some(best) = self.best_next_hops.get(&destination) else {
            return Err(RouteSelectionError::NoCandidate.into());
        };
        let expected = self.candidate_for(objective, best);
        if expected.backend_ref != candidate.backend_ref {
            return Err(RouteSelectionError::Inadmissible(
                RouteAdmissionRejection::BackendUnavailable,
            )
            .into());
        }
        let admission = self.admission_for(objective, profile, &expected);
        if let AdmissionDecision::Rejected(reason) = admission.admission_check.decision {
            return Err(RouteSelectionError::Inadmissible(reason).into());
        }
        Ok(admission)
    }
}

fn destination_supports_objective(
    topology: &Observation<Configuration>,
    destination: jacquard_core::NodeId,
    service_kind: jacquard_core::RouteServiceKind,
) -> bool {
    topology
        .value
        .nodes
        .get(&destination)
        .map(|node| {
            node.profile.services.iter().any(|service| {
                service.service_kind == service_kind
                    && service.routing_engines.contains(&OLSRV2_ENGINE_ID)
            })
        })
        .unwrap_or(false)
}
