//! Engine-owned pure planner model surface for Babel.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, Fact, FactBasis, HealthScore, Limit, MaterializedRoute, NodeId, Observation,
    PenaltyPoints, PublicationId, RatioPermille, ReachabilityState, RouteAdmission, RouteCandidate,
    RouteDegradation, RouteError, RouteHealth, RouteIdentityStamp, RouteLifecycleEvent,
    RouteMaterializationProof, RouteProgressContract, RouteProgressState, RouteRuntimeState,
    RouteSelectionError, SelectedRoutingParameters, Tick, TimeWindow, TransportKind,
};
use jacquard_traits::RoutingEnginePlannerModel;

use crate::{
    admit_route_from_snapshot, candidate_routes_from_snapshot, private_state::route_id_for,
    BabelBestNextHop, BabelPlannerSnapshot,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BabelPlannerSeed {
    pub local_node_id: NodeId,
    pub selected_neighbor: NodeId,
}

pub struct BabelPlannerModel;

impl RoutingEnginePlannerModel for BabelPlannerModel {
    type PlannerSnapshot = BabelPlannerSeed;
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
    jacquard_core::BackendRouteId(
        [destination.0.as_slice(), selected_neighbor.0.as_slice()].concat(),
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
    seed: &BabelPlannerSeed,
    objective: &jacquard_core::RoutingObjective,
    topology: &Observation<Configuration>,
) -> BabelPlannerSnapshot {
    let jacquard_core::DestinationId::Node(destination) = objective.destination else {
        return BabelPlannerSnapshot {
            local_node_id: seed.local_node_id,
            stale_after_ticks: 8,
            best_next_hops: BTreeMap::new(),
        };
    };
    BabelPlannerSnapshot {
        local_node_id: seed.local_node_id,
        stale_after_ticks: 8,
        best_next_hops: BTreeMap::from([(
            destination,
            BabelBestNextHop {
                destination,
                next_hop: seed.selected_neighbor,
                metric: 512,
                tq: RatioPermille(488),
                degradation: RouteDegradation::None,
                transport_kind: TransportKind::WifiAware,
                updated_at_tick: topology.observed_at_tick,
                topology_epoch: topology.value.epoch,
                backend_route_id: backend_route_id(destination, seed.selected_neighbor),
            },
        )]),
    }
}

// long-block-exception: this helper assembles one authoritative Babel
// materialized route from an engine-owned planner seed and router facts.
pub fn materialize_route_from_seed(
    owner_node_id: NodeId,
    seed: &BabelPlannerSeed,
    objective: &jacquard_core::RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
    now: Tick,
) -> Result<MaterializedRoute, RouteError> {
    let snapshot = planner_snapshot(seed, objective, topology);
    let candidate = candidate_routes_from_snapshot(&snapshot, objective, topology)
        .into_iter()
        .next()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = admit_route_from_snapshot(&snapshot, objective, profile, &candidate, topology)?;
    let jacquard_core::DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let route_id = route_id_for(owner_node_id, destination);
    Ok(MaterializedRoute {
        identity: jacquard_core::PublishedRouteRecord {
            stamp: RouteIdentityStamp {
                route_id,
                topology_epoch: topology.value.epoch,
                materialized_at_tick: now,
                publication_id: PublicationId([7; 16]),
            },
            proof: RouteMaterializationProof {
                stamp: RouteIdentityStamp {
                    route_id,
                    topology_epoch: topology.value.epoch,
                    materialized_at_tick: now,
                    publication_id: PublicationId([7; 16]),
                },
                witness: Fact {
                    basis: FactBasis::Admitted,
                    value: admission.witness.clone(),
                    established_at_tick: now,
                },
            },
            admission,
            lease: jacquard_core::RouteLease {
                owner_node_id,
                lease_epoch: topology.value.epoch,
                valid_for: TimeWindow::new(Tick(1), Tick(20))
                    .expect("babel model fixture lease window"),
            },
        },
        runtime: RouteRuntimeState {
            last_lifecycle_event: RouteLifecycleEvent::Activated,
            health: RouteHealth {
                reachability_state: ReachabilityState::Reachable,
                stability_score: HealthScore(1000),
                congestion_penalty_points: PenaltyPoints(0),
                last_validated_at_tick: now,
            },
            progress: RouteProgressContract {
                productive_step_count_max: Limit::Bounded(1),
                total_step_count_max: Limit::Bounded(1),
                last_progress_at_tick: now,
                state: RouteProgressState::Pending,
            },
        },
    })
}
