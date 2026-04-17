//! Simulator-facing Babel snapshot, reducer, and restore helpers.

use std::collections::BTreeMap;

use jacquard_core::{
    AdmissionDecision, BackendRouteId, Configuration, DestinationId, Fact, FactBasis, HealthScore,
    Limit, MaterializedRoute, NodeId, Observation, PenaltyPoints, PublicationId, ReachabilityState,
    RouteDegradation, RouteError, RouteHealth, RouteId, RouteIdentityStamp, RouteLifecycleEvent,
    RouteMaterializationProof, RouteProgressContract, RouteProgressState, RouteRuntimeState,
    RouteSelectionError, RoutingObjective, RoutingTickChange, SelectedRoutingParameters, Tick,
    TimeWindow, TransportKind,
};

use crate::{
    private_state::{
        admission_for_candidate, backend_route_id_for, candidate_for_snapshot, reduce_round_state,
        route_id_for, BabelRoundInput, BabelRoundState,
    },
    public_state::{BabelBestNextHop, BabelPlannerSnapshot, FeasibilityEntry, RouteEntry},
    runtime::restored_active_route,
    scoring::metric_to_ratio,
    DecayWindow,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelPlannerChoiceView {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub metric: u16,
    pub degradation: RouteDegradation,
    pub transport_kind: TransportKind,
    pub updated_at_tick: Tick,
    pub topology_epoch: jacquard_core::RouteEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelPlannerSnapshotView {
    pub local_node_id: NodeId,
    pub stale_after_ticks: u64,
    pub choices: Vec<BabelPlannerChoiceView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundRouteEntryView {
    pub destination: NodeId,
    pub via_neighbor: NodeId,
    pub router_id: NodeId,
    pub seqno: u16,
    pub metric: u16,
    pub observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelFeasibilityEntryView {
    pub destination: NodeId,
    pub seqno: u16,
    pub metric: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundStateView {
    pub route_entries: Vec<BabelRoundRouteEntryView>,
    pub feasibility_entries: Vec<BabelFeasibilityEntryView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundInputView {
    pub topology: Observation<Configuration>,
    pub now: Tick,
    pub local_node_id: NodeId,
    pub decay_window: DecayWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRoundOutputView {
    pub change: RoutingTickChange,
    pub planner_snapshot: BabelPlannerSnapshotView,
    pub selected_destination_count: usize,
    pub best_next_hop_count: usize,
    pub feasibility_destination_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelRestoredRouteView {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BabelPlannerDecisionResult {
    pub candidate_count: usize,
    pub backend_route_id: BackendRouteId,
    pub next_hop: NodeId,
    pub admitted: bool,
}

#[must_use]
pub fn planner_snapshot_view(
    local_node_id: NodeId,
    stale_after_ticks: u64,
    choices: Vec<BabelPlannerChoiceView>,
) -> BabelPlannerSnapshotView {
    BabelPlannerSnapshotView {
        local_node_id,
        stale_after_ticks,
        choices,
    }
}

#[must_use]
pub fn route_id(local_node_id: NodeId, destination: NodeId) -> RouteId {
    route_id_for(local_node_id, destination)
}

#[must_use]
pub fn backend_route_id(destination: NodeId, next_hop: NodeId) -> BackendRouteId {
    backend_route_id_for(destination, next_hop)
}

fn candidate_routes_from_snapshot(
    snapshot: &BabelPlannerSnapshotView,
    objective: &RoutingObjective,
    topology: &Observation<Configuration>,
) -> Vec<jacquard_core::RouteCandidate> {
    let DestinationId::Node(destination) = objective.destination else {
        return Vec::new();
    };
    if !crate::planner::destination_supports_objective(
        topology,
        destination,
        objective.service_kind,
    ) {
        return Vec::new();
    }
    let snapshot = private_snapshot(snapshot);
    snapshot
        .best_next_hops
        .get(&destination)
        .map(|best| vec![candidate_for_snapshot(&snapshot, objective, best)])
        .unwrap_or_default()
}

fn admit_candidate_from_snapshot(
    snapshot: &BabelPlannerSnapshotView,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
    candidate: &jacquard_core::RouteCandidate,
) -> Result<jacquard_core::RouteAdmission, RouteError> {
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    if !crate::planner::destination_supports_objective(
        topology,
        destination,
        objective.service_kind,
    ) {
        return Err(RouteSelectionError::Inadmissible(
            jacquard_core::RouteAdmissionRejection::BackendUnavailable,
        )
        .into());
    }
    let snapshot = private_snapshot(snapshot);
    let Some(best) = snapshot.best_next_hops.get(&destination) else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let expected = candidate_for_snapshot(&snapshot, objective, best);
    if expected.backend_ref != candidate.backend_ref {
        return Err(RouteSelectionError::Inadmissible(
            jacquard_core::RouteAdmissionRejection::BackendUnavailable,
        )
        .into());
    }
    let admission = admission_for_candidate(objective, profile, &expected);
    if let AdmissionDecision::Rejected(reason) = admission.admission_check.decision {
        return Err(RouteSelectionError::Inadmissible(reason).into());
    }
    Ok(admission)
}

#[must_use]
fn reduce_round_view(
    state: &BabelRoundStateView,
    input: &BabelRoundInputView,
) -> BabelRoundOutputView {
    let transition = reduce_round_state(
        private_round_state(state),
        &BabelRoundInput {
            topology: input.topology.clone(),
            now: input.now,
            local_node_id: input.local_node_id,
            decay_window: input.decay_window,
        },
    );
    BabelRoundOutputView {
        change: transition.change,
        planner_snapshot: view_snapshot(&transition.planner_snapshot),
        selected_destination_count: transition.next_state.selected_routes.len(),
        best_next_hop_count: transition.next_state.best_next_hops.len(),
        feasibility_destination_count: transition.next_state.feasibility_distances.len(),
    }
}

#[must_use]
fn restore_route_view(route: &MaterializedRoute) -> Option<BabelRestoredRouteView> {
    let restored = restored_active_route(route)?;
    Some(BabelRestoredRouteView {
        destination: restored.destination,
        next_hop: restored.next_hop,
        backend_route_id: restored.backend_route_id,
        installed_at_tick: restored.installed_at_tick,
    })
}

pub fn run_planner_decision_fixture(
    snapshot: &BabelPlannerSnapshotView,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
) -> Result<BabelPlannerDecisionResult, RouteError> {
    let candidates = candidate_routes_from_snapshot(snapshot, objective, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission =
        admit_candidate_from_snapshot(snapshot, objective, profile, topology, &candidate)?;
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let next_hop = private_snapshot(snapshot)
        .best_next_hops
        .get(&destination)
        .map(|best| best.next_hop)
        .ok_or(RouteSelectionError::NoCandidate)?;
    Ok(BabelPlannerDecisionResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        next_hop,
        admitted: matches!(
            admission.admission_check.decision,
            AdmissionDecision::Admissible
        ),
    })
}

#[must_use]
pub fn run_round_refresh_fixture(
    state: &BabelRoundStateView,
    input: &BabelRoundInputView,
) -> BabelRoundOutputView {
    reduce_round_view(state, input)
}

#[must_use]
pub fn run_checkpoint_restore_fixture(route: &MaterializedRoute) -> Option<BabelRestoredRouteView> {
    restore_route_view(route)
}

// long-block-exception: the simulator helper assembles one fully materialized
// Babel route fixture from snapshot data and router-owned runtime fields.
pub fn materialized_route_from_snapshot(
    owner_node_id: NodeId,
    snapshot: &BabelPlannerSnapshotView,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    topology: &Observation<Configuration>,
    now: Tick,
) -> Result<MaterializedRoute, RouteError> {
    let candidate = candidate_routes_from_snapshot(snapshot, objective, topology)
        .into_iter()
        .next()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission =
        admit_candidate_from_snapshot(snapshot, objective, profile, topology, &candidate)?;
    let DestinationId::Node(destination) = objective.destination else {
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
                    .expect("babel simulator fixture lease window"),
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

fn private_snapshot(snapshot: &BabelPlannerSnapshotView) -> BabelPlannerSnapshot {
    let best_next_hops = snapshot
        .choices
        .iter()
        .map(|choice| {
            let best = BabelBestNextHop {
                destination: choice.destination,
                next_hop: choice.next_hop,
                metric: choice.metric,
                tq: metric_to_ratio(choice.metric),
                degradation: choice.degradation,
                transport_kind: choice.transport_kind.clone(),
                updated_at_tick: choice.updated_at_tick,
                topology_epoch: choice.topology_epoch,
                backend_route_id: backend_route_id_for(choice.destination, choice.next_hop),
            };
            (choice.destination, best)
        })
        .collect();
    BabelPlannerSnapshot {
        local_node_id: snapshot.local_node_id,
        stale_after_ticks: snapshot.stale_after_ticks,
        best_next_hops,
    }
}

fn private_round_state(state: &BabelRoundStateView) -> BabelRoundState {
    let mut route_table: BTreeMap<NodeId, BTreeMap<NodeId, RouteEntry>> = BTreeMap::new();
    for entry in &state.route_entries {
        route_table.entry(entry.destination).or_default().insert(
            entry.via_neighbor,
            RouteEntry {
                router_id: entry.router_id,
                seqno: entry.seqno,
                metric: entry.metric,
                observed_at_tick: entry.observed_at_tick,
            },
        );
    }
    let feasibility_distances = state
        .feasibility_entries
        .iter()
        .map(|entry| {
            (
                entry.destination,
                FeasibilityEntry {
                    seqno: entry.seqno,
                    metric: entry.metric,
                },
            )
        })
        .collect();
    BabelRoundState {
        route_table,
        selected_routes: BTreeMap::new(),
        best_next_hops: BTreeMap::new(),
        feasibility_distances,
    }
}

fn view_snapshot(snapshot: &BabelPlannerSnapshot) -> BabelPlannerSnapshotView {
    BabelPlannerSnapshotView {
        local_node_id: snapshot.local_node_id,
        stale_after_ticks: snapshot.stale_after_ticks,
        choices: snapshot
            .best_next_hops
            .values()
            .map(|best| BabelPlannerChoiceView {
                destination: best.destination,
                next_hop: best.next_hop,
                metric: best.metric,
                degradation: best.degradation,
                transport_kind: best.transport_kind.clone(),
                updated_at_tick: best.updated_at_tick,
                topology_epoch: best.topology_epoch,
            })
            .collect(),
    }
}
