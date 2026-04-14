//! Deterministic tuning experiment matrix for BATMAN and Pathway.
//!
//! long-file-exception: the experiment catalog, aggregation rules, and
//! artifact-writing logic are intentionally kept in one maintained module so
//! the tuning corpus remains auditable as one coherent surface.
// long-file-exception: the experiment catalog, scenario builders, reduction
// schema, and artifact-writing path are intentionally maintained together so
// the full tuning matrix remains auditable as one cohesive simulator surface.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use jacquard_babel::BABEL_ENGINE_ID;
use jacquard_batman_bellman::{DecayWindow, BATMAN_BELLMAN_ENGINE_ID};
use jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID;
use jacquard_core::{
    Belief, Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment,
    FactSourceClass, Node, NodeId, Observation, OriginAuthenticationClass, PriorityPoints,
    RatioPermille, RouteEpoch, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
    RouteServiceKind, RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters,
    SimulationSeed, Tick,
};
use jacquard_field::{
    FieldForwardSummaryObservation, FieldSearchConfig, FieldSearchHeuristicMode, FIELD_ENGINE_ID,
};
use jacquard_pathway::{PathwaySearchConfig, PathwaySearchHeuristicMode, PATHWAY_ENGINE_ID};
use jacquard_reference_client::topology;
use jacquard_traits::{RoutingScenario, RoutingSimulator};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    environment::{EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel},
    harness::{JacquardHostAdapter, JacquardSimulator, SimulationError},
    scenario::{BoundObjective, FieldBootstrapSummary, HostSpec, JacquardScenario},
    ReducedReplayView,
};

const NODE_A: NodeId = NodeId([1; 32]);
const NODE_B: NodeId = NodeId([2; 32]);
const NODE_C: NodeId = NodeId([3; 32]);
const NODE_D: NodeId = NodeId([4; 32]);
const NODE_E: NodeId = NodeId([5; 32]);

#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("simulation failed: {0}")]
    Simulation(#[from] SimulationError),
    #[error("io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegimeDescriptor {
    pub density: String,
    pub loss: String,
    pub interference: String,
    pub asymmetry: String,
    pub churn: String,
    pub node_pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentParameterSet {
    pub engine_family: String,
    pub config_id: String,
    pub comparison_engine_set: Option<String>,
    pub batman_bellman_stale_after_ticks: Option<u32>,
    pub batman_bellman_next_refresh_within_ticks: Option<u32>,
    pub batman_classic_stale_after_ticks: Option<u32>,
    pub batman_classic_next_refresh_within_ticks: Option<u32>,
    pub babel_stale_after_ticks: Option<u32>,
    pub babel_next_refresh_within_ticks: Option<u32>,
    pub pathway_query_budget: Option<usize>,
    pub pathway_heuristic_mode: Option<String>,
    pub field_query_budget: Option<usize>,
    pub field_heuristic_mode: Option<String>,
    pub field_service_publication_neighbor_limit: Option<usize>,
    pub field_service_freshness_weight: Option<u16>,
    pub field_service_narrowing_bias: Option<u16>,
}

impl ExperimentParameterSet {
    #[must_use]
    pub fn batman_bellman(stale_after_ticks: u32, next_refresh_within_ticks: u32) -> Self {
        Self {
            engine_family: "batman-bellman".to_string(),
            config_id: format!(
                "batman-bellman-{}-{}",
                stale_after_ticks, next_refresh_within_ticks
            ),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: Some(stale_after_ticks),
            batman_bellman_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
        }
    }

    #[must_use]
    pub fn pathway(
        per_objective_query_budget: usize,
        heuristic_mode: PathwaySearchHeuristicMode,
    ) -> Self {
        Self {
            engine_family: "pathway".to_string(),
            config_id: format!(
                "pathway-{}-{}",
                per_objective_query_budget,
                heuristic_mode_label(heuristic_mode)
            ),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            pathway_query_budget: Some(per_objective_query_budget),
            pathway_heuristic_mode: Some(heuristic_mode_label(heuristic_mode).to_string()),
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
        }
    }

    #[must_use]
    pub fn field(
        per_objective_query_budget: usize,
        heuristic_mode: FieldSearchHeuristicMode,
    ) -> Self {
        Self::field_tuned(per_objective_query_budget, heuristic_mode, 3, 100, 100)
    }

    #[must_use]
    pub fn field_tuned(
        per_objective_query_budget: usize,
        heuristic_mode: FieldSearchHeuristicMode,
        service_publication_neighbor_limit: usize,
        service_freshness_weight: u16,
        service_narrowing_bias: u16,
    ) -> Self {
        Self {
            engine_family: "field".to_string(),
            config_id: format!(
                "field-{}-{}-p{}-f{}-n{}",
                per_objective_query_budget,
                field_heuristic_mode_label(heuristic_mode),
                service_publication_neighbor_limit,
                service_freshness_weight,
                service_narrowing_bias,
            ),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            field_query_budget: Some(per_objective_query_budget),
            field_heuristic_mode: Some(field_heuristic_mode_label(heuristic_mode).to_string()),
            field_service_publication_neighbor_limit: Some(service_publication_neighbor_limit),
            field_service_freshness_weight: Some(service_freshness_weight),
            field_service_narrowing_bias: Some(service_narrowing_bias),
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
        }
    }

    #[must_use]
    pub fn comparison(
        stale_after_ticks: u32,
        next_refresh_within_ticks: u32,
        per_objective_query_budget: usize,
        heuristic_mode: PathwaySearchHeuristicMode,
    ) -> Self {
        Self {
            engine_family: "comparison".to_string(),
            config_id: format!(
                "comparison-b{}-{}-p{}-{}",
                stale_after_ticks,
                next_refresh_within_ticks,
                per_objective_query_budget,
                heuristic_mode_label(heuristic_mode)
            ),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: Some(stale_after_ticks),
            batman_bellman_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            batman_classic_stale_after_ticks: Some(stale_after_ticks),
            batman_classic_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            babel_stale_after_ticks: Some(stale_after_ticks),
            babel_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            pathway_query_budget: Some(per_objective_query_budget),
            pathway_heuristic_mode: Some(heuristic_mode_label(heuristic_mode).to_string()),
            field_query_budget: Some(per_objective_query_budget),
            field_heuristic_mode: Some(
                field_heuristic_mode_label(FieldSearchHeuristicMode::HopLowerBound).to_string(),
            ),
            field_service_publication_neighbor_limit: Some(3),
            field_service_freshness_weight: Some(100),
            field_service_narrowing_bias: Some(100),
        }
    }

    #[must_use]
    pub fn head_to_head(
        comparison_engine_set: &str,
        batman_bellman_decay_window: Option<(u32, u32)>,
        pathway_search: Option<(usize, PathwaySearchHeuristicMode)>,
        field_search: Option<(usize, FieldSearchHeuristicMode)>,
    ) -> Self {
        let config_suffix = match comparison_engine_set {
            "batman-bellman" => {
                let (stale_after_ticks, next_refresh_within_ticks) =
                    batman_bellman_decay_window.unwrap_or((1, 1));
                format!(
                    "batman-bellman-{}-{}",
                    stale_after_ticks, next_refresh_within_ticks
                )
            }
            "batman-classic" => {
                let (stale_after_ticks, next_refresh_within_ticks) =
                    batman_bellman_decay_window.unwrap_or((4, 2));
                format!(
                    "batman-classic-{}-{}",
                    stale_after_ticks, next_refresh_within_ticks
                )
            }
            "babel" => {
                let (stale_after_ticks, next_refresh_within_ticks) =
                    batman_bellman_decay_window.unwrap_or((4, 2));
                format!("babel-{}-{}", stale_after_ticks, next_refresh_within_ticks)
            }
            "pathway" => {
                let (budget, heuristic_mode) =
                    pathway_search.unwrap_or((2, PathwaySearchHeuristicMode::Zero));
                format!(
                    "pathway-{}-{}",
                    budget,
                    heuristic_mode_label(heuristic_mode)
                )
            }
            "field" => {
                let (budget, heuristic_mode) =
                    field_search.unwrap_or((4, FieldSearchHeuristicMode::HopLowerBound));
                format!(
                    "field-{}-{}",
                    budget,
                    field_heuristic_mode_label(heuristic_mode)
                )
            }
            "pathway-batman-bellman" => {
                let (stale_after_ticks, next_refresh_within_ticks) =
                    batman_bellman_decay_window.unwrap_or((1, 1));
                let (budget, heuristic_mode) =
                    pathway_search.unwrap_or((2, PathwaySearchHeuristicMode::Zero));
                format!(
                    "pathway-batman-b{}-{}-p{}-{}",
                    stale_after_ticks,
                    next_refresh_within_ticks,
                    budget,
                    heuristic_mode_label(heuristic_mode)
                )
            }
            other => other.to_string(),
        };
        let (batman_bellman_stale_after_ticks, batman_bellman_next_refresh_within_ticks) =
            batman_bellman_decay_window.map_or((None, None), |(stale, refresh)| {
                (Some(stale), Some(refresh))
            });
        let (pathway_query_budget, pathway_heuristic_mode) =
            pathway_search.map_or((None, None), |(budget, heuristic)| {
                (
                    Some(budget),
                    Some(heuristic_mode_label(heuristic).to_string()),
                )
            });
        let (field_query_budget, field_heuristic_mode) =
            field_search.map_or((None, None), |(budget, heuristic)| {
                (
                    Some(budget),
                    Some(field_heuristic_mode_label(heuristic).to_string()),
                )
            });
        let (
            field_service_publication_neighbor_limit,
            field_service_freshness_weight,
            field_service_narrowing_bias,
        ) = if field_search.is_some() {
            (Some(3), Some(100), Some(100))
        } else {
            (None, None, None)
        };
        Self {
            engine_family: "head-to-head".to_string(),
            config_id: format!("head-to-head-{}", config_suffix),
            comparison_engine_set: Some(comparison_engine_set.to_string()),
            batman_bellman_stale_after_ticks,
            batman_bellman_next_refresh_within_ticks,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            pathway_query_budget,
            pathway_heuristic_mode,
            field_query_budget,
            field_heuristic_mode,
            field_service_publication_neighbor_limit,
            field_service_freshness_weight,
            field_service_narrowing_bias,
        }
    }

    #[must_use]
    pub fn batman_bellman_decay_window(&self) -> Option<DecayWindow> {
        match (
            self.batman_bellman_stale_after_ticks,
            self.batman_bellman_next_refresh_within_ticks,
        ) {
            (Some(stale_after_ticks), Some(next_refresh_within_ticks)) => Some(DecayWindow::new(
                u64::from(stale_after_ticks),
                u64::from(next_refresh_within_ticks),
            )),
            _ => None,
        }
    }

    #[must_use]
    pub fn pathway_search_config(&self) -> Option<PathwaySearchConfig> {
        let budget = self.pathway_query_budget?;
        let heuristic_mode =
            heuristic_mode_from_str(self.pathway_heuristic_mode.as_deref().unwrap_or("zero"));
        Some(
            PathwaySearchConfig::default()
                .with_per_objective_query_budget(budget)
                .with_heuristic_mode(heuristic_mode),
        )
    }

    #[must_use]
    pub fn field_search_config(&self) -> Option<FieldSearchConfig> {
        let budget = self.field_query_budget?;
        let heuristic_mode = field_heuristic_mode_from_str(
            self.field_heuristic_mode
                .as_deref()
                .unwrap_or("hop-lower-bound"),
        );
        Some(
            FieldSearchConfig::default()
                .with_per_objective_query_budget(budget)
                .with_heuristic_mode(heuristic_mode)
                .with_service_publication_neighbor_limit(
                    self.field_service_publication_neighbor_limit.unwrap_or(3),
                )
                .with_service_freshness_weight(self.field_service_freshness_weight.unwrap_or(100))
                .with_service_narrowing_bias(self.field_service_narrowing_bias.unwrap_or(100)),
        )
    }

    #[must_use]
    pub fn batman_classic(stale_after_ticks: u32, next_refresh_within_ticks: u32) -> Self {
        Self {
            engine_family: "batman-classic".to_string(),
            config_id: format!(
                "batman-classic-{}-{}",
                stale_after_ticks, next_refresh_within_ticks
            ),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            batman_classic_stale_after_ticks: Some(stale_after_ticks),
            batman_classic_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
        }
    }

    #[must_use]
    pub fn batman_classic_decay_window(&self) -> Option<jacquard_batman_classic::DecayWindow> {
        match (
            self.batman_classic_stale_after_ticks,
            self.batman_classic_next_refresh_within_ticks,
        ) {
            (Some(stale), Some(refresh)) => Some(jacquard_batman_classic::DecayWindow::new(
                u64::from(stale),
                u64::from(refresh),
            )),
            _ => None,
        }
    }

    #[must_use]
    pub fn babel(stale_after_ticks: u32, next_refresh_within_ticks: u32) -> Self {
        Self {
            engine_family: "babel".to_string(),
            config_id: format!("babel-{}-{}", stale_after_ticks, next_refresh_within_ticks),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: Some(stale_after_ticks),
            babel_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
        }
    }

    #[must_use]
    pub fn babel_decay_window(&self) -> Option<jacquard_babel::DecayWindow> {
        match (
            self.babel_stale_after_ticks,
            self.babel_next_refresh_within_ticks,
        ) {
            (Some(stale), Some(refresh)) => Some(jacquard_babel::DecayWindow::new(
                u64::from(stale),
                u64::from(refresh),
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ExperimentRunSpec {
    pub run_id: String,
    pub suite_id: String,
    pub family_id: String,
    pub engine_family: String,
    pub seed: SimulationSeed,
    pub regime: RegimeDescriptor,
    pub parameters: ExperimentParameterSet,
    pub scenario: JacquardScenario,
    pub environment: ScriptedEnvironmentModel,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentRunSummary {
    pub run_id: String,
    pub suite_id: String,
    pub family_id: String,
    pub scenario_name: String,
    pub engine_family: String,
    pub config_id: String,
    pub comparison_engine_set: Option<String>,
    pub batman_bellman_stale_after_ticks: Option<u32>,
    pub batman_bellman_next_refresh_within_ticks: Option<u32>,
    pub batman_classic_stale_after_ticks: Option<u32>,
    pub batman_classic_next_refresh_within_ticks: Option<u32>,
    pub babel_stale_after_ticks: Option<u32>,
    pub babel_next_refresh_within_ticks: Option<u32>,
    pub pathway_query_budget: Option<usize>,
    pub pathway_heuristic_mode: Option<String>,
    pub field_query_budget: Option<usize>,
    pub field_heuristic_mode: Option<String>,
    pub field_service_publication_neighbor_limit: Option<usize>,
    pub field_service_freshness_weight: Option<u16>,
    pub field_service_narrowing_bias: Option<u16>,
    pub seed: u64,
    pub density: String,
    pub loss: String,
    pub interference: String,
    pub asymmetry: String,
    pub churn: String,
    pub node_pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
    pub objective_count: u32,
    pub activation_success_permille: u32,
    pub route_present_permille: u32,
    pub first_materialization_round_mean: Option<u32>,
    pub first_loss_round_mean: Option<u32>,
    pub recovery_round_mean: Option<u32>,
    pub route_churn_count: u32,
    pub engine_handoff_count: u32,
    pub route_observation_count: u32,
    pub batman_bellman_selected_rounds: u32,
    pub batman_classic_selected_rounds: u32,
    pub babel_selected_rounds: u32,
    pub pathway_selected_rounds: u32,
    pub field_selected_rounds: u32,
    pub field_selected_result_rounds: u32,
    pub field_search_reconfiguration_rounds: u32,
    pub field_bootstrap_active_rounds: u32,
    pub field_continuity_band: Option<String>,
    pub field_commitment_resolution: Option<String>,
    pub field_last_outcome: Option<String>,
    pub field_last_continuity_transition: Option<String>,
    pub field_last_promotion_decision: Option<String>,
    pub field_last_promotion_blocker: Option<String>,
    pub field_bootstrap_activation_permille: u32,
    pub field_bootstrap_hold_permille: u32,
    pub field_bootstrap_narrow_permille: u32,
    pub field_bootstrap_upgrade_permille: u32,
    pub field_bootstrap_withdraw_permille: u32,
    pub field_degraded_steady_entry_permille: u32,
    pub field_degraded_steady_recovery_permille: u32,
    pub field_degraded_to_bootstrap_permille: u32,
    pub field_degraded_steady_round_permille: u32,
    pub field_service_retention_carry_forward_permille: u32,
    pub field_asymmetric_shift_success_permille: u32,
    pub field_protocol_reconfiguration_count: u32,
    pub field_route_bound_reconfiguration_count: u32,
    pub field_continuation_shift_count: u32,
    pub field_corridor_narrow_count: u32,
    pub field_checkpoint_restore_count: u32,
    pub no_route_rounds: u32,
    pub dominant_engine: Option<String>,
    pub stability_min: Option<u32>,
    pub stability_first: Option<u32>,
    pub stability_last: Option<u32>,
    pub stability_median: Option<u32>,
    pub stability_max: Option<u32>,
    pub stability_total: u32,
    pub maintenance_failure_count: u32,
    pub failure_summary_count: u32,
    pub no_candidate_count: u32,
    pub inadmissible_candidate_count: u32,
    pub lost_reachability_count: u32,
    pub replacement_loop_count: u32,
    pub activation_failure_count: u32,
    pub persistent_degraded_count: u32,
    pub other_failure_count: u32,
    pub replace_topology_count: u32,
    pub medium_degradation_count: u32,
    pub asymmetric_degradation_count: u32,
    pub partition_count: u32,
    pub cascade_partition_count: u32,
    pub mobility_relink_count: u32,
    pub intrinsic_limit_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentAggregateSummary {
    pub suite_id: String,
    pub family_id: String,
    pub engine_family: String,
    pub config_id: String,
    pub comparison_engine_set: Option<String>,
    pub batman_bellman_stale_after_ticks: Option<u32>,
    pub batman_bellman_next_refresh_within_ticks: Option<u32>,
    pub batman_classic_stale_after_ticks: Option<u32>,
    pub batman_classic_next_refresh_within_ticks: Option<u32>,
    pub babel_stale_after_ticks: Option<u32>,
    pub babel_next_refresh_within_ticks: Option<u32>,
    pub pathway_query_budget: Option<usize>,
    pub pathway_heuristic_mode: Option<String>,
    pub field_query_budget: Option<usize>,
    pub field_heuristic_mode: Option<String>,
    pub field_service_publication_neighbor_limit: Option<usize>,
    pub field_service_freshness_weight: Option<u16>,
    pub field_service_narrowing_bias: Option<u16>,
    pub density: String,
    pub loss: String,
    pub interference: String,
    pub asymmetry: String,
    pub churn: String,
    pub node_pressure: String,
    pub objective_regime: String,
    pub stress_score: u32,
    pub run_count: u32,
    pub activation_success_permille_mean: u32,
    pub route_present_permille_mean: u32,
    pub first_materialization_round_mean: Option<u32>,
    pub first_loss_round_mean: Option<u32>,
    pub recovery_round_mean: Option<u32>,
    pub route_churn_count_mean: u32,
    pub engine_handoff_count_mean: u32,
    pub dominant_engine: Option<String>,
    pub field_selected_result_rounds_mean: u32,
    pub field_search_reconfiguration_rounds_mean: u32,
    pub field_bootstrap_active_rounds_mean: u32,
    pub field_continuity_band_mode: Option<String>,
    pub field_commitment_resolution_mode: Option<String>,
    pub field_last_outcome_mode: Option<String>,
    pub field_last_continuity_transition_mode: Option<String>,
    pub field_last_promotion_decision_mode: Option<String>,
    pub field_last_promotion_blocker_mode: Option<String>,
    pub field_bootstrap_activation_permille_mean: u32,
    pub field_bootstrap_hold_permille_mean: u32,
    pub field_bootstrap_narrow_permille_mean: u32,
    pub field_bootstrap_upgrade_permille_mean: u32,
    pub field_bootstrap_withdraw_permille_mean: u32,
    pub field_degraded_steady_entry_permille_mean: u32,
    pub field_degraded_steady_recovery_permille_mean: u32,
    pub field_degraded_to_bootstrap_permille_mean: u32,
    pub field_degraded_steady_round_permille_mean: u32,
    pub field_service_retention_carry_forward_permille_mean: u32,
    pub field_asymmetric_shift_success_permille_mean: u32,
    pub field_protocol_reconfiguration_count_mean: u32,
    pub field_route_bound_reconfiguration_count_mean: u32,
    pub field_continuation_shift_count_mean: u32,
    pub field_corridor_narrow_count_mean: u32,
    pub field_checkpoint_restore_count_mean: u32,
    pub stability_first_mean: Option<u32>,
    pub stability_last_mean: Option<u32>,
    pub stability_median_mean: Option<u32>,
    pub stability_total_mean: u32,
    pub maintenance_failure_count_mean: u32,
    pub failure_summary_count_mean: u32,
    pub no_candidate_count_mean: u32,
    pub inadmissible_candidate_count_mean: u32,
    pub lost_reachability_count_mean: u32,
    pub replacement_loop_count_mean: u32,
    pub persistent_degraded_count_mean: u32,
    pub acceptable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentBreakdownSummary {
    pub suite_id: String,
    pub engine_family: String,
    pub config_id: String,
    pub max_sustained_stress_score: u32,
    pub first_failed_family_id: Option<String>,
    pub first_failed_stress_score: Option<u32>,
    pub breakdown_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentManifest {
    pub suite_id: String,
    pub generated_at_unix_seconds: u64,
    pub run_count: u32,
    pub aggregate_count: u32,
    pub breakdown_count: u32,
}

#[derive(Clone, Debug)]
pub struct ExperimentSuite {
    suite_id: String,
    runs: Vec<ExperimentRunSpec>,
}

impl ExperimentSuite {
    #[must_use]
    pub fn suite_id(&self) -> &str {
        &self.suite_id
    }

    #[must_use]
    pub fn run_count(&self) -> usize {
        self.runs.len()
    }
}

#[derive(Clone, Debug)]
pub struct ExperimentArtifacts {
    pub output_dir: PathBuf,
    pub manifest: ExperimentManifest,
    pub runs: Vec<ExperimentRunSummary>,
    pub aggregates: Vec<ExperimentAggregateSummary>,
    pub breakdowns: Vec<ExperimentBreakdownSummary>,
}

#[must_use]
pub fn smoke_suite() -> ExperimentSuite {
    build_suite("smoke", &[41], true)
}

#[must_use]
pub fn local_suite() -> ExperimentSuite {
    build_suite("local", &[41, 43], false)
}

pub fn run_suite<A>(
    simulator: &mut JacquardSimulator<A>,
    suite: &ExperimentSuite,
    output_dir: &Path,
) -> Result<ExperimentArtifacts, ExperimentError>
where
    A: JacquardHostAdapter,
{
    fs::create_dir_all(output_dir)?;
    let mut runs = Vec::new();
    let run_path = output_dir.join("runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);

    for spec in &suite.runs {
        let (replay, _) = simulator.run_scenario(&spec.scenario, &spec.environment)?;
        let reduced = ReducedReplayView::from_replay(&replay);
        let summary = summarize_run(spec, &reduced);
        serde_json::to_writer(&mut writer, &summary)?;
        writer.write_all(b"\n")?;
        runs.push(summary);
    }
    writer.flush()?;

    let aggregates = aggregate_runs(&runs);
    let breakdowns = summarize_breakdowns(&aggregates);
    let manifest = ExperimentManifest {
        suite_id: suite.suite_id.clone(),
        generated_at_unix_seconds: 0,
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        breakdown_count: u32::try_from(breakdowns.len()).unwrap_or(u32::MAX),
    };

    serde_json::to_writer_pretty(File::create(output_dir.join("manifest.json"))?, &manifest)?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("aggregates.json"))?,
        &aggregates,
    )?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("breakdowns.json"))?,
        &breakdowns,
    )?;

    Ok(ExperimentArtifacts {
        output_dir: output_dir.to_path_buf(),
        manifest,
        runs,
        aggregates,
        breakdowns,
    })
}

fn build_suite(suite_id: &str, seeds: &[u64], smoke: bool) -> ExperimentSuite {
    let mut runs = Vec::new();
    runs.extend(build_batman_bellman_runs(suite_id, seeds, smoke));
    runs.extend(build_batman_classic_runs(suite_id, seeds, smoke));
    runs.extend(build_babel_runs(suite_id, seeds, smoke));
    runs.extend(build_pathway_runs(suite_id, seeds, smoke));
    runs.extend(build_field_runs(suite_id, seeds, smoke));
    runs.extend(build_comparison_runs(suite_id, seeds, smoke));
    runs.extend(build_head_to_head_runs(suite_id, seeds, smoke));
    ExperimentSuite {
        suite_id: suite_id.to_string(),
        runs,
    }
}

// long-block-exception: the BATMAN family catalog is kept in one function so the
// full coarse/fine sweep roster stays reviewable in one place.
fn build_batman_bellman_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let coarse = vec![
        ExperimentParameterSet::batman_bellman(1, 1),
        ExperimentParameterSet::batman_bellman(2, 1),
        ExperimentParameterSet::batman_bellman(4, 2),
        ExperimentParameterSet::batman_bellman(8, 4),
    ];
    let fine = vec![
        ExperimentParameterSet::batman_bellman(1, 1),
        ExperimentParameterSet::batman_bellman(3, 1),
        ExperimentParameterSet::batman_bellman(4, 2),
        ExperimentParameterSet::batman_bellman(5, 2),
        ExperimentParameterSet::batman_bellman(6, 3),
    ];
    let parameter_sets = if smoke {
        vec![coarse[1].clone(), coarse[3].clone()]
    } else {
        coarse.into_iter().chain(fine).collect()
    };

    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "batman-bellman-sparse-line-low-loss",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "low".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 12,
            },
            build_batman_bellman_sparse_line_low_loss,
        ),
        (
            "batman-bellman-decay-window-pressure",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 44,
            },
            build_batman_bellman_decay_window_pressure,
        ),
        (
            "batman-bellman-partition-recovery",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 38,
            },
            build_batman_bellman_partition_recovery,
        ),
        (
            "batman-bellman-medium-ring-contention",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 28,
            },
            build_batman_bellman_medium_ring_contention,
        ),
        (
            "batman-bellman-asymmetric-bridge",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "severe".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 52,
            },
            build_batman_bellman_asymmetric_bridge,
        ),
        (
            "batman-bellman-asymmetry-relink-transition",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "repeated-relink".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 48,
            },
            build_batman_bellman_asymmetry_relink_transition,
        ),
        (
            "batman-bellman-churn-intrinsic-limit",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "low".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "repeated-relink".to_string(),
                node_pressure: "mixed".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 56,
            },
            build_batman_bellman_churn_intrinsic_limit,
        ),
    ];

    expand_runs(
        suite_id,
        "batman-bellman",
        seeds,
        &parameter_sets,
        &families,
    )
}

// long-block-exception: the batman-classic family catalog mirrors batman-bellman
// for direct comparison between spec-faithful and enhanced engines.
fn build_batman_classic_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let coarse = vec![
        ExperimentParameterSet::batman_classic(2, 1),
        ExperimentParameterSet::batman_classic(4, 2),
        ExperimentParameterSet::batman_classic(8, 4),
    ];
    let fine = vec![
        ExperimentParameterSet::batman_classic(4, 2),
        ExperimentParameterSet::batman_classic(6, 3),
        ExperimentParameterSet::batman_classic(10, 5),
    ];
    let parameter_sets = if smoke {
        vec![coarse[1].clone(), coarse[2].clone()]
    } else {
        coarse.into_iter().chain(fine).collect()
    };
    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "batman-classic-decay-window-pressure",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 44,
            },
            build_batman_classic_decay_window_pressure,
        ),
        (
            "batman-classic-partition-recovery",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 38,
            },
            build_batman_classic_partition_recovery,
        ),
    ];
    expand_runs(
        suite_id,
        "batman-classic",
        seeds,
        &parameter_sets,
        &families,
    )
}

// long-block-exception: the Babel family catalog mirrors batman-bellman for
// direct comparison under identical regimes.
fn build_babel_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let coarse = vec![
        ExperimentParameterSet::babel(2, 1),
        ExperimentParameterSet::babel(4, 2),
        ExperimentParameterSet::babel(8, 4),
    ];
    let fine = vec![
        ExperimentParameterSet::babel(4, 2),
        ExperimentParameterSet::babel(6, 3),
    ];
    let parameter_sets = if smoke {
        vec![coarse[1].clone(), coarse[2].clone()]
    } else {
        coarse.into_iter().chain(fine).collect()
    };
    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "babel-decay-window-pressure",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 44,
            },
            build_babel_decay_window_pressure,
        ),
        (
            "babel-asymmetry-cost-penalty",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "severe".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 52,
            },
            build_babel_asymmetry_cost_penalty,
        ),
        (
            "babel-partition-feasibility-recovery",
            RegimeDescriptor {
                density: "sparse-line".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 38,
            },
            build_babel_partition_feasibility_recovery,
        ),
    ];
    expand_runs(suite_id, "babel", seeds, &parameter_sets, &families)
}

// long-block-exception: the Pathway family catalog is kept in one function so the
// full coarse/fine sweep roster stays reviewable in one place.
fn build_pathway_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let coarse = vec![
        ExperimentParameterSet::pathway(1, PathwaySearchHeuristicMode::Zero),
        ExperimentParameterSet::pathway(2, PathwaySearchHeuristicMode::Zero),
        ExperimentParameterSet::pathway(4, PathwaySearchHeuristicMode::Zero),
        ExperimentParameterSet::pathway(6, PathwaySearchHeuristicMode::HopLowerBound),
    ];
    let fine = vec![
        ExperimentParameterSet::pathway(2, PathwaySearchHeuristicMode::HopLowerBound),
        ExperimentParameterSet::pathway(3, PathwaySearchHeuristicMode::Zero),
        ExperimentParameterSet::pathway(4, PathwaySearchHeuristicMode::HopLowerBound),
    ];
    let parameter_sets = if smoke {
        vec![coarse[0].clone(), coarse[2].clone()]
    } else {
        coarse.into_iter().chain(fine).collect()
    };

    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "pathway-search-budget-pressure",
            RegimeDescriptor {
                density: "sparse-fanout".to_string(),
                loss: "moderate".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 42,
            },
            build_pathway_search_budget_pressure,
        ),
        (
            "pathway-high-fanout-budget-pressure",
            RegimeDescriptor {
                density: "high-fanout".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "partition".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 50,
            },
            build_pathway_high_fanout_budget_pressure,
        ),
        (
            "pathway-sparse-service-fanout",
            RegimeDescriptor {
                density: "sparse-fanout".to_string(),
                loss: "low".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 14,
            },
            build_pathway_sparse_service_fanout,
        ),
        (
            "pathway-medium-service-mesh",
            RegimeDescriptor {
                density: "medium-mesh".to_string(),
                loss: "low".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 24,
            },
            build_pathway_medium_service_mesh,
        ),
        (
            "pathway-dense-contention-service",
            RegimeDescriptor {
                density: "dense-mesh".to_string(),
                loss: "moderate".to_string(),
                interference: "high".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 46,
            },
            build_pathway_dense_contention_service,
        ),
        (
            "pathway-churn-replacement",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "relink-and-replace".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "explicit-path".to_string(),
                stress_score: 50,
            },
            build_pathway_churn_replacement,
        ),
        (
            "pathway-bridge-failure-service",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "partition-recovery".to_string(),
                node_pressure: "tight-hold".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 58,
            },
            build_pathway_bridge_failure_service,
        ),
    ];

    expand_runs(suite_id, "pathway", seeds, &parameter_sets, &families)
}

// long-block-exception: the Field family catalog is kept in one function so the
// corridor-specific tuning sweep remains auditable in one place.
fn build_field_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let coarse = vec![
        ExperimentParameterSet::field_tuned(4, FieldSearchHeuristicMode::Zero, 1, 140, 180),
        ExperimentParameterSet::field_tuned(
            6,
            FieldSearchHeuristicMode::HopLowerBound,
            2,
            130,
            130,
        ),
        ExperimentParameterSet::field_tuned(6, FieldSearchHeuristicMode::HopLowerBound, 3, 170, 90),
    ];
    let fine = vec![
        ExperimentParameterSet::field_tuned(4, FieldSearchHeuristicMode::Zero, 3, 80, 70),
        ExperimentParameterSet::field_tuned(
            8,
            FieldSearchHeuristicMode::HopLowerBound,
            1,
            120,
            190,
        ),
    ];
    let parameter_sets = if smoke {
        vec![coarse[0].clone(), coarse[1].clone()]
    } else {
        coarse.into_iter().chain(fine).collect()
    };

    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "field-partial-observability-bridge",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 44,
            },
            build_field_partial_observability_bridge,
        ),
        (
            "field-reconfiguration-recovery",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "relink-and-replace".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 48,
            },
            build_field_reconfiguration_recovery,
        ),
        (
            "field-asymmetric-envelope-shift",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "severe".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 54,
            },
            build_field_asymmetric_envelope_shift,
        ),
        (
            "field-uncertain-service-fanout",
            RegimeDescriptor {
                density: "high-fanout".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 46,
            },
            build_field_uncertain_service_fanout,
        ),
        (
            "field-service-overlap-reselection",
            RegimeDescriptor {
                density: "high-fanout".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "branch-reselection".to_string(),
                node_pressure: "moderate".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 54,
            },
            build_field_service_overlap_reselection,
        ),
        (
            "field-service-freshness-inversion",
            RegimeDescriptor {
                density: "high-fanout".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "freshness-inversion".to_string(),
                node_pressure: "moderate".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 58,
            },
            build_field_service_freshness_inversion,
        ),
        (
            "field-service-publication-pressure",
            RegimeDescriptor {
                density: "high-fanout".to_string(),
                loss: "moderate".to_string(),
                interference: "high".to_string(),
                asymmetry: "mild".to_string(),
                churn: "overpublish-pressure".to_string(),
                node_pressure: "moderate".to_string(),
                objective_regime: "service".to_string(),
                stress_score: 60,
            },
            build_field_service_publication_pressure,
        ),
        (
            "field-bridge-anti-entropy-continuity",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "intermittent-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 52,
            },
            build_field_bridge_anti_entropy_continuity,
        ),
        (
            "field-bootstrap-upgrade-window",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "alternating-repair".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 50,
            },
            build_field_bootstrap_upgrade_window,
        ),
    ];

    expand_runs(suite_id, "field", seeds, &parameter_sets, &families)
}

fn build_comparison_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let configs = if smoke {
        vec![ExperimentParameterSet::comparison(
            4,
            2,
            3,
            PathwaySearchHeuristicMode::Zero,
        )]
    } else {
        vec![
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero),
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound),
        ]
    };
    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "comparison-connected-low-loss",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "low".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 18,
            },
            build_comparison_connected_low_loss,
        ),
        (
            "comparison-connected-high-loss",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "high".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "relink-and-replace".to_string(),
                node_pressure: "mixed".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 54,
            },
            build_comparison_connected_high_loss,
        ),
        (
            "comparison-bridge-transition",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 42,
            },
            build_comparison_bridge_transition,
        ),
        (
            "comparison-partial-observability-bridge",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 46,
            },
            build_comparison_partial_observability_bridge,
        ),
        (
            "comparison-concurrent-mixed",
            RegimeDescriptor {
                density: "medium-mesh".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "tight-connection".to_string(),
                objective_regime: "concurrent-mixed".to_string(),
                stress_score: 48,
            },
            build_comparison_concurrent_mixed,
        ),
        (
            "comparison-corridor-continuity-uncertainty",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "intermittent-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 50,
            },
            build_comparison_corridor_continuity_uncertainty,
        ),
    ];
    expand_runs(suite_id, "comparison", seeds, &configs, &families)
}

fn build_head_to_head_runs(suite_id: &str, seeds: &[u64], _smoke: bool) -> Vec<ExperimentRunSpec> {
    let configs = vec![
        ExperimentParameterSet::head_to_head("batman-bellman", Some((1, 1)), None, None),
        ExperimentParameterSet::head_to_head("batman-classic", Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head("babel", Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head(
            "pathway",
            None,
            Some((2, PathwaySearchHeuristicMode::Zero)),
            None,
        ),
        ExperimentParameterSet::head_to_head(
            "field",
            None,
            None,
            Some((4, FieldSearchHeuristicMode::HopLowerBound)),
        ),
        ExperimentParameterSet::head_to_head(
            "pathway-batman-bellman",
            Some((1, 1)),
            Some((2, PathwaySearchHeuristicMode::Zero)),
            None,
        ),
    ];
    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "head-to-head-connected-low-loss",
            RegimeDescriptor {
                density: "medium-ring".to_string(),
                loss: "low".to_string(),
                interference: "low".to_string(),
                asymmetry: "none".to_string(),
                churn: "static".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "connected-only".to_string(),
                stress_score: 18,
            },
            build_comparison_connected_low_loss,
        ),
        (
            "head-to-head-connected-high-loss",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "high".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "relink-and-replace".to_string(),
                node_pressure: "mixed".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 54,
            },
            build_comparison_connected_high_loss,
        ),
        (
            "head-to-head-bridge-transition",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 42,
            },
            build_comparison_bridge_transition,
        ),
        (
            "head-to-head-partial-observability-bridge",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "mild".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 46,
            },
            build_comparison_partial_observability_bridge,
        ),
        (
            "head-to-head-concurrent-mixed",
            RegimeDescriptor {
                density: "medium-mesh".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "none".to_string(),
                churn: "partial-recovery".to_string(),
                node_pressure: "tight-connection".to_string(),
                objective_regime: "concurrent-mixed".to_string(),
                stress_score: 48,
            },
            build_comparison_concurrent_mixed,
        ),
        (
            "head-to-head-corridor-continuity-uncertainty",
            RegimeDescriptor {
                density: "bridge-cluster".to_string(),
                loss: "moderate".to_string(),
                interference: "medium".to_string(),
                asymmetry: "moderate".to_string(),
                churn: "intermittent-recovery".to_string(),
                node_pressure: "none".to_string(),
                objective_regime: "repairable-connected".to_string(),
                stress_score: 50,
            },
            build_comparison_corridor_continuity_uncertainty,
        ),
    ];
    expand_runs(suite_id, "head-to-head", seeds, &configs, &families)
}

type FamilyBuilder =
    fn(&ExperimentParameterSet, SimulationSeed) -> (JacquardScenario, ScriptedEnvironmentModel);

fn expand_runs(
    suite_id: &str,
    engine_family: &str,
    seeds: &[u64],
    parameter_sets: &[ExperimentParameterSet],
    families: &[(&str, RegimeDescriptor, FamilyBuilder)],
) -> Vec<ExperimentRunSpec> {
    let mut runs = Vec::new();
    for (family_id, regime, builder) in families {
        for parameters in parameter_sets {
            for seed in seeds {
                let seed = SimulationSeed(*seed);
                let (scenario, environment) = builder(parameters, seed);
                runs.push(ExperimentRunSpec {
                    run_id: format!(
                        "{}-{}-{}-{}",
                        suite_id, family_id, parameters.config_id, seed.0
                    ),
                    suite_id: suite_id.to_string(),
                    family_id: (*family_id).to_string(),
                    engine_family: engine_family.to_string(),
                    seed,
                    regime: regime.clone(),
                    parameters: parameters.clone(),
                    scenario,
                    environment,
                });
            }
        }
    }
    runs
}

// long-block-exception: one reducer intentionally computes the stable per-run
// summary schema directly from the replay view in one auditable pass.
fn summarize_run(spec: &ExperimentRunSpec, reduced: &ReducedReplayView) -> ExperimentRunSummary {
    let mut objective_count = 0u32;
    let mut activation_successes = 0u32;
    let mut present_round_total = 0u32;
    let mut first_route_rounds = Vec::new();
    let mut first_loss_rounds = Vec::new();
    let mut recovery_rounds = Vec::new();
    let mut churn_count = 0u32;
    let mut handoff_count = 0u32;
    let mut route_observation_count = 0u32;
    let mut stability_scores = Vec::new();
    let owner_nodes = spec
        .scenario
        .bound_objectives()
        .iter()
        .map(|binding| binding.owner_node_id)
        .collect::<BTreeSet<_>>();

    for binding in spec.scenario.bound_objectives() {
        objective_count = objective_count.saturating_add(1);
        if reduced.route_seen(binding.owner_node_id, &binding.objective.destination) {
            activation_successes = activation_successes.saturating_add(1);
        }
        present_round_total = present_round_total.saturating_add(
            u32::try_from(
                reduced
                    .route_present_rounds(binding.owner_node_id, &binding.objective.destination)
                    .len(),
            )
            .unwrap_or(u32::MAX),
        );
        first_route_rounds.push(
            reduced.first_round_with_route(binding.owner_node_id, &binding.objective.destination),
        );
        first_loss_rounds.push(reduced.first_round_without_route_after_presence(
            binding.owner_node_id,
            &binding.objective.destination,
        ));
        recovery_rounds.push(
            reduced.recovery_delta_rounds(binding.owner_node_id, &binding.objective.destination),
        );
        churn_count = churn_count.saturating_add(
            reduced.route_churn_count(binding.owner_node_id, &binding.objective.destination),
        );
        handoff_count = handoff_count.saturating_add(
            reduced.engine_handoff_count(binding.owner_node_id, &binding.objective.destination),
        );
        route_observation_count = route_observation_count.saturating_add(
            u32::try_from(
                reduced
                    .route_observations()
                    .into_iter()
                    .filter(|observation| {
                        observation.key.owner_node_id == binding.owner_node_id
                            && observation.key.destination == binding.objective.destination
                    })
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        stability_scores.extend(
            reduced.route_stability_scores(binding.owner_node_id, &binding.objective.destination),
        );
    }

    let engine_round_counts = engine_round_counts(reduced);
    let no_route_rounds = reduced
        .rounds
        .iter()
        .filter(|round| round.active_routes.is_empty())
        .count();
    let hook_counts = reduced.environment_hook_counts();
    let failure_counts = reduced.failure_class_counts();
    let stability_first = stability_scores.first().copied();
    let stability_last = stability_scores.last().copied();
    let stability_total = stability_scores
        .iter()
        .fold(0u32, |acc, score| acc.saturating_add(*score));
    let mut field_selected_result_rounds = 0u32;
    let mut field_search_reconfiguration_rounds = 0u32;
    let mut field_bootstrap_active_rounds = 0u32;
    let mut field_continuity_band = None;
    let mut field_commitment_resolution = None;
    let mut field_last_outcome = None;
    let mut field_last_continuity_transition = None;
    let mut field_last_promotion_decision = None;
    let mut field_last_promotion_blocker = None;
    let mut field_bootstrap_activation_count = 0u32;
    let mut field_bootstrap_hold_count = 0u32;
    let mut field_bootstrap_narrow_count = 0u32;
    let mut field_bootstrap_upgrade_count = 0u32;
    let mut field_bootstrap_withdraw_count = 0u32;
    let mut field_degraded_steady_entry_count = 0u32;
    let mut field_degraded_steady_recovery_count = 0u32;
    let mut field_degraded_to_bootstrap_count = 0u32;
    let mut field_degraded_steady_round_count = 0u32;
    let mut field_service_retention_carry_forward_count = 0u32;
    let mut field_asymmetric_shift_success_count = 0u32;
    let mut field_protocol_reconfiguration_count = 0u32;
    let mut field_route_bound_reconfiguration_count = 0u32;
    let mut field_continuation_shift_count = 0u32;
    let mut field_corridor_narrow_count = 0u32;
    let mut field_checkpoint_restore_count = 0u32;
    for owner_node_id in owner_nodes {
        let field_replays = reduced.field_replays_for(owner_node_id);
        field_selected_result_rounds = field_selected_result_rounds.saturating_add(
            u32::try_from(
                field_replays
                    .iter()
                    .filter(|summary| summary.selected_result_present)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        field_search_reconfiguration_rounds = field_search_reconfiguration_rounds.saturating_add(
            u32::try_from(
                field_replays
                    .iter()
                    .filter(|summary| summary.search_reconfiguration_present)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        field_bootstrap_active_rounds = field_bootstrap_active_rounds.saturating_add(
            u32::try_from(
                field_replays
                    .iter()
                    .filter(|summary| summary.bootstrap_active)
                    .count(),
            )
            .unwrap_or(u32::MAX),
        );
        field_continuity_band = field_continuity_band.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.continuity_band.clone())
        });
        field_last_continuity_transition = field_last_continuity_transition.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.last_continuity_transition.clone())
        });
        field_last_promotion_decision = field_last_promotion_decision.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.last_promotion_decision.clone())
        });
        field_last_promotion_blocker = field_last_promotion_blocker.or_else(|| {
            field_replays
                .iter()
                .find_map(|summary| summary.last_promotion_blocker.clone())
        });
        field_bootstrap_activation_count = field_bootstrap_activation_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_activation_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_hold_count = field_bootstrap_hold_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_hold_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_narrow_count = field_bootstrap_narrow_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_narrow_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_upgrade_count = field_bootstrap_upgrade_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_upgrade_count)
                .max()
                .unwrap_or(0),
        );
        field_bootstrap_withdraw_count = field_bootstrap_withdraw_count.max(
            field_replays
                .iter()
                .map(|summary| summary.bootstrap_withdraw_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_steady_entry_count = field_degraded_steady_entry_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_steady_entry_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_steady_recovery_count = field_degraded_steady_recovery_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_steady_recovery_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_to_bootstrap_count = field_degraded_to_bootstrap_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_to_bootstrap_count)
                .max()
                .unwrap_or(0),
        );
        field_degraded_steady_round_count = field_degraded_steady_round_count.max(
            field_replays
                .iter()
                .map(|summary| summary.degraded_steady_round_count)
                .max()
                .unwrap_or(0),
        );
        field_service_retention_carry_forward_count = field_service_retention_carry_forward_count
            .max(
                field_replays
                    .iter()
                    .map(|summary| summary.service_retention_carry_forward_count)
                    .max()
                    .unwrap_or(0),
            );
        field_asymmetric_shift_success_count = field_asymmetric_shift_success_count.max(
            field_replays
                .iter()
                .map(|summary| summary.asymmetric_shift_success_count)
                .max()
                .unwrap_or(0),
        );
        field_protocol_reconfiguration_count = field_protocol_reconfiguration_count.max(
            field_replays
                .iter()
                .map(|summary| {
                    u32::try_from(summary.protocol_reconfiguration_count).unwrap_or(u32::MAX)
                })
                .max()
                .unwrap_or(0),
        );
        field_route_bound_reconfiguration_count = field_route_bound_reconfiguration_count.max(
            field_replays
                .iter()
                .map(|summary| {
                    u32::try_from(summary.route_bound_reconfiguration_count).unwrap_or(u32::MAX)
                })
                .max()
                .unwrap_or(0),
        );
        field_continuation_shift_count = field_continuation_shift_count.max(
            field_replays
                .iter()
                .map(|summary| summary.continuation_shift_count)
                .max()
                .unwrap_or(0),
        );
        field_corridor_narrow_count = field_corridor_narrow_count.max(
            field_replays
                .iter()
                .map(|summary| summary.corridor_narrow_count)
                .max()
                .unwrap_or(0),
        );
        field_checkpoint_restore_count = field_checkpoint_restore_count.max(
            field_replays
                .iter()
                .map(|summary| summary.checkpoint_restore_count)
                .max()
                .unwrap_or(0),
        );
    }

    for binding in spec.scenario.bound_objectives() {
        field_commitment_resolution = field_commitment_resolution.or_else(|| {
            reduced.last_field_commitment_resolution(
                binding.owner_node_id,
                &binding.objective.destination,
            )
        });
        field_last_outcome = field_last_outcome.or_else(|| {
            reduced.last_field_route_outcome(binding.owner_node_id, &binding.objective.destination)
        });
        field_continuity_band = field_continuity_band.or_else(|| {
            reduced
                .last_field_continuity_band(binding.owner_node_id, &binding.objective.destination)
        });
        field_last_promotion_decision = field_last_promotion_decision.or_else(|| {
            reduced.last_field_promotion_decision(
                binding.owner_node_id,
                &binding.objective.destination,
            )
        });
        field_last_promotion_blocker = field_last_promotion_blocker.or_else(|| {
            reduced
                .last_field_promotion_blocker(binding.owner_node_id, &binding.objective.destination)
        });
        field_continuation_shift_count =
            field_continuation_shift_count.max(reduced.field_continuation_shift_count(
                binding.owner_node_id,
                &binding.objective.destination,
            ));
    }

    ExperimentRunSummary {
        run_id: spec.run_id.clone(),
        suite_id: spec.suite_id.clone(),
        family_id: spec.family_id.clone(),
        scenario_name: spec.scenario.name().to_string(),
        engine_family: spec.engine_family.clone(),
        config_id: spec.parameters.config_id.clone(),
        comparison_engine_set: spec.parameters.comparison_engine_set.clone(),
        batman_bellman_stale_after_ticks: spec.parameters.batman_bellman_stale_after_ticks,
        batman_bellman_next_refresh_within_ticks: spec
            .parameters
            .batman_bellman_next_refresh_within_ticks,
        batman_classic_stale_after_ticks: spec.parameters.batman_classic_stale_after_ticks,
        batman_classic_next_refresh_within_ticks: spec
            .parameters
            .batman_classic_next_refresh_within_ticks,
        babel_stale_after_ticks: spec.parameters.babel_stale_after_ticks,
        babel_next_refresh_within_ticks: spec.parameters.babel_next_refresh_within_ticks,
        pathway_query_budget: spec.parameters.pathway_query_budget,
        pathway_heuristic_mode: spec.parameters.pathway_heuristic_mode.clone(),
        field_query_budget: spec.parameters.field_query_budget,
        field_heuristic_mode: spec.parameters.field_heuristic_mode.clone(),
        field_service_publication_neighbor_limit: spec
            .parameters
            .field_service_publication_neighbor_limit,
        field_service_freshness_weight: spec.parameters.field_service_freshness_weight,
        field_service_narrowing_bias: spec.parameters.field_service_narrowing_bias,
        seed: spec.seed.0,
        density: spec.regime.density.clone(),
        loss: spec.regime.loss.clone(),
        interference: spec.regime.interference.clone(),
        asymmetry: spec.regime.asymmetry.clone(),
        churn: spec.regime.churn.clone(),
        node_pressure: spec.regime.node_pressure.clone(),
        objective_regime: spec.regime.objective_regime.clone(),
        stress_score: spec.regime.stress_score,
        objective_count,
        activation_success_permille: ratio_permille(activation_successes, objective_count),
        route_present_permille: ratio_permille(
            present_round_total,
            objective_count.saturating_mul(reduced.round_count.max(1)),
        ),
        first_materialization_round_mean: average_option_u32(&first_route_rounds),
        first_loss_round_mean: average_option_u32(&first_loss_rounds),
        recovery_round_mean: average_option_u32(&recovery_rounds),
        route_churn_count: churn_count,
        engine_handoff_count: handoff_count,
        route_observation_count,
        batman_bellman_selected_rounds: *engine_round_counts.get("batman-bellman").unwrap_or(&0),
        batman_classic_selected_rounds: *engine_round_counts.get("batman-classic").unwrap_or(&0),
        babel_selected_rounds: *engine_round_counts.get("babel").unwrap_or(&0),
        pathway_selected_rounds: *engine_round_counts.get("pathway").unwrap_or(&0),
        field_selected_rounds: *engine_round_counts.get("field").unwrap_or(&0),
        field_selected_result_rounds,
        field_search_reconfiguration_rounds,
        field_bootstrap_active_rounds,
        field_continuity_band,
        field_commitment_resolution,
        field_last_outcome,
        field_last_continuity_transition,
        field_last_promotion_decision,
        field_last_promotion_blocker,
        field_bootstrap_activation_permille: ratio_permille(
            field_bootstrap_activation_count,
            objective_count.max(1),
        ),
        field_bootstrap_hold_permille: ratio_permille(
            field_bootstrap_hold_count,
            objective_count.max(1),
        ),
        field_bootstrap_narrow_permille: ratio_permille(
            field_bootstrap_narrow_count,
            objective_count.max(1),
        ),
        field_bootstrap_upgrade_permille: ratio_permille(
            field_bootstrap_upgrade_count,
            objective_count.max(1),
        ),
        field_bootstrap_withdraw_permille: ratio_permille(
            field_bootstrap_withdraw_count,
            objective_count.max(1),
        ),
        field_degraded_steady_entry_permille: ratio_permille(
            field_degraded_steady_entry_count,
            objective_count.max(1),
        ),
        field_degraded_steady_recovery_permille: ratio_permille(
            field_degraded_steady_recovery_count,
            objective_count.max(1),
        ),
        field_degraded_to_bootstrap_permille: ratio_permille(
            field_degraded_to_bootstrap_count,
            objective_count.max(1),
        ),
        field_degraded_steady_round_permille: ratio_permille(
            field_degraded_steady_round_count,
            objective_count
                .saturating_mul(reduced.round_count.max(1))
                .max(1),
        ),
        field_service_retention_carry_forward_permille: ratio_permille(
            field_service_retention_carry_forward_count,
            objective_count.max(1),
        ),
        field_asymmetric_shift_success_permille: ratio_permille(
            field_asymmetric_shift_success_count,
            objective_count.max(1),
        ),
        field_protocol_reconfiguration_count,
        field_route_bound_reconfiguration_count,
        field_continuation_shift_count,
        field_corridor_narrow_count,
        field_checkpoint_restore_count,
        no_route_rounds: u32::try_from(no_route_rounds).unwrap_or(u32::MAX),
        dominant_engine: dominant_engine(&engine_round_counts),
        stability_min: stability_scores.iter().copied().min(),
        stability_first,
        stability_last,
        stability_median: median_u32(&stability_scores),
        stability_max: stability_scores.iter().copied().max(),
        stability_total,
        maintenance_failure_count: reduced.maintenance_failure_count(),
        failure_summary_count: u32::try_from(reduced.failure_summaries.len()).unwrap_or(u32::MAX),
        no_candidate_count: failure_counts.no_candidate,
        inadmissible_candidate_count: failure_counts.inadmissible_candidate,
        lost_reachability_count: failure_counts.lost_reachability,
        replacement_loop_count: failure_counts.replacement_loop,
        activation_failure_count: failure_counts.activation_failure,
        persistent_degraded_count: failure_counts.persistent_degraded,
        other_failure_count: failure_counts.other,
        replace_topology_count: hook_counts.replace_topology,
        medium_degradation_count: hook_counts.medium_degradation,
        asymmetric_degradation_count: hook_counts.asymmetric_degradation,
        partition_count: hook_counts.partition,
        cascade_partition_count: hook_counts.cascade_partition,
        mobility_relink_count: hook_counts.mobility_relink,
        intrinsic_limit_count: hook_counts.intrinsic_limit,
    }
}

// long-block-exception: the aggregate summary intentionally stays in one grouped
// reduction so the output schema remains easy to audit against the run schema.
fn aggregate_runs(runs: &[ExperimentRunSummary]) -> Vec<ExperimentAggregateSummary> {
    let mut grouped: BTreeMap<
        (String, String, Option<String>, String),
        Vec<&ExperimentRunSummary>,
    > = BTreeMap::new();
    for run in runs {
        grouped
            .entry((
                run.engine_family.clone(),
                run.family_id.clone(),
                run.comparison_engine_set.clone(),
                run.config_id.clone(),
            ))
            .or_default()
            .push(run);
    }

    grouped
        .into_values()
        // long-block-exception: one aggregate-group reduction keeps the
        // complete derived schema together at the grouping site.
        .map(|group| {
            let first = group
                .first()
                .expect("experiment aggregate group must be non-empty");
            let run_count = u32::try_from(group.len()).unwrap_or(u32::MAX);
            let engine_mode = mode(group.iter().filter_map(|run| run.dominant_engine.clone()));
            let activation_success_permille_mean =
                average_u32(group.iter().map(|run| run.activation_success_permille));
            let route_present_permille_mean =
                average_u32(group.iter().map(|run| run.route_present_permille));
            let first_materialization_round_mean = average_option_u32_from_iter(
                group.iter().map(|run| run.first_materialization_round_mean),
            );
            let first_loss_round_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.first_loss_round_mean));
            let recovery_round_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.recovery_round_mean));
            let route_churn_count_mean = average_u32(group.iter().map(|run| run.route_churn_count));
            let engine_handoff_count_mean =
                average_u32(group.iter().map(|run| run.engine_handoff_count));
            let stability_first_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.stability_first));
            let stability_last_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.stability_last));
            let stability_median_mean =
                average_option_u32_from_iter(group.iter().map(|run| run.stability_median));
            let stability_total_mean = average_u32(group.iter().map(|run| run.stability_total));
            let maintenance_failure_count_mean =
                average_u32(group.iter().map(|run| run.maintenance_failure_count));
            let failure_summary_count_mean =
                average_u32(group.iter().map(|run| run.failure_summary_count));
            let field_selected_result_rounds_mean =
                average_u32(group.iter().map(|run| run.field_selected_result_rounds));
            let field_search_reconfiguration_rounds_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_search_reconfiguration_rounds),
            );
            let field_bootstrap_active_rounds_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_active_rounds));
            let field_continuity_band_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_continuity_band.clone()),
            );
            let field_commitment_resolution_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_commitment_resolution.clone()),
            );
            let field_last_outcome_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_outcome.clone()),
            );
            let field_last_continuity_transition_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_continuity_transition.clone()),
            );
            let field_last_promotion_decision_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_promotion_decision.clone()),
            );
            let field_last_promotion_blocker_mode = mode(
                group
                    .iter()
                    .filter_map(|run| run.field_last_promotion_blocker.clone()),
            );
            let field_bootstrap_activation_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_bootstrap_activation_permille),
            );
            let field_bootstrap_hold_permille_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_hold_permille));
            let field_bootstrap_narrow_permille_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_narrow_permille));
            let field_bootstrap_upgrade_permille_mean =
                average_u32(group.iter().map(|run| run.field_bootstrap_upgrade_permille));
            let field_bootstrap_withdraw_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_bootstrap_withdraw_permille),
            );
            let field_degraded_steady_entry_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_steady_entry_permille),
            );
            let field_degraded_steady_recovery_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_steady_recovery_permille),
            );
            let field_degraded_to_bootstrap_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_to_bootstrap_permille),
            );
            let field_degraded_steady_round_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_degraded_steady_round_permille),
            );
            let field_service_retention_carry_forward_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_service_retention_carry_forward_permille),
            );
            let field_asymmetric_shift_success_permille_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_asymmetric_shift_success_permille),
            );
            let field_protocol_reconfiguration_count_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_protocol_reconfiguration_count),
            );
            let field_route_bound_reconfiguration_count_mean = average_u32(
                group
                    .iter()
                    .map(|run| run.field_route_bound_reconfiguration_count),
            );
            let field_continuation_shift_count_mean =
                average_u32(group.iter().map(|run| run.field_continuation_shift_count));
            let field_corridor_narrow_count_mean =
                average_u32(group.iter().map(|run| run.field_corridor_narrow_count));
            let field_checkpoint_restore_count_mean =
                average_u32(group.iter().map(|run| run.field_checkpoint_restore_count));
            let no_candidate_count_mean =
                average_u32(group.iter().map(|run| run.no_candidate_count));
            let inadmissible_candidate_count_mean =
                average_u32(group.iter().map(|run| run.inadmissible_candidate_count));
            let lost_reachability_count_mean =
                average_u32(group.iter().map(|run| run.lost_reachability_count));
            let replacement_loop_count_mean =
                average_u32(group.iter().map(|run| run.replacement_loop_count));
            let persistent_degraded_count_mean =
                average_u32(group.iter().map(|run| run.persistent_degraded_count));
            let acceptable = activation_success_permille_mean >= 900
                && route_present_permille_mean >= 500
                && lost_reachability_count_mean == 0
                && maintenance_failure_count_mean == 0;

            ExperimentAggregateSummary {
                suite_id: first.suite_id.clone(),
                family_id: first.family_id.clone(),
                engine_family: first.engine_family.clone(),
                config_id: first.config_id.clone(),
                comparison_engine_set: first.comparison_engine_set.clone(),
                batman_bellman_stale_after_ticks: first.batman_bellman_stale_after_ticks,
                batman_bellman_next_refresh_within_ticks: first
                    .batman_bellman_next_refresh_within_ticks,
                batman_classic_stale_after_ticks: first.batman_classic_stale_after_ticks,
                batman_classic_next_refresh_within_ticks: first
                    .batman_classic_next_refresh_within_ticks,
                babel_stale_after_ticks: first.babel_stale_after_ticks,
                babel_next_refresh_within_ticks: first.babel_next_refresh_within_ticks,
                pathway_query_budget: first.pathway_query_budget,
                pathway_heuristic_mode: first.pathway_heuristic_mode.clone(),
                field_query_budget: first.field_query_budget,
                field_heuristic_mode: first.field_heuristic_mode.clone(),
                field_service_publication_neighbor_limit: first
                    .field_service_publication_neighbor_limit,
                field_service_freshness_weight: first.field_service_freshness_weight,
                field_service_narrowing_bias: first.field_service_narrowing_bias,
                density: first.density.clone(),
                loss: first.loss.clone(),
                interference: first.interference.clone(),
                asymmetry: first.asymmetry.clone(),
                churn: first.churn.clone(),
                node_pressure: first.node_pressure.clone(),
                objective_regime: first.objective_regime.clone(),
                stress_score: first.stress_score,
                run_count,
                activation_success_permille_mean,
                route_present_permille_mean,
                first_materialization_round_mean,
                first_loss_round_mean,
                recovery_round_mean,
                route_churn_count_mean,
                engine_handoff_count_mean,
                dominant_engine: engine_mode,
                field_selected_result_rounds_mean,
                field_search_reconfiguration_rounds_mean,
                field_bootstrap_active_rounds_mean,
                field_continuity_band_mode,
                field_commitment_resolution_mode,
                field_last_outcome_mode,
                field_last_continuity_transition_mode,
                field_last_promotion_decision_mode,
                field_last_promotion_blocker_mode,
                field_bootstrap_activation_permille_mean,
                field_bootstrap_hold_permille_mean,
                field_bootstrap_narrow_permille_mean,
                field_bootstrap_upgrade_permille_mean,
                field_bootstrap_withdraw_permille_mean,
                field_degraded_steady_entry_permille_mean,
                field_degraded_steady_recovery_permille_mean,
                field_degraded_to_bootstrap_permille_mean,
                field_degraded_steady_round_permille_mean,
                field_service_retention_carry_forward_permille_mean,
                field_asymmetric_shift_success_permille_mean,
                field_protocol_reconfiguration_count_mean,
                field_route_bound_reconfiguration_count_mean,
                field_continuation_shift_count_mean,
                field_corridor_narrow_count_mean,
                field_checkpoint_restore_count_mean,
                stability_first_mean,
                stability_last_mean,
                stability_median_mean,
                stability_total_mean,
                maintenance_failure_count_mean,
                failure_summary_count_mean,
                no_candidate_count_mean,
                inadmissible_candidate_count_mean,
                lost_reachability_count_mean,
                replacement_loop_count_mean,
                persistent_degraded_count_mean,
                acceptable,
            }
        })
        .collect()
}

fn summarize_breakdowns(
    aggregates: &[ExperimentAggregateSummary],
) -> Vec<ExperimentBreakdownSummary> {
    let mut grouped: BTreeMap<(String, String), Vec<&ExperimentAggregateSummary>> = BTreeMap::new();
    for aggregate in aggregates {
        grouped
            .entry((aggregate.engine_family.clone(), aggregate.config_id.clone()))
            .or_default()
            .push(aggregate);
    }

    grouped
        .into_iter()
        .map(|((engine_family, config_id), mut group)| {
            group.sort_by_key(|aggregate| (aggregate.stress_score, aggregate.family_id.clone()));
            let max_sustained_stress_score = group
                .iter()
                .filter(|aggregate| aggregate.acceptable)
                .map(|aggregate| aggregate.stress_score)
                .max()
                .unwrap_or(0);
            let first_failed = group.iter().find(|aggregate| !aggregate.acceptable);
            let breakdown_reason = first_failed.map(|aggregate| {
                if aggregate.activation_success_permille_mean < 900 {
                    "activation-success".to_string()
                } else if aggregate.route_present_permille_mean < 500 {
                    "route-presence".to_string()
                } else if aggregate.lost_reachability_count_mean > 0 {
                    "lost-reachability".to_string()
                } else if aggregate.maintenance_failure_count_mean > 0 {
                    "maintenance-failure".to_string()
                } else {
                    "failure-density".to_string()
                }
            });
            ExperimentBreakdownSummary {
                suite_id: group
                    .first()
                    .expect("breakdown groups must be non-empty")
                    .suite_id
                    .clone(),
                engine_family,
                config_id,
                max_sustained_stress_score,
                first_failed_family_id: first_failed.map(|aggregate| aggregate.family_id.clone()),
                first_failed_stress_score: first_failed.map(|aggregate| aggregate.stress_score),
                breakdown_reason,
            }
        })
        .collect()
}

fn build_batman_bellman_sparse_line_low_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(0), RatioPermille(20));
    let hosts = vec![
        HostSpec::batman_bellman(NODE_A),
        HostSpec::batman_bellman(NODE_B),
        HostSpec::batman_bellman(NODE_C),
    ];
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-sparse-line-low-loss-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            hosts,
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

fn build_batman_bellman_partition_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("batman-bellman-partition-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            26,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_batman_bellman_decay_window_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-decay-window-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(6)],
            36,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(26),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_batman_bellman_medium_ring_contention(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(150), RatioPermille(100));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-medium-ring-contention-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
                HostSpec::batman_bellman(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            20,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

fn build_batman_bellman_asymmetric_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(200), RatioPermille(80));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("batman-bellman-asymmetric-bridge-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
                HostSpec::batman_bellman(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(6),
        EnvironmentHook::AsymmetricDegradation {
            left: NODE_B,
            right: NODE_C,
            forward_confidence: RatioPermille(520),
            forward_loss: RatioPermille(380),
            reverse_confidence: RatioPermille(760),
            reverse_loss: RatioPermille(180),
        },
    )]);
    (scenario, environment)
}

fn build_batman_bellman_asymmetry_relink_transition(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(140));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-asymmetry-relink-transition-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
                HostSpec::batman_bellman(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(560),
                forward_loss: RatioPermille(260),
                reverse_confidence: RatioPermille(740),
                reverse_loss: RatioPermille(140),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(3).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(15),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_C,
                to_right: NODE_B,
                link: Box::new(topology::link(2).build()),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_batman_bellman_churn_intrinsic_limit(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(50), RatioPermille(50));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-churn-intrinsic-limit-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
                HostSpec::batman_bellman(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(256),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(3).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_C,
                to_right: NODE_B,
                link: Box::new(topology::link(2).build()),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_batman_classic_decay_window_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(100));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-classic-decay-window-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_classic(NODE_A),
                HostSpec::batman_classic(NODE_B),
                HostSpec::batman_classic(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(12)],
            50,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

fn build_batman_classic_partition_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("batman-classic-partition-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_classic(NODE_A),
                HostSpec::batman_classic(NODE_B),
                HostSpec::batman_classic(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(12)],
            60,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(30),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_A, NODE_B), (NODE_B, NODE_A)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(45),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_babel_decay_window_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(100));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("babel-decay-window-pressure-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::babel(NODE_A),
                HostSpec::babel(NODE_B),
                HostSpec::babel(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            26,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MediumDegradation {
                left: NODE_B,
                right: NODE_C,
                confidence: RatioPermille(600),
                loss: RatioPermille(250),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(20),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_babel_asymmetry_cost_penalty(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
        topology::node(4).babel().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(200), RatioPermille(80));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("babel-asymmetry-cost-penalty-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::babel(NODE_A),
                HostSpec::babel(NODE_B),
                HostSpec::babel(NODE_C),
                HostSpec::babel(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            30,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(520),
                forward_loss: RatioPermille(380),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(180),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(22),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_babel_partition_feasibility_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "babel-partition-feasibility-recovery-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::babel(NODE_A),
                HostSpec::babel(NODE_B),
                HostSpec::babel(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            36,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_pathway_sparse_service_fanout(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology4(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(0), RatioPermille(20));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-sparse-service-fanout-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![9; 16]))
                .with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

fn build_pathway_search_budget_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology4(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(20), RatioPermille(40));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-search-budget-pressure-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![14; 16]))
                .with_activation_round(2)],
            10,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(4),
        EnvironmentHook::CascadePartition {
            cuts: vec![(NODE_A, NODE_B), (NODE_B, NODE_A)],
        },
    )]);
    (scenario, environment)
}

fn build_pathway_medium_service_mesh(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = full_mesh_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(140), RatioPermille(40));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-medium-service-mesh-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![10; 16]))
                .with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

fn build_pathway_dense_contention_service(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = full_mesh_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(320), RatioPermille(140));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-dense-contention-service-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![11; 16]))
                .with_activation_round(2)],
            20,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(700),
                loss: RatioPermille(250),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_C,
                confidence: RatioPermille(660),
                loss: RatioPermille(300),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_pathway_high_fanout_budget_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology5(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
        topology::node(5).pathway().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(120), RatioPermille(80));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "pathway-high-fanout-budget-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
                HostSpec::pathway(NODE_E),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![15; 16]))
                .with_activation_round(2)],
            12,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_A, NODE_B), (NODE_B, NODE_A)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_C,
                confidence: RatioPermille(680),
                loss: RatioPermille(240),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_D,
                confidence: RatioPermille(700),
                loss: RatioPermille(180),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_pathway_churn_replacement(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(200), RatioPermille(80));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-churn-replacement-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_D,
                link: Box::new(topology::link(4).build()),
            },
        ),
    ]);
    (scenario, environment)
}

// long-block-exception: this regime fixture keeps the topology, objective, and
// staged failure/recovery hooks together so the tuned boundary is clear.
fn build_pathway_bridge_failure_service(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(180), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-bridge-failure-service-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![12; 16]))
                .with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::Partition {
                left: NODE_B,
                right: NODE_C,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::Partition {
                left: NODE_C,
                right: NODE_B,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(384),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_partial_observability_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(140));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "field-partial-observability-bridge-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A).with_field_bootstrap_summary(field_bootstrap_summary(
                    DestinationId::Node(NODE_D),
                    NODE_B,
                    900,
                    2,
                    3,
                    Some(860),
                )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(620),
                forward_loss: RatioPermille(220),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(150),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_reconfiguration_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(100), RatioPermille(120));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-reconfiguration-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_C),
                        NODE_B,
                        920,
                        1,
                        2,
                        Some(880),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_C),
                        NODE_D,
                        840,
                        1,
                        2,
                        Some(810),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_C)).with_activation_round(3)],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_D,
                link: Box::new(topology::link(3).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::ReplaceTopology {
                configuration: ring_topology(
                    topology::node(1).field().build(),
                    topology::node(2).field().build(),
                    topology::node(3).field().build(),
                    topology::node(4).field().build(),
                )
                .value,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_asymmetric_envelope_shift(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(120), RatioPermille(120));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-asymmetric-envelope-shift-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A).with_field_bootstrap_summary(field_bootstrap_summary(
                    DestinationId::Node(NODE_D),
                    NODE_B,
                    910,
                    2,
                    3,
                    Some(870),
                )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(540),
                forward_loss: RatioPermille(320),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(120),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(13),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(2).build()),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_uncertain_service_fanout(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = full_mesh_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(140), RatioPermille(110));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-uncertain-service-fanout-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![13; 16])),
                        NODE_B,
                        910,
                        1,
                        1,
                        Some(860),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![13; 16])),
                        NODE_C,
                        840,
                        1,
                        1,
                        Some(790),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![13; 16])),
                        NODE_D,
                        760,
                        1,
                        1,
                        Some(730),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![13; 16]))
                    .with_activation_round(3),
            ],
            20,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(10),
        EnvironmentHook::IntrinsicLimit {
            node_id: NODE_C,
            connection_count_max: 1,
            hold_capacity_bytes_max: jacquard_core::ByteCount(384),
        },
    )]);
    (scenario, environment)
}

fn build_field_service_overlap_reselection(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology5(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
        topology::node(5).field().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(120), RatioPermille(90));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-service-overlap-reselection-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![14; 16])),
                        NODE_B,
                        920,
                        1,
                        1,
                        Some(880),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![14; 16])),
                        NODE_C,
                        860,
                        1,
                        1,
                        Some(820),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![14; 16])),
                        NODE_D,
                        760,
                        1,
                        1,
                        Some(730),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![14; 16])),
                        NODE_E,
                        720,
                        1,
                        1,
                        Some(690),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
                HostSpec::field(NODE_E),
            ],
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![14; 16]))
                    .with_activation_round(3),
            ],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(320),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_A,
                right: NODE_C,
                forward_confidence: RatioPermille(520),
                forward_loss: RatioPermille(320),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(120),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_service_freshness_inversion(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology5(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
        topology::node(5).field().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(130), RatioPermille(100));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-service-freshness-inversion-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![15; 16])),
                        NODE_B,
                        930,
                        1,
                        1,
                        Some(900),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![15; 16])),
                        NODE_C,
                        860,
                        1,
                        1,
                        Some(820),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![15; 16])),
                        NODE_D,
                        780,
                        1,
                        1,
                        Some(740),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![15; 16])),
                        NODE_E,
                        720,
                        1,
                        1,
                        Some(690),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
                HostSpec::field(NODE_E),
            ],
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![15; 16]))
                    .with_activation_round(3),
            ],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_A,
                right: NODE_B,
                forward_confidence: RatioPermille(520),
                forward_loss: RatioPermille(340),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(120),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(11),
            EnvironmentHook::ReplaceTopology {
                configuration: restore.clone(),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(13),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_A,
                right: NODE_C,
                forward_confidence: RatioPermille(560),
                forward_loss: RatioPermille(300),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(130),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore.clone(),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_A,
                right: NODE_D,
                forward_confidence: RatioPermille(600),
                forward_loss: RatioPermille(260),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(140),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_service_publication_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology5(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
        topology::node(5).field().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(180), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "field-service-publication-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![16; 16])),
                        NODE_B,
                        910,
                        1,
                        1,
                        Some(870),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![16; 16])),
                        NODE_C,
                        860,
                        1,
                        1,
                        Some(820),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![16; 16])),
                        NODE_D,
                        790,
                        1,
                        1,
                        Some(760),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Service(jacquard_core::ServiceId(vec![16; 16])),
                        NODE_E,
                        750,
                        1,
                        1,
                        Some(700),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
                HostSpec::field(NODE_E),
            ],
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![16; 16]))
                    .with_activation_round(3),
            ],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_D,
                confidence: RatioPermille(600),
                loss: RatioPermille(220),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_A,
                right: NODE_E,
                forward_confidence: RatioPermille(520),
                forward_loss: RatioPermille(320),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(150),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_C,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(288),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(17),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_bridge_anti_entropy_continuity(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(130), RatioPermille(130));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "field-bridge-anti-entropy-continuity-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_D),
                        NODE_B,
                        900,
                        2,
                        3,
                        Some(850),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_D),
                        NODE_C,
                        820,
                        2,
                        4,
                        Some(760),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            28,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(560),
                forward_loss: RatioPermille(260),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(140),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(11),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(640),
                loss: RatioPermille(180),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore.clone(),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(19),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(610),
                forward_loss: RatioPermille(220),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(150),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(23),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_field_bootstrap_upgrade_window(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(100), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-bootstrap-upgrade-window-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A)
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_C),
                        NODE_B,
                        830,
                        1,
                        2,
                        Some(770),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_C),
                        NODE_D,
                        780,
                        1,
                        2,
                        Some(730),
                    )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_C)).with_activation_round(3)],
            26,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(580),
                forward_loss: RatioPermille(240),
                reverse_confidence: RatioPermille(720),
                reverse_loss: RatioPermille(150),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::ReplaceTopology {
                configuration: restore.clone(),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(15),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_D,
                right: NODE_C,
                forward_confidence: RatioPermille(560),
                forward_loss: RatioPermille(250),
                reverse_confidence: RatioPermille(730),
                reverse_loss: RatioPermille(160),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(20),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_comparison_connected_low_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = ring_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 2, RatioPermille(30), RatioPermille(20));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-connected-low-loss-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(best_effort_connected_profile()),
                comparison_host_spec(NODE_B, comparison_engine_set),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

fn build_comparison_connected_high_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(220), RatioPermille(220));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-connected-high-loss-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(repairable_connected_profile()),
                comparison_host_spec(NODE_B, comparison_engine_set),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(600),
                forward_loss: RatioPermille(280),
                reverse_confidence: RatioPermille(680),
                reverse_loss: RatioPermille(220),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(3).build()),
            },
        ),
    ]);
    (scenario, environment)
}

fn build_comparison_bridge_transition(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(140), RatioPermille(140));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-bridge-transition-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(repairable_connected_profile()),
                comparison_host_spec(NODE_B, comparison_engine_set),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(620),
                forward_loss: RatioPermille(220),
                reverse_confidence: RatioPermille(720),
                reverse_loss: RatioPermille(160),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(11),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_comparison_partial_observability_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "comparison-partial-observability-bridge-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set).with_field_bootstrap_summary(
                    field_bootstrap_summary(
                        DestinationId::Node(NODE_D),
                        NODE_B,
                        900,
                        2,
                        3,
                        Some(860),
                    ),
                ),
                comparison_host_spec(NODE_B, comparison_engine_set),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(640),
                forward_loss: RatioPermille(210),
                reverse_confidence: RatioPermille(780),
                reverse_loss: RatioPermille(130),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn build_comparison_concurrent_mixed(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = full_mesh_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 3, RatioPermille(160), RatioPermille(90));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-concurrent-mixed-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(best_effort_connected_profile()),
                comparison_host_spec(NODE_B, comparison_engine_set)
                    .with_profile(best_effort_connected_profile()),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![
                BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2),
                BoundObjective::new(NODE_B, service_objective(vec![13; 16]))
                    .with_activation_round(4),
            ],
            20,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_C,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(384),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_A, NODE_D), (NODE_D, NODE_A)],
            },
        ),
    ]);
    (scenario, environment)
}

fn build_comparison_corridor_continuity_uncertainty(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(130), RatioPermille(130));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "comparison-corridor-continuity-uncertainty-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(repairable_connected_profile())
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_D),
                        NODE_B,
                        900,
                        2,
                        3,
                        Some(850),
                    ))
                    .with_field_bootstrap_summary(field_bootstrap_summary(
                        DestinationId::Node(NODE_D),
                        NODE_C,
                        820,
                        2,
                        4,
                        Some(760),
                    )),
                comparison_host_spec(NODE_B, comparison_engine_set),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            28,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(560),
                forward_loss: RatioPermille(250),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(140),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(11),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(650),
                loss: RatioPermille(170),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: restore.clone(),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(19),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(610),
                forward_loss: RatioPermille(220),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(150),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(23),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

fn apply_overrides(
    scenario: &JacquardScenario,
    parameters: &ExperimentParameterSet,
) -> JacquardScenario {
    let hosts = scenario
        .hosts()
        .iter()
        .cloned()
        .map(|mut host| {
            if let Some(decay_window) = parameters.batman_bellman_decay_window() {
                host = host.with_batman_bellman_decay_window(decay_window);
            }
            if let Some(decay_window) = parameters.batman_classic_decay_window() {
                host = host.with_batman_classic_decay_window(decay_window);
            }
            if let Some(decay_window) = parameters.babel_decay_window() {
                host = host.with_babel_decay_window(decay_window);
            }
            if let Some(search_config) = parameters.pathway_search_config() {
                host = host.with_pathway_search_config(search_config);
            }
            if let Some(search_config) = parameters.field_search_config() {
                host = host.with_field_search_config(search_config);
            }
            host
        })
        .collect::<Vec<_>>();
    JacquardScenario::new(
        scenario.name().to_string(),
        scenario.seed(),
        scenario.deployment_profile().clone(),
        scenario.initial_configuration().clone(),
        hosts,
        scenario.bound_objectives().to_vec(),
        scenario.round_limit(),
    )
    .with_seed(scenario.seed())
}

fn comparison_topology_node(node_byte: u8, comparison_engine_set: Option<&str>) -> Node {
    match comparison_engine_set.unwrap_or("all-engines") {
        "batman-bellman" => topology::node(node_byte).batman_bellman().build(),
        "batman-classic" => topology::node(node_byte).batman_classic().build(),
        "babel" => topology::node(node_byte).babel().build(),
        "pathway" => topology::node(node_byte).pathway().build(),
        "field" => topology::node(node_byte).field().build(),
        "pathway-batman-bellman" => topology::node(node_byte)
            .pathway_and_batman_bellman()
            .build(),
        _ => topology::node(node_byte).all_engines().build(),
    }
}

fn comparison_host_spec(local_node_id: NodeId, comparison_engine_set: Option<&str>) -> HostSpec {
    match comparison_engine_set.unwrap_or("all-engines") {
        "batman-bellman" => HostSpec::batman_bellman(local_node_id),
        "batman-classic" => HostSpec::batman_classic(local_node_id),
        "babel" => HostSpec::babel(local_node_id),
        "pathway" => HostSpec::pathway(local_node_id),
        "field" => HostSpec::field(local_node_id),
        "pathway-batman-bellman" => HostSpec::pathway_and_batman_bellman(local_node_id),
        _ => HostSpec::all_engines(local_node_id),
    }
}

fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn average_u32<I>(iter: I) -> u32
where
    I: Iterator<Item = u32>,
{
    let values = iter.collect::<Vec<_>>();
    if values.is_empty() {
        return 0;
    }
    let sum = values
        .iter()
        .fold(0u64, |acc, value| acc.saturating_add(u64::from(*value)));
    u32::try_from(sum / u64::try_from(values.len()).unwrap_or(1)).unwrap_or(u32::MAX)
}

fn average_option_u32(values: &[Option<u32>]) -> Option<u32> {
    average_option_u32_from_iter(values.iter().copied())
}

fn average_option_u32_from_iter<I>(iter: I) -> Option<u32>
where
    I: Iterator<Item = Option<u32>>,
{
    let values = iter.flatten().collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(average_u32(values.into_iter()))
}

fn median_u32(values: &[u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    Some(sorted[sorted.len() / 2])
}

fn mode<I>(iter: I) -> Option<String>
where
    I: Iterator<Item = String>,
{
    let mut counts = BTreeMap::new();
    for value in iter {
        *counts.entry(value).or_insert(0u32) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(value, count)| (*count, value.clone()))
        .map(|(value, _)| value)
}

fn engine_round_counts(reduced: &ReducedReplayView) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for round in &reduced.rounds {
        let engines = round
            .active_routes
            .iter()
            .map(|route| normalized_engine_id(&route.engine_id))
            .collect::<BTreeSet<_>>();
        for engine in engines {
            *counts.entry(engine.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn dominant_engine(engine_counts: &BTreeMap<String, u32>) -> Option<String> {
    engine_counts
        .iter()
        .max_by_key(|(engine, count)| (**count, engine.as_str()))
        .map(|(engine, _)| engine.clone())
}

fn heuristic_mode_label(mode: PathwaySearchHeuristicMode) -> &'static str {
    match mode {
        PathwaySearchHeuristicMode::Zero => "zero",
        PathwaySearchHeuristicMode::HopLowerBound => "hop-lower-bound",
    }
}

fn heuristic_mode_from_str(label: &str) -> PathwaySearchHeuristicMode {
    match label {
        "hop-lower-bound" => PathwaySearchHeuristicMode::HopLowerBound,
        _ => PathwaySearchHeuristicMode::Zero,
    }
}

fn field_heuristic_mode_label(mode: FieldSearchHeuristicMode) -> &'static str {
    match mode {
        FieldSearchHeuristicMode::Zero => "zero",
        FieldSearchHeuristicMode::HopLowerBound => "hop-lower-bound",
    }
}

fn field_heuristic_mode_from_str(label: &str) -> FieldSearchHeuristicMode {
    match label {
        "hop-lower-bound" => FieldSearchHeuristicMode::HopLowerBound,
        _ => FieldSearchHeuristicMode::Zero,
    }
}

fn normalized_engine_id(engine_id: &jacquard_core::RoutingEngineId) -> &'static str {
    if engine_id == &BATMAN_BELLMAN_ENGINE_ID {
        "batman-bellman"
    } else if engine_id == &BATMAN_CLASSIC_ENGINE_ID {
        "batman-classic"
    } else if engine_id == &BABEL_ENGINE_ID {
        "babel"
    } else if engine_id == &PATHWAY_ENGINE_ID {
        "pathway"
    } else if engine_id == &FIELD_ENGINE_ID {
        "field"
    } else {
        "other"
    }
}

fn set_environment(
    topology: &mut Observation<Configuration>,
    reachable_neighbor_count: u32,
    contention_permille: RatioPermille,
    loss_permille: RatioPermille,
) {
    topology.value.environment = Environment {
        reachable_neighbor_count,
        churn_permille: RatioPermille(0),
        contention_permille,
    };
    for link in topology.value.links.values_mut() {
        link.state.loss_permille = loss_permille;
        link.state.delivery_confidence_permille = Belief::certain(
            RatioPermille(950u16.saturating_sub(loss_permille.0 / 2)),
            topology.observed_at_tick,
        );
    }
}

fn routing_observation(configuration: Configuration) -> Observation<Configuration> {
    Observation {
        value: configuration,
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn bidirectional_line_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_B, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_B), topology::link(2).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 2,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn ring_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, node_a),
            (NODE_B, node_b),
            (NODE_C, node_c),
            (NODE_D, node_d),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_B, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_B), topology::link(2).build()),
            ((NODE_C, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_C), topology::link(3).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 2,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn full_mesh_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, node_a),
            (NODE_B, node_b),
            (NODE_C, node_c),
            (NODE_D, node_d),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
            ((NODE_B, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_B), topology::link(2).build()),
            ((NODE_C, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_C), topology::link(3).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 3,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn bridge_cluster_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, node_a),
            (NODE_B, node_b),
            (NODE_C, node_c),
            (NODE_D, node_d),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_B, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_B), topology::link(2).build()),
            ((NODE_C, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_C), topology::link(3).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn fanout_service_topology4(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, node_a),
            (NODE_B, node_b),
            (NODE_C, node_c),
            (NODE_D, node_d),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 3,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn fanout_service_topology5(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
    node_e: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, node_a),
            (NODE_B, node_b),
            (NODE_C, node_c),
            (NODE_D, node_d),
            (NODE_E, node_e),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_E), topology::link(5).build()),
            ((NODE_E, NODE_A), topology::link(1).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 4,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn connected_objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn field_bootstrap_summary(
    destination: DestinationId,
    from_neighbor: NodeId,
    delivery_support: u16,
    min_hops: u8,
    max_hops: u8,
    reverse_feedback: Option<u16>,
) -> FieldBootstrapSummary {
    let observation = FieldForwardSummaryObservation::new(
        RouteEpoch(1),
        Tick(1),
        delivery_support,
        min_hops,
        max_hops,
    );
    let summary = FieldBootstrapSummary::new(destination, from_neighbor, observation);
    if let Some(reverse_feedback) = reverse_feedback {
        summary.with_reverse_feedback(reverse_feedback, Tick(1))
    } else {
        summary
    }
}

fn default_objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
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

fn service_objective(service_id: Vec<u8>) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(jacquard_core::ServiceId(service_id)),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn field_service_objective(service_id: Vec<u8>) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(jacquard_core::ServiceId(service_id)),
        service_kind: RouteServiceKind::Move,
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

fn best_effort_connected_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

fn repairable_connected_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_field_bridge_anti_entropy_continuity, build_field_service_freshness_inversion,
        build_field_service_overlap_reselection, build_field_service_publication_pressure,
        build_field_uncertain_service_fanout, local_suite, run_suite, smoke_suite,
        ExperimentParameterSet,
    };
    use crate::ReducedReplayView;
    use jacquard_core::{DestinationId, NodeId, ServiceId};

    #[test]
    fn smoke_suite_runs_and_writes_artifacts() {
        let suite = smoke_suite();
        let mut simulator = crate::JacquardSimulator::new(crate::ReferenceClientAdapter);
        let output_dir = std::env::temp_dir().join("jacquard-simulator-tuning-smoke");
        let artifacts = run_suite(&mut simulator, &suite, &output_dir).expect("run tuning suite");
        assert!(artifacts.manifest.run_count > 0);
        assert!(!artifacts.aggregates.is_empty());
        assert!(!artifacts.breakdowns.is_empty());
        match std::fs::remove_dir_all(&output_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove smoke artifact dir: {error}"),
        }
    }

    #[test]
    fn local_suite_contains_cross_regime_runs() {
        let suite = local_suite();
        assert!(suite.run_count() > smoke_suite().run_count());
    }

    #[test]
    fn field_bridge_anti_entropy_continuity_activates() {
        let parameters =
            ExperimentParameterSet::field(2, jacquard_field::FieldSearchHeuristicMode::Zero);
        let (scenario, environment) = build_field_bridge_anti_entropy_continuity(
            &parameters,
            jacquard_core::SimulationSeed(41),
        );
        let simulator = crate::JacquardSimulationHarness::new(crate::ReferenceClientAdapter);
        let (replay, _stats) = simulator
            .run(&scenario, &environment)
            .expect("run bridge anti-entropy scenario");
        if replay
            .failure_summaries
            .iter()
            .any(|summary| summary.detail.contains("objective activation failed"))
        {
            for summary in &replay.failure_summaries {
                eprintln!("failure: {}", summary.detail);
            }
        }
        let any_route_present = replay.rounds.iter().any(|round| {
            round
                .host_rounds
                .iter()
                .any(|host| !host.active_routes.is_empty())
        });
        let reduced = ReducedReplayView::from_replay(&replay);
        assert!(
            any_route_present,
            "expected field bridge anti-entropy scenario to activate at least one route"
        );
        let owner = NodeId([1; 32]);
        let destination = DestinationId::Node(NodeId([4; 32]));
        assert!(
            reduced.field_continuation_shift_count(owner, &destination) >= 1,
            "expected at least one continuation shift in bridge continuity scenario"
        );
        assert!(
            reduced
                .last_field_route_outcome(owner, &destination)
                .is_some(),
            "expected replay-visible route outcome for bridge continuity scenario"
        );
        assert!(
            reduced
                .last_field_commitment_resolution(owner, &destination)
                .is_some(),
            "expected replay-visible commitment resolution for bridge continuity scenario"
        );
    }

    #[test]
    fn field_uncertain_service_fanout_activates() {
        let parameters = ExperimentParameterSet::field(
            8,
            jacquard_field::FieldSearchHeuristicMode::HopLowerBound,
        );
        let (scenario, environment) =
            build_field_uncertain_service_fanout(&parameters, jacquard_core::SimulationSeed(41));
        let simulator = crate::JacquardSimulationHarness::new(crate::ReferenceClientAdapter);
        let (replay, _stats) = simulator
            .run(&scenario, &environment)
            .expect("run field uncertain service scenario");
        if replay
            .failure_summaries
            .iter()
            .any(|summary| summary.detail.contains("objective activation failed"))
        {
            for summary in &replay.failure_summaries {
                eprintln!("failure: {}", summary.detail);
            }
        }
        let any_route_present = replay.rounds.iter().any(|round| {
            round
                .host_rounds
                .iter()
                .any(|host| !host.active_routes.is_empty())
        });
        let reduced = ReducedReplayView::from_replay(&replay);
        assert!(
            any_route_present,
            "expected field uncertain service scenario to activate at least one route"
        );
        let owner = NodeId([1; 32]);
        let destination = DestinationId::Service(ServiceId(vec![13; 16]));
        assert!(
            reduced
                .last_field_commitment_resolution(owner, &destination)
                .is_some(),
            "expected replay-visible commitment resolution for service fanout scenario"
        );
        assert!(
            reduced
                .last_field_route_outcome(owner, &destination)
                .is_some()
                || !reduced.route_present_rounds(owner, &destination).is_empty(),
            "expected service fanout replay to preserve route lifecycle visibility"
        );
    }

    #[test]
    fn field_service_overlap_reselection_activates() {
        let parameters =
            ExperimentParameterSet::field(4, jacquard_field::FieldSearchHeuristicMode::Zero);
        let (scenario, environment) =
            build_field_service_overlap_reselection(&parameters, jacquard_core::SimulationSeed(43));
        let simulator = crate::JacquardSimulationHarness::new(crate::ReferenceClientAdapter);
        let (replay, _stats) = simulator
            .run(&scenario, &environment)
            .expect("run field overlap reselection scenario");
        let reduced = ReducedReplayView::from_replay(&replay);
        let owner = NodeId([1; 32]);
        let destination = DestinationId::Service(ServiceId(vec![14; 16]));
        assert!(
            !reduced.route_present_rounds(owner, &destination).is_empty(),
            "expected overlap reselection scenario to keep a route-visible service corridor"
        );
        assert!(
            reduced.field_continuation_shift_count(owner, &destination) >= 1,
            "expected at least one continuation shift in overlap reselection scenario"
        );
    }

    #[test]
    fn field_service_freshness_inversion_activates() {
        let parameters = ExperimentParameterSet::field_tuned(
            6,
            jacquard_field::FieldSearchHeuristicMode::HopLowerBound,
            3,
            170,
            90,
        );
        let (scenario, environment) =
            build_field_service_freshness_inversion(&parameters, jacquard_core::SimulationSeed(47));
        let simulator = crate::JacquardSimulationHarness::new(crate::ReferenceClientAdapter);
        let (replay, _stats) = simulator
            .run(&scenario, &environment)
            .expect("run field service freshness inversion scenario");
        let reduced = ReducedReplayView::from_replay(&replay);
        let owner = NodeId([1; 32]);
        let destination = DestinationId::Service(ServiceId(vec![15; 16]));
        assert!(
            !reduced.route_present_rounds(owner, &destination).is_empty(),
            "expected freshness inversion scenario to keep a route-visible service corridor"
        );
        assert!(
            reduced.field_continuation_shift_count(owner, &destination) >= 2,
            "expected repeated continuation shifts in freshness inversion scenario"
        );
    }

    #[test]
    fn field_service_publication_pressure_activates() {
        let parameters = ExperimentParameterSet::field_tuned(
            4,
            jacquard_field::FieldSearchHeuristicMode::Zero,
            1,
            140,
            180,
        );
        let (scenario, environment) = build_field_service_publication_pressure(
            &parameters,
            jacquard_core::SimulationSeed(49),
        );
        let simulator = crate::JacquardSimulationHarness::new(crate::ReferenceClientAdapter);
        let (replay, _stats) = simulator
            .run(&scenario, &environment)
            .expect("run field service publication pressure scenario");
        let reduced = ReducedReplayView::from_replay(&replay);
        let owner = NodeId([1; 32]);
        let destination = DestinationId::Service(ServiceId(vec![16; 16]));
        assert!(
            !reduced.route_present_rounds(owner, &destination).is_empty(),
            "expected publication pressure scenario to keep a route-visible service corridor"
        );
        assert!(
            reduced
                .last_field_commitment_resolution(owner, &destination)
                .is_some(),
            "expected publication pressure scenario to expose commitment resolution"
        );
    }
}
