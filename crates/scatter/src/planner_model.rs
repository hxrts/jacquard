//! Engine-owned pure planner model surface for Scatter.

use jacquard_core::{
    Configuration, NodeId, Observation, RouteAdmission, RouteCandidate, RouteError,
    SelectedRoutingParameters, Tick, TransportError,
};
use jacquard_traits::{
    effect_handler, RoutingEnginePlanner, RoutingEnginePlannerModel, TimeEffects,
    TransportSenderEffects,
};

use crate::{ScatterEngine, ScatterEngineConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScatterPlannerSeed {
    pub local_node_id: NodeId,
    pub observed_at_tick: Tick,
    pub config: ScatterEngineConfig,
}

pub struct ScatterPlannerModel;

struct NullTransport;

#[effect_handler]
impl TransportSenderEffects for NullTransport {
    fn send_transport(
        &mut self,
        _endpoint: &jacquard_core::LinkEndpoint,
        _payload: &[u8],
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

struct FixedTime {
    now: Tick,
}

#[effect_handler]
impl TimeEffects for FixedTime {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

impl RoutingEnginePlannerModel for ScatterPlannerModel {
    type PlannerSnapshot = ScatterPlannerSeed;
    type PlannerCandidate = RouteCandidate;
    type PlannerAdmission = RouteAdmission;

    fn candidate_routes_from_snapshot(
        snapshot: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate> {
        let engine = ScatterEngine::with_config(
            snapshot.local_node_id,
            NullTransport,
            FixedTime {
                now: snapshot.observed_at_tick,
            },
            snapshot.config,
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
        let engine = ScatterEngine::with_config(
            snapshot.local_node_id,
            NullTransport,
            FixedTime {
                now: snapshot.observed_at_tick,
            },
            snapshot.config,
        );
        engine.admit_route(objective, profile, candidate.clone(), topology)
    }
}
