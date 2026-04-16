use std::{collections::BTreeMap, sync::Arc};

use jacquard_core::{
    ConnectivityPosture, DestinationId, DurationMs, HoldFallbackPolicy, NodeId, PriorityPoints,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RoutingObjective, Tick,
};
use jacquard_field::{FieldExportedReplayBundle, FIELD_ENGINE_ID};
use jacquard_mem_link_profile::SharedInMemoryNetwork;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_reference_client::{
    BridgeRoundProgress, BridgeRoundReport, ClientBuilder,
    FieldBootstrapSummary as ClientFieldBootstrapSummary, ReferenceClient,
    ReferenceClientBuildError, ReferenceRouter,
};
use jacquard_traits::{
    purity, Router, RoutingControlPlane, RoutingEnvironmentModel, RoutingReplayView,
    RoutingScenario, RoutingSimulator,
};
use telltale_simulator::{BatchConfig, SimRng};
use thiserror::Error;

use crate::{
    environment::ScriptedEnvironmentModel,
    reduced_replay::{ReducedFieldReplayObservation, ReducedReplayRound, ReducedReplayView},
    replay::{
        ActiveRouteSummary, DriverStatusEvent, FieldReplaySummary, HostCheckpointSnapshot,
        HostRoundArtifact, HostRoundStatus, IngressBatchBoundary, JacquardCheckpointArtifact,
        JacquardReplayArtifact, JacquardRoundArtifact, JacquardSimulationStats,
        SimulationFailureSummary, TelltaleNativeArtifactRef,
    },
    scenario::{BoundObjective, EngineLane, JacquardScenario},
};

mod replay_support;

pub(crate) use replay_support::default_objective;
use replay_support::{
    activate_ready_objectives, capture_host_snapshots, collect_route_events, failure_summaries_for,
    host_artifact, maintain_active_routes, refresh_host_round_routes, restore_pathway_hosts,
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
                EngineLane::Scatter => ClientBuilder::scatter(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::PathwayAndBatmanBellman => ClientBuilder::pathway_and_batman_bellman(
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
                EngineLane::FieldAndBatmanBellman => ClientBuilder::field_and_batman_bellman(
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
                EngineLane::BatmanBellman => ClientBuilder::batman_bellman(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::BatmanClassic => ClientBuilder::batman_classic(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::Babel => ClientBuilder::babel(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::OlsrV2 => ClientBuilder::olsrv2(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::PathwayAndBabel => ClientBuilder::pathway_and_babel(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::PathwayAndOlsrV2 => ClientBuilder::pathway_and_olsrv2(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::BabelAndBatmanBellman => ClientBuilder::babel_and_batman_bellman(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                ),
                EngineLane::OlsrV2AndBatmanBellman => ClientBuilder::olsrv2_and_batman_bellman(
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
            if let Some(batman_bellman_decay_window) = host.overrides.batman_bellman_decay_window {
                builder = builder.with_batman_bellman_decay_window(batman_bellman_decay_window);
            }
            if let Some(batman_classic_decay_window) = host.overrides.batman_classic_decay_window {
                builder = builder.with_batman_classic_decay_window(batman_classic_decay_window);
            }
            if let Some(babel_decay_window) = host.overrides.babel_decay_window {
                builder = builder.with_babel_decay_window(babel_decay_window);
            }
            if let Some(olsrv2_decay_window) = host.overrides.olsrv2_decay_window {
                builder = builder.with_olsrv2_decay_window(olsrv2_decay_window);
            }
            if let Some(pathway_search_config) = host.overrides.pathway_search_config.clone() {
                builder = builder.with_pathway_search_config(pathway_search_config);
            }
            if let Some(field_search_config) = host.overrides.field_search_config.clone() {
                builder = builder.with_field_search_config(field_search_config);
            }
            if let Some(scatter_config) = host.overrides.scatter_config {
                builder = builder.with_scatter_config(scatter_config);
            }
            for bootstrap in &host.overrides.field_bootstrap_summaries {
                builder = builder.with_field_bootstrap_summary(ClientFieldBootstrapSummary {
                    destination: bootstrap.destination.clone(),
                    from_neighbor: bootstrap.from_neighbor,
                    forward_observation: bootstrap.forward_observation,
                    reverse_feedback: bootstrap.reverse_feedback,
                });
            }
            let client = builder.build()?;
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
        if !replay.scenario.all_hosts_pathway() {
            return self.run(&replay.scenario, &replay.environment_model);
        }
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
        let mut full_rounds = Vec::new();
        let mut reduced_rounds = Vec::new();
        let mut checkpoints = Vec::new();
        let mut checkpoint_count = 0usize;
        let mut route_event_count = 0usize;
        let mut advanced_round_count = 0u32;
        let mut waiting_round_count = 0u32;
        let mut completed_round_count = 0u32;

        let mut activated_objectives =
            vec![resume_from.is_some(); scenario.bound_objectives().len()];
        let checkpoint_round_offset =
            resume_from.map_or(0, |checkpoint| checkpoint.completed_rounds);

        for round_index in 0..scenario.round_limit() {
            let prior_topology_epoch = topology.value.epoch;
            let at_tick = Tick(topology.observed_at_tick.0.saturating_add(1));
            let (next_topology, environment_artifacts) =
                environment.advance_environment(&topology.value, at_tick);
            topology = next_topology;
            let topology_advanced = topology.value.epoch != prior_topology_epoch;
            let shared_topology = Arc::new(topology.clone());

            let mut host_rounds = Vec::new();
            let mut all_waiting = true;
            for host in scenario.hosts() {
                let bridge = hosts
                    .get_mut(&host.local_node_id)
                    .ok_or(SimulationError::MissingBridge(host.local_node_id))?;
                let mut bound = bridge.bind();
                bound.replace_shared_topology_shared(shared_topology.clone());
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

            if activate_ready_objectives(
                scenario.bound_objectives(),
                checkpoint_round_offset + round_index,
                &mut activated_objectives,
                &mut hosts,
                &mut failure_summaries,
            ) {
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
                        active_routes: host_rounds
                            .iter()
                            .flat_map(|host| host.active_routes.iter().cloned())
                            .collect(),
                        environment_hooks: environment_artifacts,
                        field_replays: host_rounds
                            .iter()
                            .filter_map(|host| {
                                host.field_replay.clone().map(|summary| {
                                    ReducedFieldReplayObservation {
                                        local_node_id: host.local_node_id,
                                        summary,
                                    }
                                })
                            })
                            .collect(),
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
                let distinct_engine_ids = reduced_rounds
                    .iter()
                    .flat_map(|round| {
                        round
                            .active_routes
                            .iter()
                            .map(|route| route.engine_id.clone())
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
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
