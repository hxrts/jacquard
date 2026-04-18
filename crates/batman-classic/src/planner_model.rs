//! Engine-owned pure planner model surface for BATMAN Classic.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, NodeId, Observation, RatioPermille, RouteAdmission, RouteCandidate,
    RouteDegradation, RouteError, SelectedRoutingParameters, TransportKind,
};
use jacquard_traits::RoutingEnginePlannerModel;

use crate::{
    admit_route_from_snapshot, candidate_routes_from_snapshot, BatmanClassicPlannerSnapshot,
    BestNextHop, DecayWindow,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatmanClassicPlannerSeed {
    pub local_node_id: NodeId,
    pub selected_neighbor: NodeId,
}

pub struct BatmanClassicPlannerModel;

impl RoutingEnginePlannerModel for BatmanClassicPlannerModel {
    type PlannerSnapshot = BatmanClassicPlannerSeed;
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
) -> jacquard_core::BackendRouteId {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(&destination.0);
    bytes.extend_from_slice(&selected_neighbor.0);
    jacquard_core::BackendRouteId(bytes)
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
    seed: &BatmanClassicPlannerSeed,
    objective: &jacquard_core::RoutingObjective,
    topology: &Observation<Configuration>,
) -> BatmanClassicPlannerSnapshot {
    let jacquard_core::DestinationId::Node(destination) = objective.destination else {
        return BatmanClassicPlannerSnapshot {
            local_node_id: seed.local_node_id,
            stale_after_ticks: DecayWindow::default().stale_after_ticks,
            best_next_hops: BTreeMap::new(),
        };
    };
    BatmanClassicPlannerSnapshot {
        local_node_id: seed.local_node_id,
        stale_after_ticks: DecayWindow::default().stale_after_ticks,
        best_next_hops: BTreeMap::from([(
            destination,
            BestNextHop {
                originator: destination,
                next_hop: seed.selected_neighbor,
                tq: RatioPermille(950),
                receive_quality: RatioPermille(950),
                hop_count: 1,
                updated_at_tick: topology.observed_at_tick,
                transport_kind: TransportKind::WifiAware,
                degradation: RouteDegradation::None,
                backend_route_id: backend_route_id(destination, seed.selected_neighbor),
                topology_epoch: topology.value.epoch,
                is_bidirectional: true,
            },
        )]),
    }
}
