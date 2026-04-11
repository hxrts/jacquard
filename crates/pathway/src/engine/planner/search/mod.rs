//! Telltale-backed search domain, config, and replay diagnostics for Pathway.
//!
//! Pathway keeps route-shape derivation, admission, and backend-token
//! semantics locally. This module owns only the search substrate boundary:
//! frozen snapshot identity, search configuration, and replay-ready run
//! records for one objective's goal set.

mod domain;
mod runner;

use std::collections::BTreeSet;

use jacquard_core::{Blake3Digest, LinkEndpoint, NodeId, RouteEpoch, RoutingObjective};
use serde::{Deserialize, Serialize};
use telltale_search::{
    EpsilonMilli, SearchExecutionReport, SearchFairnessAssumption, SearchReplayArtifact,
    SearchSchedulerProfile,
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
    /// Search epsilon must be non-zero.
    ZeroEpsilon,
    /// Goal-set budget must be non-zero.
    ZeroPerObjectiveSearchBudget,
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
    scheduler_profile: SearchSchedulerProfile,
    batch_width: u64,
    fairness_assumptions: BTreeSet<SearchFairnessAssumption>,
    epsilon: EpsilonMilli,
    per_objective_search_budget: usize,
    heuristic_mode: PathwaySearchHeuristicMode,
}

impl PathwaySearchConfig {
    /// Construct one validated Pathway search config.
    pub fn try_new(
        scheduler_profile: SearchSchedulerProfile,
        batch_width: u64,
        fairness_assumptions: BTreeSet<SearchFairnessAssumption>,
        epsilon: EpsilonMilli,
        per_objective_search_budget: usize,
        heuristic_mode: PathwaySearchHeuristicMode,
    ) -> Result<Self, PathwaySearchConfigError> {
        Self::validate_scheduler_profile(scheduler_profile)?;
        if batch_width == 0 {
            return Err(PathwaySearchConfigError::ZeroBatchWidth);
        }
        if batch_width != 1 {
            return Err(PathwaySearchConfigError::RequiresBatchWidthOne(
                scheduler_profile,
            ));
        }
        if epsilon.0 == 0 {
            return Err(PathwaySearchConfigError::ZeroEpsilon);
        }
        if per_objective_search_budget == 0 {
            return Err(PathwaySearchConfigError::ZeroPerObjectiveSearchBudget);
        }
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
            scheduler_profile,
            batch_width,
            fairness_assumptions,
            epsilon,
            per_objective_search_budget,
            heuristic_mode,
        })
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
            SearchSchedulerProfile::CanonicalSerial,
            1,
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            super::super::PATHWAY_CANDIDATE_COUNT_MAX,
            PathwaySearchHeuristicMode::Zero,
        )
        .expect("canonical serial config is valid")
    }

    #[must_use]
    pub fn threaded_exact_single_lane() -> Self {
        Self::try_new(
            SearchSchedulerProfile::ThreadedExactSingleLane,
            1,
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            super::super::PATHWAY_CANDIDATE_COUNT_MAX,
            PathwaySearchHeuristicMode::Zero,
        )
        .expect("threaded exact config is valid")
    }

    #[must_use]
    pub fn scheduler_profile(&self) -> SearchSchedulerProfile {
        self.scheduler_profile
    }

    #[must_use]
    pub fn batch_width(&self) -> u64 {
        self.batch_width
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
    pub fn per_objective_search_budget(&self) -> usize {
        self.per_objective_search_budget
    }

    #[must_use]
    pub fn heuristic_mode(&self) -> PathwaySearchHeuristicMode {
        self.heuristic_mode
    }

    #[must_use]
    pub(super) fn run_config(&self) -> telltale_search::SearchRunConfig {
        telltale_search::SearchRunConfig {
            scheduler_profile: self.scheduler_profile,
            batch_width: self.batch_width,
            fairness_assumptions: self.fairness_assumptions.clone(),
        }
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
    pub fn with_per_objective_search_budget(mut self, budget: usize) -> Self {
        assert!(budget != 0, "Pathway search budget must be non-zero");
        self.per_objective_search_budget = budget;
        self
    }
}

impl Default for PathwaySearchConfig {
    fn default() -> Self {
        Self::canonical_serial()
    }
}

/// Deterministic search-goal mapping for one Pathway routing objective.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathwaySearchGoalResolution {
    /// One exact destination node.
    ExactDestination(NodeId),
    /// A finite acceptable set of destination nodes.
    AcceptableGoalSet(Vec<NodeId>),
}

impl PathwaySearchGoalResolution {
    #[must_use]
    pub fn goal_nodes(&self) -> &[NodeId] {
        match self {
            Self::ExactDestination(node_id) => std::slice::from_ref(node_id),
            Self::AcceptableGoalSet(node_ids) => node_ids.as_slice(),
        }
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
    /// Classified transition relation.
    pub transition_class: PathwaySearchTransitionClass,
}

/// One completed search-machine run for one concrete destination node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathwaySearchRun {
    /// Concrete goal node used for this run.
    pub goal_node_id: NodeId,
    /// Classified topology transition observed before this run.
    pub topology_transition: PathwaySearchTransitionClass,
    /// Incumbent node path when the goal is reachable.
    pub node_path: Option<Vec<NodeId>>,
    /// Pathway-owned reconfiguration summary, when one was applied.
    pub reconfiguration: Option<PathwaySearchReconfiguration>,
    /// Final execution report.
    pub report: SearchExecutionReport<NodeId, PathwaySearchEpoch, u32>,
    /// Replay artifact for canonical reconstruction.
    pub replay: SearchReplayArtifact<NodeId, PathwaySearchEpoch, PathwaySearchSnapshotId, u32>,
}

/// One objective-scoped search record persisted by Pathway for diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathwayPlannerSearchRecord {
    /// Objective that was translated into one or more exact-node searches.
    pub objective: RoutingObjective,
    /// Deterministic goal-resolution strategy used for the objective.
    pub goal_resolution: PathwaySearchGoalResolution,
    /// Completed concrete goal-node runs in deterministic order.
    pub runs: Vec<PathwaySearchRun>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_rejects_unsupported_profiles() {
        let config = PathwaySearchConfig::try_new(
            SearchSchedulerProfile::BatchedParallelExact,
            2,
            BTreeSet::from([
                SearchFairnessAssumption::DeterministicSchedulerConfluence,
                SearchFairnessAssumption::EventualLiveBatchService,
                SearchFairnessAssumption::NoStarvationWithinLegalWindow,
            ]),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
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
            SearchSchedulerProfile::CanonicalSerial,
            1,
            BTreeSet::new(),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
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
            SearchSchedulerProfile::ThreadedExactSingleLane,
            1,
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            4,
            PathwaySearchHeuristicMode::Zero,
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
}
