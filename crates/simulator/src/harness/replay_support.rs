//! Route event collection, host snapshots, and simulation artifact assembly.

use super::{
    ActiveRouteSummary, BTreeMap, BoundObjective, BridgeRoundProgress, BridgeRoundReport,
    ConnectivityPosture, DestinationId, DriverStatusEvent, DurationMs, FieldReplaySummary,
    HoldFallbackPolicy, HostCheckpointSnapshot, HostRoundArtifact, HostRoundStatus,
    IngressBatchBoundary, JacquardCheckpointArtifact, JacquardReplayArtifact,
    JacquardSimulationStats, MercatorReplaySummary, NodeId, PriorityPoints, ReferenceClient,
    ReferenceRouter, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, Router,
    RoutingControlPlane, RoutingDataPlane, RoutingObjective, SimulationError,
    SimulationFailureSummary, Tick, PATHWAY_ENGINE_ID,
};
use jacquard_babel::{
    selected_neighbor_from_backend_route_id as selected_babel_neighbor, BABEL_ENGINE_ID,
};
use jacquard_batman_bellman::{
    selected_neighbor_from_backend_route_id as selected_batman_bellman_neighbor,
    BATMAN_BELLMAN_ENGINE_ID,
};
use jacquard_batman_classic::{
    selected_neighbor_from_backend_route_id as selected_batman_classic_neighbor,
    BATMAN_CLASSIC_ENGINE_ID,
};
use jacquard_core::{RouteError, RouteSelectionError};
use jacquard_mercator::{
    selected_neighbor_from_backend_route_id as selected_mercator_neighbor,
    MercatorRouterAnalysisSnapshot, MERCATOR_ENGINE_ID,
};
use jacquard_olsrv2::{
    selected_neighbor_from_backend_route_id as selected_olsr_neighbor, OLSRV2_ENGINE_ID,
};
use jacquard_pathway::first_hop_node_id_from_backend_route_id;
use jacquard_scatter::{ScatterRouterAnalysisSnapshot, SCATTER_ENGINE_ID};

pub(super) fn host_artifact(
    local_node_id: NodeId,
    at_tick: Tick,
    progress: &BridgeRoundProgress,
    active_routes: Vec<ActiveRouteSummary>,
    field_replay: Option<FieldReplaySummary>,
    mercator_replay: Option<MercatorReplaySummary>,
) -> HostRoundArtifact {
    let ingress_batch_boundary = IngressBatchBoundary {
        observed_at_tick: at_tick,
        ingested_transport_observation_count: match progress {
            BridgeRoundProgress::Advanced(report) => report.ingested_transport_observations.len(),
            BridgeRoundProgress::Waiting(_) => 0,
        },
    };
    let status = match progress {
        BridgeRoundProgress::Advanced(report) => advanced_status(report),
        BridgeRoundProgress::Waiting(waiting) => HostRoundStatus::Waiting {
            next_round_hint: waiting.next_round_hint,
            pending_transport_observations: waiting.pending_transport_observations,
            pending_transport_commands: waiting.pending_transport_commands,
            dropped_transport_observations: waiting.dropped_transport_observations,
        },
    };
    HostRoundArtifact {
        local_node_id,
        ingress_batch_boundary,
        status,
        active_routes,
        field_replay,
        mercator_replay,
    }
}

pub(super) fn summarize_mercator_replay(router: &ReferenceRouter) -> Option<MercatorReplaySummary> {
    let snapshot = router
        .engine_analysis_snapshot(&MERCATOR_ENGINE_ID)?
        .downcast::<MercatorRouterAnalysisSnapshot>()
        .ok()
        .map(|boxed| *boxed)?;
    let diagnostics = snapshot.diagnostics;
    Some(MercatorReplaySummary {
        selected_result_rounds: diagnostics.selected_result_rounds,
        no_candidate_attempts: diagnostics.no_candidate_attempts,
        inadmissible_candidate_attempts: diagnostics.inadmissible_candidate_attempts,
        support_withdrawal_count: diagnostics.support_withdrawal_count,
        stale_persistence_rounds: diagnostics.stale_persistence_rounds,
        active_stale_route_count: diagnostics.active_stale_route_count,
        repair_attempt_count: diagnostics.repair_attempt_count,
        repair_success_count: diagnostics.repair_success_count,
        recovery_rounds: diagnostics.recovery_rounds,
        objective_count: diagnostics.objective_count,
        active_objective_count: diagnostics.active_objective_count,
        weakest_objective_presence_rounds: diagnostics.weakest_objective_presence_rounds,
        zero_service_objective_count: diagnostics.zero_service_objective_count,
        broker_participation_count: diagnostics.broker_participation_count,
        hottest_broker_route_count: diagnostics.hottest_broker_route_count,
        hottest_broker_concentration_permille: diagnostics.hottest_broker_concentration_permille,
        broker_switch_count: diagnostics.broker_switch_count,
        overloaded_broker_penalty_count: diagnostics.overloaded_broker_penalty_count,
        weakest_flow_reserved_search_count: diagnostics.weakest_flow_reserved_search_count,
        active_route_count: snapshot.active_route_count,
        latest_topology_epoch: snapshot.latest_topology_epoch,
    })
}

pub(super) fn advanced_status(report: &BridgeRoundReport) -> HostRoundStatus {
    HostRoundStatus::Advanced {
        router_outcome: report.router_outcome.clone(),
        ingested_transport_observation_count: report.ingested_transport_observations.len(),
        flushed_transport_commands: report.flushed_transport_commands,
        dropped_transport_observations: report.dropped_transport_observations,
    }
}

pub(super) fn collect_route_events(
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    cursors: &mut BTreeMap<NodeId, usize>,
    route_events: &mut Vec<jacquard_core::RouteEvent>,
    stamped_route_events: &mut Vec<jacquard_core::RouteEventStamped>,
) -> usize {
    let mut new_event_count = 0usize;
    for (node_id, host) in hosts {
        let cursor = cursors.entry(*node_id).or_insert(0);
        let new_events = {
            let owner = host.bind();
            owner.router().effects().events[*cursor..].to_vec()
        };
        new_event_count = new_event_count.saturating_add(new_events.len());
        route_events.extend(new_events.iter().map(|event| event.event.clone()));
        stamped_route_events.extend(new_events.iter().cloned());
        *cursor = cursor.saturating_add(new_events.len());
    }
    new_event_count
}

pub(super) fn advance_route_event_cursors(
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    cursors: &mut BTreeMap<NodeId, usize>,
) -> usize {
    let mut new_event_count = 0usize;
    for (node_id, host) in hosts {
        let cursor = cursors.entry(*node_id).or_insert(0);
        let mut owner = host.bind();
        let next_len = owner.router().effects().events.len();
        let delta = next_len.saturating_sub(*cursor);
        new_event_count = new_event_count.saturating_add(delta);
        owner.router_mut().effects_mut().events.clear();
        *cursor = 0;
    }
    new_event_count
}

// long-block-exception: active-route summaries merge router state with engine-specific snapshots deterministically.
pub(super) fn summarize_active_routes(
    owner_node_id: NodeId,
    router: &ReferenceRouter,
) -> Vec<ActiveRouteSummary> {
    let scatter_snapshot = router
        .engine_analysis_snapshot(&SCATTER_ENGINE_ID)
        .and_then(|snapshot| snapshot.downcast::<ScatterRouterAnalysisSnapshot>().ok())
        .map(|boxed| *boxed);
    router
        .active_routes_snapshot()
        .into_iter()
        .map(|route| {
            let route_id = route.identity.stamp.route_id;
            let next_hop_node_id = next_hop_node_id_for_route(&route);
            let scatter_entry = scatter_snapshot.as_ref().and_then(|snapshot| {
                snapshot
                    .route_summaries
                    .iter()
                    .find(|entry| entry.route_id == route_id)
            });
            let commitment_resolution = router
                .route_commitments(&route_id)
                .ok()
                .and_then(|commitments| commitments.into_iter().next())
                .map(|commitment| format!("{:?}", commitment.resolution));
            let hop_count_hint = route.identity.admission.summary.hop_count_hint.value_or(0);
            ActiveRouteSummary {
                owner_node_id,
                route_id,
                destination: route.identity.admission.objective.destination,
                engine_id: route.identity.admission.summary.engine,
                next_hop_node_id,
                hop_count_hint: (hop_count_hint > 0).then_some(hop_count_hint),
                last_lifecycle_event: route.runtime.last_lifecycle_event,
                reachability_state: route.runtime.health.reachability_state,
                stability_score: route.runtime.health.stability_score,
                commitment_resolution,
                field_continuity_band: None,
                field_last_outcome: None,
                field_last_promotion_decision: None,
                field_last_promotion_blocker: None,
                field_continuation_shift_count: None,
                scatter_current_regime: scatter_snapshot
                    .as_ref()
                    .map(|snapshot| format!("{:?}", snapshot.current_regime)),
                scatter_last_action: scatter_entry.map(|entry| format!("{:?}", entry.last_action)),
                scatter_retained_message_count: scatter_entry
                    .map(|entry| entry.retained_message_count),
                scatter_delivered_message_count: scatter_entry
                    .map(|entry| entry.delivered_message_count),
                scatter_contact_rate: scatter_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.last_local_summary.contact_rate),
                scatter_diversity_score: scatter_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.last_local_summary.diversity_score),
                scatter_resource_pressure_permille: scatter_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.last_local_summary.resource_pressure_permille),
            }
        })
        .collect()
}

fn next_hop_node_id_for_route(route: &jacquard_core::MaterializedRoute) -> Option<NodeId> {
    let backend_route_id = &route.identity.admission.backend_ref.backend_route_id;
    match route.identity.admission.summary.engine {
        BATMAN_BELLMAN_ENGINE_ID => selected_batman_bellman_neighbor(backend_route_id),
        BATMAN_CLASSIC_ENGINE_ID => selected_batman_classic_neighbor(backend_route_id),
        BABEL_ENGINE_ID => selected_babel_neighbor(backend_route_id),
        OLSRV2_ENGINE_ID => selected_olsr_neighbor(backend_route_id),
        PATHWAY_ENGINE_ID => first_hop_node_id_from_backend_route_id(backend_route_id),
        MERCATOR_ENGINE_ID => selected_mercator_neighbor(backend_route_id),
        _ => None,
    }
}

pub(super) fn refresh_host_round_routes(
    host_rounds: &mut [HostRoundArtifact],
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
) {
    for artifact in host_rounds {
        let Some(host) = hosts.get_mut(&artifact.local_node_id) else {
            continue;
        };
        let bound = host.bind();
        artifact.active_routes = summarize_active_routes(artifact.local_node_id, bound.router());
        artifact.field_replay = None;
        artifact.mercator_replay = summarize_mercator_replay(bound.router());
    }
}

pub(super) fn stimulate_scatter_routes(
    router: &mut ReferenceRouter,
    round_index: u32,
    failure_summaries: &mut Vec<SimulationFailureSummary>,
) {
    const SCATTER_STIMULUS_LEN: usize = 160;

    let route_ids = router
        .active_routes_snapshot()
        .into_iter()
        .filter(|route| route.identity.admission.summary.engine == SCATTER_ENGINE_ID)
        .map(|route| route.identity.stamp.route_id)
        .collect::<Vec<_>>();
    for route_id in route_ids {
        let payload = vec![u8::try_from(round_index & 0xff).unwrap_or(0); SCATTER_STIMULUS_LEN];
        if let Err(error) = router.forward_payload(&route_id, &payload) {
            failure_summaries.push(SimulationFailureSummary {
                round_index: Some(round_index),
                detail: format!(
                    "scatter stimulus forwarding failed for route {:?}: {}",
                    route_id, error
                ),
            });
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TopologyAdvance {
    Advanced,
    Unchanged,
}

pub(super) fn maintain_active_routes(
    router: &mut ReferenceRouter,
    topology_advance: TopologyAdvance,
    round_index: u32,
    failure_summaries: &mut Vec<SimulationFailureSummary>,
) {
    let route_ids = router
        .active_routes_snapshot()
        .into_iter()
        .map(|route| {
            let trigger = if matches!(topology_advance, TopologyAdvance::Advanced)
                && route.identity.admission.summary.engine == PATHWAY_ENGINE_ID
                && route.identity.admission.objective.hold_fallback_policy
                    == HoldFallbackPolicy::Allowed
                && route.identity.admission.summary.connectivity.partition
                    == RoutePartitionClass::PartitionTolerant
            {
                jacquard_core::RouteMaintenanceTrigger::PartitionDetected
            } else if matches!(topology_advance, TopologyAdvance::Advanced) {
                jacquard_core::RouteMaintenanceTrigger::EpochAdvanced
            } else {
                jacquard_core::RouteMaintenanceTrigger::AntiEntropyRequired
            };
            (route.identity.stamp.route_id, trigger)
        })
        .collect::<Vec<_>>();
    for (route_id, trigger) in route_ids {
        if let Err(error) = router.maintain_route(&route_id, trigger) {
            failure_summaries.push(SimulationFailureSummary {
                round_index: Some(round_index),
                detail: format!(
                    "route maintenance failed for route {:?} with trigger {:?}: {}",
                    route_id, trigger, error
                ),
            });
        }
    }
}

pub(super) fn capture_host_snapshots(
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
) -> BTreeMap<NodeId, HostCheckpointSnapshot> {
    hosts
        .iter_mut()
        .map(|(node_id, host)| {
            let effects = {
                let bound = host.bind();
                bound.router().effects().clone()
            };
            (
                *node_id,
                HostCheckpointSnapshot {
                    local_node_id: *node_id,
                    runtime_effects: effects,
                },
            )
        })
        .collect()
}

pub(super) fn restore_checkpointed_hosts(
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    checkpoint: &JacquardCheckpointArtifact,
) -> Result<(), SimulationError> {
    for (node_id, snapshot) in &checkpoint.host_snapshots {
        let host = hosts
            .get_mut(node_id)
            .ok_or(SimulationError::MissingBridge(*node_id))?;
        let mut bound = host.bind();
        *bound.router_mut().effects_mut() = snapshot.runtime_effects.clone();
        let _recovered_route_count = bound.router_mut().recover_checkpointed_routes()?;
    }
    Ok(())
}

pub(super) fn failure_summaries_for(
    checkpoint_count: usize,
    route_event_count: usize,
    driver_status_events: &[DriverStatusEvent],
) -> Vec<SimulationFailureSummary> {
    let mut summaries = Vec::new();
    if checkpoint_count == 0 {
        summaries.push(SimulationFailureSummary {
            round_index: None,
            detail: "no deterministic checkpoints were emitted during the run".to_string(),
        });
    }
    if route_event_count == 0 {
        summaries.push(SimulationFailureSummary {
            round_index: None,
            detail: "run completed without any route lifecycle events".to_string(),
        });
    }
    if !driver_status_events.is_empty() {
        summaries.push(SimulationFailureSummary {
            round_index: None,
            detail: format!(
                "driver surfaced {} status event(s) during the run",
                driver_status_events.len()
            ),
        });
    }
    summaries
}

// long-block-exception: checkpoint replay stitching compares prefix and suffix
// artifacts in one place to preserve deterministic accounting.
pub(super) fn stitch_replay_from_checkpoint(
    replay: &JacquardReplayArtifact,
    completed_rounds: u32,
    suffix_replay: JacquardReplayArtifact,
    _suffix_stats: JacquardSimulationStats,
) -> (JacquardReplayArtifact, JacquardSimulationStats) {
    let prefix_len = usize::try_from(completed_rounds).unwrap_or(usize::MAX);
    let mut rounds = replay.rounds[..std::cmp::min(prefix_len, replay.rounds.len())].to_vec();
    rounds.extend(suffix_replay.rounds);

    let mut route_events = replay.route_events.clone();
    route_events.extend(suffix_replay.route_events);

    let mut stamped_route_events = replay.stamped_route_events.clone();
    stamped_route_events.extend(suffix_replay.stamped_route_events);

    let mut driver_status_events = replay.driver_status_events.clone();
    driver_status_events.extend(suffix_replay.driver_status_events);

    let mut failure_summaries = replay.failure_summaries.clone();
    failure_summaries.extend(suffix_replay.failure_summaries);

    let mut checkpoints = replay.checkpoints.clone();
    checkpoints.extend(suffix_replay.checkpoints);

    let stitched = JacquardReplayArtifact {
        scenario: replay.scenario.clone(),
        environment_model: replay.environment_model.clone(),
        rounds,
        route_events,
        stamped_route_events,
        driver_status_events,
        failure_summaries,
        checkpoints,
        telltale_native: replay
            .telltale_native
            .clone()
            .or(suffix_replay.telltale_native),
    };
    let stats = JacquardSimulationStats {
        executed_round_count: u32::try_from(stitched.rounds.len())
            .expect("simulation rounds must fit in u32"),
        advanced_round_count: u32::try_from(
            stitched
                .rounds
                .iter()
                .flat_map(|round| round.host_rounds.iter())
                .filter(|host| matches!(host.status, HostRoundStatus::Advanced { .. }))
                .count(),
        )
        .unwrap_or(u32::MAX),
        waiting_round_count: u32::try_from(
            stitched
                .rounds
                .iter()
                .flat_map(|round| round.host_rounds.iter())
                .filter(|host| matches!(host.status, HostRoundStatus::Waiting { .. }))
                .count(),
        )
        .unwrap_or(u32::MAX),
        route_event_count: stitched.route_events.len(),
        checkpoint_count: stitched.checkpoints.len(),
        driver_status_event_count: stitched.driver_status_events.len(),
        failure_summary_count: stitched.failure_summaries.len(),
    };
    (stitched, stats)
}

pub(super) fn activate_ready_objectives(
    objectives: &[BoundObjective],
    round_index: u32,
    activated: &mut [bool],
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    failure_summaries: &mut Vec<SimulationFailureSummary>,
) -> bool {
    let mut activated_any = false;
    for (index, binding) in objectives.iter().enumerate() {
        if activated.get(index).copied().unwrap_or(true) || round_index < binding.activate_at_round
        {
            continue;
        }
        let Some(bridge) = hosts.get_mut(&binding.owner_node_id) else {
            failure_summaries.push(SimulationFailureSummary {
                round_index: Some(round_index),
                detail: format!(
                    "objective activation failed for owner {:?}: missing host bridge",
                    binding.owner_node_id
                ),
            });
            activated[index] = true;
            continue;
        };
        let mut bound = bridge.bind();
        if let Err(error) = bound
            .router_mut()
            .activate_route_without_tick(&binding.objective)
        {
            failure_summaries.push(SimulationFailureSummary {
                round_index: Some(round_index),
                detail: format!(
                    "objective activation failed for owner {:?} destination {:?}: {}",
                    binding.owner_node_id, binding.objective.destination, error
                ),
            });
            continue;
        }
        activated[index] = true;
        activated_any = true;
    }
    activated_any
}

pub(super) fn reactivate_missing_objectives(
    objectives: &[BoundObjective],
    round_index: u32,
    activated: &[bool],
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    failure_summaries: &mut Vec<SimulationFailureSummary>,
) -> bool {
    let mut reactivated_any = false;
    for (index, binding) in objectives.iter().enumerate() {
        if !activated.get(index).copied().unwrap_or(false)
            || round_index < binding.activate_at_round
        {
            continue;
        }
        let Some(bridge) = hosts.get_mut(&binding.owner_node_id) else {
            failure_summaries.push(SimulationFailureSummary {
                round_index: Some(round_index),
                detail: format!(
                    "objective activation failed for owner {:?}: missing host bridge during reactivation",
                    binding.owner_node_id
                ),
            });
            continue;
        };
        let mut bound = bridge.bind();
        let route_active = bound.router().active_routes_snapshot().iter().any(|route| {
            route.identity.admission.objective.destination == binding.objective.destination
        });
        if route_active {
            continue;
        }
        if let Err(error) = bound
            .router_mut()
            .activate_route_without_tick(&binding.objective)
        {
            record_reactivation_failure(failure_summaries, round_index, binding, &error);
            continue;
        }
        reactivated_any = true;
    }
    reactivated_any
}

fn record_reactivation_failure(
    failure_summaries: &mut Vec<SimulationFailureSummary>,
    round_index: u32,
    binding: &BoundObjective,
    error: &RouteError,
) {
    let detail = match error {
        RouteError::Selection(RouteSelectionError::NoCandidate) => format!(
            "objective reactivation no candidate for owner {:?} destination {:?}: {}",
            binding.owner_node_id, binding.objective.destination, error
        ),
        RouteError::Selection(RouteSelectionError::Inadmissible(_)) => format!(
            "objective reactivation inadmissible candidate for owner {:?} destination {:?}: {}",
            binding.owner_node_id, binding.objective.destination, error
        ),
        _ => format!(
            "objective activation failed for owner {:?} destination {:?} during reactivation: {}",
            binding.owner_node_id, binding.objective.destination, error
        ),
    };
    failure_summaries.push(SimulationFailureSummary {
        round_index: Some(round_index),
        detail,
    });
}

pub(crate) fn default_objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: jacquard_core::RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}
