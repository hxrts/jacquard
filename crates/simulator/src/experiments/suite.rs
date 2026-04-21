//! Suite assembly for route-visible experiment matrices.
// long-file-exception: the maintained experiment-suite catalog keeps the full
// route-visible family matrix in one file so suite ids, fixture composition,
// and report-facing ordering stay auditable together.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::experiments::catalog::{
    batman::{BABEL_FAMILIES, BATMAN_BELLMAN_FAMILIES, BATMAN_CLASSIC_FAMILIES, OLSRV2_FAMILIES},
    comparative::{
        comparison_family_descriptors, head_to_head_family_descriptors, scatter_family_descriptors,
        ComparativeSuiteScale,
    },
    materialize_families, FamilyBuilder,
};
use crate::SimulationExecutionLane;
use jacquard_babel::{
    materialize_route_from_seed, BabelMaintenanceBestNextHopView, BabelMaintenanceInputView,
    BabelMaintenanceStateView, BabelPlannerSeed, BabelRestoredRouteView, BabelRoundInputView,
    BabelRoundRouteEntryView, BabelRoundStateView,
};
use jacquard_batman_bellman::BatmanBellmanPlannerSeed;
use jacquard_batman_classic::BatmanClassicPlannerSeed;
use jacquard_core::OperatingMode;
use jacquard_core::{
    RatioPermille, RouteDegradation, RouteLifecycleEvent, RouteMaintenanceOutcome,
    RouteMaintenanceResult, RoutingTickChange, TransportKind,
};
use jacquard_field::FieldPlannerSeed;
use jacquard_olsrv2::OlsrPlannerSeed;
use jacquard_pathway::PathwayPlannerSeed;
use jacquard_scatter::{ScatterEngineConfig, ScatterPlannerSeed};

#[must_use]
pub fn smoke_suite() -> ExperimentSuite {
    build_suite("smoke", &[41], true)
}

#[must_use]
pub fn local_suite() -> ExperimentSuite {
    build_suite("local", &[41, 43, 47, 53], false)
}

#[must_use]
pub fn local_stage_suite(stage_id: &str) -> Option<ExperimentSuite> {
    local_stage_suite_with_seeds_and_config(stage_id, &[41, 43, 47, 53], None)
}

#[must_use]
pub fn local_stage_suite_with_seeds(stage_id: &str, seeds: &[u64]) -> Option<ExperimentSuite> {
    local_stage_suite_with_seeds_and_config(stage_id, seeds, None)
}

// long-block-exception: stage-to-suite dispatch is kept inline so the maintained local matrix remains easy to audit.
#[must_use]
pub fn local_stage_suite_with_seeds_and_config(
    stage_id: &str,
    seeds: &[u64],
    config_id: Option<&str>,
) -> Option<ExperimentSuite> {
    let suite_id = "local";
    let comparative_scale = ComparativeSuiteScale::Full;
    let mut runs = if let Some(family_id) = comparison_routing_fitness_family(stage_id) {
        build_comparison_runs_for_families(suite_id, seeds, comparative_scale, &[family_id])
    } else if let Some(family_id) = head_to_head_routing_fitness_family(stage_id) {
        build_head_to_head_runs_for_families(suite_id, seeds, comparative_scale, &[family_id])
    } else {
        match stage_id {
            "local-batman-bellman" => build_batman_bellman_runs(suite_id, seeds, false),
            "local-batman-classic" => build_batman_classic_runs(suite_id, seeds, false),
            "local-babel" => build_babel_runs(suite_id, seeds, false),
            "local-olsrv2" => build_olsrv2_runs(suite_id, seeds, false),
            "local-scatter" => build_scatter_runs(suite_id, seeds, comparative_scale),
            "local-mercator" => build_mercator_runs(suite_id, seeds, comparative_scale),
            "local-pathway" => build_pathway_runs(suite_id, seeds, false),
            "local-field" => build_field_runs(suite_id, seeds, false),
            "local-comparison" => build_comparison_runs(suite_id, seeds, comparative_scale),
            "local-head-to-head" => build_head_to_head_runs(suite_id, seeds, comparative_scale),
            "local-comparison-stage-1" => build_comparison_runs_for_families(
                suite_id,
                seeds,
                comparative_scale,
                &[
                    "comparison-connected-low-loss",
                    "comparison-connected-high-loss",
                    "comparison-bridge-transition",
                    "comparison-partial-observability-bridge",
                    "comparison-concurrent-mixed",
                    "comparison-pathway-budget-boundary",
                    "comparison-corridor-continuity-uncertainty",
                ],
            ),
            "local-comparison-stage-2" => build_comparison_runs_for_families(
                suite_id,
                seeds,
                comparative_scale,
                &[
                    "comparison-medium-bridge-repair",
                    "comparison-large-core-periphery-moderate",
                    "comparison-large-core-periphery-high",
                    "comparison-large-multi-bottleneck-moderate",
                    "comparison-large-multi-bottleneck-high",
                ],
            ),
            "local-head-to-head-stage-1" => build_head_to_head_runs_for_families(
                suite_id,
                seeds,
                comparative_scale,
                &[
                    "head-to-head-connected-low-loss",
                    "head-to-head-connected-high-loss",
                    "head-to-head-bridge-transition",
                    "head-to-head-partial-observability-bridge",
                    "head-to-head-concurrent-mixed",
                    "head-to-head-corridor-continuity-uncertainty",
                ],
            ),
            "local-head-to-head-stage-2" => build_head_to_head_runs_for_families(
                suite_id,
                seeds,
                comparative_scale,
                &[
                    "head-to-head-medium-bridge-repair",
                    "head-to-head-large-core-periphery-moderate",
                    "head-to-head-large-core-periphery-high",
                    "head-to-head-large-multi-bottleneck-moderate",
                    "head-to-head-large-multi-bottleneck-high",
                ],
            ),
            _ => return None,
        }
    };
    if let Some(config_id) = config_id {
        runs.retain(|run| run.parameters.config_id == config_id);
    }
    if runs.is_empty() {
        return None;
    }
    Some(ExperimentSuite {
        suite_id: suite_id.to_string(),
        runs,
    })
}

fn unique_parameter_sets(
    parameter_sets: Vec<ExperimentParameterSet>,
) -> Vec<ExperimentParameterSet> {
    let mut unique = Vec::new();
    for parameter_set in parameter_sets {
        if unique
            .iter()
            .all(|existing: &ExperimentParameterSet| existing.config_id != parameter_set.config_id)
        {
            unique.push(parameter_set);
        }
    }
    unique
}

fn comparison_routing_fitness_family(stage_id: &str) -> Option<&'static str> {
    match stage_id {
        "local-comparison-multi-flow-shared-corridor" => {
            Some("comparison-multi-flow-shared-corridor")
        }
        "local-comparison-multi-flow-asymmetric-demand" => {
            Some("comparison-multi-flow-asymmetric-demand")
        }
        "local-comparison-multi-flow-detour-choice" => Some("comparison-multi-flow-detour-choice"),
        "local-comparison-stale-observation-delay" => Some("comparison-stale-observation-delay"),
        "local-comparison-stale-asymmetric-region" => Some("comparison-stale-asymmetric-region"),
        "local-comparison-stale-recovery-window" => Some("comparison-stale-recovery-window"),
        _ => None,
    }
}

fn head_to_head_routing_fitness_family(stage_id: &str) -> Option<&'static str> {
    match stage_id {
        "local-head-to-head-multi-flow-shared-corridor" => {
            Some("head-to-head-multi-flow-shared-corridor")
        }
        "local-head-to-head-multi-flow-asymmetric-demand" => {
            Some("head-to-head-multi-flow-asymmetric-demand")
        }
        "local-head-to-head-multi-flow-detour-choice" => {
            Some("head-to-head-multi-flow-detour-choice")
        }
        "local-head-to-head-stale-observation-delay" => {
            Some("head-to-head-stale-observation-delay")
        }
        "local-head-to-head-stale-asymmetric-region" => {
            Some("head-to-head-stale-asymmetric-region")
        }
        "local-head-to-head-stale-recovery-window" => Some("head-to-head-stale-recovery-window"),
        _ => None,
    }
}

#[must_use]
pub fn babel_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "babel-model-smoke".to_string(),
        runs: build_babel_pilot_model_runs("babel-model-smoke", SimulationSeed(91)),
    }
}

#[must_use]
pub fn babel_equivalence_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "babel-equivalence-smoke".to_string(),
        runs: build_babel_pilot_equivalence_runs("babel-equivalence-smoke", SimulationSeed(93)),
    }
}

#[must_use]
pub fn field_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "field-model-smoke".to_string(),
        runs: build_field_pilot_model_runs("field-model-smoke", SimulationSeed(95)),
    }
}

#[must_use]
pub fn pathway_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "pathway-model-smoke".to_string(),
        runs: build_pathway_pilot_model_runs("pathway-model-smoke", SimulationSeed(97)),
    }
}

#[must_use]
pub fn batman_bellman_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "batman-bellman-model-smoke".to_string(),
        runs: build_batman_bellman_pilot_model_runs(
            "batman-bellman-model-smoke",
            SimulationSeed(99),
        ),
    }
}

#[must_use]
pub fn batman_classic_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "batman-classic-model-smoke".to_string(),
        runs: build_batman_classic_pilot_model_runs(
            "batman-classic-model-smoke",
            SimulationSeed(101),
        ),
    }
}

#[must_use]
pub fn olsrv2_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "olsrv2-model-smoke".to_string(),
        runs: build_olsrv2_pilot_model_runs("olsrv2-model-smoke", SimulationSeed(103)),
    }
}

#[must_use]
pub fn scatter_model_smoke_suite() -> ExperimentSuite {
    ExperimentSuite {
        suite_id: "scatter-model-smoke".to_string(),
        runs: build_scatter_pilot_model_runs("scatter-model-smoke", SimulationSeed(105)),
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
        unique_parameter_sets(coarse.into_iter().chain(fine).collect())
    };

    let families = materialize_families(&BATMAN_BELLMAN_FAMILIES);

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
        unique_parameter_sets(coarse.into_iter().chain(fine).collect())
    };
    let families = materialize_families(&BATMAN_CLASSIC_FAMILIES);
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
        unique_parameter_sets(coarse.into_iter().chain(fine).collect())
    };
    let families = materialize_families(&BABEL_FAMILIES);
    expand_runs(suite_id, "babel", seeds, &parameter_sets, &families)
}

// long-block-exception: the OLSRv2 family catalog is kept together so the full
// sweep roster remains auditable as one tuning surface.
fn build_olsrv2_runs(suite_id: &str, seeds: &[u64], smoke: bool) -> Vec<ExperimentRunSpec> {
    let coarse = vec![
        ExperimentParameterSet::olsrv2(2, 1),
        ExperimentParameterSet::olsrv2(4, 2),
        ExperimentParameterSet::olsrv2(8, 4),
    ];
    let fine = vec![
        ExperimentParameterSet::olsrv2(4, 2),
        ExperimentParameterSet::olsrv2(6, 3),
    ];
    let parameter_sets = if smoke {
        vec![coarse[1].clone(), coarse[2].clone()]
    } else {
        unique_parameter_sets(coarse.into_iter().chain(fine).collect())
    };
    let families = materialize_families(&OLSRV2_FAMILIES);
    expand_runs(suite_id, "olsrv2", seeds, &parameter_sets, &families)
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

fn scatter_parameter_sets(scale: ComparativeSuiteScale) -> Vec<ExperimentParameterSet> {
    match scale {
        ComparativeSuiteScale::Smoke => vec![
            ExperimentParameterSet::scatter("balanced"),
            ExperimentParameterSet::scatter("degraded-network"),
        ],
        ComparativeSuiteScale::Full => vec![
            ExperimentParameterSet::scatter("balanced"),
            ExperimentParameterSet::scatter("conservative"),
            ExperimentParameterSet::scatter("degraded-network"),
        ],
    }
}

fn comparison_configs(scale: ComparativeSuiteScale) -> Vec<ExperimentParameterSet> {
    match scale {
        ComparativeSuiteScale::Smoke => vec![ExperimentParameterSet::comparison(
            4,
            2,
            3,
            PathwaySearchHeuristicMode::Zero,
        )],
        ComparativeSuiteScale::Full => vec![
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero),
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound),
        ],
    }
}

fn head_to_head_configs() -> Vec<ExperimentParameterSet> {
    vec![
        ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::BatmanBellman,
            Some((1, 1)),
            None,
            None,
        ),
        ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::BatmanClassic,
            Some((4, 2)),
            None,
            None,
        ),
        ExperimentParameterSet::head_to_head(ComparisonEngineSet::Babel, Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head(ComparisonEngineSet::OlsrV2, Some((4, 2)), None, None),
        ExperimentParameterSet::head_to_head(ComparisonEngineSet::Scatter, None, None, None),
        ExperimentParameterSet::head_to_head(ComparisonEngineSet::Mercator, None, None, None),
        ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::Pathway,
            None,
            Some((6, PathwaySearchHeuristicMode::HopLowerBound)),
            None,
        ),
        ExperimentParameterSet::head_to_head_field_low_churn(),
        ExperimentParameterSet::head_to_head(
            ComparisonEngineSet::PathwayAndBatmanBellman,
            Some((6, 3)),
            Some((6, PathwaySearchHeuristicMode::HopLowerBound)),
            None,
        ),
    ]
}

fn build_scatter_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let parameter_sets = scatter_parameter_sets(scale);
    let families = scatter_family_descriptors(scale);
    expand_runs(suite_id, "scatter", seeds, &parameter_sets, &families)
}

fn build_mercator_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let parameter_sets = vec![ExperimentParameterSet::mercator()];
    let families = head_to_head_family_descriptors(scale);
    expand_runs(suite_id, "mercator", seeds, &parameter_sets, &families)
}

fn build_comparison_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let configs = comparison_configs(scale);
    let families = comparison_family_descriptors(scale);
    expand_runs(suite_id, "comparison", seeds, &configs, &families)
}

fn build_comparison_runs_for_families(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
    family_ids: &[&str],
) -> Vec<ExperimentRunSpec> {
    let configs = comparison_configs(scale);
    let families = comparison_family_descriptors(scale)
        .into_iter()
        .filter(|(family_id, _, _)| family_ids.contains(family_id))
        .collect::<Vec<_>>();
    expand_runs(suite_id, "comparison", seeds, &configs, &families)
}

fn build_head_to_head_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let configs = head_to_head_configs();
    let families = head_to_head_family_descriptors(scale);
    expand_runs(suite_id, "head-to-head", seeds, &configs, &families)
}

fn build_head_to_head_runs_for_families(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
    family_ids: &[&str],
) -> Vec<ExperimentRunSpec> {
    let configs = head_to_head_configs();
    let families = head_to_head_family_descriptors(scale)
        .into_iter()
        .filter(|(family_id, _, _)| family_ids.contains(family_id))
        .collect::<Vec<_>>();
    expand_runs(suite_id, "head-to-head", seeds, &configs, &families)
}

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
                runs.push(ExperimentRunSpec {
                    run_id: format!(
                        "{}-{}-{}-{}",
                        suite_id, family_id, parameters.config_id, seed.0
                    ),
                    suite_id: suite_id.to_string(),
                    family_id: (*family_id).to_string(),
                    engine_family: engine_family.to_string(),
                    execution_lane: SimulationExecutionLane::FullStack,
                    seed,
                    regime: regime.clone(),
                    parameters: parameters.clone(),
                    world: ExperimentRunWorld::Generated { builder: *builder },
                    model_case: None,
                });
            }
        }
    }
    runs
}

fn build_suite(suite_id: &str, seeds: &[u64], smoke: bool) -> ExperimentSuite {
    let mut runs = Vec::new();
    let comparative_scale = if smoke {
        ComparativeSuiteScale::Smoke
    } else {
        ComparativeSuiteScale::Full
    };
    runs.extend(build_batman_bellman_runs(suite_id, seeds, smoke));
    runs.extend(build_batman_classic_runs(suite_id, seeds, smoke));
    runs.extend(build_babel_runs(suite_id, seeds, smoke));
    runs.extend(build_olsrv2_runs(suite_id, seeds, smoke));
    runs.extend(build_scatter_runs(suite_id, seeds, comparative_scale));
    runs.extend(build_mercator_runs(suite_id, seeds, comparative_scale));
    runs.extend(build_pathway_runs(suite_id, seeds, smoke));
    runs.extend(build_field_runs(suite_id, seeds, smoke));
    runs.extend(build_comparison_runs(suite_id, seeds, comparative_scale));
    runs.extend(build_head_to_head_runs(suite_id, seeds, comparative_scale));
    ExperimentSuite {
        suite_id: suite_id.to_string(),
        runs,
    }
}

// long-block-exception: the Babel pilot model suite builder keeps the
// maintained planner, round, and restore fixture matrix together in one place.
fn build_babel_pilot_model_runs(suite_id: &str, seed: SimulationSeed) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_babel_line_scenario("babel-model-line", seed, false);
    let planner_seed = BabelPlannerSeed {
        local_node_id: NODE_A,
        selected_neighbor: NODE_B,
    };
    let checkpoint_route = materialize_route_from_seed(
        NODE_A,
        &planner_seed,
        &objective,
        &profile,
        &topology,
        Tick(4),
    )
    .expect("pilot Babel planner fixture must materialize a checkpoint route");

    vec![
        ExperimentRunSpec {
            run_id: format!("{suite_id}-planner-{}", seed.0),
            suite_id: suite_id.to_string(),
            family_id: "babel-planner-decision".to_string(),
            engine_family: "babel".to_string(),
            execution_lane: SimulationExecutionLane::Model,
            seed,
            regime: regime((
                "medium-line",
                "low",
                "low",
                "none",
                "static",
                "none",
                "repairable-connected",
                12,
            )),
            parameters: ExperimentParameterSet::babel(4, 2),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario.clone()),
                environment: environment.clone(),
            },
            model_case: Some(ExperimentModelCase::Planner(PlannerModelCase::Babel(
                BabelPlannerDecisionCase {
                    fixture_id: "babel-planner-line".to_string(),
                    owner_node_id: NODE_A,
                    destination: NODE_C,
                    expected_next_hop: NODE_B,
                    expected_visible_round: Some(2),
                    objective: objective.clone(),
                    profile: profile.clone(),
                    topology: topology.clone(),
                    seed: planner_seed,
                },
            ))),
        },
        ExperimentRunSpec {
            run_id: format!("{suite_id}-round-{}", seed.0),
            suite_id: suite_id.to_string(),
            family_id: "babel-round-refresh".to_string(),
            engine_family: "babel".to_string(),
            execution_lane: SimulationExecutionLane::Model,
            seed,
            regime: regime((
                "medium-line",
                "low",
                "low",
                "none",
                "static",
                "none",
                "repairable-connected",
                12,
            )),
            parameters: ExperimentParameterSet::babel(4, 2),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario.clone()),
                environment: environment.clone(),
            },
            model_case: Some(ExperimentModelCase::Round(RoundModelCase::Babel(
                BabelRoundRefreshCase {
                    fixture_id: "babel-round-refresh-line".to_string(),
                    expected_change: RoutingTickChange::PrivateStateUpdated,
                    expected_destinations: vec![(NODE_C, NODE_B)],
                    prior_state: BabelRoundStateView {
                        route_entries: vec![BabelRoundRouteEntryView {
                            destination: NODE_C,
                            via_neighbor: NODE_B,
                            router_id: NODE_C,
                            seqno: 1,
                            metric: 512,
                            observed_at_tick: Tick(3),
                        }],
                        feasibility_entries: Vec::new(),
                    },
                    input: BabelRoundInputView {
                        topology: topology.clone(),
                        now: Tick(4),
                        local_node_id: NODE_A,
                        decay_window: jacquard_babel::DecayWindow::new(8, 4),
                    },
                },
            ))),
        },
        ExperimentRunSpec {
            run_id: format!("{suite_id}-maintenance-{}", seed.0),
            suite_id: suite_id.to_string(),
            family_id: "babel-maintenance-refresh".to_string(),
            engine_family: "babel".to_string(),
            execution_lane: SimulationExecutionLane::Model,
            seed,
            regime: regime((
                "medium-line",
                "low",
                "low",
                "none",
                "static",
                "none",
                "repairable-connected",
                12,
            )),
            parameters: ExperimentParameterSet::babel(4, 2),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario.clone()),
                environment: environment.clone(),
            },
            model_case: Some(ExperimentModelCase::Maintenance(
                MaintenanceModelCase::Babel(BabelMaintenanceCase {
                    fixture_id: "babel-maintenance-line".to_string(),
                    expected_result: RouteMaintenanceResult {
                        event: RouteLifecycleEvent::Activated,
                        outcome: RouteMaintenanceOutcome::Continued,
                    },
                    prior_state: BabelMaintenanceStateView {
                        runtime: checkpoint_route.runtime.clone(),
                        active_route: BabelRestoredRouteView {
                            destination: NODE_C,
                            next_hop: NODE_B,
                            backend_route_id: checkpoint_route
                                .identity
                                .admission
                                .backend_ref
                                .backend_route_id
                                .clone(),
                            installed_at_tick: Tick(4),
                        },
                        best_next_hop: Some(BabelMaintenanceBestNextHopView {
                            destination: NODE_C,
                            next_hop: NODE_B,
                            metric: 512,
                            tq: RatioPermille(488),
                            degradation: RouteDegradation::None,
                            transport_kind: TransportKind::WifiAware,
                            updated_at_tick: Tick(4),
                            topology_epoch: topology.value.epoch,
                            backend_route_id: checkpoint_route
                                .identity
                                .admission
                                .backend_ref
                                .backend_route_id
                                .clone(),
                        }),
                    },
                    input: BabelMaintenanceInputView { now_tick: Tick(4) },
                }),
            )),
        },
        ExperimentRunSpec {
            run_id: format!("{suite_id}-checkpoint-{}", seed.0),
            suite_id: suite_id.to_string(),
            family_id: "babel-checkpoint-restore".to_string(),
            engine_family: "babel".to_string(),
            execution_lane: SimulationExecutionLane::Model,
            seed,
            regime: regime((
                "medium-line",
                "low",
                "low",
                "none",
                "static",
                "none",
                "repairable-connected",
                12,
            )),
            parameters: ExperimentParameterSet::babel(4, 2),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario),
                environment,
            },
            model_case: Some(ExperimentModelCase::Restore(RestoreModelCase::Babel(
                Box::new(BabelCheckpointRestoreCase {
                    fixture_id: "babel-checkpoint-line".to_string(),
                    owner_node_id: NODE_A,
                    destination: NODE_C,
                    expected_next_hop: NODE_B,
                    expected_visible_round: 2,
                    route: checkpoint_route,
                }),
            ))),
        },
    ]
}

// long-block-exception: the Babel equivalence suite builder keeps the
// maintained full-stack/model comparison matrix together in one place.
fn build_babel_pilot_equivalence_runs(
    suite_id: &str,
    seed: SimulationSeed,
) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_babel_line_scenario("babel-equivalence-line", seed, true);
    let planner_seed = BabelPlannerSeed {
        local_node_id: NODE_A,
        selected_neighbor: NODE_B,
    };
    let checkpoint_route = materialize_route_from_seed(
        NODE_A,
        &planner_seed,
        &objective,
        &profile,
        &topology,
        Tick(4),
    )
    .expect("pilot Babel planner fixture must materialize a checkpoint route");

    vec![
        ExperimentRunSpec {
            run_id: format!("{suite_id}-planner-{}", seed.0),
            suite_id: suite_id.to_string(),
            family_id: "babel-planner-equivalence".to_string(),
            engine_family: "babel".to_string(),
            execution_lane: SimulationExecutionLane::Equivalence,
            seed,
            regime: regime((
                "medium-line",
                "low",
                "low",
                "none",
                "static",
                "none",
                "repairable-connected",
                12,
            )),
            parameters: ExperimentParameterSet::babel(4, 2),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario.clone()),
                environment: environment.clone(),
            },
            model_case: Some(ExperimentModelCase::Planner(PlannerModelCase::Babel(
                BabelPlannerDecisionCase {
                    fixture_id: "babel-planner-equivalence-line".to_string(),
                    owner_node_id: NODE_A,
                    destination: NODE_C,
                    expected_next_hop: NODE_B,
                    expected_visible_round: Some(2),
                    objective: objective.clone(),
                    profile: profile.clone(),
                    topology: topology.clone(),
                    seed: planner_seed,
                },
            ))),
        },
        ExperimentRunSpec {
            run_id: format!("{suite_id}-checkpoint-{}", seed.0),
            suite_id: suite_id.to_string(),
            family_id: "babel-checkpoint-equivalence".to_string(),
            engine_family: "babel".to_string(),
            execution_lane: SimulationExecutionLane::Equivalence,
            seed,
            regime: regime((
                "medium-line",
                "low",
                "low",
                "none",
                "static",
                "none",
                "repairable-connected",
                12,
            )),
            parameters: ExperimentParameterSet::babel(4, 2),
            world: ExperimentRunWorld::Prepared {
                scenario: Box::new(scenario),
                environment,
            },
            model_case: Some(ExperimentModelCase::Restore(RestoreModelCase::Babel(
                Box::new(BabelCheckpointRestoreCase {
                    fixture_id: "babel-checkpoint-equivalence-line".to_string(),
                    owner_node_id: NODE_A,
                    destination: NODE_C,
                    expected_next_hop: NODE_B,
                    expected_visible_round: 2,
                    route: checkpoint_route,
                }),
            ))),
        },
    ]
}

fn build_field_pilot_model_runs(suite_id: &str, seed: SimulationSeed) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_field_line_scenario("field-model-line", seed);
    vec![ExperimentRunSpec {
        run_id: format!("{suite_id}-planner-{}", seed.0),
        suite_id: suite_id.to_string(),
        family_id: "field-planner-decision".to_string(),
        engine_family: "field".to_string(),
        execution_lane: SimulationExecutionLane::Model,
        seed,
        regime: regime((
            "medium-line",
            "low",
            "low",
            "none",
            "static",
            "none",
            "repairable-connected",
            12,
        )),
        parameters: ExperimentParameterSet::field(4, FieldSearchHeuristicMode::Zero),
        world: ExperimentRunWorld::Prepared {
            scenario: Box::new(scenario),
            environment,
        },
        model_case: Some(ExperimentModelCase::Planner(PlannerModelCase::Field(
            FieldPlannerDecisionCase {
                fixture_id: "field-planner-line".to_string(),
                owner_node_id: NODE_A,
                destination: NODE_C,
                expected_next_hop: NODE_B,
                expected_visible_round: None,
                objective,
                profile,
                topology: topology.clone(),
                seed: FieldPlannerSeed {
                    local_node_id: NODE_A,
                    selected_neighbor: NODE_B,
                    observed_at_tick: topology.observed_at_tick,
                },
            },
        ))),
    }]
}

fn build_pathway_pilot_model_runs(suite_id: &str, seed: SimulationSeed) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_pathway_line_scenario("pathway-model-line", seed);
    vec![ExperimentRunSpec {
        run_id: format!("{suite_id}-planner-{}", seed.0),
        suite_id: suite_id.to_string(),
        family_id: "pathway-planner-decision".to_string(),
        engine_family: "pathway".to_string(),
        execution_lane: SimulationExecutionLane::Model,
        seed,
        regime: regime((
            "medium-line",
            "low",
            "low",
            "none",
            "static",
            "none",
            "repairable-connected",
            12,
        )),
        parameters: ExperimentParameterSet::pathway(4, PathwaySearchHeuristicMode::Zero),
        world: ExperimentRunWorld::Prepared {
            scenario: Box::new(scenario),
            environment,
        },
        model_case: Some(ExperimentModelCase::Planner(PlannerModelCase::Pathway(
            PathwayPlannerDecisionCase {
                fixture_id: "pathway-planner-line".to_string(),
                owner_node_id: NODE_A,
                destination: NODE_C,
                expected_next_hop: NODE_B,
                expected_visible_round: None,
                objective,
                profile,
                topology,
                seed: PathwayPlannerSeed {
                    local_node_id: NODE_A,
                },
            },
        ))),
    }]
}

fn build_batman_bellman_pilot_model_runs(
    suite_id: &str,
    seed: SimulationSeed,
) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_batman_bellman_line_scenario("batman-bellman-model-line", seed);
    vec![ExperimentRunSpec {
        run_id: format!("{suite_id}-planner-{}", seed.0),
        suite_id: suite_id.to_string(),
        family_id: "batman-bellman-planner-decision".to_string(),
        engine_family: "batman-bellman".to_string(),
        execution_lane: SimulationExecutionLane::Model,
        seed,
        regime: regime((
            "medium-line",
            "low",
            "low",
            "none",
            "static",
            "none",
            "repairable-connected",
            12,
        )),
        parameters: ExperimentParameterSet::batman_bellman(4, 2),
        world: ExperimentRunWorld::Prepared {
            scenario: Box::new(scenario),
            environment,
        },
        model_case: Some(ExperimentModelCase::Planner(
            PlannerModelCase::BatmanBellman(BatmanBellmanPlannerDecisionCase {
                fixture_id: "batman-bellman-planner-line".to_string(),
                owner_node_id: NODE_A,
                destination: NODE_C,
                expected_next_hop: NODE_B,
                expected_visible_round: None,
                objective,
                profile,
                topology,
                seed: BatmanBellmanPlannerSeed {
                    local_node_id: NODE_A,
                    selected_neighbor: NODE_B,
                },
            }),
        )),
    }]
}

fn build_batman_classic_pilot_model_runs(
    suite_id: &str,
    seed: SimulationSeed,
) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_batman_classic_line_scenario("batman-classic-model-line", seed);
    vec![ExperimentRunSpec {
        run_id: format!("{suite_id}-planner-{}", seed.0),
        suite_id: suite_id.to_string(),
        family_id: "batman-classic-planner-decision".to_string(),
        engine_family: "batman-classic".to_string(),
        execution_lane: SimulationExecutionLane::Model,
        seed,
        regime: regime((
            "medium-line",
            "low",
            "low",
            "none",
            "static",
            "none",
            "repairable-connected",
            12,
        )),
        parameters: ExperimentParameterSet::batman_classic(4, 2),
        world: ExperimentRunWorld::Prepared {
            scenario: Box::new(scenario),
            environment,
        },
        model_case: Some(ExperimentModelCase::Planner(
            PlannerModelCase::BatmanClassic(BatmanClassicPlannerDecisionCase {
                fixture_id: "batman-classic-planner-line".to_string(),
                owner_node_id: NODE_A,
                destination: NODE_C,
                expected_next_hop: NODE_B,
                expected_visible_round: None,
                objective,
                profile,
                topology,
                seed: BatmanClassicPlannerSeed {
                    local_node_id: NODE_A,
                    selected_neighbor: NODE_B,
                },
            }),
        )),
    }]
}

fn build_olsrv2_pilot_model_runs(suite_id: &str, seed: SimulationSeed) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_olsrv2_line_scenario("olsrv2-model-line", seed);
    vec![ExperimentRunSpec {
        run_id: format!("{suite_id}-planner-{}", seed.0),
        suite_id: suite_id.to_string(),
        family_id: "olsrv2-planner-decision".to_string(),
        engine_family: "olsrv2".to_string(),
        execution_lane: SimulationExecutionLane::Model,
        seed,
        regime: regime((
            "medium-line",
            "low",
            "low",
            "none",
            "static",
            "none",
            "repairable-connected",
            12,
        )),
        parameters: ExperimentParameterSet::olsrv2(4, 2),
        world: ExperimentRunWorld::Prepared {
            scenario: Box::new(scenario),
            environment,
        },
        model_case: Some(ExperimentModelCase::Planner(PlannerModelCase::Olsr(
            OlsrPlannerDecisionCase {
                fixture_id: "olsrv2-planner-line".to_string(),
                owner_node_id: NODE_A,
                destination: NODE_C,
                expected_next_hop: NODE_B,
                expected_visible_round: None,
                objective,
                profile,
                topology,
                seed: OlsrPlannerSeed {
                    local_node_id: NODE_A,
                    selected_neighbor: NODE_B,
                },
            },
        ))),
    }]
}

fn build_scatter_pilot_model_runs(suite_id: &str, seed: SimulationSeed) -> Vec<ExperimentRunSpec> {
    let (scenario, environment, objective, profile, topology) =
        pilot_scatter_line_scenario("scatter-model-line", seed);
    vec![ExperimentRunSpec {
        run_id: format!("{suite_id}-planner-{}", seed.0),
        suite_id: suite_id.to_string(),
        family_id: "scatter-planner-decision".to_string(),
        engine_family: "scatter".to_string(),
        execution_lane: SimulationExecutionLane::Model,
        seed,
        regime: regime((
            "medium-line",
            "low",
            "low",
            "none",
            "static",
            "none",
            "repairable-connected",
            12,
        )),
        parameters: ExperimentParameterSet::scatter("balanced"),
        world: ExperimentRunWorld::Prepared {
            scenario: Box::new(scenario),
            environment,
        },
        model_case: Some(ExperimentModelCase::Planner(PlannerModelCase::Scatter(
            ScatterPlannerDecisionCase {
                fixture_id: "scatter-planner-line".to_string(),
                owner_node_id: NODE_A,
                destination: NODE_C,
                expected_visible_round: None,
                objective,
                profile,
                topology: topology.clone(),
                seed: ScatterPlannerSeed {
                    local_node_id: NODE_A,
                    observed_at_tick: topology.observed_at_tick,
                    config: ScatterEngineConfig::default(),
                },
            },
        ))),
    }]
}

fn pilot_babel_line_scenario(
    name: &str,
    seed: SimulationSeed,
    with_checkpoints: bool,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::babel(NODE_A),
            HostSpec::babel(NODE_B),
            HostSpec::babel(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    let scenario = if with_checkpoints {
        scenario.with_checkpoint_interval(2)
    } else {
        scenario
    };
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

fn pilot_field_line_scenario(
    name: &str,
    seed: SimulationSeed,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

fn pilot_pathway_line_scenario(
    name: &str,
    seed: SimulationSeed,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

fn pilot_batman_bellman_line_scenario(
    name: &str,
    seed: SimulationSeed,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_bellman(NODE_A),
            HostSpec::batman_bellman(NODE_B),
            HostSpec::batman_bellman(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

fn pilot_batman_classic_line_scenario(
    name: &str,
    seed: SimulationSeed,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_classic(NODE_A),
            HostSpec::batman_classic(NODE_B),
            HostSpec::batman_classic(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

fn pilot_olsrv2_line_scenario(
    name: &str,
    seed: SimulationSeed,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).olsrv2().build(),
        topology::node(2).olsrv2().build(),
        topology::node(3).olsrv2().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::olsrv2(NODE_A),
            HostSpec::olsrv2(NODE_B),
            HostSpec::olsrv2(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

fn pilot_scatter_line_scenario(
    name: &str,
    seed: SimulationSeed,
) -> (
    JacquardScenario,
    ScriptedEnvironmentModel,
    RoutingObjective,
    SelectedRoutingParameters,
    Observation<Configuration>,
) {
    let topology = bidirectional_line_topology(
        topology::node(1).scatter().build(),
        topology::node(2).scatter().build(),
        topology::node(3).scatter().build(),
    );
    let objective = connected_objective(NODE_C);
    let scenario = JacquardScenario::new(
        name,
        seed,
        OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::scatter(NODE_A),
            HostSpec::scatter(NODE_B),
            HostSpec::scatter(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, objective.clone()).with_activation_round(2)],
        7,
    );
    (
        scenario,
        ScriptedEnvironmentModel::default(),
        objective,
        best_effort_connected_profile(),
        topology,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        local_stage_suite, local_stage_suite_with_seeds_and_config, local_suite, smoke_suite,
    };
    use crate::experiments::runner::{
        execute_suite_runs_parallel, execute_suite_runs_serial, run_suite,
    };
    use crate::{JacquardSimulator, ReferenceClientAdapter};

    #[test]
    fn route_visible_parallel_suite_matches_serial_ordered_runs() {
        let mut suite = smoke_suite();
        suite.runs.truncate(12);
        let adapter = ReferenceClientAdapter;
        let serial = execute_suite_runs_serial(&adapter, &suite)
            .expect("serial route-visible smoke suite should run");
        let parallel = execute_suite_runs_parallel(&adapter, &suite)
            .expect("parallel route-visible smoke suite should run");

        assert_eq!(serial, parallel.0);
    }

    #[test]
    fn local_suite_includes_all_routing_fitness_families() {
        let suite = local_suite();
        let family_ids = suite
            .runs
            .iter()
            .map(|run| run.family_id.as_str())
            .collect::<BTreeSet<_>>();

        for family_id in [
            "head-to-head-connected-low-loss",
            "head-to-head-large-core-periphery-moderate",
            "head-to-head-large-core-periphery-high",
            "head-to-head-medium-bridge-repair",
            "head-to-head-large-multi-bottleneck-moderate",
            "head-to-head-large-multi-bottleneck-high",
            "head-to-head-multi-flow-shared-corridor",
            "head-to-head-multi-flow-asymmetric-demand",
            "head-to-head-multi-flow-detour-choice",
            "head-to-head-stale-observation-delay",
            "head-to-head-stale-asymmetric-region",
            "head-to-head-stale-recovery-window",
        ] {
            assert!(
                family_ids.contains(family_id),
                "{family_id} missing from local suite"
            );
        }
    }

    #[test]
    fn standalone_local_stage_suites_do_not_repeat_run_ids() {
        for stage_id in [
            "local-batman-bellman",
            "local-batman-classic",
            "local-babel",
            "local-olsrv2",
            "local-mercator",
        ] {
            let suite = local_stage_suite(stage_id).expect("standalone local stage should exist");
            let run_ids = suite
                .runs
                .iter()
                .map(|run| run.run_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(
                run_ids.len(),
                suite.runs.len(),
                "{stage_id} should not emit duplicate run ids"
            );
        }
    }

    #[test]
    fn grouped_local_comparison_stage_suite_exists() {
        let suite = local_stage_suite("local-comparison")
            .expect("grouped local comparison stage should exist");
        let family_ids = suite
            .runs
            .iter()
            .map(|run| run.family_id.as_str())
            .collect::<BTreeSet<_>>();

        for family_id in [
            "comparison-connected-low-loss",
            "comparison-large-multi-bottleneck-high",
            "comparison-multi-flow-shared-corridor",
            "comparison-stale-recovery-window",
        ] {
            assert!(
                family_ids.contains(family_id),
                "{family_id} missing from grouped local comparison stage"
            );
        }
    }

    #[test]
    fn grouped_local_head_to_head_stage_suite_exists() {
        let suite = local_stage_suite("local-head-to-head")
            .expect("grouped local head-to-head stage should exist");
        let family_ids = suite
            .runs
            .iter()
            .map(|run| run.family_id.as_str())
            .collect::<BTreeSet<_>>();

        for family_id in [
            "head-to-head-connected-low-loss",
            "head-to-head-large-multi-bottleneck-high",
            "head-to-head-multi-flow-shared-corridor",
            "head-to-head-stale-recovery-window",
        ] {
            assert!(
                family_ids.contains(family_id),
                "{family_id} missing from grouped local head-to-head stage"
            );
        }
    }

    #[test]
    fn mercator_matrix_includes_maintained_route_visible_surfaces() {
        let suite = local_suite();
        let run_keys = suite
            .runs
            .iter()
            .map(|run| {
                (
                    run.engine_family.as_str(),
                    run.family_id.as_str(),
                    run.parameters.config_id.as_str(),
                )
            })
            .collect::<BTreeSet<_>>();

        for family_id in [
            "head-to-head-connected-low-loss",
            "head-to-head-large-multi-bottleneck-high",
            "head-to-head-multi-flow-shared-corridor",
            "head-to-head-stale-recovery-window",
        ] {
            assert!(
                run_keys.contains(&("mercator", family_id, "mercator")),
                "{family_id} missing from standalone mercator tuning matrix"
            );
            assert!(
                run_keys.contains(&("head-to-head", family_id, "head-to-head-mercator")),
                "{family_id} missing from head-to-head mercator matrix"
            );
        }
    }

    #[test]
    fn mercator_full_battery_staged_filters_include_mercator() {
        let standalone = local_stage_suite("local-mercator")
            .expect("standalone mercator local stage should exist");
        assert!(
            standalone.runs.iter().all(
                |run| run.engine_family == "mercator" && run.parameters.config_id == "mercator"
            ),
            "standalone mercator stage should contain only mercator rows"
        );

        let head_to_head =
            local_stage_suite("local-head-to-head").expect("head-to-head local stage should exist");
        assert!(
            head_to_head
                .runs
                .iter()
                .any(|run| run.parameters.config_id == "head-to-head-mercator"),
            "staged full battery should retain mercator head-to-head rows"
        );
    }

    #[test]
    fn mercator_generated_artifacts_include_aggregate_and_breakdown_rows() {
        let suite = local_stage_suite_with_seeds_and_config(
            "local-head-to-head-multi-flow-shared-corridor",
            &[41],
            Some("head-to-head-mercator"),
        )
        .expect("mercator head-to-head stage should exist");
        let output_dir = std::env::temp_dir().join(format!(
            "jacquard-mercator-artifacts-{}",
            std::process::id()
        ));
        // allow-ignored-result: temp artifact cleanup may target a directory that does not exist.
        let _ = std::fs::remove_dir_all(&output_dir);

        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let artifacts = run_suite(&mut simulator, &suite, &output_dir)
            .expect("mercator artifact suite should run");

        assert!(artifacts.runs.iter().any(|run| {
            run.engine_family == "head-to-head"
                && run.comparison_engine_set.as_deref() == Some("mercator")
                && run.config_id == "head-to-head-mercator"
        }));
        assert!(artifacts.aggregates.iter().any(|aggregate| {
            aggregate.engine_family == "head-to-head"
                && aggregate.comparison_engine_set.as_deref() == Some("mercator")
                && aggregate.config_id == "head-to-head-mercator"
        }));
        assert!(artifacts.breakdowns.iter().any(|breakdown| {
            breakdown.engine_family == "head-to-head"
                && breakdown.config_id == "head-to-head-mercator"
        }));
        assert!(output_dir.join("runs.jsonl").exists());
        assert!(output_dir.join("aggregates.json").exists());
        assert!(output_dir.join("breakdowns.json").exists());

        // allow-ignored-result: temp artifact cleanup must not hide assertion failures.
        let _ = std::fs::remove_dir_all(output_dir);
    }
}
