//! Telltale-backed search domain, config, and replay diagnostics for field.
//!
//! Field keeps corridor-envelope publication, admission, and backend-token
//! semantics locally. This module owns the search substrate boundary: frozen
//! field snapshots, execution policy, and replay-ready search records.

mod domain;
mod runner;

use std::collections::BTreeSet;

use jacquard_core::{Blake3Digest, NodeId, RouteEpoch, RoutingObjective};
use serde::{Deserialize, Serialize};
use telltale_search::{
    EpsilonMilli, SearchCachingProfile, SearchEffortProfile, SearchExecutionPolicy,
    SearchExecutionReport, SearchFairnessAssumption, SearchQuery, SearchReplayArtifact,
    SearchReseedingPolicy, SearchSchedulerProfile,
};

pub(crate) use runner::FieldSearchSnapshotState;

/// Search-visible metadata for one traversable field edge.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldSearchEdgeMeta {
    /// Canonical source node.
    pub from_node_id: NodeId,
    /// Canonical destination node.
    pub to_node_id: NodeId,
    /// Local support hint used when freezing this edge.
    pub support_hint: u16,
}

/// Stable digest of one frozen field search snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FieldSearchSnapshotId(pub Blake3Digest);

/// Search epoch for one frozen field search snapshot.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct FieldSearchEpoch {
    /// Shared route epoch from the topology observation.
    pub route_epoch: RouteEpoch,
    /// Strong identity for the exact frozen field search snapshot.
    pub snapshot_id: FieldSearchSnapshotId,
}

/// Field-owned heuristic mode layered on top of the generic search machine.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum FieldSearchHeuristicMode {
    /// Exact Dijkstra-equivalent behavior.
    Zero,
    /// Reverse-hop lower bound multiplied by the minimum observed edge cost.
    HopLowerBound,
}

/// Fail-closed field search-config validation error.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldSearchConfigError {
    /// Field does not expose this scheduler profile.
    UnsupportedSchedulerProfile(SearchSchedulerProfile),
    /// The requested profile requires native threads on this target.
    RequiresNativeThreads(SearchSchedulerProfile),
    /// Batch width must be non-zero.
    ZeroBatchWidth,
    /// Exact field profiles require batch width one.
    RequiresBatchWidthOne(SearchSchedulerProfile),
    /// Field does not expose cached execution modes.
    UnsupportedCachingProfile(SearchCachingProfile),
    /// Field currently requires exact run-to-completion execution.
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

/// Field-owned planner search configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSearchConfig {
    execution_policy: SearchExecutionPolicy,
    fairness_assumptions: BTreeSet<SearchFairnessAssumption>,
    epsilon: EpsilonMilli,
    per_objective_query_budget: usize,
    heuristic_mode: FieldSearchHeuristicMode,
    reseeding_policy: SearchReseedingPolicy,
}

impl FieldSearchConfig {
    /// Construct one validated field search config.
    pub fn try_new(
        execution_policy: SearchExecutionPolicy,
        fairness_assumptions: BTreeSet<SearchFairnessAssumption>,
        epsilon: EpsilonMilli,
        per_objective_query_budget: usize,
        heuristic_mode: FieldSearchHeuristicMode,
        reseeding_policy: SearchReseedingPolicy,
    ) -> Result<Self, FieldSearchConfigError> {
        Self::validate_execution_policy(execution_policy)?;
        if epsilon.0 == 0 {
            return Err(FieldSearchConfigError::ZeroEpsilon);
        }
        if per_objective_query_budget == 0 {
            return Err(FieldSearchConfigError::ZeroPerObjectiveQueryBudget);
        }

        let scheduler_profile = execution_policy.scheduler_profile;
        let required = BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]);
        for assumption in required {
            if !fairness_assumptions.contains(&assumption) {
                return Err(FieldSearchConfigError::MissingFairnessAssumption {
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
        })
    }

    fn validate_execution_policy(
        execution_policy: SearchExecutionPolicy,
    ) -> Result<(), FieldSearchConfigError> {
        Self::validate_scheduler_profile(execution_policy.scheduler_profile)?;
        if execution_policy.batch_width == 0 {
            return Err(FieldSearchConfigError::ZeroBatchWidth);
        }
        if execution_policy.batch_width != 1 {
            return Err(FieldSearchConfigError::RequiresBatchWidthOne(
                execution_policy.scheduler_profile,
            ));
        }
        if execution_policy.caching_profile != SearchCachingProfile::EphemeralPerStep {
            return Err(FieldSearchConfigError::UnsupportedCachingProfile(
                execution_policy.caching_profile,
            ));
        }
        if execution_policy.effort_profile != SearchEffortProfile::RunToCompletion {
            return Err(FieldSearchConfigError::UnsupportedEffortProfile(
                execution_policy.effort_profile,
            ));
        }
        Ok(())
    }

    fn validate_scheduler_profile(
        scheduler_profile: SearchSchedulerProfile,
    ) -> Result<(), FieldSearchConfigError> {
        match scheduler_profile {
            SearchSchedulerProfile::CanonicalSerial => Ok(()),
            SearchSchedulerProfile::ThreadedExactSingleLane => {
                if cfg!(target_arch = "wasm32") {
                    Err(FieldSearchConfigError::RequiresNativeThreads(
                        scheduler_profile,
                    ))
                } else {
                    Ok(())
                }
            }
            unsupported => Err(FieldSearchConfigError::UnsupportedSchedulerProfile(
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
            8,
            FieldSearchHeuristicMode::HopLowerBound,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        )
        .expect("canonical serial field config is valid")
    }

    #[must_use]
    pub fn threaded_exact_single_lane() -> Self {
        Self::try_new(
            SearchExecutionPolicy::new(SearchSchedulerProfile::ThreadedExactSingleLane, 1),
            BTreeSet::from([SearchFairnessAssumption::DeterministicSchedulerConfluence]),
            EpsilonMilli::one(),
            16,
            FieldSearchHeuristicMode::HopLowerBound,
            SearchReseedingPolicy::PreserveOpenAndIncons,
        )
        .expect("threaded field config is valid")
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
    pub fn epsilon(&self) -> EpsilonMilli {
        self.epsilon
    }

    #[must_use]
    pub fn per_objective_query_budget(&self) -> usize {
        self.per_objective_query_budget
    }

    #[must_use]
    pub fn heuristic_mode(&self) -> FieldSearchHeuristicMode {
        self.heuristic_mode
    }

    #[must_use]
    pub fn reseeding_policy(&self) -> SearchReseedingPolicy {
        self.reseeding_policy
    }

    #[must_use]
    pub fn with_per_objective_query_budget(mut self, budget: usize) -> Self {
        assert!(budget != 0, "field search budget must be non-zero");
        self.per_objective_query_budget = budget;
        self
    }

    pub fn with_scheduler_profile(
        mut self,
        scheduler_profile: SearchSchedulerProfile,
    ) -> Result<Self, FieldSearchConfigError> {
        let mut execution_policy = self.execution_policy;
        execution_policy.scheduler_profile = scheduler_profile;
        Self::validate_execution_policy(execution_policy)?;
        self.execution_policy = execution_policy;
        Ok(self)
    }

    #[must_use]
    pub fn with_heuristic_mode(mut self, heuristic_mode: FieldSearchHeuristicMode) -> Self {
        self.heuristic_mode = heuristic_mode;
        self
    }

    #[must_use]
    pub fn with_reseeding_policy(mut self, reseeding_policy: SearchReseedingPolicy) -> Self {
        self.reseeding_policy = reseeding_policy;
        self
    }

    #[must_use]
    pub(super) fn run_config(&self) -> telltale_search::SearchRunConfig {
        telltale_search::SearchRunConfig::new(
            self.execution_policy,
            self.fairness_assumptions.clone(),
        )
    }
}

impl Default for FieldSearchConfig {
    fn default() -> Self {
        Self::canonical_serial()
    }
}

/// Topology-transition classification for one field search reconfiguration.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum FieldSearchTransitionClass {
    /// First snapshot observed by this engine instance.
    InitialSnapshot,
    /// Route epoch and snapshot are unchanged.
    SameEpochSameSnapshot,
    /// Route epoch stayed constant but the frozen snapshot changed.
    SameEpochNewSnapshot,
    /// The shared route epoch changed.
    NewRouteEpoch,
}

/// Field-owned summary of one search-machine reconfiguration step.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldSearchReconfiguration {
    /// Prior search epoch.
    pub from: FieldSearchEpoch,
    /// Next search epoch.
    pub to: FieldSearchEpoch,
    /// Explicit reseeding policy committed for the new epoch.
    pub reseeding_policy: SearchReseedingPolicy,
    /// Classified transition relation.
    pub transition_class: FieldSearchTransitionClass,
}

/// One completed v13 search execution for one field planner query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldSearchRun {
    /// Classified topology transition observed before this run.
    pub topology_transition: FieldSearchTransitionClass,
    /// Field-owned selected path witness, when one exists.
    pub selected_node_path: Option<Vec<NodeId>>,
    /// Field-owned reconfiguration summary, when one was applied.
    pub reconfiguration: Option<FieldSearchReconfiguration>,
    /// Final execution report.
    pub report: SearchExecutionReport<NodeId, FieldSearchEpoch, u32>,
    /// Replay artifact for canonical reconstruction.
    pub replay: SearchReplayArtifact<NodeId, FieldSearchEpoch, FieldSearchSnapshotId, u32>,
}

/// One planner search record persisted by field for diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldPlannerSearchRecord {
    /// Objective resolved into one v13-native search query.
    pub objective: RoutingObjective,
    /// Effective field-owned execution config used for this planner request.
    pub effective_config: FieldSearchConfig,
    /// Resolved query, when the objective admitted at least one destination.
    pub query: Option<SearchQuery<NodeId>>,
    /// Completed query-scoped search execution, when a query was resolved.
    pub run: Option<FieldSearchRun>,
}
