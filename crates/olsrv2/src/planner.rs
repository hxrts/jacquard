//! `RoutingEnginePlanner` impl for `OlsrV2Engine`.

use jacquard_core::{
    AdmissionDecision, Configuration, DestinationId, Observation, RouteAdmission,
    RouteAdmissionCheck, RouteAdmissionRejection, RouteCandidate, RouteError, RouteSelectionError,
    RoutingEngineCapabilities, RoutingEngineId, SelectedRoutingParameters,
};
use jacquard_traits::{
    RoutingEnginePlanner, RoutingEnginePlannerModel, TimeEffects, TransportSenderEffects,
};

use crate::{
    private_state::{admission_for_candidate, candidate_for_snapshot},
    OlsrPlannerSnapshot, OlsrV2Engine, OLSRV2_CAPABILITIES, OLSRV2_ENGINE_ID,
};

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
        candidate_routes_from_snapshot(&self.planner_snapshot(), objective, topology)
    }

    fn check_candidate(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<RouteAdmissionCheck, RouteError> {
        let admission =
            self.current_candidate_admission(objective, profile, candidate, topology)?;
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
        let admission =
            self.current_candidate_admission(objective, profile, &candidate, topology)?;
        if let AdmissionDecision::Rejected(reason) = admission.admission_check.decision {
            return Err(RouteSelectionError::Inadmissible(reason).into());
        }
        Ok(admission)
    }
}

impl<Transport, Effects> OlsrV2Engine<Transport, Effects>
where
    Transport: TransportSenderEffects,
    Effects: TimeEffects,
{
    fn current_candidate_admission(
        &self,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &RouteCandidate,
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
        current_candidate_admission_from_snapshot(
            &self.planner_snapshot(),
            objective,
            profile,
            candidate,
            topology,
        )
    }
}

impl<Transport, Effects> RoutingEnginePlannerModel for OlsrV2Engine<Transport, Effects> {
    type PlannerSnapshot = OlsrPlannerSnapshot;
    type PlannerCandidate = RouteCandidate;
    type PlannerAdmission = RouteAdmission;

    fn candidate_routes_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate> {
        candidate_routes_from_snapshot(snapshot, objective, topology)
    }

    fn admit_route_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &Self::PlannerCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<Self::PlannerAdmission, RouteError> {
        admit_route_from_snapshot(snapshot, objective, profile, candidate, topology)
    }
}

#[must_use = "candidate projection from a planner snapshot must be consumed by simulator or planner checks"]
pub fn candidate_routes_from_snapshot(
    snapshot: &OlsrPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    topology: &Observation<Configuration>,
) -> Vec<RouteCandidate> {
    let DestinationId::Node(destination) = objective.destination else {
        return Vec::new();
    };
    if !destination_supports_objective(topology, destination, objective.service_kind) {
        return Vec::new();
    }
    snapshot
        .best_next_hops
        .get(&destination)
        .map(|best| vec![candidate_for_snapshot(snapshot, objective, best)])
        .unwrap_or_default()
}

pub fn admit_route_from_snapshot(
    snapshot: &OlsrPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
    topology: &Observation<Configuration>,
) -> Result<RouteAdmission, RouteError> {
    current_candidate_admission_from_snapshot(snapshot, objective, profile, candidate, topology)
}

fn current_candidate_admission_from_snapshot(
    snapshot: &OlsrPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: &RouteCandidate,
    topology: &Observation<Configuration>,
) -> Result<RouteAdmission, RouteError> {
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    if !destination_supports_objective(topology, destination, objective.service_kind) {
        return Err(
            RouteSelectionError::Inadmissible(RouteAdmissionRejection::BackendUnavailable).into(),
        );
    }
    let Some(best) = snapshot.best_next_hops.get(&destination) else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let expected = candidate_for_snapshot(snapshot, objective, best);
    if expected.backend_ref != candidate.backend_ref {
        return Err(
            RouteSelectionError::Inadmissible(RouteAdmissionRejection::BackendUnavailable).into(),
        );
    }
    Ok(admission_for_candidate(objective, profile, &expected))
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
