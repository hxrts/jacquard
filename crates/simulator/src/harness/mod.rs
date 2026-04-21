//! Deterministic simulator harness orchestrating scenario execution and round advancement.

use std::{collections::BTreeMap, env, sync::Arc};

use jacquard_core::{
    ConnectivityPosture, DestinationId, DurationMs, HoldFallbackPolicy, NodeId, PriorityPoints,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RoutingObjective, Tick,
};
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_mem_link_profile::SharedInMemoryNetwork;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_reference_client::{
    BridgeRoundProgress, BridgeRoundReport, ReferenceClient, ReferenceClientBuildError,
    ReferenceRouter,
};
use jacquard_traits::{
    purity, Router, RoutingControlPlane, RoutingDataPlane, RoutingEnvironmentModel,
    RoutingReplayView, RoutingScenario, RoutingSimulator,
};
use telltale_simulator::{BatchConfig, SimRng};
use thiserror::Error;

use crate::{
    environment::ScriptedEnvironmentModel,
    reduced_replay::{
        distinct_engine_ids_for, filter_active_routes, filter_field_replays,
        objective_owner_nodes_for, objective_route_filter_for, ReducedFieldReplayObservation,
        ReducedReplayRound, ReducedReplayView,
    },
    replay::{
        ActiveRouteSummary, DriverStatusEvent, FieldReplaySummary, HostCheckpointSnapshot,
        HostRoundArtifact, HostRoundStatus, IngressBatchBoundary, JacquardCheckpointArtifact,
        JacquardReplayArtifact, JacquardRoundArtifact, JacquardSimulationStats,
        SimulationFailureSummary, TelltaleNativeArtifactRef,
    },
    scenario::{BoundObjective, JacquardScenario},
};

mod build_plan;
mod replay_support;

use build_plan::host_build_plans;
pub(crate) use replay_support::default_objective;
use replay_support::{
    activate_ready_objectives, capture_host_snapshots, collect_route_events, failure_summaries_for,
    host_artifact, maintain_active_routes, reactivate_missing_objectives,
    refresh_host_round_routes, restore_checkpointed_hosts, stimulate_scatter_routes,
    stitch_replay_from_checkpoint, summarize_active_routes, summarize_field_replay,
    TopologyAdvance,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCaptureLevel {
    FullReplay,
    ReducedReplay,
    SummaryOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulationCaptureArtifact {
    FullReplay(Box<JacquardReplayArtifact>),
    ReducedReplay(Box<ReducedReplayView>),
    SummaryOnly,
}

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
    #[error("reference client build error: {0}")]
    Build(#[from] ReferenceClientBuildError),
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
    fn build_hosts(
        &self,
        scenario: &JacquardScenario,
    ) -> Result<BTreeMap<NodeId, ReferenceClient>, SimulationError> {
        let topology = scenario.initial_configuration().clone();
        let network = SharedInMemoryNetwork::default();
        let mut hosts = BTreeMap::new();

        for plan in host_build_plans(scenario) {
            let local_node_id = plan.local_node_id();
            let builder =
                plan.into_builder(topology.clone(), network.clone(), topology.observed_at_tick);
            let client = builder.build()?;
            hosts.insert(local_node_id, client);
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
        match self.run_with_capture(scenario, environment, SimulationCaptureLevel::FullReplay)? {
            (SimulationCaptureArtifact::FullReplay(replay), stats) => Ok((*replay, stats)),
            _ => unreachable!("full replay capture must return a full replay artifact"),
        }
    }

    pub fn run_reduced(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
    ) -> Result<(ReducedReplayView, JacquardSimulationStats), SimulationError> {
        match self.run_with_capture(scenario, environment, SimulationCaptureLevel::ReducedReplay)? {
            (SimulationCaptureArtifact::ReducedReplay(replay), stats) => Ok((*replay, stats)),
            _ => unreachable!("reduced capture must return a reduced replay artifact"),
        }
    }

    pub fn run_with_capture(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
        capture_level: SimulationCaptureLevel,
    ) -> Result<(SimulationCaptureArtifact, JacquardSimulationStats), SimulationError> {
        self.run_from_state(scenario, environment, None, capture_level)
    }

    pub fn resume_from_checkpoint(
        &self,
        replay: &JacquardReplayArtifact,
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError> {
        let Some(checkpoint) = replay.checkpoints.last() else {
            return self.run(&replay.scenario, &replay.environment_model);
        };
        let (suffix_artifact, suffix_stats) = self.run_from_state(
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
            SimulationCaptureLevel::FullReplay,
        )?;
        let SimulationCaptureArtifact::FullReplay(suffix_replay) = suffix_artifact else {
            unreachable!("checkpoint resume must return a full replay artifact");
        };
        Ok(stitch_replay_from_checkpoint(
            replay,
            checkpoint.completed_rounds,
            *suffix_replay,
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
        capture_level: SimulationCaptureLevel,
    ) -> Result<(SimulationCaptureArtifact, JacquardSimulationStats), SimulationError> {
        let mut _rng = SimRng::new(scenario.seed().0);
        let mut topology = match resume_from {
            Some(checkpoint) => checkpoint.topology.clone(),
            None => scenario.initial_configuration().clone(),
        };
        let mut hosts = self.adapter.build_hosts(scenario)?;
        if let Some(checkpoint) = resume_from {
            restore_checkpointed_hosts(&mut hosts, checkpoint)?;
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
        let mut full_rounds = Vec::new();
        let mut reduced_rounds = Vec::new();
        let mut checkpoints = Vec::new();
        let mut checkpoint_count = 0usize;
        let mut route_event_count = 0usize;
        let mut advanced_round_count = 0u32;
        let mut waiting_round_count = 0u32;
        let mut completed_round_count = 0u32;
        let reduced_objective_routes = objective_route_filter_for(scenario);
        let reduced_objective_owner_nodes = objective_owner_nodes_for(scenario);

        let mut activated_objectives =
            vec![resume_from.is_some(); scenario.bound_objectives().len()];
        let checkpoint_round_offset =
            resume_from.map_or(0, |checkpoint| checkpoint.completed_rounds);
        let mut topology_history = vec![topology.clone()];

        for round_index in 0..scenario.round_limit() {
            let prior_topology_epoch = topology.value.epoch;
            let at_tick = Tick(topology.observed_at_tick.0.saturating_add(1));
            let (next_topology, environment_artifacts) =
                environment.advance_environment(&topology.value, at_tick);
            topology = next_topology;
            topology_history.push(topology.clone());
            let topology_advanced = topology.value.epoch != prior_topology_epoch;
            let shared_topology = Arc::new(topology.clone());

            let mut host_rounds = Vec::new();
            let mut all_waiting = true;
            for host in scenario.hosts() {
                let bridge = hosts
                    .get_mut(&host.local_node_id)
                    .ok_or(SimulationError::MissingBridge(host.local_node_id))?;
                let mut bound = bridge.bind();
                let lag_rounds = scenario
                    .lag_rounds_for(host.local_node_id, checkpoint_round_offset + round_index);
                let history_index = topology_history
                    .len()
                    .saturating_sub(1)
                    .saturating_sub(usize::try_from(lag_rounds).unwrap_or(usize::MAX));
                let host_topology = Arc::new(topology_history[history_index].clone());
                bound.replace_shared_topology_shared(host_topology.clone());
                let progress = bound.advance_round()?;
                maintain_active_routes(
                    bound.router_mut(),
                    if topology_advanced {
                        TopologyAdvance::Advanced
                    } else {
                        TopologyAdvance::Unchanged
                    },
                    checkpoint_round_offset + round_index,
                    &mut failure_summaries,
                );
                stimulate_scatter_routes(
                    bound.router_mut(),
                    checkpoint_round_offset + round_index,
                    &mut failure_summaries,
                );
                let dropped_transport_observations = match &progress {
                    BridgeRoundProgress::Advanced(report) => {
                        advanced_round_count = advanced_round_count.saturating_add(1);
                        all_waiting = false;
                        report.dropped_transport_observations
                    }
                    BridgeRoundProgress::Waiting(waiting) => {
                        waiting_round_count = waiting_round_count.saturating_add(1);
                        waiting.dropped_transport_observations
                    }
                };
                if dropped_transport_observations > 0 {
                    driver_status_events.push(DriverStatusEvent::IngressDropped {
                        local_node_id: host.local_node_id,
                        at_tick,
                        dropped_transport_observations,
                    });
                }
                if !matches!(capture_level, SimulationCaptureLevel::SummaryOnly) {
                    let active_routes = summarize_active_routes(host.local_node_id, bound.router());
                    let field_replay = summarize_field_replay(bound.router());
                    host_rounds.push(host_artifact(
                        host.local_node_id,
                        at_tick,
                        &progress,
                        active_routes,
                        field_replay,
                    ));
                }
            }

            route_event_count = route_event_count.saturating_add(match capture_level {
                SimulationCaptureLevel::FullReplay => collect_route_events(
                    &mut hosts,
                    &mut route_event_cursors,
                    &mut all_route_events,
                    &mut all_stamped_route_events,
                ),
                SimulationCaptureLevel::ReducedReplay | SimulationCaptureLevel::SummaryOnly => {
                    replay_support::advance_route_event_cursors(
                        &mut hosts,
                        &mut route_event_cursors,
                    )
                }
            });

            let activated_any = activate_ready_objectives(
                scenario.bound_objectives(),
                checkpoint_round_offset + round_index,
                &mut activated_objectives,
                &mut hosts,
                &mut failure_summaries,
            );
            let reactivated_any = reactivate_missing_objectives(
                scenario.bound_objectives(),
                checkpoint_round_offset + round_index,
                &activated_objectives,
                &mut hosts,
                &mut failure_summaries,
            );
            if activated_any || reactivated_any {
                if !matches!(capture_level, SimulationCaptureLevel::SummaryOnly) {
                    refresh_host_round_routes(&mut host_rounds, &mut hosts);
                }
                route_event_count = route_event_count.saturating_add(match capture_level {
                    SimulationCaptureLevel::FullReplay => collect_route_events(
                        &mut hosts,
                        &mut route_event_cursors,
                        &mut all_route_events,
                        &mut all_stamped_route_events,
                    ),
                    SimulationCaptureLevel::ReducedReplay | SimulationCaptureLevel::SummaryOnly => {
                        replay_support::advance_route_event_cursors(
                            &mut hosts,
                            &mut route_event_cursors,
                        )
                    }
                });
            }

            trace_host_state(
                scenario.name(),
                checkpoint_round_offset + round_index,
                &mut hosts,
                route_event_count,
            );

            if let Some(interval) = scenario.checkpoint_interval() {
                if interval > 0 && (round_index + 1) % interval == 0 {
                    checkpoint_count = checkpoint_count.saturating_add(1);
                    if matches!(capture_level, SimulationCaptureLevel::FullReplay) {
                        checkpoints.push(JacquardCheckpointArtifact {
                            completed_rounds: checkpoint_round_offset + round_index + 1,
                            topology: shared_topology.as_ref().clone(),
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
            }

            match capture_level {
                SimulationCaptureLevel::FullReplay => {
                    full_rounds.push(JacquardRoundArtifact {
                        round_index: checkpoint_round_offset + round_index,
                        topology: shared_topology.as_ref().clone(),
                        environment_artifacts,
                        host_rounds,
                    });
                }
                SimulationCaptureLevel::ReducedReplay => {
                    reduced_rounds.push(ReducedReplayRound {
                        round_index: checkpoint_round_offset + round_index,
                        active_routes: filter_active_routes(
                            host_rounds
                                .iter()
                                .flat_map(|host| host.active_routes.iter().cloned()),
                            &reduced_objective_routes,
                        ),
                        environment_hooks: environment_artifacts,
                        field_replays: filter_field_replays(
                            host_rounds.iter().filter_map(|host| {
                                host.field_replay.clone().map(|summary| {
                                    ReducedFieldReplayObservation {
                                        local_node_id: host.local_node_id,
                                        summary,
                                    }
                                })
                            }),
                            &reduced_objective_owner_nodes,
                        ),
                    });
                }
                SimulationCaptureLevel::SummaryOnly => {}
            }

            completed_round_count = completed_round_count.saturating_add(1);

            if all_waiting
                && scenario.bound_objectives().is_empty()
                && environment.is_quiescent_after(at_tick)
            {
                break;
            }
        }

        let total_completed_rounds = checkpoint_round_offset.saturating_add(completed_round_count);
        failure_summaries.extend(failure_summaries_for(
            checkpoint_count,
            route_event_count,
            &driver_status_events,
        ));
        let stats = JacquardSimulationStats {
            executed_round_count: completed_round_count,
            advanced_round_count,
            waiting_round_count,
            route_event_count,
            checkpoint_count,
            driver_status_event_count: driver_status_events.len(),
            failure_summary_count: failure_summaries.len(),
        };

        let artifact = match capture_level {
            SimulationCaptureLevel::FullReplay => {
                let replay = JacquardReplayArtifact {
                    scenario: scenario.clone(),
                    environment_model: environment.clone(),
                    rounds: full_rounds,
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
                self.adapter.validate_result(scenario, &replay, &stats)?;
                SimulationCaptureArtifact::FullReplay(Box::new(replay))
            }
            SimulationCaptureLevel::ReducedReplay => {
                let distinct_engine_ids = distinct_engine_ids_for(&reduced_rounds);
                SimulationCaptureArtifact::ReducedReplay(Box::new(ReducedReplayView {
                    scenario_name: scenario.name().to_owned(),
                    round_count: completed_round_count,
                    rounds: reduced_rounds,
                    distinct_engine_ids,
                    driver_status_events,
                    failure_summaries,
                }))
            }
            SimulationCaptureLevel::SummaryOnly => SimulationCaptureArtifact::SummaryOnly,
        };
        Ok((artifact, stats))
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

    #[must_use]
    pub fn host_adapter(&self) -> &A {
        &self.harness.adapter
    }

    pub fn run_scenario_with_capture(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
        capture_level: SimulationCaptureLevel,
    ) -> Result<(SimulationCaptureArtifact, JacquardSimulationStats), SimulationError> {
        self.harness
            .run_with_capture(scenario, environment, capture_level)
    }

    pub fn run_scenario_reduced(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
    ) -> Result<(ReducedReplayView, JacquardSimulationStats), SimulationError> {
        self.harness.run_reduced(scenario, environment)
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

fn trace_host_state(
    scenario_name: &str,
    round_index: u32,
    hosts: &mut BTreeMap<NodeId, ReferenceClient>,
    route_event_count: usize,
) {
    if env::var("JACQUARD_TUNING_HOST_STATE").as_deref() != Ok("1") {
        return;
    }
    let mut total_events = 0usize;
    let mut total_storage_entries = 0usize;
    let mut total_storage_bytes = 0usize;
    let mut total_active_routes = 0usize;
    for host in hosts.values_mut() {
        let binding = host.bind();
        let router = binding.router();
        total_events = total_events.saturating_add(router.effects().events.len());
        total_storage_entries =
            total_storage_entries.saturating_add(router.effects().storage.len());
        total_storage_bytes = total_storage_bytes.saturating_add(
            router
                .effects()
                .storage
                .iter()
                .map(|(key, value): (&Vec<u8>, &Vec<u8>)| key.len().saturating_add(value.len()))
                .sum::<usize>(),
        );
        total_active_routes = total_active_routes.saturating_add(router.active_route_count());
    }
    eprintln!(
        "[tuning-host-state] scenario={scenario_name} round={round_index} active_routes={total_active_routes} route_events_seen={route_event_count} live_event_log={total_events} storage_entries={total_storage_entries} storage_bytes={total_storage_bytes}"
    );
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
