//! Telltale-backed search domain, config, and replay diagnostics for Pathway.
//!
//! Pathway keeps route-shape derivation, admission, and backend-token
//! semantics locally. This module owns only the search substrate boundary:
//! frozen snapshot identity, search configuration, and replay-ready run
//! records for one objective-scoped v13 query.

mod domain;
mod runner;

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{Blake3Digest, LinkEndpoint, NodeId, RouteEpoch, RoutingObjective};
use serde::{Deserialize, Serialize};
use telltale_search::machine::ParentRecord;
use telltale_search::{
    EpsilonMilli, SearchCachingProfile, SearchEffortProfile, SearchExecutionPolicy,
    SearchExecutionReport, SearchFairnessAssumption, SearchQuery, SearchReplayArtifact,
    SearchReseedingPolicy, SearchSchedulerProfile,
};

use domain::{freeze_snapshot_for_search, snapshot_id_for_configuration, PathwaySearchDomain};
pub(crate) use runner::PathwaySearchSnapshotState;

/// Search-visible metadata for one traversable topology edge.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PathwaySearchEdgeMeta {
    /// Canonical source node.
    pub from_node_id: NodeId,
    /// Canonical destination node.
    pub to_node_id: NodeId,
    /// Transport endpoint chosen for this hop.
    pub endpoint: LinkEndpoint,
}

/// Stable digest of one frozen topology snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PathwaySearchSnapshotId(pub Blake3Digest);

/// Search epoch for one frozen Pathway topology snapshot.
///
/// This separates Pathway's route epoch from the stronger search-visible
/// snapshot identity. A route epoch can stay constant while the topology
/// snapshot changes, and the search machine still reconfigures fail-closed.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PathwaySearchEpoch {
    /// Shared route epoch from the topology observation.
    pub route_epoch: RouteEpoch,
    /// Strong snapshot identity for the exact frozen graph.
    pub snapshot_id: PathwaySearchSnapshotId,
}

/// Pathway-owned heuristic mode layered on top of the generic search machine.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PathwaySearchHeuristicMode {
    /// Exact Dijkstra-equivalent behavior.
    Zero,
    /// Reverse-hop lower bound multiplied by the minimum observed edge cost.
    HopLowerBound,
}

/// Replay-capture policy for retained Pathway search diagnostics.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PathwayReplayCapture {
    /// Retain the full replay artifact for diagnostics and reporting.
    Enabled,
    /// Skip replay-artifact retention and keep only the report surface.
    Disabled,
}

/// Fail-closed Pathway search-config validation error.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PathwaySearchConfigError {
    /// Pathway does not expose this scheduler profile.
    UnsupportedSchedulerProfile(SearchSchedulerProfile),
    /// The requested profile requires native threads on this target.
    RequiresNativeThreads(SearchSchedulerProfile),
    /// Batch width must be non-zero.
    ZeroBatchWidth,
    /// Exact Pathway profiles require batch width one.
    RequiresBatchWidthOne(SearchSchedulerProfile),
    /// Pathway does not expose cached execution modes.
    UnsupportedCachingProfile(SearchCachingProfile),
    /// Pathway currently requires exact run-to-completion execution.
    UnsupportedEffortProfile(SearchEffortProfile),
    /// Search epsilon must be non-zero.
    ZeroEpsilon,
    /// Objective-query budget must be non-zero.
    ZeroPerObjectiveQueryBudget,
    /// The scheduler profile requires one fairness assumption.
    MissingFairnessAssumption {
        /// Profile being validated.
        profile: SearchSchedulerProfile,
        /// Missing assumption.
        assumption: SearchFairnessAssumption,
    },
}

/// Pathway-owned planner search configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathwaySearchConfig {
    execution_policy: SearchExecutionPolicy,
    fairness_assumptions: BTreeSet<SearchFairnessAssumption>,
    epsilon: EpsilonMilli,
    per_objective_query_budget: usize,
    heuristic_mode: PathwaySearchHeuristicMode,
    reseeding_policy: SearchReseedingPolicy,
    capture_replay_artifact: bool,
}

impl PathwaySearchConfig {
    /// Construct one validated Pathway search config.
    pub fn try_new(
        execution_policy: SearchExecutionPolicy,
        fairness_assumptions: BTreeSet<SearchFairnessAssumption>,
        epsilon: EpsilonMilli,
        per_objective_query_budget: usize,
        heuristic_mode: PathwaySearchHeuristicMode,
        reseeding_policy: SearchReseedingPolicy,
    ) -> Result<Self, PathwaySearchConfigError> {
        Self::validate_execution_policy(execution_policy)?;
        if epsilon.0 == 0 {
            return Err(PathwaySearchConfigError::ZeroEpsilon);
        }
        if per_objective_query_budget == 0 {
            return Err(PathwaySearchConfigError::ZeroPerObjectiveQueryBudget);
        }

        let scheduler_profile = execution_policy.scheduler_profile;
        let required = BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]);
        for assumption in required {
            if !fairness_assumptions.contains(&assumption) {
                return Err(PathwaySearchConfigError::MissingFairnessAssumption {
                    profile: scheduler_profile,
                    assumption,
                });
            }
        }

        Ok(Self {
            execution_policy,
            fairness_assumptions,
            epsilon,
            per_objective_query_budget,
            heuristic_mode,
            reseeding_policy,
            capture_replay_artifact: true,
        })
    }

    fn validate_execution_policy(
        execution_policy: SearchExecutionPolicy,
    ) -> Result<(), PathwaySearchConfigError> {
        Self::validate_scheduler_profile(execution_policy.scheduler_profile)?;
        if execution_policy.batch_width == 0 {
            return Err(PathwaySearchConfigError::ZeroBatchWidth);
        }
        if execution_policy.batch_width != 1 {
            return Err(PathwaySearchConfigError::RequiresBatchWidthOne(
                execution_policy.scheduler_profile,
            ));
        }
        if execution_policy.caching_profile != SearchCachingProfile::EphemeralPerStep {
            return Err(PathwaySearchConfigError::UnsupportedCachingProfile(
                execution_policy.caching_profile,
            ));
        }
        if execution_policy.effort_profile != SearchEffortProfile::RunToCompletion {
            return Err(PathwaySearchConfigError::UnsupportedEffortProfile(
                execution_policy.effort_profile,
            ));
        }
        Ok(())
    }

    fn validate_scheduler_profile(
        scheduler_profile: SearchSchedulerProfile,
    ) -> Result<(), PathwaySearchConfigError> {
        match scheduler_profile {
            SearchSchedulerProfile::CanonicalSerial => Ok(()),
            SearchSchedulerProfile::ThreadedExactSingleLane => {
                if cfg!(target_arch = "wasm32") {
                    Err(PathwaySearchConfigError::RequiresNativeThreads(
                        scheduler_profile,
                    ))
                } else {
                    Ok(())
                }
            }
            unsupported => Err(PathwaySearchConfigError::UnsupportedSchedulerProfile(
                unsupported,
            )),
        }
    }

    #[must_use]
    pub fn canonical_serial() -> Self {
        Self::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::CanonicalSerial, 1),
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            super::super::PATHWAY_CANDIDATE_COUNT_MAX,
            PathwaySearchHeuristicMode::Zero,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        )
        .expect("canonical serial config is valid")
    }

    #[must_use]
    pub fn threaded_exact_single_lane() -> Self {
        Self::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::ThreadedExactSingleLane, 1),
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            super::super::PATHWAY_CANDIDATE_COUNT_MAX,
            PathwaySearchHeuristicMode::Zero,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        )
        .expect("threaded exact config is valid")
    }

    #[must_use]
    pub fn execution_policy(&self) -> SearchExecutionPolicy {
        self.execution_policy
    }

    #[must_use]
    pub fn scheduler_profile(&self) -> SearchSchedulerProfile {
        self.execution_policy.scheduler_profile
    }

    #[must_use]
    pub fn batch_width(&self) -> u64 {
        self.execution_policy.batch_width
    }

    #[must_use]
    pub fn caching_profile(&self) -> SearchCachingProfile {
        self.execution_policy.caching_profile
    }

    #[must_use]
    pub fn effort_profile(&self) -> SearchEffortProfile {
        self.execution_policy.effort_profile
    }

    #[must_use]
    pub fn fairness_assumptions(&self) -> &BTreeSet<SearchFairnessAssumption> {
        &self.fairness_assumptions
    }

    #[must_use]
    pub fn epsilon(&self) -> EpsilonMilli {
        self.epsilon
    }

    #[must_use]
    pub fn per_objective_query_budget(&self) -> usize {
        self.per_objective_query_budget
    }

    #[must_use]
    pub fn heuristic_mode(&self) -> PathwaySearchHeuristicMode {
        self.heuristic_mode
    }

    #[must_use]
    pub fn reseeding_policy(&self) -> SearchReseedingPolicy {
        self.reseeding_policy
    }

    #[must_use]
    pub fn capture_replay_artifact(&self) -> bool {
        self.capture_replay_artifact
    }

    #[must_use]
    pub(super) fn run_config(&self) -> telltale_search::SearchRunConfig {
        telltale_search::SearchRunConfig::new(
            self.execution_policy,
            self.fairness_assumptions.clone(),
        )
    }

    #[must_use]
    pub fn with_heuristic_mode(mut self, heuristic_mode: PathwaySearchHeuristicMode) -> Self {
        self.heuristic_mode = heuristic_mode;
        self
    }

    #[must_use]
    pub fn with_epsilon(mut self, epsilon: EpsilonMilli) -> Self {
        assert!(epsilon.0 != 0, "Pathway search epsilon must be non-zero");
        self.epsilon = epsilon;
        self
    }

    #[must_use]
    pub fn with_per_objective_query_budget(mut self, budget: usize) -> Self {
        assert!(budget != 0, "Pathway search budget must be non-zero");
        self.per_objective_query_budget = budget;
        self
    }

    #[must_use]
    pub fn with_reseeding_policy(mut self, reseeding_policy: SearchReseedingPolicy) -> Self {
        self.reseeding_policy = reseeding_policy;
        self
    }

    #[must_use]
    pub fn with_replay_capture(mut self, replay_capture: PathwayReplayCapture) -> Self {
        self.capture_replay_artifact = matches!(replay_capture, PathwayReplayCapture::Enabled);
        self
    }

    #[must_use]
    pub fn disable_replay_capture(self) -> Self {
        self.with_replay_capture(PathwayReplayCapture::Disabled)
    }
}

impl Default for PathwaySearchConfig {
    fn default() -> Self {
        Self::canonical_serial()
    }
}

/// Topology-transition classification for one search reconfiguration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PathwaySearchTransitionClass {
    /// First snapshot observed by this engine instance.
    InitialSnapshot,
    /// Route epoch and snapshot are unchanged.
    SameEpochSameSnapshot,
    /// Route epoch stayed constant but the frozen snapshot changed.
    SameEpochNewSnapshot,
    /// The shared route epoch changed.
    NewRouteEpoch,
}

/// Pathway-owned summary of one search-machine reconfiguration step.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PathwaySearchReconfiguration {
    /// Prior search epoch.
    pub from: PathwaySearchEpoch,
    /// Next search epoch.
    pub to: PathwaySearchEpoch,
    /// Explicit reseeding policy committed for the new epoch.
    pub reseeding_policy: SearchReseedingPolicy,
    /// Classified transition relation.
    pub transition_class: PathwaySearchTransitionClass,
}

/// One completed v13 search execution for one objective-scoped query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathwaySearchRun {
    /// Classified topology transition observed before this run.
    pub topology_transition: PathwaySearchTransitionClass,
    /// Pathway-owned path witness for the selected result when one exists.
    pub selected_node_path: Option<Vec<NodeId>>,
    /// Deterministic candidate node paths reconstructed for accepted nodes.
    pub candidate_node_paths: Vec<Vec<NodeId>>,
    /// Pathway-owned reconfiguration summary, when one was applied.
    pub reconfiguration: Option<PathwaySearchReconfiguration>,
    /// Final execution report.
    pub report: SearchExecutionReport<NodeId, PathwaySearchEpoch, u32>,
    /// Replay artifact for canonical reconstruction when capture is enabled.
    pub replay:
        Option<SearchReplayArtifact<NodeId, PathwaySearchEpoch, PathwaySearchSnapshotId, u32>>,
}

/// One objective-scoped search record persisted by Pathway for diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathwayPlannerSearchRecord {
    /// Objective that was translated into one v13-native search query.
    pub objective: RoutingObjective,
    /// Resolved query, when the objective admitted at least one destination.
    pub query: Option<SearchQuery<NodeId>>,
    /// Completed objective-scoped search execution, when a query was resolved.
    pub run: Option<PathwaySearchRun>,
}

impl PathwayPlannerSearchRecord {
    /// Reconstruct deterministic candidate node paths discovered by the final
    /// v13 run state for this objective.
    #[must_use]
    pub fn candidate_node_paths(&self) -> Vec<Vec<NodeId>> {
        let Some(query) = self.query.as_ref() else {
            return Vec::new();
        };
        let Some(run) = self.run.as_ref() else {
            return Vec::new();
        };
        let _query = query;
        run.candidate_node_paths.clone()
    }
}

pub(super) fn reconstruct_candidate_node_paths(
    query: &SearchQuery<NodeId>,
    parent_map: &[(NodeId, NodeId)],
) -> Vec<Vec<NodeId>> {
    let start = query.start();
    let parent_of = parent_map
        .iter()
        .map(|(child, parent)| (*child, *parent))
        .collect::<BTreeMap<_, _>>();
    query
        .accepted_nodes()
        .iter()
        .filter_map(|node_id| reconstruct_node_path(start, node_id, &parent_of))
        .collect()
}

pub(super) fn reconstruct_candidate_node_paths_from_parent_records(
    query: &SearchQuery<NodeId>,
    parent_map: &BTreeMap<NodeId, ParentRecord<NodeId, PathwaySearchEdgeMeta, u32>>,
) -> Vec<Vec<NodeId>> {
    let start = query.start();
    query
        .accepted_nodes()
        .iter()
        .filter_map(|node_id| reconstruct_node_path_from_parent_records(start, node_id, parent_map))
        .collect()
}

fn reconstruct_node_path(
    start: &NodeId,
    target: &NodeId,
    parent_of: &BTreeMap<NodeId, NodeId>,
) -> Option<Vec<NodeId>> {
    let mut node_path = vec![*target];
    let mut cursor = *target;
    let mut remaining_steps = parent_of.len().saturating_add(1);
    while &cursor != start {
        if remaining_steps == 0 {
            return None;
        }
        remaining_steps -= 1;
        cursor = *parent_of.get(&cursor)?;
        node_path.push(cursor);
    }
    node_path.reverse();
    Some(node_path)
}

fn reconstruct_node_path_from_parent_records(
    start: &NodeId,
    target: &NodeId,
    parent_of: &BTreeMap<NodeId, ParentRecord<NodeId, PathwaySearchEdgeMeta, u32>>,
) -> Option<Vec<NodeId>> {
    let mut node_path = vec![*target];
    let mut cursor = *target;
    let mut remaining_steps = parent_of.len().saturating_add(1);
    while &cursor != start {
        if remaining_steps == 0 {
            return None;
        }
        remaining_steps -= 1;
        cursor = parent_of.get(&cursor)?.from;
        node_path.push(cursor);
    }
    node_path.reverse();
    Some(node_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_rejects_unsupported_profiles() {
        let config = PathwaySearchConfig::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::BatchedParallelExact, 2),
            BTreeSet::from([
                SearchFairnessAssumption::DeterministicSchedulerConfluence,
                SearchFairnessAssumption::EventualLiveBatchService,
                SearchFairnessAssumption::NoStarvationWithinLegalWindow,
            ]),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        );
        assert_eq!(
            config,
            Err(PathwaySearchConfigError::UnsupportedSchedulerProfile(
                SearchSchedulerProfile::BatchedParallelExact,
            )),
        );
    }

    #[test]
    fn config_rejects_missing_fairness() {
        let config = PathwaySearchConfig::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::CanonicalSerial, 1),
            BTreeSet::new(),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        );
        assert_eq!(
            config,
            Err(PathwaySearchConfigError::MissingFairnessAssumption {
                profile: SearchSchedulerProfile::CanonicalSerial,
                assumption: SearchFairnessAssumption::DeterministicSchedulerConfluence,
            }),
        );
    }

    #[test]
    fn threaded_exact_support_matches_target_capability() {
        let config = PathwaySearchConfig::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::ThreadedExactSingleLane, 1),
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        );

        if cfg!(target_arch = "wasm32") {
            assert_eq!(
                config,
                Err(PathwaySearchConfigError::RequiresNativeThreads(
                    SearchSchedulerProfile::ThreadedExactSingleLane,
                )),
            );
        } else {
            assert!(config.is_ok());
        }
    }

    #[test]
    fn config_rejects_budgeted_execution() {
        let config = PathwaySearchConfig::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::CanonicalSerial, 1)
                .with_effort_profile(SearchEffortProfile::SchedulerStepBudget(4)),
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        );
        assert_eq!(
            config,
            Err(PathwaySearchConfigError::UnsupportedEffortProfile(
                SearchEffortProfile::SchedulerStepBudget(4),
            )),
        );
    }
}
