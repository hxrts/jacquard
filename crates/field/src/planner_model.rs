//! Engine-owned pure planner model surface for Field.

use jacquard_core::{
    BackendRouteId, Configuration, DestinationId, NodeId, Observation, RouteAdmission,
    RouteCandidate, RouteError, SelectedRoutingParameters, Tick,
};
use jacquard_traits::{RoutingEnginePlanner, RoutingEnginePlannerModel};

use crate::{
    route::decode_backend_token,
    state::{DestinationInterestClass, HopBand, NeighborContinuation, SupportBucket},
    FieldEngine,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldPlannerSeed {
    pub local_node_id: NodeId,
    pub selected_neighbor: NodeId,
    pub observed_at_tick: Tick,
}

pub struct FieldPlannerModel;

#[must_use]
pub fn selected_neighbor_from_backend_route_id(
    backend_route_id: &BackendRouteId,
) -> Option<NodeId> {
    decode_backend_token(backend_route_id).map(|token| token.selected_neighbor)
}

impl RoutingEnginePlannerModel for FieldPlannerModel {
    type PlannerSnapshot = FieldPlannerSeed;
    type PlannerCandidate = RouteCandidate;
    type PlannerAdmission = RouteAdmission;

    fn candidate_routes_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate> {
        let engine = seeded_planner_engine(snapshot, objective);
        engine
            .planning_artifacts(&engine.planner_snapshot(), objective, profile, topology)
            .map(|artifacts| vec![artifacts.candidate])
            .unwrap_or_default()
    }

    fn admit_route_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &Self::PlannerCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<Self::PlannerAdmission, RouteError> {
        let engine = seeded_planner_engine(snapshot, objective);
        engine.admit_route(objective, profile, candidate.clone(), topology)
    }
}

fn seeded_planner_engine(
    snapshot: &FieldPlannerSeed,
    objective: &jacquard_core::RoutingObjective,
) -> FieldEngine<(), ()> {
    let destination = match objective.destination {
        DestinationId::Node(destination) => destination,
        DestinationId::Gateway(_) | DestinationId::Service(_) => {
            return FieldEngine::new(snapshot.local_node_id, (), ());
        }
    };
    let mut engine = FieldEngine::new(snapshot.local_node_id, (), ());
    engine.state.note_tick(snapshot.observed_at_tick);
    let state = engine.state.upsert_destination_interest(
        &objective.destination,
        DestinationInterestClass::Transit,
        snapshot.observed_at_tick,
    );
    state.posterior.top_corridor_mass = SupportBucket::new(860);
    state.corridor_belief.expected_hop_band = HopBand::new(1, 2);
    state.corridor_belief.delivery_support = SupportBucket::new(780);
    state.corridor_belief.retention_affinity = SupportBucket::new(640);
    state.frontier = state.frontier.clone().insert(NeighborContinuation {
        neighbor_id: snapshot.selected_neighbor,
        net_value: SupportBucket::new(920),
        downstream_support: SupportBucket::new(840),
        expected_hop_band: HopBand::new(1, 2),
        freshness: snapshot.observed_at_tick,
    });
    if destination == snapshot.selected_neighbor {
        state.corridor_belief.expected_hop_band = HopBand::new(1, 1);
    }
    engine
}
