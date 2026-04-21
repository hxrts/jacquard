//! Run/config/result schema: error types, node constants, and shared data structures.
// long-file-exception: this file remains the single schema surface for
// experiment ids, parameter encodings, and shared result types.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::experiments::catalog::FamilyBuilder;
use crate::SimulationExecutionLane;
use jacquard_babel::{
    BabelMaintenanceInputView, BabelMaintenanceStateView, BabelPlannerSeed, BabelRoundInputView,
    BabelRoundStateView,
};
use jacquard_batman_bellman::BatmanBellmanPlannerSeed;
use jacquard_batman_classic::BatmanClassicPlannerSeed;
use jacquard_core::{ByteCount, RoutingTickChange};
use jacquard_field::FieldPlannerSeed;
use jacquard_olsrv2::OlsrPlannerSeed;
use jacquard_pathway::PathwayPlannerSeed;
use jacquard_scatter::{
    ScatterBudgetPolicy, ScatterDecisionThresholds, ScatterEngineConfig, ScatterExpiryPolicy,
    ScatterOperationalBounds, ScatterPlannerSeed, ScatterRegimeThresholds, ScatterTransportPolicy,
};

pub(super) const NODE_A: NodeId = NodeId([1; 32]);
pub(super) const NODE_B: NodeId = NodeId([2; 32]);
pub(super) const NODE_C: NodeId = NodeId([3; 32]);
pub(super) const NODE_D: NodeId = NodeId([4; 32]);
pub(super) const NODE_E: NodeId = NodeId([5; 32]);
pub(super) const NODE_F: NodeId = NodeId([6; 32]);
pub(super) type FieldBootstrapSeed = (NodeId, u16, u8, u8, Option<u16>);

#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("simulation failed: {0}")]
    Simulation(#[from] SimulationError),
    #[error("simulation failed for {run_id}: {source}")]
    SimulationRun {
        run_id: String,
        #[source]
        source: SimulationError,
    },
    #[error("io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("model execution is not configured for {run_id}")]
    MissingModelCase { run_id: String },
    #[error("model expectation failed for {run_id}: {detail}")]
    ModelExpectationFailed { run_id: String, detail: String },
    #[error("equivalence mismatch for {run_id}: {detail}")]
    EquivalenceMismatch { run_id: String, detail: String },
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

pub(super) type RegimeFields<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    u32,
);

pub(super) fn regime(
    (density, loss, interference, asymmetry, churn, node_pressure, objective_regime, stress_score): RegimeFields<'_>,
) -> RegimeDescriptor {
    RegimeDescriptor {
        density: density.to_string(),
        loss: loss.to_string(),
        interference: interference.to_string(),
        asymmetry: asymmetry.to_string(),
        churn: churn.to_string(),
        node_pressure: node_pressure.to_string(),
        objective_regime: objective_regime.to_string(),
        stress_score,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ComparisonEngineSet {
    #[serde(rename = "all-engines")]
    AllEngines,
    #[serde(rename = "batman-bellman")]
    BatmanBellman,
    #[serde(rename = "batman-classic")]
    BatmanClassic,
    #[serde(rename = "babel")]
    Babel,
    #[serde(rename = "olsrv2")]
    OlsrV2,
    #[serde(rename = "pathway")]
    Pathway,
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "scatter")]
    Scatter,
    #[serde(rename = "pathway-batman-bellman")]
    PathwayAndBatmanBellman,
}

impl ComparisonEngineSet {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AllEngines => "all-engines",
            Self::BatmanBellman => "batman-bellman",
            Self::BatmanClassic => "batman-classic",
            Self::Babel => "babel",
            Self::OlsrV2 => "olsrv2",
            Self::Pathway => "pathway",
            Self::Field => "field",
            Self::Scatter => "scatter",
            Self::PathwayAndBatmanBellman => "pathway-batman-bellman",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentParameterSet {
    pub engine_family: String,
    pub config_id: String,
    pub comparison_engine_set: Option<ComparisonEngineSet>,
    pub batman_bellman_stale_after_ticks: Option<u32>,
    pub batman_bellman_next_refresh_within_ticks: Option<u32>,
    pub batman_classic_stale_after_ticks: Option<u32>,
    pub batman_classic_next_refresh_within_ticks: Option<u32>,
    pub babel_stale_after_ticks: Option<u32>,
    pub babel_next_refresh_within_ticks: Option<u32>,
    pub olsrv2_stale_after_ticks: Option<u32>,
    pub olsrv2_next_refresh_within_ticks: Option<u32>,
    pub pathway_query_budget: Option<usize>,
    pub pathway_heuristic_mode: Option<String>,
    pub scatter_profile_id: Option<String>,
    pub field_query_budget: Option<usize>,
    pub field_heuristic_mode: Option<String>,
    pub field_service_publication_neighbor_limit: Option<usize>,
    pub field_service_freshness_weight: Option<u16>,
    pub field_service_narrowing_bias: Option<u16>,
    pub field_node_bootstrap_support_floor: Option<u16>,
    pub field_node_bootstrap_top_mass_floor: Option<u16>,
    pub field_node_bootstrap_entropy_ceiling: Option<u16>,
    pub field_node_discovery_enabled: Option<bool>,
}

fn optional_decay_fields(decay_window: Option<(u32, u32)>) -> (Option<u32>, Option<u32>) {
    decay_window.map_or((None, None), |(stale, refresh)| {
        (Some(stale), Some(refresh))
    })
}

fn optional_pathway_search_fields(
    pathway_search: Option<(usize, PathwaySearchHeuristicMode)>,
) -> (Option<usize>, Option<String>) {
    pathway_search.map_or((None, None), |(budget, heuristic)| {
        (
            Some(budget),
            Some(heuristic_mode_label(heuristic).to_string()),
        )
    })
}

fn optional_field_search_fields(
    field_search: Option<(usize, FieldSearchHeuristicMode)>,
) -> FieldSearchFields {
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
        field_node_bootstrap_support_floor,
        field_node_bootstrap_top_mass_floor,
        field_node_bootstrap_entropy_ceiling,
        field_node_discovery_enabled,
    ) = if field_search.is_some() {
        (
            Some(1),
            Some(120),
            Some(190),
            Some(180),
            Some(180),
            Some(970),
            Some(true),
        )
    } else {
        (None, None, None, None, None, None, None)
    };
    FieldSearchFields {
        field_query_budget,
        field_heuristic_mode,
        field_service_publication_neighbor_limit,
        field_service_freshness_weight,
        field_service_narrowing_bias,
        field_node_bootstrap_support_floor,
        field_node_bootstrap_top_mass_floor,
        field_node_bootstrap_entropy_ceiling,
        field_node_discovery_enabled,
    }
}

struct FieldSearchFields {
    field_query_budget: Option<usize>,
    field_heuristic_mode: Option<String>,
    field_service_publication_neighbor_limit: Option<usize>,
    field_service_freshness_weight: Option<u16>,
    field_service_narrowing_bias: Option<u16>,
    field_node_bootstrap_support_floor: Option<u16>,
    field_node_bootstrap_top_mass_floor: Option<u16>,
    field_node_bootstrap_entropy_ceiling: Option<u16>,
    field_node_discovery_enabled: Option<bool>,
}

impl ExperimentParameterSet {
    // long-block-exception: the suffix mapping is a compact canonical encoding
    // table for all maintained head-to-head engine configurations.
    fn head_to_head_config_suffix(
        comparison_engine_set: ComparisonEngineSet,
        batman_bellman_decay_window: Option<(u32, u32)>,
        pathway_search: Option<(usize, PathwaySearchHeuristicMode)>,
        field_search: Option<(usize, FieldSearchHeuristicMode)>,
    ) -> String {
        match comparison_engine_set {
            ComparisonEngineSet::BatmanBellman => {
                let (stale_after_ticks, next_refresh_within_ticks) =
                    batman_bellman_decay_window.unwrap_or((1, 1));
                format!(
                    "batman-bellman-{}-{}",
                    stale_after_ticks, next_refresh_within_ticks
                )
            }
            ComparisonEngineSet::BatmanClassic
            | ComparisonEngineSet::Babel
            | ComparisonEngineSet::OlsrV2 => {
                let (stale_after_ticks, next_refresh_within_ticks) =
                    batman_bellman_decay_window.unwrap_or((4, 2));
                format!(
                    "{}-{}-{}",
                    comparison_engine_set.label(),
                    stale_after_ticks,
                    next_refresh_within_ticks
                )
            }
            ComparisonEngineSet::Pathway => {
                let (budget, heuristic_mode) =
                    pathway_search.unwrap_or((2, PathwaySearchHeuristicMode::Zero));
                format!(
                    "pathway-{}-{}",
                    budget,
                    heuristic_mode_label(heuristic_mode)
                )
            }
            ComparisonEngineSet::Field => {
                let (budget, heuristic_mode) =
                    field_search.unwrap_or((4, FieldSearchHeuristicMode::HopLowerBound));
                format!(
                    "field-{}-{}",
                    budget,
                    field_heuristic_mode_label(heuristic_mode)
                )
            }
            ComparisonEngineSet::PathwayAndBatmanBellman => {
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
            ComparisonEngineSet::Scatter | ComparisonEngineSet::AllEngines => {
                comparison_engine_set.label().to_string()
            }
        }
    }

    #[must_use]
    pub fn head_to_head_field_low_churn() -> Self {
        Self {
            engine_family: "head-to-head".to_string(),
            config_id: "head-to-head-field-6-zero-p1-f140-n180".to_string(),
            comparison_engine_set: Some(ComparisonEngineSet::Field),
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            scatter_profile_id: None,
            field_query_budget: Some(6),
            field_heuristic_mode: Some(
                field_heuristic_mode_label(FieldSearchHeuristicMode::Zero).to_string(),
            ),
            field_service_publication_neighbor_limit: Some(1),
            field_service_freshness_weight: Some(140),
            field_service_narrowing_bias: Some(180),
            field_node_bootstrap_support_floor: Some(180),
            field_node_bootstrap_top_mass_floor: Some(180),
            field_node_bootstrap_entropy_ceiling: Some(970),
            field_node_discovery_enabled: Some(true),
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
        }
    }

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
            scatter_profile_id: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
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
            scatter_profile_id: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
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
            scatter_profile_id: None,
            field_query_budget: Some(per_objective_query_budget),
            field_heuristic_mode: Some(field_heuristic_mode_label(heuristic_mode).to_string()),
            field_service_publication_neighbor_limit: Some(service_publication_neighbor_limit),
            field_service_freshness_weight: Some(service_freshness_weight),
            field_service_narrowing_bias: Some(service_narrowing_bias),
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
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
            olsrv2_stale_after_ticks: Some(stale_after_ticks),
            olsrv2_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            pathway_query_budget: Some(per_objective_query_budget),
            pathway_heuristic_mode: Some(heuristic_mode_label(heuristic_mode).to_string()),
            scatter_profile_id: None,
            field_query_budget: Some(per_objective_query_budget),
            field_heuristic_mode: Some(
                field_heuristic_mode_label(FieldSearchHeuristicMode::HopLowerBound).to_string(),
            ),
            field_service_publication_neighbor_limit: Some(3),
            field_service_freshness_weight: Some(100),
            field_service_narrowing_bias: Some(100),
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
        }
    }

    // long-block-exception: this constructor maps each engine-specific head-to-head parameter surface into one config id.
    #[must_use]
    pub fn head_to_head(
        comparison_engine_set: ComparisonEngineSet,
        batman_bellman_decay_window: Option<(u32, u32)>,
        pathway_search: Option<(usize, PathwaySearchHeuristicMode)>,
        field_search: Option<(usize, FieldSearchHeuristicMode)>,
    ) -> Self {
        let config_suffix = Self::head_to_head_config_suffix(
            comparison_engine_set,
            batman_bellman_decay_window,
            pathway_search,
            field_search,
        );
        let (stale_after_ticks, next_refresh_within_ticks) =
            optional_decay_fields(batman_bellman_decay_window);
        let (pathway_query_budget, pathway_heuristic_mode) =
            optional_pathway_search_fields(pathway_search);
        let FieldSearchFields {
            field_query_budget,
            field_heuristic_mode,
            field_service_publication_neighbor_limit,
            field_service_freshness_weight,
            field_service_narrowing_bias,
            field_node_bootstrap_support_floor,
            field_node_bootstrap_top_mass_floor,
            field_node_bootstrap_entropy_ceiling,
            field_node_discovery_enabled,
        } = optional_field_search_fields(field_search);
        Self {
            engine_family: "head-to-head".to_string(),
            config_id: format!("head-to-head-{}", config_suffix),
            comparison_engine_set: Some(comparison_engine_set),
            batman_bellman_stale_after_ticks: matches!(
                comparison_engine_set,
                ComparisonEngineSet::BatmanBellman | ComparisonEngineSet::PathwayAndBatmanBellman
            )
            .then_some(stale_after_ticks)
            .flatten(),
            batman_bellman_next_refresh_within_ticks: matches!(
                comparison_engine_set,
                ComparisonEngineSet::BatmanBellman | ComparisonEngineSet::PathwayAndBatmanBellman
            )
            .then_some(next_refresh_within_ticks)
            .flatten(),
            batman_classic_stale_after_ticks: (comparison_engine_set
                == ComparisonEngineSet::BatmanClassic)
                .then_some(stale_after_ticks)
                .flatten(),
            batman_classic_next_refresh_within_ticks: (comparison_engine_set
                == ComparisonEngineSet::BatmanClassic)
                .then_some(next_refresh_within_ticks)
                .flatten(),
            babel_stale_after_ticks: (comparison_engine_set == ComparisonEngineSet::Babel)
                .then_some(stale_after_ticks)
                .flatten(),
            babel_next_refresh_within_ticks: (comparison_engine_set == ComparisonEngineSet::Babel)
                .then_some(next_refresh_within_ticks)
                .flatten(),
            olsrv2_stale_after_ticks: (comparison_engine_set == ComparisonEngineSet::OlsrV2)
                .then_some(stale_after_ticks)
                .flatten(),
            olsrv2_next_refresh_within_ticks: (comparison_engine_set
                == ComparisonEngineSet::OlsrV2)
                .then_some(next_refresh_within_ticks)
                .flatten(),
            pathway_query_budget,
            pathway_heuristic_mode,
            scatter_profile_id: None,
            field_query_budget,
            field_heuristic_mode,
            field_service_publication_neighbor_limit,
            field_service_freshness_weight,
            field_service_narrowing_bias,
            field_node_bootstrap_support_floor,
            field_node_bootstrap_top_mass_floor,
            field_node_bootstrap_entropy_ceiling,
            field_node_discovery_enabled,
        }
    }

    #[must_use]
    pub fn comparison_engine_set_label(&self) -> Option<&'static str> {
        self.comparison_engine_set.map(ComparisonEngineSet::label)
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
                .with_heuristic_mode(heuristic_mode)
                .disable_replay_capture(),
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
        let config = FieldSearchConfig::default()
            .with_per_objective_query_budget(budget)
            .with_heuristic_mode(heuristic_mode)
            .disable_replay_capture()
            .with_service_publication_neighbor_limit(
                self.field_service_publication_neighbor_limit.unwrap_or(3),
            )
            .with_service_freshness_weight(self.field_service_freshness_weight.unwrap_or(100))
            .with_service_narrowing_bias(self.field_service_narrowing_bias.unwrap_or(100))
            .with_node_bootstrap_support_floor(
                self.field_node_bootstrap_support_floor.unwrap_or(220),
            )
            .with_node_bootstrap_top_mass_floor(
                self.field_node_bootstrap_top_mass_floor.unwrap_or(260),
            )
            .with_node_bootstrap_entropy_ceiling(
                self.field_node_bootstrap_entropy_ceiling.unwrap_or(950),
            );
        Some(if self.field_node_discovery_enabled.unwrap_or(false) {
            config.enable_node_discovery()
        } else {
            config.disable_node_discovery()
        })
    }

    #[must_use]
    pub fn scatter(profile_id: &str) -> Self {
        Self {
            engine_family: "scatter".to_string(),
            config_id: format!("scatter-{profile_id}"),
            comparison_engine_set: Some(ComparisonEngineSet::Scatter),
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            scatter_profile_id: Some(profile_id.to_string()),
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
        }
    }

    #[must_use]
    pub fn scatter_config(&self) -> Option<ScatterEngineConfig> {
        match self.scatter_profile_id.as_deref()? {
            "balanced" => Some(ScatterEngineConfig::default()),
            "conservative" => Some(conservative_scatter_config()),
            "degraded-network" => Some(degraded_network_scatter_config()),
            _ => None,
        }
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
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            scatter_profile_id: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
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
            olsrv2_stale_after_ticks: None,
            olsrv2_next_refresh_within_ticks: None,
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            scatter_profile_id: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
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

    #[must_use]
    pub fn olsrv2(stale_after_ticks: u32, next_refresh_within_ticks: u32) -> Self {
        Self {
            engine_family: "olsrv2".to_string(),
            config_id: format!("olsrv2-{}-{}", stale_after_ticks, next_refresh_within_ticks),
            comparison_engine_set: None,
            batman_bellman_stale_after_ticks: None,
            batman_bellman_next_refresh_within_ticks: None,
            batman_classic_stale_after_ticks: None,
            batman_classic_next_refresh_within_ticks: None,
            babel_stale_after_ticks: None,
            babel_next_refresh_within_ticks: None,
            olsrv2_stale_after_ticks: Some(stale_after_ticks),
            olsrv2_next_refresh_within_ticks: Some(next_refresh_within_ticks),
            pathway_query_budget: None,
            pathway_heuristic_mode: None,
            scatter_profile_id: None,
            field_query_budget: None,
            field_heuristic_mode: None,
            field_service_publication_neighbor_limit: None,
            field_service_freshness_weight: None,
            field_service_narrowing_bias: None,
            field_node_bootstrap_support_floor: None,
            field_node_bootstrap_top_mass_floor: None,
            field_node_bootstrap_entropy_ceiling: None,
            field_node_discovery_enabled: None,
        }
    }

    #[must_use]
    pub fn olsrv2_decay_window(&self) -> Option<OlsrV2DecayWindow> {
        match (
            self.olsrv2_stale_after_ticks,
            self.olsrv2_next_refresh_within_ticks,
        ) {
            (Some(stale), Some(refresh)) => {
                Some(OlsrV2DecayWindow::new(u64::from(stale), u64::from(refresh)))
            }
            _ => None,
        }
    }
}

fn conservative_scatter_config() -> ScatterEngineConfig {
    ScatterEngineConfig {
        expiry: ScatterExpiryPolicy {
            emergency_expiry_ms: DurationMs(10_000),
            normal_expiry_ms: DurationMs(30_000),
            background_expiry_ms: DurationMs(60_000),
        },
        budget: ScatterBudgetPolicy {
            emergency_copy_budget: 6,
            normal_copy_budget: 3,
            background_copy_budget: 1,
        },
        regime: ScatterRegimeThresholds {
            sparse_neighbor_count_max: 1,
            dense_neighbor_count_min: 5,
            constrained_hold_capacity_floor_bytes: ByteCount(2_048),
            constrained_relay_utilization_floor_permille: 650,
            bridging_diversity_floor: 2,
            history_window_ticks: 8,
        },
        decision: ScatterDecisionThresholds {
            sparse_delta_floor: 220,
            dense_delta_floor: 160,
            bridging_delta_floor: 100,
            constrained_delta_floor: 260,
            preferential_handoff_delta_floor: 300,
        },
        transport: ScatterTransportPolicy {
            min_transfer_rate_bytes_per_sec: 96,
            min_stability_horizon_ms: DurationMs(350),
            low_rate_payload_bytes_max: ByteCount(96),
        },
        bounds: ScatterOperationalBounds {
            message_count_max: 24,
            byte_count_max: ByteCount(12_288),
            hold_bytes_reserved: ByteCount(2_048),
            work_step_count_max: 6,
            validity_window_ticks: 10,
            engine_tick_within_ticks: 2,
        },
    }
}

fn degraded_network_scatter_config() -> ScatterEngineConfig {
    ScatterEngineConfig {
        expiry: ScatterExpiryPolicy {
            emergency_expiry_ms: DurationMs(25_000),
            normal_expiry_ms: DurationMs(60_000),
            background_expiry_ms: DurationMs(120_000),
        },
        budget: ScatterBudgetPolicy {
            emergency_copy_budget: 10,
            normal_copy_budget: 5,
            background_copy_budget: 3,
        },
        regime: ScatterRegimeThresholds {
            sparse_neighbor_count_max: 2,
            dense_neighbor_count_min: 3,
            constrained_hold_capacity_floor_bytes: ByteCount(384),
            constrained_relay_utilization_floor_permille: 800,
            bridging_diversity_floor: 1,
            history_window_ticks: 12,
        },
        decision: ScatterDecisionThresholds {
            sparse_delta_floor: 140,
            dense_delta_floor: 90,
            bridging_delta_floor: 40,
            constrained_delta_floor: 170,
            preferential_handoff_delta_floor: 220,
        },
        transport: ScatterTransportPolicy {
            min_transfer_rate_bytes_per_sec: 48,
            min_stability_horizon_ms: DurationMs(180),
            low_rate_payload_bytes_max: ByteCount(192),
        },
        bounds: ScatterOperationalBounds {
            message_count_max: 48,
            byte_count_max: ByteCount(24_576),
            hold_bytes_reserved: ByteCount(768),
            work_step_count_max: 10,
            validity_window_ticks: 16,
            engine_tick_within_ticks: 2,
        },
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ExperimentModelCase {
    Planner(PlannerModelCase),
    Round(RoundModelCase),
    Maintenance(MaintenanceModelCase),
    Restore(RestoreModelCase),
}

#[derive(Clone, Debug)]
pub(crate) enum PlannerModelCase {
    BatmanBellman(BatmanBellmanPlannerDecisionCase),
    BatmanClassic(BatmanClassicPlannerDecisionCase),
    Babel(BabelPlannerDecisionCase),
    Field(FieldPlannerDecisionCase),
    Olsr(OlsrPlannerDecisionCase),
    Pathway(PathwayPlannerDecisionCase),
    Scatter(ScatterPlannerDecisionCase),
}

#[derive(Clone, Debug)]
pub(crate) enum RoundModelCase {
    Babel(BabelRoundRefreshCase),
}

#[derive(Clone, Debug)]
pub(crate) enum MaintenanceModelCase {
    Babel(BabelMaintenanceCase),
}

#[derive(Clone, Debug)]
pub(crate) enum RestoreModelCase {
    Babel(Box<BabelCheckpointRestoreCase>),
}

#[derive(Clone, Debug)]
pub(crate) struct ExpectedNextHopPlannerDecisionCase<Seed> {
    pub fixture_id: String,
    pub owner_node_id: NodeId,
    pub destination: NodeId,
    pub expected_next_hop: NodeId,
    pub expected_visible_round: Option<u32>,
    pub objective: RoutingObjective,
    pub profile: SelectedRoutingParameters,
    pub topology: Observation<Configuration>,
    pub seed: Seed,
}

#[derive(Clone, Debug)]
pub(crate) struct PlannerDecisionCase<Seed> {
    pub fixture_id: String,
    pub owner_node_id: NodeId,
    pub destination: NodeId,
    pub expected_visible_round: Option<u32>,
    pub objective: RoutingObjective,
    pub profile: SelectedRoutingParameters,
    pub topology: Observation<Configuration>,
    pub seed: Seed,
}

pub(crate) type BatmanBellmanPlannerDecisionCase =
    ExpectedNextHopPlannerDecisionCase<BatmanBellmanPlannerSeed>;
pub(crate) type BatmanClassicPlannerDecisionCase =
    ExpectedNextHopPlannerDecisionCase<BatmanClassicPlannerSeed>;
pub(crate) type BabelPlannerDecisionCase = ExpectedNextHopPlannerDecisionCase<BabelPlannerSeed>;
pub(crate) type FieldPlannerDecisionCase = ExpectedNextHopPlannerDecisionCase<FieldPlannerSeed>;
pub(crate) type PathwayPlannerDecisionCase = ExpectedNextHopPlannerDecisionCase<PathwayPlannerSeed>;
pub(crate) type OlsrPlannerDecisionCase = ExpectedNextHopPlannerDecisionCase<OlsrPlannerSeed>;

#[derive(Clone, Debug)]
pub(crate) struct BabelRoundRefreshCase {
    pub fixture_id: String,
    pub expected_change: RoutingTickChange,
    pub expected_destinations: Vec<(NodeId, NodeId)>,
    pub prior_state: BabelRoundStateView,
    pub input: BabelRoundInputView,
}

#[derive(Clone, Debug)]
pub(crate) struct BabelMaintenanceCase {
    pub fixture_id: String,
    pub expected_result: jacquard_core::RouteMaintenanceResult,
    pub prior_state: BabelMaintenanceStateView,
    pub input: BabelMaintenanceInputView,
}

#[derive(Clone, Debug)]
pub(crate) struct BabelCheckpointRestoreCase {
    pub fixture_id: String,
    pub owner_node_id: NodeId,
    pub destination: NodeId,
    pub expected_next_hop: NodeId,
    pub expected_visible_round: u32,
    pub route: jacquard_core::MaterializedRoute,
}

pub(crate) type ScatterPlannerDecisionCase = PlannerDecisionCase<ScatterPlannerSeed>;

#[derive(Clone, Debug)]
pub(crate) enum ExperimentRunWorld {
    Prepared {
        scenario: Box<JacquardScenario>,
        environment: ScriptedEnvironmentModel,
    },
    Generated {
        builder: FamilyBuilder,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ExperimentRunSpec {
    pub run_id: String,
    pub suite_id: String,
    pub family_id: String,
    pub engine_family: String,
    pub execution_lane: SimulationExecutionLane,
    pub seed: SimulationSeed,
    pub regime: RegimeDescriptor,
    pub parameters: ExperimentParameterSet,
    pub world: ExperimentRunWorld,
    pub model_case: Option<ExperimentModelCase>,
}

impl ExperimentRunSpec {
    pub(crate) fn materialize_world(&self) -> (JacquardScenario, ScriptedEnvironmentModel) {
        match &self.world {
            ExperimentRunWorld::Prepared {
                scenario,
                environment,
            } => ((**scenario).clone(), environment.clone()),
            ExperimentRunWorld::Generated { builder } => builder(&self.parameters, self.seed),
        }
    }

    pub(crate) fn prepared_scenario(&self) -> Option<&JacquardScenario> {
        match &self.world {
            ExperimentRunWorld::Prepared { scenario, .. } => Some(scenario.as_ref()),
            ExperimentRunWorld::Generated { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentRunSummary {
    pub run_id: String,
    pub suite_id: String,
    pub family_id: String,
    pub scenario_name: String,
    pub engine_family: String,
    pub execution_lane: String,
    pub config_id: String,
    pub comparison_engine_set: Option<String>,
    pub batman_bellman_stale_after_ticks: Option<u32>,
    pub batman_bellman_next_refresh_within_ticks: Option<u32>,
    pub batman_classic_stale_after_ticks: Option<u32>,
    pub batman_classic_next_refresh_within_ticks: Option<u32>,
    pub babel_stale_after_ticks: Option<u32>,
    pub babel_next_refresh_within_ticks: Option<u32>,
    pub olsrv2_stale_after_ticks: Option<u32>,
    pub olsrv2_next_refresh_within_ticks: Option<u32>,
    pub pathway_query_budget: Option<usize>,
    pub pathway_heuristic_mode: Option<String>,
    pub scatter_profile_id: Option<String>,
    pub field_query_budget: Option<usize>,
    pub field_heuristic_mode: Option<String>,
    pub field_service_publication_neighbor_limit: Option<usize>,
    pub field_service_freshness_weight: Option<u16>,
    pub field_service_narrowing_bias: Option<u16>,
    pub field_node_bootstrap_support_floor: Option<u16>,
    pub field_node_bootstrap_top_mass_floor: Option<u16>,
    pub field_node_bootstrap_entropy_ceiling: Option<u16>,
    pub field_node_discovery_enabled: Option<bool>,
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
    pub route_present_total_window_permille: u32,
    pub objective_route_presence_min_permille: u32,
    pub objective_route_presence_max_permille: u32,
    pub objective_route_presence_spread: u32,
    pub objective_starvation_count: u32,
    pub concurrent_route_round_count: u32,
    pub first_materialization_round_mean: Option<u32>,
    pub first_disruption_round_mean: Option<u32>,
    pub first_loss_round_mean: Option<u32>,
    pub stale_persistence_round_mean: Option<u32>,
    pub recovery_round_mean: Option<u32>,
    pub recovery_success_permille: u32,
    pub unrecovered_after_loss_count: u32,
    pub broker_participation_permille: Option<u32>,
    pub broker_concentration_permille: Option<u32>,
    pub broker_route_churn_count: Option<u32>,
    pub active_route_hop_count_mean: Option<u32>,
    pub route_churn_count: u32,
    pub engine_handoff_count: u32,
    pub route_observation_count: u32,
    pub batman_bellman_selected_rounds: u32,
    pub batman_classic_selected_rounds: u32,
    pub babel_selected_rounds: u32,
    pub olsrv2_selected_rounds: u32,
    pub pathway_selected_rounds: u32,
    pub scatter_selected_rounds: u32,
    pub scatter_sparse_rounds: u32,
    pub scatter_dense_rounds: u32,
    pub scatter_bridging_rounds: u32,
    pub scatter_constrained_rounds: u32,
    pub scatter_replicate_rounds: u32,
    pub scatter_handoff_rounds: u32,
    pub scatter_retained_message_peak: Option<u32>,
    pub scatter_delivered_message_peak: Option<u32>,
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
    pub activation_attempt_failure_count: u32,
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
    pub model_artifact_count: u32,
    pub equivalence_passed: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentAggregateSummary {
    pub suite_id: String,
    pub family_id: String,
    pub engine_family: String,
    pub execution_lane: String,
    pub config_id: String,
    pub comparison_engine_set: Option<String>,
    pub batman_bellman_stale_after_ticks: Option<u32>,
    pub batman_bellman_next_refresh_within_ticks: Option<u32>,
    pub batman_classic_stale_after_ticks: Option<u32>,
    pub batman_classic_next_refresh_within_ticks: Option<u32>,
    pub babel_stale_after_ticks: Option<u32>,
    pub babel_next_refresh_within_ticks: Option<u32>,
    pub olsrv2_stale_after_ticks: Option<u32>,
    pub olsrv2_next_refresh_within_ticks: Option<u32>,
    pub pathway_query_budget: Option<usize>,
    pub pathway_heuristic_mode: Option<String>,
    pub scatter_profile_id: Option<String>,
    pub field_query_budget: Option<usize>,
    pub field_heuristic_mode: Option<String>,
    pub field_service_publication_neighbor_limit: Option<usize>,
    pub field_service_freshness_weight: Option<u16>,
    pub field_service_narrowing_bias: Option<u16>,
    pub field_node_bootstrap_support_floor: Option<u16>,
    pub field_node_bootstrap_top_mass_floor: Option<u16>,
    pub field_node_bootstrap_entropy_ceiling: Option<u16>,
    pub field_node_discovery_enabled: Option<bool>,
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
    pub activation_success_permille_min: u32,
    pub activation_success_permille_max: u32,
    pub activation_success_permille_spread: u32,
    pub route_present_permille_mean: u32,
    pub route_present_permille_min: u32,
    pub route_present_permille_max: u32,
    pub route_present_permille_spread: u32,
    pub route_present_total_window_permille_mean: u32,
    pub objective_route_presence_min_permille_mean: u32,
    pub objective_route_presence_max_permille_mean: u32,
    pub objective_route_presence_spread_mean: u32,
    pub objective_starvation_count_mean: u32,
    pub concurrent_route_round_count_mean: u32,
    pub first_materialization_round_mean: Option<u32>,
    pub first_disruption_round_mean: Option<u32>,
    pub first_loss_round_mean: Option<u32>,
    pub stale_persistence_round_mean: Option<u32>,
    pub recovery_round_mean: Option<u32>,
    pub recovery_success_permille_mean: u32,
    pub unrecovered_after_loss_count_mean: u32,
    pub broker_participation_permille_mean: Option<u32>,
    pub broker_concentration_permille_mean: Option<u32>,
    pub broker_route_churn_count_mean: Option<u32>,
    pub active_route_hop_count_mean: Option<u32>,
    pub route_churn_count_mean: u32,
    pub engine_handoff_count_mean: u32,
    pub route_observation_count_mean: u32,
    pub dominant_engine: Option<String>,
    pub batman_bellman_selected_rounds_mean: u32,
    pub batman_classic_selected_rounds_mean: u32,
    pub babel_selected_rounds_mean: u32,
    pub olsrv2_selected_rounds_mean: u32,
    pub pathway_selected_rounds_mean: u32,
    pub scatter_selected_rounds_mean: u32,
    pub scatter_sparse_rounds_mean: u32,
    pub scatter_dense_rounds_mean: u32,
    pub scatter_bridging_rounds_mean: u32,
    pub scatter_constrained_rounds_mean: u32,
    pub scatter_replicate_rounds_mean: u32,
    pub scatter_handoff_rounds_mean: u32,
    pub scatter_retained_message_peak_mean: Option<u32>,
    pub scatter_delivered_message_peak_mean: Option<u32>,
    pub field_selected_rounds_mean: u32,
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
    pub activation_attempt_failure_count_mean: u32,
    pub activation_failure_count_mean: u32,
    pub persistent_degraded_count_mean: u32,
    pub other_failure_count_mean: u32,
    pub cascade_partition_count_mean: u32,
    pub intrinsic_limit_count_mean: u32,
    pub model_artifact_count_mean: u32,
    pub equivalence_pass_count: u32,
    pub acceptable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentBreakdownSummary {
    pub suite_id: String,
    pub engine_family: String,
    pub execution_lane: String,
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
    pub model_artifact_count: u32,
}

#[derive(Clone, Debug)]
pub struct ExperimentSuite {
    pub(super) suite_id: String,
    pub(super) runs: Vec<ExperimentRunSpec>,
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
    pub model_artifacts: Vec<ExperimentModelArtifact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExperimentModelArtifact {
    pub run_id: String,
    pub suite_id: String,
    pub engine_family: String,
    pub execution_lane: String,
    pub fixture_id: String,
    pub artifact_kind: String,
    pub owner_node_id: Option<String>,
    pub destination_node_id: Option<String>,
    pub next_hop_node_id: Option<String>,
    pub topology_epoch: Option<u64>,
    pub candidate_count: Option<u32>,
    pub reducer_route_entry_count: Option<u32>,
    pub reducer_best_next_hop_count: Option<u32>,
    pub reducer_change: Option<String>,
    pub backend_route_id_hex: Option<String>,
    pub visible_round: Option<u32>,
    pub equivalence_passed: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_to_head_decay_knobs_are_written_to_the_selected_engine_family() {
        let classic = ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::BatmanClassic,
            Some((5, 2)),
            None,
            None,
        );
        assert_eq!(classic.batman_classic_stale_after_ticks, Some(5));
        assert_eq!(classic.batman_classic_next_refresh_within_ticks, Some(2));
        assert_eq!(classic.batman_bellman_stale_after_ticks, None);
        assert_eq!(classic.babel_stale_after_ticks, None);
        assert_eq!(classic.olsrv2_stale_after_ticks, None);

        let babel = ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::Babel,
            Some((6, 3)),
            None,
            None,
        );
        assert_eq!(babel.babel_stale_after_ticks, Some(6));
        assert_eq!(babel.babel_next_refresh_within_ticks, Some(3));
        assert_eq!(babel.batman_bellman_stale_after_ticks, None);
        assert_eq!(babel.batman_classic_stale_after_ticks, None);
        assert_eq!(babel.olsrv2_stale_after_ticks, None);

        let olsrv2 = ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::OlsrV2,
            Some((7, 4)),
            None,
            None,
        );
        assert_eq!(olsrv2.olsrv2_stale_after_ticks, Some(7));
        assert_eq!(olsrv2.olsrv2_next_refresh_within_ticks, Some(4));
        assert_eq!(olsrv2.batman_bellman_stale_after_ticks, None);
        assert_eq!(olsrv2.batman_classic_stale_after_ticks, None);
        assert_eq!(olsrv2.babel_stale_after_ticks, None);
    }

    #[test]
    fn head_to_head_hybrid_keeps_batman_bellman_and_pathway_knobs() {
        let hybrid = ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::PathwayAndBatmanBellman,
            Some((6, 3)),
            Some((8, PathwaySearchHeuristicMode::HopLowerBound)),
            None,
        );

        assert_eq!(hybrid.batman_bellman_stale_after_ticks, Some(6));
        assert_eq!(hybrid.batman_bellman_next_refresh_within_ticks, Some(3));
        assert_eq!(hybrid.pathway_query_budget, Some(8));
        assert_eq!(
            hybrid.pathway_heuristic_mode.as_deref(),
            Some("hop-lower-bound")
        );
        assert_eq!(hybrid.batman_classic_stale_after_ticks, None);
        assert_eq!(hybrid.babel_stale_after_ticks, None);
        assert_eq!(hybrid.olsrv2_stale_after_ticks, None);
    }
}
