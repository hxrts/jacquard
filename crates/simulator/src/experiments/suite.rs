//! Suite assembly and execution: builds run matrices and drives the simulator loop.

#![allow(clippy::wildcard_imports)]

mod comparative;

use super::*;
use comparative::{
    build_comparison_runs, build_head_to_head_runs, build_scatter_runs, ComparativeSuiteScale,
};

#[must_use]
pub fn smoke_suite() -> ExperimentSuite {
    build_suite("smoke", &[41], true)
}

#[must_use]
pub fn local_suite() -> ExperimentSuite {
    build_suite("local", &[41, 43, 47, 53], false)
}

use rayon::prelude::*;

#[cfg(test)]
fn execute_suite_runs_serial<A>(
    adapter: &A,
    suite: &ExperimentSuite,
) -> Result<Vec<ExperimentRunSummary>, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    suite
        .runs
        .iter()
        .map(|spec| {
            let simulator = JacquardSimulator::new(adapter.clone());
            let (reduced, _) = simulator
                .run_scenario_reduced(&spec.scenario, &spec.environment)
                .map_err(|source| ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                })?;
            Ok(summarize_run(spec, &reduced))
        })
        .collect()
}

fn execute_suite_runs_parallel<A>(
    adapter: &A,
    suite: &ExperimentSuite,
) -> Result<Vec<ExperimentRunSummary>, ExperimentError>
where
    A: JacquardHostAdapter + Clone + Send + Sync,
{
    let mut indexed = suite
        .runs
        .par_iter()
        .enumerate()
        .map(|(index, spec)| {
            let simulator = JacquardSimulator::new(adapter.clone());
            let reduced = simulator
                .run_scenario_reduced(&spec.scenario, &spec.environment)
                .map_err(|source| ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                })?
                .0;
            Ok::<_, ExperimentError>((index, summarize_run(spec, &reduced)))
        })
        .collect::<Vec<_>>();
    let mut runs = Vec::with_capacity(indexed.len());
    indexed.sort_by_key(|result| match result {
        Ok((index, _)) => *index,
        Err(_) => usize::MAX,
    });
    for result in indexed {
        let (_, summary) = result?;
        runs.push(summary);
    }
    Ok(runs)
}

pub fn run_suite<A>(
    simulator: &mut JacquardSimulator<A>,
    suite: &ExperimentSuite,
    output_dir: &Path,
) -> Result<ExperimentArtifacts, ExperimentError>
where
    A: JacquardHostAdapter + Clone + Send + Sync,
{
    fs::create_dir_all(output_dir)?;
    let runs = execute_suite_runs_parallel(simulator.host_adapter(), suite)?;
    let run_path = output_dir.join("runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);

    for summary in &runs {
        serde_json::to_writer(&mut writer, summary)?;
        writer.write_all(b"\n")?;
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
        (
            "batman-classic-asymmetry-relink-transition",
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
            build_batman_classic_asymmetry_relink_transition,
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
    let families: Vec<(&str, RegimeDescriptor, FamilyBuilder)> = vec![
        (
            "olsrv2-topology-propagation-latency",
            regime((
                "sparse-line",
                "moderate",
                "low",
                "none",
                "partition-recovery",
                "none",
                "connected-only",
                42,
            )),
            build_olsrv2_topology_propagation_latency,
        ),
        (
            "olsrv2-partition-recovery",
            regime((
                "sparse-line",
                "moderate",
                "low",
                "none",
                "partition-recovery",
                "none",
                "connected-only",
                38,
            )),
            build_olsrv2_partition_recovery,
        ),
        (
            "olsrv2-mpr-flooding-stability",
            regime((
                "medium-ring",
                "moderate",
                "medium",
                "none",
                "relink-and-replace",
                "none",
                "connected-only",
                46,
            )),
            build_olsrv2_mpr_flooding_stability,
        ),
        (
            "olsrv2-asymmetric-relink-transition",
            regime((
                "bridge-cluster",
                "moderate",
                "medium",
                "severe",
                "relink-and-replace",
                "none",
                "connected-only",
                52,
            )),
            build_olsrv2_asymmetric_relink_transition,
        ),
    ];
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
    use super::{execute_suite_runs_parallel, execute_suite_runs_serial, smoke_suite};
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
