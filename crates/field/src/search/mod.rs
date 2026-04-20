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

/// Replay-capture policy for retained field search diagnostics.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum FieldReplayCapture {
    /// Retain the full replay artifact for diagnostics and reporting.
    Enabled,
    /// Skip replay-artifact retention and keep only the report surface.
    Disabled,
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
    capture_replay_artifact: bool,
    service_publication_neighbor_limit: usize,
    service_freshness_weight: u16,
    service_narrowing_bias: u16,
    node_bootstrap_support_floor: u16,
    node_bootstrap_top_mass_floor: u16,
    node_bootstrap_entropy_ceiling: u16,
    node_discovery_enabled: bool,
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
            capture_replay_artifact: true,
            service_publication_neighbor_limit: 3,
            service_freshness_weight: 100,
            service_narrowing_bias: 100,
            node_bootstrap_support_floor: 220,
            node_bootstrap_top_mass_floor: 260,
            node_bootstrap_entropy_ceiling: 950,
            node_discovery_enabled: false,
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
    pub fn capture_replay_artifact(&self) -> bool {
        self.capture_replay_artifact
    }

    #[must_use]
    pub fn service_publication_neighbor_limit(&self) -> usize {
        self.service_publication_neighbor_limit
    }

    #[must_use]
    pub fn service_freshness_weight(&self) -> u16 {
        self.service_freshness_weight
    }

    #[must_use]
    pub fn service_narrowing_bias(&self) -> u16 {
        self.service_narrowing_bias
    }

    #[must_use]
    pub fn node_bootstrap_support_floor(&self) -> u16 {
        self.node_bootstrap_support_floor
    }

    #[must_use]
    pub fn node_bootstrap_top_mass_floor(&self) -> u16 {
        self.node_bootstrap_top_mass_floor
    }

    #[must_use]
    pub fn node_bootstrap_entropy_ceiling(&self) -> u16 {
        self.node_bootstrap_entropy_ceiling
    }

    #[must_use]
    pub fn node_discovery_enabled(&self) -> bool {
        self.node_discovery_enabled
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
    pub fn with_replay_capture(mut self, replay_capture: FieldReplayCapture) -> Self {
        self.capture_replay_artifact = matches!(replay_capture, FieldReplayCapture::Enabled);
        self
    }

    #[must_use]
    pub fn disable_replay_capture(self) -> Self {
        self.with_replay_capture(FieldReplayCapture::Disabled)
    }

    #[must_use]
    pub fn with_service_publication_neighbor_limit(mut self, limit: usize) -> Self {
        assert!(
            limit != 0,
            "field service publication limit must be non-zero"
        );
        self.service_publication_neighbor_limit = limit;
        self
    }

    #[must_use]
    pub fn with_service_freshness_weight(mut self, weight: u16) -> Self {
        assert!(
            weight != 0,
            "field service freshness weight must be non-zero"
        );
        self.service_freshness_weight = weight;
        self
    }

    #[must_use]
    pub fn with_service_narrowing_bias(mut self, bias: u16) -> Self {
        assert!(bias != 0, "field service narrowing bias must be non-zero");
        self.service_narrowing_bias = bias;
        self
    }

    #[must_use]
    pub fn with_node_bootstrap_support_floor(mut self, floor: u16) -> Self {
        assert!(
            floor != 0,
            "field node bootstrap support floor must be non-zero"
        );
        self.node_bootstrap_support_floor = floor;
        self
    }

    #[must_use]
    pub fn with_node_bootstrap_top_mass_floor(mut self, floor: u16) -> Self {
        assert!(
            floor != 0,
            "field node bootstrap top corridor mass floor must be non-zero"
        );
        self.node_bootstrap_top_mass_floor = floor;
        self
    }

    #[must_use]
    pub fn with_node_bootstrap_entropy_ceiling(mut self, ceiling: u16) -> Self {
        assert!(
            ceiling != 0,
            "field node bootstrap entropy ceiling must be non-zero"
        );
        self.node_bootstrap_entropy_ceiling = ceiling;
        self
    }

    #[must_use]
    pub fn enable_node_discovery(mut self) -> Self {
        self.node_discovery_enabled = true;
        self
    }

    #[must_use]
    pub fn disable_node_discovery(mut self) -> Self {
        self.node_discovery_enabled = false;
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

/// Typed planner-visible reason why a field search record did not yield a
/// publishable continuation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FieldSearchPlanningFailure {
    /// The objective did not admit any search query under the current topology
    /// and field state.
    NoAdmittedQuery,
    /// The objective admitted a query, but the search machine did not produce a
    /// selected private result.
    NoSelectedResult,
    /// The selected private result existed, but it did not map to a valid
    /// first continuation from the local field node.
    NoPublishableContinuation,
}

/// Explicit field-owned publication boundary between private search output and
/// router-visible candidate production.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FieldSelectedContinuation {
    /// Objective-scoped query that drove the selection.
    pub query: SearchQuery<NodeId>,
    /// Full selected private witness retained for replay-oriented diagnostics.
    pub selected_private_witness: Vec<NodeId>,
    /// First continuation chosen for route publication.
    pub chosen_neighbor: NodeId,
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
    /// Replay artifact for canonical reconstruction when capture is enabled.
    pub replay: Option<SearchReplayArtifact<NodeId, FieldSearchEpoch, FieldSearchSnapshotId, u32>>,
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
    /// Explicit selected continuation boundary, when search produced a
    /// publishable result.
    pub selected_continuation: Option<FieldSelectedContinuation>,
    /// Typed failure reason when no publishable continuation was derived.
    pub planning_failure: Option<FieldSearchPlanningFailure>,
}
