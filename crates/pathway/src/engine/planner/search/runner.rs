// Search runner: drives telltale-search for a single pathway planning request.
//
// `search_record_for_objective` resolves the routing objective to one or more
// goal nodes, runs an independent search toward each, and returns a
// `PathwayPlannerSearchRecord` that the candidate-ranking layer consumes.
// Epoch reconfiguration is committed when the observed topology changes between
// calls, so the underlying `SearchMachine` always operates on a consistent
// frozen snapshot matched to the current `PathwaySearchEpoch`.

use std::collections::BTreeMap;

use cfg_if::cfg_if;
use jacquard_core::{Configuration, NodeId, Observation, RoutingObjective};
use telltale_search::{
    commit_epoch_reconfiguration, run_with_executor, EpochReconfigurationRequest, SearchMachine,
    SearchSchedulerProfile, SerialProposalExecutor,
};

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::num::NonZeroU64;
        use telltale_search::NativeParallelExecutor;
    }
}

use super::{
    freeze_snapshot_for_search, snapshot_id_for_configuration, PathwayPlannerSearchRecord,
    PathwaySearchConfig, PathwaySearchDomain, PathwaySearchEdgeMeta, PathwaySearchEpoch,
    PathwaySearchGoalResolution, PathwaySearchReconfiguration, PathwaySearchRun,
    PathwaySearchTransitionClass,
};
use crate::{
    engine::PathwayEngine,
    topology::{adjacent_node_ids, estimate_hop_link, objective_matches_node},
    PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess, PATHWAY_ENGINE_ID,
};

type PathwaySearchExecutionReport =
    telltale_search::SearchExecutionReport<NodeId, PathwaySearchEpoch, u32>;
type PathwaySearchReplayArtifact = telltale_search::SearchReplayArtifact<
    NodeId,
    PathwaySearchEpoch,
    super::PathwaySearchSnapshotId,
    u32,
>;
type PathwaySearchRunResult = Result<
    (PathwaySearchExecutionReport, PathwaySearchReplayArtifact),
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
        let goal_resolution = self.resolve_goal_resolution(objective, topology);
        let prior_state = self.search_snapshot_state.borrow().clone();
        let runs = goal_resolution
            .goal_nodes()
            .iter()
            .copied()
            .map(|goal_node_id| {
                self.run_search_for_goal(objective, topology, goal_node_id, prior_state.as_ref())
            })
            .collect::<Vec<_>>();

        *self.search_snapshot_state.borrow_mut() =
            Some(PathwaySearchSnapshotState::from_topology(topology));

        let record = PathwayPlannerSearchRecord {
            objective: objective.clone(),
            goal_resolution,
            runs,
        };
        *self.last_search_record.borrow_mut() = Some(record.clone());
        record
    }

    fn resolve_goal_resolution(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> PathwaySearchGoalResolution {
        let mut goals = match &objective.destination {
            jacquard_core::DestinationId::Node(target_node_id) => topology
                .value
                .nodes
                .get(target_node_id)
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
        goals.truncate(self.search_config.per_objective_search_budget());

        match (&objective.destination, goals.as_slice()) {
            (jacquard_core::DestinationId::Node(_), [goal_node_id]) => {
                PathwaySearchGoalResolution::ExactDestination(*goal_node_id)
            }
            _ => PathwaySearchGoalResolution::AcceptableGoalSet(goals),
        }
    }

    fn run_search_for_goal(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        goal_node_id: NodeId,
        prior_state: Option<&PathwaySearchSnapshotState>,
    ) -> PathwaySearchRun {
        let current_state = PathwaySearchSnapshotState::from_topology(topology);
        let topology_transition =
            classify_transition(prior_state.map(|state| &state.epoch), &current_state.epoch);
        let reconfiguration = prior_state
            .filter(|state| state.epoch != current_state.epoch)
            .map(|state| PathwaySearchReconfiguration {
                from: state.epoch.clone(),
                to: current_state.epoch.clone(),
                transition_class: topology_transition,
            });

        let domain = self.domain_for_goal(objective, topology, goal_node_id, prior_state);
        let mut machine = SearchMachine::new(
            domain,
            reconfiguration
                .as_ref()
                .map_or_else(|| current_state.epoch.clone(), |step| step.from.clone()),
            self.local_node_id,
            goal_node_id,
            self.search_config.epsilon(),
        );
        if let Some(step) = reconfiguration.as_ref() {
            commit_epoch_reconfiguration(
                &mut machine,
                EpochReconfigurationRequest {
                    next_epoch: step.to.clone(),
                },
            );
        }
        let (report, replay) = self
            .execute_search_machine(&mut machine, &self.search_config)
            .expect("pathway search config is validated and domain snapshots are present");

        PathwaySearchRun {
            goal_node_id,
            topology_transition,
            node_path: report.observation.incumbent_path.clone(),
            reconfiguration,
            report,
            replay,
        }
    }

    fn domain_for_goal(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        goal_node_id: NodeId,
        prior_state: Option<&PathwaySearchSnapshotState>,
    ) -> PathwaySearchDomain {
        let mut snapshots = BTreeMap::new();
        if let Some(state) = prior_state.filter(|state| {
            state.epoch != PathwaySearchSnapshotState::from_topology(topology).epoch
        }) {
            let prior_successors = self.freeze_successors_for_search(objective, &state.topology);
            let (prior_epoch, prior_snapshot) = freeze_snapshot_for_search(
                &state.topology,
                prior_successors,
                goal_node_id,
                self.search_config.heuristic_mode(),
            );
            snapshots.insert(prior_epoch, prior_snapshot);
        }

        let current_successors = self.freeze_successors_for_search(objective, topology);
        let (current_epoch, current_snapshot) = freeze_snapshot_for_search(
            topology,
            current_successors,
            goal_node_id,
            self.search_config.heuristic_mode(),
        );
        snapshots.insert(current_epoch, current_snapshot);
        PathwaySearchDomain::new(snapshots)
    }

    fn freeze_successors_for_search(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> BTreeMap<NodeId, Vec<(NodeId, PathwaySearchEdgeMeta, u32)>> {
        let mut successors = BTreeMap::new();
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
            successors.insert(from_node_id, edges);
        }
        successors
    }

    fn execute_search_machine(
        &self,
        machine: &mut SearchMachine<PathwaySearchDomain>,
        config: &PathwaySearchConfig,
    ) -> PathwaySearchRunResult {
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                match config.scheduler_profile() {
                    SearchSchedulerProfile::CanonicalSerial => {
                        run_with_executor(machine, &SerialProposalExecutor, config.run_config())
                    }
                    SearchSchedulerProfile::ThreadedExactSingleLane => {
                        panic!("threaded Pathway search is not available on wasm32")
                    }
                    unsupported => panic!("unsupported pathway search profile: {unsupported:?}"),
                }
            } else {
                match config.scheduler_profile() {
                    SearchSchedulerProfile::CanonicalSerial => {
                        run_with_executor(machine, &SerialProposalExecutor, config.run_config())
                    }
                    SearchSchedulerProfile::ThreadedExactSingleLane => {
                        let executor = NativeParallelExecutor::new(
                            NonZeroU64::new(config.batch_width()).expect("batch width is non-zero"),
                        )
                        .expect("threaded exact config requires native parallel executor support");
                        run_with_executor(machine, &executor, config.run_config())
                    }
                    unsupported => panic!("unsupported pathway search profile: {unsupported:?}"),
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
