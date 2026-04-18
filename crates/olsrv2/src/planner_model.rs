//! Engine-owned pure planner model surface for OLSRv2.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, NodeId, Observation, RouteAdmission, RouteCandidate, RouteDegradation,
    RouteError, SelectedRoutingParameters, TransportKind,
};
use jacquard_traits::RoutingEnginePlannerModel;

use crate::{
    admit_route_from_snapshot, candidate_routes_from_snapshot, DecayWindow, OlsrBestNextHop,
    OlsrPlannerSnapshot,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OlsrPlannerSeed {
    pub local_node_id: NodeId,
    pub selected_neighbor: NodeId,
}

pub struct OlsrPlannerModel;

impl RoutingEnginePlannerModel for OlsrPlannerModel {
    type PlannerSnapshot = OlsrPlannerSeed;
    type PlannerCandidate = RouteCandidate;
    type PlannerAdmission = RouteAdmission;

    fn candidate_routes_from_snapshot(
        seed: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        _profile: &SelectedRoutingParameters,
        topology: &Observation<Configuration>,
    ) -> Vec<Self::PlannerCandidate> {
        let snapshot = planner_snapshot(seed, objective, topology);
        candidate_routes_from_snapshot(&snapshot, objective, topology)
    }

    fn admit_route_from_snapshot(
        seed: &Self::PlannerSnapshot,
        objective: &jacquard_core::RoutingObjective,
        profile: &SelectedRoutingParameters,
        candidate: &Self::PlannerCandidate,
        topology: &Observation<Configuration>,
    ) -> Result<Self::PlannerAdmission, RouteError> {
        let snapshot = planner_snapshot(seed, objective, topology);
        admit_route_from_snapshot(&snapshot, objective, profile, candidate, topology)
    }
}

#[must_use]
pub fn backend_route_id(
    destination: NodeId,
    selected_neighbor: NodeId,
    path_cost: u32,
) -> jacquard_core::BackendRouteId {
    jacquard_core::BackendRouteId(
        [
            destination.0.as_slice(),
            selected_neighbor.0.as_slice(),
            path_cost.to_le_bytes().as_slice(),
        ]
        .concat(),
    )
}

#[must_use]
pub fn selected_neighbor_from_backend_route_id(
    backend_route_id: &jacquard_core::BackendRouteId,
) -> Option<NodeId> {
    if backend_route_id.0.len() < 64 {
        return None;
    }
    let mut node = [0_u8; 32];
    node.copy_from_slice(&backend_route_id.0[32..64]);
    Some(NodeId(node))
}

fn planner_snapshot(
    seed: &OlsrPlannerSeed,
    objective: &jacquard_core::RoutingObjective,
    topology: &Observation<Configuration>,
) -> OlsrPlannerSnapshot {
    let jacquard_core::DestinationId::Node(destination) = objective.destination else {
        return OlsrPlannerSnapshot {
            local_node_id: seed.local_node_id,
            stale_after_ticks: DecayWindow::default().stale_after_ticks,
            best_next_hops: BTreeMap::new(),
        };
    };
    let path_cost = 10;
    OlsrPlannerSnapshot {
        local_node_id: seed.local_node_id,
        stale_after_ticks: DecayWindow::default().stale_after_ticks,
        best_next_hops: BTreeMap::from([(
            destination,
            OlsrBestNextHop {
                destination,
                next_hop: seed.selected_neighbor,
                hop_count: 1,
                path_cost,
                degradation: RouteDegradation::None,
                transport_kind: TransportKind::WifiAware,
                updated_at_tick: topology.observed_at_tick,
                topology_epoch: topology.value.epoch,
                backend_route_id: backend_route_id(destination, seed.selected_neighbor, path_cost),
            },
        )]),
    }
}
