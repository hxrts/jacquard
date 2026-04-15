//! Search runner: drives telltale-search for one field planning request.

use std::collections::{BTreeMap, BTreeSet};

use cfg_if::cfg_if;
use jacquard_core::{Configuration, NodeId, Observation, RoutingObjective};
use telltale_search::{
    commit_epoch_reconfiguration, run_with_executor, validate_run_config,
    EpochReconfigurationRequest, SearchMachine, SearchQuery, SearchSchedulerProfile,
    SerialProposalExecutor,
};

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::num::NonZeroU64;
        use telltale_search::NativeParallelExecutor;
    }
}

use super::{
    domain::{freeze_snapshot_for_search, FieldSearchDomain},
    FieldPlannerSearchRecord, FieldSearchConfig, FieldSearchEdgeMeta, FieldSearchEpoch,
    FieldSearchPlanningFailure, FieldSearchReconfiguration, FieldSearchRun,
    FieldSearchTransitionClass, FieldSelectedContinuation,
};
use crate::{
    state::{DestinationKey, NeighborContinuation},
    FieldEngine,
};

const FIELD_DIRECT_EDGE_SUPPORT_FLOOR: u16 = 180;

type FieldSearchExecutionReport =
    telltale_search::SearchExecutionReport<NodeId, FieldSearchEpoch, u32>;
type FieldSearchMachine = SearchMachine<FieldSearchDomain>;
type FieldSearchReplayArtifact = telltale_search::SearchReplayArtifact<
    NodeId,
    FieldSearchEpoch,
    super::FieldSearchSnapshotId,
    u32,
>;
type FieldSearchRunResult = Result<
    (FieldSearchExecutionReport, FieldSearchReplayArtifact),
    telltale_search::SearchRunError<&'static str>,
>;
type FieldSearchSuccessors = BTreeMap<NodeId, Vec<(NodeId, FieldSearchEdgeMeta, u32)>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldSearchSnapshotState {
    pub epoch: FieldSearchEpoch,
    pub topology: Observation<Configuration>,
}

impl FieldSearchSnapshotState {
    #[must_use]
    pub(crate) fn from_topology_and_config(
        topology: &Observation<Configuration>,
        accepted_node_ids: &[NodeId],
        successors: BTreeMap<NodeId, Vec<(NodeId, FieldSearchEdgeMeta, u32)>>,
        search_config: &FieldSearchConfig,
    ) -> Self {
        let (epoch, _) = freeze_snapshot_for_search(
            topology,
            successors,
            accepted_node_ids,
            search_config.heuristic_mode(),
        );
        Self {
            epoch,
            topology: topology.clone(),
        }
    }
}

impl<Transport, Effects> FieldEngine<Transport, Effects> {
    pub(crate) fn search_record_for_objective(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> FieldPlannerSearchRecord {
        let effective_config = self.effective_search_config();
        let query = self.resolve_query_for_objective(objective, topology, &effective_config);
        let prior_state = self.search_snapshot_state.borrow().clone();
        let run = query.as_ref().map(|query| {
            self.run_search_for_query(
                objective,
                topology,
                query,
                prior_state.as_ref(),
                &effective_config,
            )
        });

        if let Some(query) = query.as_ref() {
            let successors = self.freeze_successors_for_search(objective, topology);
            *self.search_snapshot_state.borrow_mut() =
                Some(FieldSearchSnapshotState::from_topology_and_config(
                    topology,
                    query.accepted_nodes(),
                    successors,
                    &effective_config,
                ));
        }

        let (selected_continuation, planning_failure) =
            selected_continuation_from_record(query.as_ref(), run.as_ref());

        let record = FieldPlannerSearchRecord {
            objective: objective.clone(),
            effective_config,
            query,
            run,
            selected_continuation,
            planning_failure,
        };
        *self.last_search_record.borrow_mut() = Some(record.clone());
        record
    }

    fn resolve_query_for_objective(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        search_config: &FieldSearchConfig,
    ) -> Option<SearchQuery<NodeId>> {
        if !self.destination_supports_objective(topology, objective) {
            return None;
        }
        let destination_key = DestinationKey::from(&objective.destination);
        self.state.destinations.get(&destination_key)?;
        match objective.destination {
            jacquard_core::DestinationId::Node(goal_node_id) => {
                if goal_node_id == self.local_node_id {
                    return None;
                }
                Some(SearchQuery::single_goal(self.local_node_id, goal_node_id))
            }
            jacquard_core::DestinationId::Gateway(_) | jacquard_core::DestinationId::Service(_) => {
                let mut accepted_nodes = self.local_search_neighbor_ids(objective, topology);
                accepted_nodes.truncate(search_config.per_objective_query_budget());
                SearchQuery::try_candidate_set(self.local_node_id, accepted_nodes, None).ok()
            }
        }
    }

    fn run_search_for_query(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        query: &SearchQuery<NodeId>,
        prior_state: Option<&FieldSearchSnapshotState>,
        search_config: &FieldSearchConfig,
    ) -> FieldSearchRun {
        let (current_state, current_successors) =
            self.current_search_state_for_query(objective, topology, query, search_config);
        let (topology_transition, reconfiguration) =
            self.reconfiguration_for_states(prior_state, &current_state, search_config);

        let domain = self.domain_for_query(
            objective,
            topology,
            query.accepted_nodes(),
            prior_state,
            current_successors,
            search_config,
        );
        let mut machine = self.machine_for_query(
            domain,
            &current_state,
            query,
            reconfiguration.as_ref(),
            search_config,
        );
        let (report, replay) = self
            .execute_search_machine(&mut machine, search_config)
            .expect("field search config is validated and domain snapshots are present");

        FieldSearchRun {
            topology_transition,
            selected_node_path: report.observation.selected_result_witness.clone(),
            reconfiguration,
            report,
            replay,
        }
    }

    fn current_search_state_for_query(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        query: &SearchQuery<NodeId>,
        search_config: &FieldSearchConfig,
    ) -> (FieldSearchSnapshotState, FieldSearchSuccessors) {
        let current_successors = self.freeze_successors_for_search(objective, topology);
        let current_state = FieldSearchSnapshotState::from_topology_and_config(
            topology,
            query.accepted_nodes(),
            current_successors.clone(),
            search_config,
        );
        (current_state, current_successors)
    }

    fn reconfiguration_for_states(
        &self,
        prior_state: Option<&FieldSearchSnapshotState>,
        current_state: &FieldSearchSnapshotState,
        search_config: &FieldSearchConfig,
    ) -> (
        FieldSearchTransitionClass,
        Option<FieldSearchReconfiguration>,
    ) {
        let topology_transition =
            classify_transition(prior_state.map(|state| &state.epoch), &current_state.epoch);
        let reconfiguration = prior_state
            .filter(|state| state.epoch != current_state.epoch)
            .map(|state| FieldSearchReconfiguration {
                from: state.epoch.clone(),
                to: current_state.epoch.clone(),
                reseeding_policy: search_config.reseeding_policy(),
                transition_class: topology_transition,
            });
        (topology_transition, reconfiguration)
    }

    fn machine_for_query(
        &self,
        domain: FieldSearchDomain,
        current_state: &FieldSearchSnapshotState,
        query: &SearchQuery<NodeId>,
        reconfiguration: Option<&FieldSearchReconfiguration>,
        search_config: &FieldSearchConfig,
    ) -> FieldSearchMachine {
        let mut machine = SearchMachine::new_with_query(
            domain,
            reconfiguration
                .as_ref()
                .map_or_else(|| current_state.epoch.clone(), |step| step.from.clone()),
            query.clone(),
            search_config.epsilon(),
        );
        if let Some(step) = reconfiguration {
            commit_epoch_reconfiguration(
                &mut machine,
                EpochReconfigurationRequest {
                    next_epoch: step.to.clone(),
                    reseeding_policy: step.reseeding_policy,
                },
            );
        }
        machine
    }

    fn domain_for_query(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        accepted_node_ids: &[NodeId],
        prior_state: Option<&FieldSearchSnapshotState>,
        current_successors: BTreeMap<NodeId, Vec<(NodeId, FieldSearchEdgeMeta, u32)>>,
        search_config: &FieldSearchConfig,
    ) -> FieldSearchDomain {
        let current_state = FieldSearchSnapshotState::from_topology_and_config(
            topology,
            accepted_node_ids,
            current_successors.clone(),
            search_config,
        );
        let mut snapshots = BTreeMap::new();
        if let Some(state) = prior_state.filter(|state| state.epoch != current_state.epoch) {
            let prior_successors = self.freeze_successors_for_search(objective, &state.topology);
            let (prior_epoch, prior_snapshot) = freeze_snapshot_for_search(
                &state.topology,
                prior_successors,
                accepted_node_ids,
                search_config.heuristic_mode(),
            );
            snapshots.insert(prior_epoch, prior_snapshot);
        }
        let (current_epoch, current_snapshot) = freeze_snapshot_for_search(
            topology,
            current_successors,
            accepted_node_ids,
            search_config.heuristic_mode(),
        );
        snapshots.insert(current_epoch, current_snapshot);
        FieldSearchDomain::new(snapshots)
    }

    fn freeze_successors_for_search(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> FieldSearchSuccessors {
        let frontier_neighbor_ids = self
            .local_search_neighbor_ids(objective, topology)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let node_ids = topology
            .value
            .nodes
            .keys()
            .copied()
            .chain([self.local_node_id])
            .chain(frontier_neighbor_ids.iter().copied())
            .collect::<BTreeSet<_>>();
        let mut successors = BTreeMap::new();
        for from_node_id in node_ids {
            let mut edges = adjacent_node_ids(
                &from_node_id,
                &topology.value,
                self.local_node_id,
                &frontier_neighbor_ids,
            )
            .into_iter()
            .filter_map(|to_node_id| {
                let edge_cost =
                    self.edge_cost_for_search(objective, topology, &from_node_id, &to_node_id)?;
                let support_hint =
                    self.support_hint_for_edge(objective, topology, &from_node_id, &to_node_id);
                Some((
                    to_node_id,
                    FieldSearchEdgeMeta {
                        from_node_id,
                        to_node_id,
                        support_hint,
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

    fn edge_cost_for_search(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        from_node_id: &NodeId,
        to_node_id: &NodeId,
    ) -> Option<u32> {
        if *from_node_id == self.local_node_id {
            return self
                .frontier_continuation_for_search(objective, to_node_id)
                .or_else(|| self.weak_forward_continuation_for_search(objective, to_node_id))
                .map(|continuation| continuation_edge_cost(&continuation))
                .or_else(|| self.inferred_direct_edge_cost(topology, to_node_id));
        }
        if !nodes_are_adjacent(&topology.value, from_node_id, to_node_id) {
            return None;
        }
        Some(1000)
    }

    fn support_hint_for_edge(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        from_node_id: &NodeId,
        to_node_id: &NodeId,
    ) -> u16 {
        if *from_node_id == self.local_node_id {
            return self
                .frontier_continuation_for_search(objective, to_node_id)
                .or_else(|| self.weak_forward_continuation_for_search(objective, to_node_id))
                .map(|entry| entry.net_value.value())
                .or_else(|| self.inferred_direct_edge_support_hint(topology, objective, to_node_id))
                .unwrap_or(0);
        }
        0
    }

    fn local_search_neighbor_ids(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> Vec<NodeId> {
        let destination_key = DestinationKey::from(&objective.destination);
        let service_bias = matches!(
            objective.destination,
            jacquard_core::DestinationId::Service(_)
        );
        let mut neighbors = self
            .state
            .destinations
            .get(&destination_key)
            .map(|destination_state| {
                destination_state
                    .frontier
                    .as_slice()
                    .iter()
                    .map(|entry| entry.neighbor_id)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        neighbors.extend(
            adjacent_node_ids(
                &self.local_node_id,
                &topology.value,
                self.local_node_id,
                &BTreeSet::new(),
            )
            .into_iter()
            .filter(|neighbor| {
                self.inferred_direct_edge_support_hint(topology, objective, neighbor)
                    .is_some_and(|support| support >= FIELD_DIRECT_EDGE_SUPPORT_FLOOR)
            }),
        );
        neighbors.extend(
            self.state
                .destinations
                .get(&destination_key)
                .into_iter()
                .flat_map(|destination_state| destination_state.pending_forward_evidence.iter())
                .filter(|evidence| {
                    evidence.summary.retention_support.value()
                        >= if service_bias { 150 } else { 280 }
                        && evidence.summary.delivery_support.value()
                            >= if service_bias { 70 } else { 140 }
                        && evidence.summary.uncertainty_penalty.value()
                            <= if service_bias { 860 } else { 700 }
                })
                .map(|evidence| evidence.from_neighbor),
        );
        neighbors.into_iter().collect()
    }

    fn frontier_continuation_for_search(
        &self,
        objective: &RoutingObjective,
        to_node_id: &NodeId,
    ) -> Option<NeighborContinuation> {
        self.state
            .destinations
            .get(&DestinationKey::from(&objective.destination))
            .and_then(|destination_state| {
                destination_state
                    .frontier
                    .as_slice()
                    .iter()
                    .find(|entry| entry.neighbor_id == *to_node_id)
                    .cloned()
            })
    }

    fn inferred_direct_edge_cost(
        &self,
        topology: &Observation<Configuration>,
        to_node_id: &NodeId,
    ) -> Option<u32> {
        let link = topology
            .value
            .links
            .get(&(self.local_node_id, *to_node_id))
            .or_else(|| topology.value.links.get(&(*to_node_id, self.local_node_id)))?;
        let support = u32::from(self.inferred_direct_edge_support_hint_for_link(link));
        if support < u32::from(FIELD_DIRECT_EDGE_SUPPORT_FLOOR) {
            return None;
        }
        let support_penalty = 1000_u32.saturating_sub(support);
        let loss_penalty = u32::from(link.state.loss_permille.0);
        let latency_penalty_ms = link.profile.latency_floor_ms.0 / 2;
        Some(
            50_u32
                .saturating_add(support_penalty)
                .saturating_add(loss_penalty)
                .saturating_add(latency_penalty_ms),
        )
    }

    fn inferred_direct_edge_support_hint(
        &self,
        topology: &Observation<Configuration>,
        objective: &RoutingObjective,
        to_node_id: &NodeId,
    ) -> Option<u16> {
        let link = topology
            .value
            .links
            .get(&(self.local_node_id, *to_node_id))
            .or_else(|| topology.value.links.get(&(*to_node_id, self.local_node_id)))?;
        let destination_support = self
            .state
            .destinations
            .get(&DestinationKey::from(&objective.destination))
            .map(|destination_state| {
                average_support_hint(
                    destination_state.posterior.top_corridor_mass.value(),
                    destination_state.corridor_belief.delivery_support.value(),
                )
            })
            .unwrap_or(320);
        Some(
            self.inferred_direct_edge_support_hint_for_link(link)
                .saturating_add(destination_support / 4)
                .min(1000),
        )
    }

    fn inferred_direct_edge_support_hint_for_link(&self, link: &jacquard_core::Link) -> u16 {
        let confidence = link
            .state
            .delivery_confidence_permille
            .value()
            .map(|ratio| ratio.0)
            .unwrap_or(1000_u16.saturating_sub(link.state.loss_permille.0 / 2));
        let symmetry = link
            .state
            .symmetry_permille
            .value()
            .map(|ratio| ratio.0)
            .unwrap_or(800);
        average_support_hint(
            confidence,
            1000_u16
                .saturating_sub(link.state.loss_permille.0 / 2)
                .min(symmetry),
        )
    }

    fn weak_forward_continuation_for_search(
        &self,
        objective: &RoutingObjective,
        to_node_id: &NodeId,
    ) -> Option<NeighborContinuation> {
        let service_bias = matches!(
            objective.destination,
            jacquard_core::DestinationId::Service(_)
        );
        let destination_state = self
            .state
            .destinations
            .get(&DestinationKey::from(&objective.destination))?;
        let evidence = destination_state
            .pending_forward_evidence
            .iter()
            .filter(|evidence| evidence.from_neighbor == *to_node_id)
            .filter(|evidence| {
                evidence.summary.retention_support.value() >= if service_bias { 130 } else { 220 }
                    && evidence.summary.delivery_support.value()
                        >= if service_bias { 60 } else { 120 }
                    && evidence.summary.uncertainty_penalty.value()
                        <= if service_bias { 900 } else { 780 }
            })
            .max_by_key(|evidence| {
                (
                    evidence.summary.retention_support.value(),
                    evidence.summary.delivery_support.value(),
                    evidence.observed_at_tick,
                )
            })?;
        Some(NeighborContinuation {
            neighbor_id: evidence.from_neighbor,
            net_value: crate::state::SupportBucket::new(
                evidence
                    .summary
                    .delivery_support
                    .value()
                    .saturating_add(evidence.summary.retention_support.value() / 3)
                    .min(1000),
            ),
            downstream_support: evidence.summary.delivery_support,
            expected_hop_band: crate::state::HopBand::new(
                evidence.summary.hop_band.min_hops.saturating_add(1),
                evidence.summary.hop_band.max_hops.saturating_add(1),
            ),
            freshness: evidence.observed_at_tick,
        })
    }

    fn execute_search_machine(
        &self,
        machine: &mut SearchMachine<FieldSearchDomain>,
        config: &FieldSearchConfig,
    ) -> FieldSearchRunResult {
        let run_config = config.run_config();
        cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                validate_run_config::<FieldSearchDomain, _>(&SerialProposalExecutor, &run_config)
                    .map_err(telltale_search::SearchRunError::InvalidConfig)?;
                run_with_executor(machine, &SerialProposalExecutor, run_config)
            } else {
                if config.scheduler_profile() == SearchSchedulerProfile::ThreadedExactSingleLane {
                    let executor = NativeParallelExecutor::new(
                        NonZeroU64::new(config.batch_width()).expect("field batch width is non-zero"),
                    )
                    .expect("field threaded exact config requires native parallel executor support");
                    validate_run_config::<FieldSearchDomain, _>(&executor, &run_config)
                        .map_err(telltale_search::SearchRunError::InvalidConfig)?;
                    run_with_executor(machine, &executor, run_config)
                } else {
                    validate_run_config::<FieldSearchDomain, _>(&SerialProposalExecutor, &run_config)
                        .map_err(telltale_search::SearchRunError::InvalidConfig)?;
                    run_with_executor(machine, &SerialProposalExecutor, run_config)
                }
            }
        }
    }
}

fn adjacent_node_ids(
    node_id: &NodeId,
    configuration: &Configuration,
    local_node_id: NodeId,
    frontier_neighbor_ids: &BTreeSet<NodeId>,
) -> Vec<NodeId> {
    configuration
        .links
        .keys()
        .filter_map(|(from_node_id, to_node_id)| {
            if from_node_id == node_id {
                Some(*to_node_id)
            } else if to_node_id == node_id {
                Some(*from_node_id)
            } else {
                None
            }
        })
        .chain(
            (*node_id == local_node_id)
                .then_some(frontier_neighbor_ids.iter().copied())
                .into_iter()
                .flatten(),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn nodes_are_adjacent(configuration: &Configuration, left: &NodeId, right: &NodeId) -> bool {
    configuration.links.contains_key(&(*left, *right))
        || configuration.links.contains_key(&(*right, *left))
}

fn continuation_edge_cost(continuation: &NeighborContinuation) -> u32 {
    let support_penalty = u32::from(1000_u16.saturating_sub(continuation.net_value.value()));
    let downstream_penalty =
        u32::from(1000_u16.saturating_sub(continuation.downstream_support.value())) / 2;
    let hop_penalty = u32::from(continuation.expected_hop_band.max_hops.max(1)) * 100;
    1_u32
        .saturating_add(support_penalty)
        .saturating_add(downstream_penalty)
        .saturating_add(hop_penalty)
}

fn average_support_hint(left: u16, right: u16) -> u16 {
    let sum = u32::from(left).saturating_add(u32::from(right));
    u16::try_from(sum / 2).expect("support hint average fits u16")
}

fn classify_transition(
    prior: Option<&FieldSearchEpoch>,
    current: &FieldSearchEpoch,
) -> FieldSearchTransitionClass {
    match prior {
        None => FieldSearchTransitionClass::InitialSnapshot,
        Some(previous) if previous == current => FieldSearchTransitionClass::SameEpochSameSnapshot,
        Some(previous) if previous.route_epoch == current.route_epoch => {
            FieldSearchTransitionClass::SameEpochNewSnapshot
        }
        Some(_) => FieldSearchTransitionClass::NewRouteEpoch,
    }
}

fn selected_continuation_from_record(
    query: Option<&SearchQuery<NodeId>>,
    run: Option<&FieldSearchRun>,
) -> (
    Option<FieldSelectedContinuation>,
    Option<FieldSearchPlanningFailure>,
) {
    let Some(query) = query else {
        return (None, Some(FieldSearchPlanningFailure::NoAdmittedQuery));
    };
    let Some(run) = run else {
        return (None, Some(FieldSearchPlanningFailure::NoSelectedResult));
    };
    let Some(selected_private_witness) = run.selected_node_path.as_ref() else {
        return (None, Some(FieldSearchPlanningFailure::NoSelectedResult));
    };
    let Some(chosen_neighbor) = selected_private_witness.get(1).copied() else {
        return (
            None,
            Some(FieldSearchPlanningFailure::NoPublishableContinuation),
        );
    };
    (
        Some(FieldSelectedContinuation {
            query: query.clone(),
            selected_private_witness: selected_private_witness.clone(),
            chosen_neighbor,
        }),
        None,
    )
}
