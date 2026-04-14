use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DurationMs, NodeId, Observation,
    PriorityPoints, RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RoutingObjective,
    Tick,
};
use jacquard_field::{FieldExportedReplayBundle, FIELD_ENGINE_ID};
use jacquard_mem_link_profile::SharedInMemoryNetwork;
use jacquard_reference_client::{
    BridgeRoundProgress, BridgeRoundReport, ClientBuilder,
    FieldBootstrapSummary as ClientFieldBootstrapSummary, ReferenceClient, ReferenceRouter,
};
use jacquard_traits::{
    purity, RoutingControlPlane, RoutingEnvironmentModel, RoutingReplayView, RoutingScenario,
    RoutingSimulator,
};
use telltale_simulator::{BatchConfig, SimRng};
use thiserror::Error;

use crate::{
    environment::ScriptedEnvironmentModel,
    replay::{
        ActiveRouteSummary, DriverStatusEvent, FieldReplaySummary, HostCheckpointSnapshot,
        HostRoundArtifact, HostRoundStatus, IngressBatchBoundary, JacquardCheckpointArtifact,
        JacquardReplayArtifact, JacquardRoundArtifact, JacquardSimulationStats,
        SimulationFailureSummary, TelltaleNativeArtifactRef,
    },
    scenario::{BoundObjective, EngineLane, JacquardScenario},
};

#[derive(Debug, Error)]
pub enum SimulationError {
    #[error("missing topology node for host {0:?}")]
    MissingHost(NodeId),
    #[error("missing local endpoint for host {0:?}")]
    MissingEndpoint(NodeId),
    #[error("missing bridge for host {0:?}")]
    MissingBridge(NodeId),
    #[error("router error: {0}")]
    Route(#[from] jacquard_core::RouteError),
}

#[purity(pure)]
pub trait JacquardHostAdapter {
    fn build_hosts(
        &self,
        scenario: &JacquardScenario,
    ) -> Result<BTreeMap<NodeId, ReferenceClient>, SimulationError>;

    fn validate_result(
        &self,
        _scenario: &JacquardScenario,
        _replay: &JacquardReplayArtifact,
        _stats: &JacquardSimulationStats,
    ) -> Result<(), SimulationError> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceClientAdapter;

impl JacquardHostAdapter for ReferenceClientAdapter {
    // long-block-exception: host construction keeps lane selection and override
    // threading together so simulator scenarios build deterministic mixed-engine
    // hosts from one auditable adapter path.
    fn build_hosts(
        &self,
        scenario: &JacquardScenario,
    ) -> Result<BTreeMap<NodeId, ReferenceClient>, SimulationError> {
        let topology = scenario.initial_configuration().clone();
        let network = SharedInMemoryNetwork::default();
        let mut hosts = BTreeMap::new();

        for host in scenario.hosts() {
            let mut builder = match host.lane {
                EngineLane::Pathway => ClientBuilder::pathway(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::Field => ClientBuilder::field(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::PathwayAndBatman => ClientBuilder::pathway_and_batman(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::PathwayAndField => ClientBuilder::pathway_and_field(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::FieldAndBatman => ClientBuilder::field_and_batman(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::AllEngines => ClientBuilder::all_engines(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::Batman => ClientBuilder::batman(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
            };
            if let Some(routing_profile) = host.overrides.routing_profile.clone() {
                builder = builder.with_profile(routing_profile);
            }
            if let Some(policy_inputs) = host.overrides.policy_inputs.clone() {
                builder = builder.with_policy_inputs(policy_inputs);
            }
            if let Some(batman_decay_window) = host.overrides.batman_decay_window {
                builder = builder.with_batman_decay_window(batman_decay_window);
            }
            if let Some(pathway_search_config) = host.overrides.pathway_search_config.clone() {
                builder = builder.with_pathway_search_config(pathway_search_config);
            }
            if let Some(field_search_config) = host.overrides.field_search_config.clone() {
                builder = builder.with_field_search_config(field_search_config);
            }
            for bootstrap in &host.overrides.field_bootstrap_summaries {
                builder = builder.with_field_bootstrap_summary(ClientFieldBootstrapSummary {
                    destination: bootstrap.destination.clone(),
                    from_neighbor: bootstrap.from_neighbor,
                    forward_observation: bootstrap.forward_observation,
                    reverse_feedback: bootstrap.reverse_feedback,
                });
            }
            let client = builder.build();
            hosts.insert(host.local_node_id, client);
        }

        Ok(hosts)
    }
}

pub struct JacquardSimulationHarness<A> {
    adapter: A,
    telltale_batch: BatchConfig,
}

impl<A> JacquardSimulationHarness<A>
where
    A: JacquardHostAdapter,
{
    #[must_use]
    pub fn new(adapter: A) -> Self {
        Self {
            adapter,
            telltale_batch: BatchConfig::default(),
        }
    }

    #[must_use]
    pub fn with_telltale_batch(mut self, telltale_batch: BatchConfig) -> Self {
        self.telltale_batch = telltale_batch;
        self
    }

    pub fn run(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError> {
        self.run_from_state(scenario, environment, None)
    }

    pub fn resume_from_checkpoint(
        &self,
        replay: &JacquardReplayArtifact,
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError> {
        let Some(checkpoint) = replay.checkpoints.last() else {
            return self.run(&replay.scenario, &replay.environment_model);
        };
        if !replay.scenario.all_hosts_pathway() {
            return self.run(&replay.scenario, &replay.environment_model);
        }
        let (suffix_replay, suffix_stats) = self.run_from_state(
            &replay
                .scenario
                .clone()
                .with_initial_configuration(checkpoint.topology.clone())
                .with_round_limit(
                    replay
                        .scenario
                        .round_limit()
                        .saturating_sub(checkpoint.completed_rounds),
                ),
            &replay.environment_model,
            Some(checkpoint),
        )?;
        Ok(stitch_replay_from_checkpoint(
            replay,
            checkpoint.completed_rounds,
            suffix_replay,
            suffix_stats,
        ))
    }

    // long-block-exception: the simulator round loop intentionally keeps
    // checkpoint, environment, routing, and replay stitching in one harness
    // path so deterministic replay is auditable end to end.
    fn run_from_state(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
        resume_from: Option<&JacquardCheckpointArtifact>,
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError> {
        let mut _rng = SimRng::new(scenario.seed().0);
        let mut topology = match resume_from {
            Some(checkpoint) => checkpoint.topology.clone(),
            None => scenario.initial_configuration().clone(),
        };
        let mut hosts = self.adapter.build_hosts(scenario)?;
        if let Some(checkpoint) = resume_from {
            restore_pathway_hosts(&mut hosts, checkpoint)?;
        }
        let mut route_event_cursors = match resume_from {
            Some(checkpoint) => checkpoint
                .host_snapshots
                .iter()
                .map(|(node_id, snapshot)| (*node_id, snapshot.runtime_effects.events.len()))
                .collect(),
            None => hosts
                .keys()
                .copied()
                .map(|node_id| (node_id, 0usize))
                .collect(),
        };
        let mut all_route_events = Vec::new();
        let mut all_stamped_route_events = Vec::new();
        let mut driver_status_events = Vec::new();
        let mut failure_summaries = Vec::new();
        let mut rounds = Vec::new();
        let mut checkpoints = Vec::new();
        let mut advanced_round_count = 0u32;
        let mut waiting_round_count = 0u32;

        let mut activated_objectives =
            vec![resume_from.is_some(); scenario.bound_objectives().len()];
        let checkpoint_round_offset =
            resume_from.map_or(0, |checkpoint| checkpoint.completed_rounds);

        for round_index in 0..scenario.round_limit() {
            let at_tick = Tick(topology.observed_at_tick.0.saturating_add(1));
            let (next_topology, environment_artifacts) =
                environment.advance_environment(&topology.value, at_tick);
            topology = next_topology;

            let mut host_rounds = Vec::new();
            let mut all_waiting = true;
            for host in scenario.hosts() {
                let bridge = hosts
                    .get_mut(&host.local_node_id)
                    .ok_or(SimulationError::MissingBridge(host.local_node_id))?;
                let mut bound = bridge.bind();
                bound.replace_shared_topology(topology.clone());
                let progress = bound.advance_round()?;
                maintain_active_routes(
                    bound.router_mut(),
                    &topology,
                    checkpoint_round_offset + round_index,
                    &mut failure_summaries,
                );
                let active_routes = summarize_active_routes(host.local_node_id, bound.router());
                let field_replay = summarize_field_replay(bound.router());
                let artifact = host_artifact(
                    host.local_node_id,
                    at_tick,
                    &progress,
                    active_routes,
                    field_replay,
                );
                if matches!(artifact.status, HostRoundStatus::Advanced { .. }) {
                    all_waiting = false;
                    advanced_round_count = advanced_round_count.saturating_add(1);
                } else {
                    waiting_round_count = waiting_round_count.saturating_add(1);
                }
                if let HostRoundStatus::Advanced {
                    dropped_transport_observations,
                    ..
                } = artifact.status
                {
                    if dropped_transport_observations > 0 {
                        driver_status_events.push(DriverStatusEvent::IngressDropped {
                            local_node_id: host.local_node_id,
                            at_tick,
                            dropped_transport_observations,
                        });
                    }
                }
                host_rounds.push(artifact);
            }

            collect_route_events(
                &mut hosts,
                &mut route_event_cursors,
                &mut all_route_events,
                &mut all_stamped_route_events,
            );

            if activate_ready_objectives(
                scenario.bound_objectives(),
                checkpoint_round_offset + round_index,
                &mut activated_objectives,
                &mut hosts,
                &mut failure_summaries,
            ) {
                refresh_host_round_routes(&mut host_rounds, &mut hosts);
                collect_route_events(
                    &mut hosts,
                    &mut route_event_cursors,
                    &mut all_route_events,
                    &mut all_stamped_route_events,
                );
            }

            if let Some(interval) = scenario.checkpoint_interval() {
                if interval > 0 && (round_index + 1) % interval == 0 {
                    checkpoints.push(JacquardCheckpointArtifact {
                        completed_rounds: checkpoint_round_offset + round_index + 1,
                        topology: topology.clone(),
                        host_snapshots: capture_host_snapshots(&mut hosts),
                        telltale_native: scenario.all_hosts_pathway().then_some(
                            TelltaleNativeArtifactRef::PathwayCheckpointRecovery {
                                completed_rounds: checkpoint_round_offset + round_index + 1,
                                host_count: hosts.len(),
                            },
                        ),
                    });
                }
            }

            rounds.push(JacquardRoundArtifact {
                round_index: checkpoint_round_offset + round_index,
                topology: topology.clone(),
                environment_artifacts,
                host_rounds,
            });

            if all_waiting && environment.is_quiescent_after(at_tick) {
                break;
            }
        }

        let total_completed_rounds =
            checkpoint_round_offset.saturating_add(u32::try_from(rounds.len()).unwrap_or(u32::MAX));
        failure_summaries.extend(failure_summaries_for(
            &checkpoints,
            &route_event_cursors,
            &driver_status_events,
        ));
        let replay = JacquardReplayArtifact {
            scenario: scenario.clone(),
            environment_model: environment.clone(),
            rounds,
            route_events: all_route_events,
            stamped_route_events: all_stamped_route_events,
            driver_status_events,
            failure_summaries,
            checkpoints,
            telltale_native: scenario.all_hosts_pathway().then_some(
                TelltaleNativeArtifactRef::PathwayCheckpointRecovery {
                    completed_rounds: total_completed_rounds,
                    host_count: hosts.len(),
                },
            ),
        };
        let stats = JacquardSimulationStats {
            executed_round_count: u32::try_from(replay.rounds.len())
                .expect("simulation rounds must fit in u32"),
            advanced_round_count,
            waiting_round_count,
            route_event_count: replay.route_events.len(),
            checkpoint_count: replay.checkpoints.len(),
            driver_status_event_count: replay.driver_status_events.len(),
            failure_summary_count: replay.failure_summaries.len(),
        };
        self.adapter.validate_result(scenario, &replay, &stats)?;
        Ok((replay, stats))
    }
}

pub struct JacquardSimulator<A = ReferenceClientAdapter> {
    harness: JacquardSimulationHarness<A>,
}

impl<A> JacquardSimulator<A>
where
    A: JacquardHostAdapter,
{
    #[must_use]
    pub fn new(adapter: A) -> Self {
        Self {
            harness: JacquardSimulationHarness::new(adapter),
        }
    }
}

impl<A> RoutingSimulator for JacquardSimulator<A>
where
    A: JacquardHostAdapter,
{
    type EnvironmentModel = ScriptedEnvironmentModel;
    type Error = SimulationError;
    type ReplayArtifact = JacquardReplayArtifact;
    type Scenario = JacquardScenario;
    type SimulationStats = JacquardSimulationStats;

    fn run_scenario(
        &mut self,
        scenario: &Self::Scenario,
        environment: &Self::EnvironmentModel,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error> {
        self.harness.run(scenario, environment)
    }

    fn resume_replay(
        &mut self,
        replay: &Self::ReplayArtifact,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error> {
        self.harness.resume_from_checkpoint(replay)
    }
}

impl<A> RoutingReplayView for JacquardSimulator<A> {
    type ReplayArtifact = JacquardReplayArtifact;

    fn route_events<'a>(
        &self,
        replay: &'a Self::ReplayArtifact,
    ) -> &'a [jacquard_core::RouteEvent] {
        &replay.route_events
    }

    fn stamped_route_events<'a>(
        &self,
        replay: &'a Self::ReplayArtifact,
    ) -> &'a [jacquard_core::RouteEventStamped] {
        &replay.stamped_route_events
    }
}

fn host_artifact(
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

fn summarize_field_replay(router: &ReferenceRouter) -> Option<FieldReplaySummary> {
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
        last_promotion_decision,
        last_promotion_blocker,
        bootstrap_activation_count,
        bootstrap_hold_count,
        bootstrap_narrow_count,
        bootstrap_upgrade_count,
        bootstrap_withdraw_count,
        protocol_reconfiguration_count,
        route_bound_reconfiguration_count,
        continuation_shift_count,
        corridor_narrow_count,
        checkpoint_capture_count,
        checkpoint_restore_count,
        reconfiguration_causes,
    })
}

fn advanced_status(report: &BridgeRoundReport) -> HostRoundStatus {
    HostRoundStatus::Advanced {
        router_outcome: report.router_outcome.clone(),
        ingested_transport_observation_count: report.ingested_transport_observations.len(),
        flushed_transport_commands: report.flushed_transport_commands,
        dropped_transport_observations: report.dropped_transport_observations,
    }
}

fn collect_route_events(
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    cursors: &mut BTreeMap<NodeId, usize>,
    route_events: &mut Vec<jacquard_core::RouteEvent>,
    stamped_route_events: &mut Vec<jacquard_core::RouteEventStamped>,
) {
    for (node_id, host) in hosts {
        let cursor = cursors.entry(*node_id).or_insert(0);
        let new_events = {
            let owner = host.bind();
            owner.router().effects().events[*cursor..].to_vec()
        };
        route_events.extend(new_events.iter().map(|event| event.event.clone()));
        stamped_route_events.extend(new_events.iter().cloned());
        *cursor = cursor.saturating_add(new_events.len());
    }
}

fn summarize_active_routes(
    owner_node_id: NodeId,
    router: &ReferenceRouter,
) -> Vec<ActiveRouteSummary> {
    router
        .active_routes_snapshot()
        .into_iter()
        .map(|route| ActiveRouteSummary {
            owner_node_id,
            route_id: route.identity.stamp.route_id,
            destination: route.identity.admission.objective.destination,
            engine_id: route.identity.admission.summary.engine,
            last_lifecycle_event: route.runtime.last_lifecycle_event,
            reachability_state: route.runtime.health.reachability_state,
            stability_score: route.runtime.health.stability_score,
        })
        .collect()
}

fn refresh_host_round_routes(
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

fn maintain_active_routes(
    router: &mut ReferenceRouter,
    topology: &Observation<Configuration>,
    round_index: u32,
    failure_summaries: &mut Vec<SimulationFailureSummary>,
) {
    let route_ids = router
        .active_routes_snapshot()
        .into_iter()
        .map(|route| {
            let trigger = if route.identity.stamp.topology_epoch != topology.value.epoch {
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

fn capture_host_snapshots(
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

fn restore_pathway_hosts(
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

fn failure_summaries_for(
    checkpoints: &[JacquardCheckpointArtifact],
    route_event_cursors: &BTreeMap<NodeId, usize>,
    driver_status_events: &[DriverStatusEvent],
) -> Vec<SimulationFailureSummary> {
    let mut summaries = Vec::new();
    if checkpoints.is_empty() {
        summaries.push(SimulationFailureSummary {
            round_index: None,
            detail: "no deterministic checkpoints were emitted during the run".to_string(),
        });
    }
    if route_event_cursors.values().all(|count| *count == 0) {
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
fn stitch_replay_from_checkpoint(
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

fn activate_ready_objectives(
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
            .activate_route_without_tick(binding.objective.clone())
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
