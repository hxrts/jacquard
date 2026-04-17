//! Route event collection, host snapshots, and simulation artifact assembly.

use super::{
    ActiveRouteSummary, BTreeMap, BoundObjective, BridgeRoundProgress, BridgeRoundReport,
    ConnectivityPosture, DestinationId, DriverStatusEvent, DurationMs, FieldExportedReplayBundle,
    FieldReplaySummary, HoldFallbackPolicy, HostCheckpointSnapshot, HostRoundArtifact,
    HostRoundStatus, IngressBatchBoundary, JacquardCheckpointArtifact, JacquardReplayArtifact,
    JacquardSimulationStats, NodeId, PriorityPoints, ReferenceClient, ReferenceRouter,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, Router, RoutingControlPlane,
    RoutingObjective, SimulationError, SimulationFailureSummary, Tick, FIELD_ENGINE_ID,
    PATHWAY_ENGINE_ID,
};

pub(super) fn host_artifact(
    local_node_id: NodeId,
    at_tick: Tick,
    progress: &BridgeRoundProgress,
    active_routes: Vec<ActiveRouteSummary>,
    field_replay: Option<FieldReplaySummary>,
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
    }
}

// long-block-exception: this replay projection maps many Field runtime surfaces
// into one report summary, and keeping it in one pass preserves traceability.
pub(super) fn summarize_field_replay(router: &ReferenceRouter) -> Option<FieldReplaySummary> {
    let bundle = router
        .engine_analysis_snapshot(&FIELD_ENGINE_ID)?
        .downcast::<FieldExportedReplayBundle>()
        .ok()
        .map(|boxed| *boxed)?;
    let selected_result_present = bundle
        .runtime_search
        .search
        .as_ref()
        .is_some_and(|search| search.selected_result.is_some());
    let search_reconfiguration_present = bundle
        .runtime_search
        .search
        .as_ref()
        .is_some_and(|search| search.reconfiguration.is_some());
    let execution_policy = bundle
        .runtime_search
        .search
        .as_ref()
        .map(|search| search.execution_policy.scheduler_profile.clone());
    let bootstrap_active = bundle
        .recovery
        .entries
        .iter()
        .any(|entry| entry.bootstrap_active);
    let continuity_band = bundle
        .recovery
        .entries
        .iter()
        .find_map(|entry| entry.continuity_band.clone());
    let last_continuity_transition = bundle
        .recovery
        .entries
        .iter()
        .find_map(|entry| entry.last_continuity_transition.clone());
    let last_promotion_decision = bundle
        .recovery
        .entries
        .iter()
        .find_map(|entry| entry.last_promotion_decision.clone());
    let last_promotion_blocker = bundle
        .recovery
        .entries
        .iter()
        .find_map(|entry| entry.last_promotion_blocker.clone());
    let bootstrap_activation_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.bootstrap_activation_count)
        .max()
        .unwrap_or(0);
    let bootstrap_hold_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.bootstrap_hold_count)
        .max()
        .unwrap_or(0);
    let bootstrap_narrow_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.bootstrap_narrow_count)
        .max()
        .unwrap_or(0);
    let bootstrap_upgrade_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.bootstrap_upgrade_count)
        .max()
        .unwrap_or(0);
    let bootstrap_withdraw_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.bootstrap_withdraw_count)
        .max()
        .unwrap_or(0);
    let degraded_steady_entry_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.degraded_steady_entry_count)
        .max()
        .unwrap_or(0);
    let degraded_steady_recovery_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.degraded_steady_recovery_count)
        .max()
        .unwrap_or(0);
    let degraded_to_bootstrap_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.degraded_to_bootstrap_count)
        .max()
        .unwrap_or(0);
    let degraded_steady_round_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.degraded_steady_round_count)
        .max()
        .unwrap_or(0);
    let service_retention_carry_forward_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.service_retention_carry_forward_count)
        .max()
        .unwrap_or(0);
    let asymmetric_shift_success_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.asymmetric_shift_success_count)
        .max()
        .unwrap_or(0);
    let protocol_reconfiguration_count = bundle.protocol.reconfigurations.len();
    let route_bound_reconfiguration_count = bundle
        .protocol
        .reconfigurations
        .iter()
        .filter(|step| step.route_id.is_some())
        .count();
    let continuation_shift_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.continuation_shift_count)
        .max()
        .unwrap_or(0);
    let corridor_narrow_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.corridor_narrow_count)
        .max()
        .unwrap_or(0);
    let checkpoint_capture_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.checkpoint_capture_count)
        .max()
        .unwrap_or(0);
    let checkpoint_restore_count = bundle
        .recovery
        .entries
        .iter()
        .map(|entry| entry.checkpoint_restore_count)
        .max()
        .unwrap_or(0);
    let reconfiguration_causes = bundle
        .protocol
        .reconfigurations
        .iter()
        .map(|entry| entry.cause.clone())
        .collect();
    Some(FieldReplaySummary {
        bundle,
        selected_result_present,
        search_reconfiguration_present,
        execution_policy,
        bootstrap_active,
        continuity_band,
        last_continuity_transition,
        last_promotion_decision,
        last_promotion_blocker,
        bootstrap_activation_count,
        bootstrap_hold_count,
        bootstrap_narrow_count,
        bootstrap_upgrade_count,
        bootstrap_withdraw_count,
        degraded_steady_entry_count,
        degraded_steady_recovery_count,
        degraded_to_bootstrap_count,
        degraded_steady_round_count,
        service_retention_carry_forward_count,
        asymmetric_shift_success_count,
        protocol_reconfiguration_count,
        route_bound_reconfiguration_count,
        continuation_shift_count,
        corridor_narrow_count,
        checkpoint_capture_count,
        checkpoint_restore_count,
        reconfiguration_causes,
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
        let next_len = {
            let owner = host.bind();
            owner.router().effects().events.len()
        };
        let delta = next_len.saturating_sub(*cursor);
        new_event_count = new_event_count.saturating_add(delta);
        *cursor = next_len;
    }
    new_event_count
}

pub(super) fn summarize_active_routes(
    owner_node_id: NodeId,
    router: &ReferenceRouter,
) -> Vec<ActiveRouteSummary> {
    let field_bundle = router
        .engine_analysis_snapshot(&FIELD_ENGINE_ID)
        .and_then(|snapshot| snapshot.downcast::<FieldExportedReplayBundle>().ok())
        .map(|boxed| *boxed);
    router
        .active_routes_snapshot()
        .into_iter()
        .map(|route| {
            let route_id = route.identity.stamp.route_id;
            let recovery_entry = field_bundle.as_ref().and_then(|bundle| {
                bundle
                    .recovery
                    .entries
                    .iter()
                    .find(|entry| entry.route_id == route_id)
            });
            let commitment_resolution = router
                .route_commitments(&route_id)
                .ok()
                .and_then(|commitments| commitments.into_iter().next())
                .map(|commitment| format!("{:?}", commitment.resolution));
            ActiveRouteSummary {
                owner_node_id,
                route_id,
                destination: route.identity.admission.objective.destination,
                engine_id: route.identity.admission.summary.engine,
                last_lifecycle_event: route.runtime.last_lifecycle_event,
                reachability_state: route.runtime.health.reachability_state,
                stability_score: route.runtime.health.stability_score,
                commitment_resolution,
                field_continuity_band: recovery_entry
                    .and_then(|entry| entry.continuity_band.clone()),
                field_last_outcome: recovery_entry.and_then(|entry| entry.last_outcome.clone()),
                field_last_promotion_decision: recovery_entry
                    .and_then(|entry| entry.last_promotion_decision.clone()),
                field_last_promotion_blocker: recovery_entry
                    .and_then(|entry| entry.last_promotion_blocker.clone()),
                field_continuation_shift_count: recovery_entry
                    .map(|entry| entry.continuation_shift_count),
            }
        })
        .collect()
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
