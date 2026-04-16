//! Suite assembly for route-visible experiment matrices.

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

#[must_use]
pub fn smoke_suite() -> ExperimentSuite {
    build_suite("smoke", &[41], true)
}

#[must_use]
pub fn local_suite() -> ExperimentSuite {
    build_suite("local", &[41, 43, 47, 53], false)
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
        coarse.into_iter().chain(fine).collect()
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
        coarse.into_iter().chain(fine).collect()
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
        coarse.into_iter().chain(fine).collect()
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

fn build_comparison_runs(
    suite_id: &str,
    seeds: &[u64],
    scale: ComparativeSuiteScale,
) -> Vec<ExperimentRunSpec> {
    let configs = comparison_configs(scale);
    let families = comparison_family_descriptors(scale);
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
    runs.extend(build_pathway_runs(suite_id, seeds, smoke));
    runs.extend(build_field_runs(suite_id, seeds, smoke));
    runs.extend(build_comparison_runs(suite_id, seeds, comparative_scale));
    runs.extend(build_head_to_head_runs(suite_id, seeds, comparative_scale));
    ExperimentSuite {
        suite_id: suite_id.to_string(),
        runs,
    }
}

#[cfg(test)]
mod tests {
    use super::smoke_suite;
    use crate::experiments::runner::{execute_suite_runs_parallel, execute_suite_runs_serial};
    use crate::ReferenceClientAdapter;

    #[test]
    fn route_visible_parallel_suite_matches_serial_ordered_runs() {
        let suite = smoke_suite();
        let adapter = ReferenceClientAdapter;
        let serial = execute_suite_runs_serial(&adapter, &suite)
            .expect("serial route-visible smoke suite should run");
        let parallel = execute_suite_runs_parallel(&adapter, &suite)
            .expect("parallel route-visible smoke suite should run");

        assert_eq!(serial, parallel);
    }
}
