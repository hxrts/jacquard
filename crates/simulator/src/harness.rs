use std::collections::BTreeMap;

use jacquard_batman::BatmanEngine;
use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DiversityFloor, DurationMs,
    HealthScore, IdentityAssuranceClass, NodeId, Observation, OperatingMode,
    PriorityPoints, RatioPermille, RoutePartitionClass, RouteProtectionClass,
    RouteRepairClass, RouteReplacementPolicy, RoutingEngineFallbackPolicy,
    RoutingObjective, RoutingPolicyInputs, SelectedRoutingParameters, Tick,
};
use jacquard_mem_link_profile::{
    InMemoryRuntimeEffects, InMemoryTransport, SharedInMemoryNetwork,
};
use jacquard_reference_client::{
    BridgeRoundProgress, BridgeRoundReport, ClientBuilder, HostBridge, PathwayClient,
    PathwayRouter,
};
use jacquard_router::{FixedPolicyEngine, MultiEngineRouter};
use jacquard_traits::{
    Router, RoutingEnvironmentModel, RoutingReplayView, RoutingScenario,
    RoutingSimulator,
};
use telltale_simulator::{BatchConfig, SimRng};
use thiserror::Error;

use crate::{
    environment::ScriptedEnvironmentModel,
    replay::{
        DriverStatusEvent, HostCheckpointSnapshot, HostRoundArtifact, HostRoundStatus,
        IngressBatchBoundary, JacquardCheckpointArtifact, JacquardReplayArtifact,
        JacquardRoundArtifact, JacquardSimulationStats, SimulationFailureSummary,
        TelltaleNativeArtifactRef,
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

pub trait JacquardHostAdapter {
    fn build_hosts(
        &self,
        scenario: &JacquardScenario,
    ) -> Result<BTreeMap<NodeId, PathwayClient>, SimulationError>;

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
    ) -> Result<BTreeMap<NodeId, PathwayClient>, SimulationError> {
        let topology = scenario.initial_configuration().clone();
        let network = SharedInMemoryNetwork::default();
        let mut hosts = BTreeMap::new();

        for host in scenario.hosts() {
            let client = match host.lane {
                | EngineLane::Pathway => ClientBuilder::pathway(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                )
                .build(),
                | EngineLane::PathwayAndBatman => ClientBuilder::pathway_and_batman(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                    topology.observed_at_tick,
                )
                .build(),
                | EngineLane::Batman => build_batman_only_client(
                    host.local_node_id,
                    topology.clone(),
                    network.clone(),
                )?,
            };
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
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError>
    {
        self.run_from_state(scenario, environment, None)
    }

    pub fn resume_from_checkpoint(
        &self,
        replay: &JacquardReplayArtifact,
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError>
    {
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

    fn run_from_state(
        &self,
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
        resume_from: Option<&JacquardCheckpointArtifact>,
    ) -> Result<(JacquardReplayArtifact, JacquardSimulationStats), SimulationError>
    {
        let mut _rng = SimRng::new(scenario.seed().0);
        let mut topology = match resume_from {
            | Some(checkpoint) => checkpoint.topology.clone(),
            | None => scenario.initial_configuration().clone(),
        };
        let mut hosts = self.adapter.build_hosts(scenario)?;
        if let Some(checkpoint) = resume_from {
            restore_pathway_hosts(&mut hosts, checkpoint)?;
        }
        let mut route_event_cursors = match resume_from {
            | Some(checkpoint) => checkpoint
                .host_snapshots
                .iter()
                .map(|(node_id, snapshot)| {
                    (*node_id, snapshot.runtime_effects.events.len())
                })
                .collect(),
            | None => hosts
                .keys()
                .copied()
                .map(|node_id| (node_id, 0usize))
                .collect(),
        };
        let mut all_route_events = Vec::new();
        let mut all_stamped_route_events = Vec::new();
        let mut driver_status_events = Vec::new();
        let mut rounds = Vec::new();
        let mut checkpoints = Vec::new();
        let mut advanced_round_count = 0u32;
        let mut waiting_round_count = 0u32;

        let mut objectives_activated = resume_from.is_some();
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
                let artifact = host_artifact(host.local_node_id, at_tick, &progress);
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

            if !objectives_activated {
                activate_objectives(scenario.bound_objectives(), &mut hosts)?;
                collect_route_events(
                    &mut hosts,
                    &mut route_event_cursors,
                    &mut all_route_events,
                    &mut all_stamped_route_events,
                );
                objectives_activated = true;
            }

            if let Some(interval) = scenario.checkpoint_interval() {
                if interval > 0 && (round_index + 1) % interval == 0 {
                    checkpoints.push(JacquardCheckpointArtifact {
                        completed_rounds: checkpoint_round_offset + round_index + 1,
                        topology: topology.clone(),
                        host_snapshots: capture_host_snapshots(&mut hosts),
                        telltale_native: scenario.all_hosts_pathway().then_some(
                            TelltaleNativeArtifactRef::PathwayCheckpointRecovery {
                                completed_rounds: checkpoint_round_offset
                                    + round_index
                                    + 1,
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

        let total_completed_rounds = checkpoint_round_offset
            .saturating_add(u32::try_from(rounds.len()).unwrap_or(u32::MAX));
        let failure_summaries = failure_summaries_for(
            &checkpoints,
            &route_event_cursors,
            &driver_status_events,
        );
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
        let _ = self.telltale_batch;
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
) -> HostRoundArtifact {
    let ingress_batch_boundary = IngressBatchBoundary {
        observed_at_tick: at_tick,
        ingested_transport_observation_count: match progress {
            | BridgeRoundProgress::Advanced(report) => {
                report.ingested_transport_observations.len()
            },
            | BridgeRoundProgress::Waiting(_) => 0,
        },
    };
    let status = match progress {
        | BridgeRoundProgress::Advanced(report) => advanced_status(report),
        | BridgeRoundProgress::Waiting(waiting) => HostRoundStatus::Waiting {
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
    }
}

fn advanced_status(report: &BridgeRoundReport) -> HostRoundStatus {
    HostRoundStatus::Advanced {
        router_outcome: report.router_outcome.clone(),
        ingested_transport_observation_count: report
            .ingested_transport_observations
            .len(),
        flushed_transport_commands: report.flushed_transport_commands,
        dropped_transport_observations: report.dropped_transport_observations,
    }
}

fn collect_route_events(
    hosts: &mut BTreeMap<NodeId, PathwayClient>,
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

fn capture_host_snapshots(
    hosts: &mut BTreeMap<NodeId, PathwayClient>,
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
    hosts: &mut BTreeMap<NodeId, PathwayClient>,
    checkpoint: &JacquardCheckpointArtifact,
) -> Result<(), SimulationError> {
    for (node_id, snapshot) in &checkpoint.host_snapshots {
        let host = hosts
            .get_mut(node_id)
            .ok_or(SimulationError::MissingBridge(*node_id))?;
        let mut bound = host.bind();
        *bound.router_mut().effects_mut() = snapshot.runtime_effects.clone();
        let _ = bound.router_mut().recover_checkpointed_routes()?;
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
            detail: "no deterministic checkpoints were emitted during the run"
                .to_string(),
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

fn stitch_replay_from_checkpoint(
    replay: &JacquardReplayArtifact,
    completed_rounds: u32,
    suffix_replay: JacquardReplayArtifact,
    _suffix_stats: JacquardSimulationStats,
) -> (JacquardReplayArtifact, JacquardSimulationStats) {
    let prefix_len = usize::try_from(completed_rounds).unwrap_or(usize::MAX);
    let mut rounds =
        replay.rounds[..std::cmp::min(prefix_len, replay.rounds.len())].to_vec();
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

fn activate_objectives(
    objectives: &[BoundObjective],
    hosts: &mut BTreeMap<NodeId, PathwayClient>,
) -> Result<(), SimulationError> {
    for binding in objectives {
        let bridge = hosts
            .get_mut(&binding.owner_node_id)
            .ok_or(SimulationError::MissingBridge(binding.owner_node_id))?;
        let mut bound = bridge.bind();
        Router::activate_route(bound.router_mut(), binding.objective.clone())?;
    }
    Ok(())
}

fn build_batman_only_client(
    local_node_id: NodeId,
    topology: Observation<Configuration>,
    network: SharedInMemoryNetwork,
) -> Result<PathwayClient, SimulationError> {
    let local_endpoint = topology.value.nodes[&local_node_id]
        .profile
        .endpoints
        .first()
        .cloned()
        .ok_or(SimulationError::MissingEndpoint(local_node_id))?;
    let bridge_transport = InMemoryTransport::attach(
        local_node_id,
        [local_endpoint.clone()],
        network.clone(),
    );
    let engine_transport =
        InMemoryTransport::attach(local_node_id, [local_endpoint], network);
    let now = topology.observed_at_tick;
    let mut router: PathwayRouter = MultiEngineRouter::new(
        local_node_id,
        FixedPolicyEngine::new(default_profile()),
        InMemoryRuntimeEffects { now, ..Default::default() },
        topology.clone(),
        policy_inputs_for(&topology, local_node_id),
    );
    router.register_engine(Box::new(BatmanEngine::new(
        local_node_id,
        engine_transport,
        InMemoryRuntimeEffects { now, ..Default::default() },
    )))?;
    Ok(HostBridge::new(topology, router, bridge_transport))
}

fn default_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

fn policy_inputs_for(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&local_node_id].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
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
