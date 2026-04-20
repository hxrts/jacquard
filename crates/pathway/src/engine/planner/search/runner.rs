// Search runner: drives telltale-search for one pathway planning request.
//
// `search_record_for_objective` resolves the routing objective to one
// v13-native `SearchQuery`, executes one search machine for that query, and
// returns a `PathwayPlannerSearchRecord` that the candidate-ranking layer
// consumes. Epoch reconfiguration is committed when the observed topology
// changes between calls, so the underlying `SearchMachine` always operates on a
// consistent frozen snapshot matched to the current `PathwaySearchEpoch`.

use cfg_if::cfg_if;
use jacquard_core::{Configuration, NodeId, Observation, RoutingObjective};
use telltale_search::{
    commit_epoch_reconfiguration, run_with_executor, run_with_executor_report_only,
    validate_run_config, EpochReconfigurationRequest, SearchMachine, SearchQuery,
    SearchSchedulerProfile, SerialProposalExecutor,
};

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::num::NonZeroU64;
        use telltale_search::NativeParallelExecutor;
    }
}

use super::{
    freeze_snapshot_for_search, reconstruct_candidate_node_paths,
    reconstruct_candidate_node_paths_from_parent_records, snapshot_id_for_configuration,
    PathwayPlannerSearchRecord, PathwaySearchConfig, PathwaySearchDomain, PathwaySearchEdgeMeta,
    PathwaySearchEpoch, PathwaySearchReconfiguration, PathwaySearchRun,
    PathwaySearchTransitionClass,
};
use crate::{
    engine::PathwayEngine,
    topology::{adjacent_node_ids, estimate_hop_link, objective_matches_node},
    PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess, PATHWAY_ENGINE_ID,
};

type PathwaySearchExecutionReport =
    telltale_search::SearchExecutionReport<NodeId, PathwaySearchEpoch, u32>;
type PathwaySuccessorRow = (NodeId, Vec<(NodeId, PathwaySearchEdgeMeta, u32)>);
type PathwaySearchReplayArtifact = telltale_search::SearchReplayArtifact<
    NodeId,
    PathwaySearchEpoch,
    super::PathwaySearchSnapshotId,
    u32,
>;
type PathwaySearchRunResult = Result<
    (
        PathwaySearchExecutionReport,
        Option<PathwaySearchReplayArtifact>,
    ),
    telltale_search::SearchRunError<&'static str>,
>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PathwaySearchSnapshotState {
    pub epoch: PathwaySearchEpoch,
    pub topology: Observation<Configuration>,
}

impl PathwaySearchSnapshotState {
    #[must_use]
    pub(crate) fn from_topology(topology: &Observation<Configuration>) -> Self {
        Self {
            epoch: PathwaySearchEpoch {
                route_epoch: topology.value.epoch,
                snapshot_id: snapshot_id_for_configuration(&topology.value),
            },
            topology: topology.clone(),
        }
    }
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::super::PathwayTopologyBounds,
    Topology::PeerEstimate: PathwayPeerEstimateAccess,
    Topology::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
{
    pub(in crate::engine::planner) fn search_record_for_objective(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> PathwayPlannerSearchRecord {
        let current_state = PathwaySearchSnapshotState::from_topology(topology);
        let query = self.resolve_query_for_objective(objective, topology);
        let prior_state = self.search_snapshot_state.borrow().clone();
        let run = query.as_ref().map(|query| {
            self.run_search_for_query(objective, topology, query, prior_state.as_ref())
        });

        let record = PathwayPlannerSearchRecord {
            objective: objective.clone(),
            query,
            run,
        };
        *self.search_snapshot_state.borrow_mut() = Some(current_state);
        *self.last_search_record.borrow_mut() = Some(record.clone());
        record
    }

    fn resolve_query_for_objective(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> Option<SearchQuery<NodeId>> {
        let mut accepted_nodes = match &objective.destination {
            jacquard_core::DestinationId::Node(target_node_id) => topology
                .value
                .nodes
                .get(target_node_id)
                .filter(|_| *target_node_id != self.local_node_id)
                .filter(|node| {
                    objective_matches_node(
                        target_node_id,
                        node,
                        objective,
                        &PATHWAY_ENGINE_ID,
                        topology.observed_at_tick,
                    )
                })
                .map_or_else(Vec::new, |_| vec![*target_node_id]),
            jacquard_core::DestinationId::Service(_) | jacquard_core::DestinationId::Gateway(_) => {
                topology
                    .value
                    .nodes
                    .iter()
                    .filter_map(|(node_id, node)| {
                        if *node_id == self.local_node_id {
                            return None;
                        }
                        objective_matches_node(
                            node_id,
                            node,
                            objective,
                            &PATHWAY_ENGINE_ID,
                            topology.observed_at_tick,
                        )
                        .then_some(*node_id)
                    })
                    .collect::<Vec<_>>()
            }
        };
        accepted_nodes.truncate(self.search_config.per_objective_query_budget());

        match accepted_nodes.as_slice() {
            [] => None,
            [goal_node_id] => Some(SearchQuery::single_goal(self.local_node_id, *goal_node_id)),
            _ => match objective.destination {
                jacquard_core::DestinationId::Node(_) => {
                    SearchQuery::try_multi_goal(self.local_node_id, accepted_nodes).ok()
                }
                jacquard_core::DestinationId::Service(_)
                | jacquard_core::DestinationId::Gateway(_) => {
                    SearchQuery::try_candidate_set(self.local_node_id, accepted_nodes, None).ok()
                }
            },
        }
    }

    fn run_search_for_query(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        query: &SearchQuery<NodeId>,
        prior_state: Option<&PathwaySearchSnapshotState>,
    ) -> PathwaySearchRun {
        let current_state = PathwaySearchSnapshotState::from_topology(topology);
        let capture_replay_artifact = self.search_config.capture_replay_artifact();
        let topology_transition =
            classify_transition(prior_state.map(|state| &state.epoch), &current_state.epoch);
        let reconfiguration = prior_state
            .filter(|state| state.epoch != current_state.epoch)
            .map(|state| PathwaySearchReconfiguration {
                from: state.epoch.clone(),
                to: current_state.epoch.clone(),
                reseeding_policy: self.search_config.reseeding_policy(),
                transition_class: topology_transition,
            });

        let domain =
            self.domain_for_query(objective, topology, query.accepted_nodes(), prior_state);
        let mut machine = SearchMachine::new_with_query(
            domain,
            reconfiguration
                .as_ref()
                .map_or_else(|| current_state.epoch.clone(), |step| step.from.clone()),
            query.clone(),
            self.search_config.epsilon(),
        );
        if !capture_replay_artifact {
            machine.set_selected_result_witness_export_enabled(false);
        }
        if let Some(step) = reconfiguration.as_ref() {
            commit_epoch_reconfiguration(
                &mut machine,
                EpochReconfigurationRequest {
                    next_epoch: step.to.clone(),
                    reseeding_policy: step.reseeding_policy,
                },
            );
        }
        let (report, replay) = self
            .execute_search_machine(&mut machine, &self.search_config)
            .expect("pathway search config is validated and domain snapshots are present");
        let candidate_node_paths = if capture_replay_artifact {
            reconstruct_candidate_node_paths(query, &report.observation.canonical_parent_map)
        } else {
            reconstruct_candidate_node_paths_from_parent_records(query, &machine.state().parent)
        };

        PathwaySearchRun {
            topology_transition,
            selected_node_path: report.observation.selected_result_witness.clone(),
            candidate_node_paths,
            reconfiguration,
            report,
            replay,
        }
    }

    fn domain_for_query(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        accepted_node_ids: &[NodeId],
        prior_state: Option<&PathwaySearchSnapshotState>,
    ) -> PathwaySearchDomain {
        let current_state = PathwaySearchSnapshotState::from_topology(topology);
        let mut snapshots = Vec::new();
        if let Some(state) = prior_state.filter(|state| state.epoch != current_state.epoch) {
            let prior_successors = self.freeze_successors_for_search(objective, &state.topology);
            let (prior_epoch, prior_snapshot) = freeze_snapshot_for_search(
                &state.topology,
                prior_successors,
                accepted_node_ids,
                self.search_config.heuristic_mode(),
            );
            snapshots.push((prior_epoch, prior_snapshot));
        }

        let current_successors = self.freeze_successors_for_search(objective, topology);
        let (current_epoch, current_snapshot) = freeze_snapshot_for_search(
            topology,
            current_successors,
            accepted_node_ids,
            self.search_config.heuristic_mode(),
        );
        snapshots.push((current_epoch, current_snapshot));
        snapshots.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        PathwaySearchDomain::new(snapshots)
    }

    fn freeze_successors_for_search(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> Vec<PathwaySuccessorRow> {
        let mut successors = Vec::new();
        for from_node_id in topology.value.nodes.keys().copied() {
            let mut edges = adjacent_node_ids(&from_node_id, &topology.value)
                .into_iter()
                .filter_map(|to_node_id| {
                    let edge_cost =
                        self.edge_metric_score(objective, topology, &from_node_id, &to_node_id)?;
                    let (endpoint, _) =
                        estimate_hop_link(&from_node_id, &to_node_id, &topology.value)?;
                    Some((
                        to_node_id,
                        PathwaySearchEdgeMeta {
                            from_node_id,
                            to_node_id,
                            endpoint,
                        },
                        edge_cost,
                    ))
                })
                .collect::<Vec<_>>();
            edges.sort_by(|left, right| left.0.cmp(&right.0));
            successors.push((from_node_id, edges));
        }
        successors
    }

    fn execute_search_machine(
        &self,
        machine: &mut SearchMachine<PathwaySearchDomain>,
        config: &PathwaySearchConfig,
    ) -> PathwaySearchRunResult {
        let run_config = config.run_config();
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                if config.capture_replay_artifact() {
                    validate_run_config::<PathwaySearchDomain, _>(&SerialProposalExecutor, &run_config)
                        .map_err(telltale_search::SearchRunError::InvalidConfig)?;
                    run_with_executor(machine, &SerialProposalExecutor, run_config)
                        .map(|(report, replay)| (report, Some(replay)))
                } else {
                    run_with_executor_report_only(machine, &SerialProposalExecutor, run_config)
                        .map(|report| (report, None))
                }
            } else {
                if config.scheduler_profile() == SearchSchedulerProfile::ThreadedExactSingleLane {
                    let executor = NativeParallelExecutor::new(
                        NonZeroU64::new(config.batch_width()).expect("batch width is non-zero"),
                    )
                    .expect("threaded exact config requires native parallel executor support");
                    if config.capture_replay_artifact() {
                        validate_run_config::<PathwaySearchDomain, _>(&executor, &run_config)
                            .map_err(telltale_search::SearchRunError::InvalidConfig)?;
                        run_with_executor(machine, &executor, run_config)
                            .map(|(report, replay)| (report, Some(replay)))
                    } else {
                        run_with_executor_report_only(machine, &executor, run_config)
                            .map(|report| (report, None))
                    }
                } else {
                    if config.capture_replay_artifact() {
                        validate_run_config::<PathwaySearchDomain, _>(&SerialProposalExecutor, &run_config)
                            .map_err(telltale_search::SearchRunError::InvalidConfig)?;
                        run_with_executor(machine, &SerialProposalExecutor, run_config)
                            .map(|(report, replay)| (report, Some(replay)))
                    } else {
                        run_with_executor_report_only(machine, &SerialProposalExecutor, run_config)
                            .map(|report| (report, None))
                    }
                }
            }
        }
    }
}

fn classify_transition(
    prior: Option<&PathwaySearchEpoch>,
    current: &PathwaySearchEpoch,
) -> PathwaySearchTransitionClass {
    match prior {
        None => PathwaySearchTransitionClass::InitialSnapshot,
        Some(previous) if previous == current => {
            PathwaySearchTransitionClass::SameEpochSameSnapshot
        }
        Some(previous) if previous.route_epoch == current.route_epoch => {
            PathwaySearchTransitionClass::SameEpochNewSnapshot
        }
        Some(_) => PathwaySearchTransitionClass::NewRouteEpoch,
    }
}
