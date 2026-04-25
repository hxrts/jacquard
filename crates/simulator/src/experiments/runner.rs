//! Suite execution and artifact writing for route-visible experiment runs.

use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use jacquard_core::{DestinationId, NodeId};
use jacquard_traits::{RoutingScenario, RoutingSimulator};
use rayon::prelude::*;

use super::{
    aggregate_runs, model_support::execute_model_case, summarize_breakdowns, summarize_run,
    ExperimentArtifacts, ExperimentError, ExperimentManifest, ExperimentModelArtifact,
    ExperimentModelCase, ExperimentRunSpec, ExperimentRunSummary, ExperimentSuite,
    JacquardHostAdapter, JacquardSimulator, ROUTE_VISIBLE_ARTIFACT_SCHEMA_VERSION,
};
use crate::SimulationExecutionLane;

struct ExecutedRun {
    summary: ExperimentRunSummary,
    model_artifacts: Vec<ExperimentModelArtifact>,
}

#[cfg(test)]
pub(super) fn execute_suite_runs_serial<A>(
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
            execute_run(&simulator, spec).map(|executed| executed.summary)
        })
        .collect()
}

fn execute_run<A>(
    simulator: &JacquardSimulator<A>,
    spec: &ExperimentRunSpec,
) -> Result<ExecutedRun, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    match spec.execution_lane {
        SimulationExecutionLane::FullStack => execute_full_stack_run(simulator, spec),
        SimulationExecutionLane::Model => execute_model_run(spec),
        SimulationExecutionLane::Equivalence => execute_equivalence_run(simulator, spec),
    }
}

fn execute_full_stack_run<A>(
    simulator: &JacquardSimulator<A>,
    spec: &ExperimentRunSpec,
) -> Result<ExecutedRun, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    trace_run_progress(&spec.run_id, "starting full-stack run");
    let (scenario, environment) = spec.materialize_world();
    let (reduced, _) = simulator
        .run_scenario_reduced(&scenario, &environment)
        .map_err(|source| ExperimentError::SimulationRun {
            run_id: spec.run_id.clone(),
            source,
        })?;
    trace_run_progress(
        &spec.run_id,
        &format!(
            "reduced replay ready: rounds={} distinct_engines={}",
            reduced.round_count,
            reduced.distinct_engine_ids.len()
        ),
    );
    let summary = summarize_run(spec, &scenario, &reduced);
    trace_run_progress(&spec.run_id, "summary complete");
    Ok(ExecutedRun {
        summary,
        model_artifacts: Vec::new(),
    })
}

fn execute_model_run(spec: &ExperimentRunSpec) -> Result<ExecutedRun, ExperimentError> {
    let execution = execute_model_case(spec)?;
    let scenario =
        spec.prepared_scenario()
            .ok_or_else(|| ExperimentError::ModelExpectationFailed {
                run_id: spec.run_id.clone(),
                detail: "model run is missing a prepared scenario".to_string(),
            })?;
    let mut summary = summarize_run(spec, scenario, &empty_reduced_view(scenario));
    summary.model_artifact_count = u32::try_from(execution.artifacts.len()).unwrap_or(u32::MAX);
    Ok(ExecutedRun {
        summary,
        model_artifacts: execution.artifacts,
    })
}

// long-block-exception: equivalence execution keeps the model-lane run,
// full-stack replay, comparison, and artifact recording in one flow.
fn execute_equivalence_run<A>(
    simulator: &JacquardSimulator<A>,
    spec: &ExperimentRunSpec,
) -> Result<ExecutedRun, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    let (scenario, environment) = spec.materialize_world();
    let reduced = match spec.model_case.as_ref() {
        Some(ExperimentModelCase::Restore(_)) => {
            let mut resumed_simulator = JacquardSimulator::new(simulator.host_adapter().clone());
            let (replay, _) = resumed_simulator
                .run_scenario(&scenario, &environment)
                .map_err(|source| ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                })?;
            let (resumed, _) = resumed_simulator.resume_replay(&replay).map_err(|source| {
                ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                }
            })?;
            crate::ReducedReplayView::from_replay(&resumed)
        }
        _ => {
            simulator
                .run_scenario_reduced(&scenario, &environment)
                .map_err(|source| ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                })?
                .0
        }
    };
    let mut execution = execute_model_case(spec)?;
    let expectation = execution.expected_visibility.take().ok_or_else(|| {
        ExperimentError::EquivalenceMismatch {
            run_id: spec.run_id.clone(),
            detail: "equivalence run is missing a visibility expectation".to_string(),
        }
    })?;
    let visible = reduced.rounds.iter().any(|round| {
        round.round_index >= expectation.visible_round
            && round.active_routes.iter().any(|route| {
                route.owner_node_id == expectation.owner_node_id
                    && route.destination == expectation.destination
                    && route.engine_id == expectation.engine_id
            })
    });
    if !visible {
        return Err(ExperimentError::EquivalenceMismatch {
            run_id: spec.run_id.clone(),
            detail: format!(
                "full-stack replay never showed route visibility for {:?} after round {}",
                expectation.destination, expectation.visible_round
            ),
        });
    }
    execution.artifacts.push(ExperimentModelArtifact {
        run_id: spec.run_id.clone(),
        suite_id: spec.suite_id.clone(),
        engine_family: spec.engine_family.clone(),
        execution_lane: spec.execution_lane.label().to_string(),
        fixture_id: format!("{}-equivalence", spec.run_id),
        artifact_kind: "equivalence-check".to_string(),
        owner_node_id: Some(node_id_hex(expectation.owner_node_id)),
        destination_node_id: destination_hex(&expectation.destination),
        next_hop_node_id: None,
        topology_epoch: None,
        candidate_count: None,
        reducer_route_entry_count: None,
        reducer_best_next_hop_count: None,
        reducer_change: None,
        backend_route_id_hex: None,
        visible_round: Some(expectation.visible_round),
        equivalence_passed: Some(true),
    });
    let mut summary = summarize_run(spec, &scenario, &reduced);
    summary.model_artifact_count = u32::try_from(execution.artifacts.len()).unwrap_or(u32::MAX);
    summary.equivalence_passed = Some(true);
    Ok(ExecutedRun {
        summary,
        model_artifacts: execution.artifacts,
    })
}

pub(super) fn execute_suite_runs_parallel<A>(
    adapter: &A,
    suite: &ExperimentSuite,
) -> Result<(Vec<ExperimentRunSummary>, Vec<ExperimentModelArtifact>), ExperimentError>
where
    A: JacquardHostAdapter + Clone + Send + Sync,
{
    let mut indexed = suite
        .runs
        .par_iter()
        .enumerate()
        .map(|(index, spec)| {
            let simulator = JacquardSimulator::new(adapter.clone());
            execute_run(&simulator, spec).map(|executed| (index, executed))
        })
        .collect::<Vec<_>>();
    let mut runs = Vec::with_capacity(indexed.len());
    let mut model_artifacts = Vec::new();
    indexed.sort_by_key(|result| match result {
        Ok((index, _)) => *index,
        Err(_) => usize::MAX,
    });
    for result in indexed {
        let (_, executed) = result?;
        runs.push(executed.summary);
        model_artifacts.extend(executed.model_artifacts);
    }
    Ok((runs, model_artifacts))
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
    let (runs, model_artifacts) = execute_suite_runs_parallel(simulator.host_adapter(), suite)?;
    let run_path = output_dir.join("runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);

    for summary in &runs {
        serde_json::to_writer(&mut writer, summary)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;

    if !model_artifacts.is_empty() {
        let model_path = output_dir.join("model_artifacts.jsonl");
        let mut model_writer = BufWriter::new(File::create(&model_path)?);
        for artifact in &model_artifacts {
            serde_json::to_writer(&mut model_writer, artifact)?;
            model_writer.write_all(b"\n")?;
        }
        model_writer.flush()?;
    }

    let aggregates = aggregate_runs(&runs);
    let breakdowns = summarize_breakdowns(&aggregates);
    let manifest = ExperimentManifest {
        schema_version: ROUTE_VISIBLE_ARTIFACT_SCHEMA_VERSION,
        suite_id: suite.suite_id.clone(),
        generated_at_unix_seconds: 0,
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        breakdown_count: u32::try_from(breakdowns.len()).unwrap_or(u32::MAX),
        model_artifact_count: u32::try_from(model_artifacts.len()).unwrap_or(u32::MAX),
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
        model_artifacts,
    })
}

fn empty_reduced_view(scenario: &crate::JacquardScenario) -> crate::ReducedReplayView {
    crate::ReducedReplayView {
        scenario_name: scenario.name().to_string(),
        round_count: 0,
        rounds: Vec::new(),
        distinct_engine_ids: Vec::new(),
        driver_status_events: Vec::new(),
        failure_summaries: Vec::new(),
    }
}

fn trace_run_progress(run_id: &str, message: &str) {
    if env::var("JACQUARD_TUNING_PROGRESS").as_deref() != Ok("1") {
        return;
    }
    eprintln!("[tuning-progress] {run_id}: {message}");
}

fn node_id_hex(node_id: NodeId) -> String {
    bytes_to_hex(&node_id.0)
}

fn destination_hex(destination: &DestinationId) -> Option<String> {
    match destination {
        DestinationId::Node(node_id) => Some(node_id_hex(*node_id)),
        DestinationId::Service(service_id) => Some(bytes_to_hex(&service_id.0)),
        DestinationId::Gateway(gateway_id) => Some(bytes_to_hex(&gateway_id.0)),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::{
        tuning_babel_equivalence_smoke_suite, tuning_babel_model_smoke_suite,
        tuning_batman_bellman_model_smoke_suite, tuning_batman_classic_model_smoke_suite,
        tuning_field_model_smoke_suite, tuning_local_stage_suite_with_seeds_and_config,
        tuning_olsrv2_model_smoke_suite, tuning_pathway_model_smoke_suite,
        tuning_scatter_model_smoke_suite, ReferenceClientAdapter,
    };

    static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn temp_output_dir(label: &str) -> std::path::PathBuf {
        let suffix = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jacquard-{label}-{}-{suffix}", std::process::id()))
    }

    fn remove_temp_output_dir(output_dir: &std::path::Path) {
        // allow-ignored-result: temp test-artifact cleanup must not hide model-lane assertion failures.
        let _ = std::fs::remove_dir_all(output_dir);
    }

    #[test]
    fn babel_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("babel-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_babel_model_smoke_suite(),
            &output_dir,
        )
        .expect("babel model smoke suite should run");

        assert!(artifacts.manifest.model_artifact_count >= 4);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model", "model", "model", "model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn babel_equivalence_smoke_suite_records_passing_equivalence() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("babel-equivalence-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_babel_equivalence_smoke_suite(),
            &output_dir,
        )
        .expect("babel equivalence smoke suite should run");

        assert!(artifacts.manifest.model_artifact_count >= 4);
        assert!(artifacts
            .runs
            .iter()
            .all(|run| run.equivalence_passed == Some(true)));

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn field_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("field-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_field_model_smoke_suite(),
            &output_dir,
        )
        .expect("field model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn pathway_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("pathway-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_pathway_model_smoke_suite(),
            &output_dir,
        )
        .expect("pathway model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn batman_bellman_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("batman-bellman-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_batman_bellman_model_smoke_suite(),
            &output_dir,
        )
        .expect("batman bellman model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn batman_classic_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("batman-classic-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_batman_classic_model_smoke_suite(),
            &output_dir,
        )
        .expect("batman classic model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn olsrv2_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("olsrv2-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_olsrv2_model_smoke_suite(),
            &output_dir,
        )
        .expect("olsrv2 model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn scatter_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("scatter-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_scatter_model_smoke_suite(),
            &output_dir,
        )
        .expect("scatter model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    #[ignore = "heavy regression for the maintained shared-corridor blocker"]
    fn shared_corridor_stage_single_seed_config_runs_serially() {
        let suite = tuning_local_stage_suite_with_seeds_and_config(
            "local-comparison-multi-flow-shared-corridor",
            &[41],
            Some("comparison-b4-2-p3-zero"),
        )
        .expect("shared-corridor stage should resolve for the maintained config");

        let runs = execute_suite_runs_serial(&ReferenceClientAdapter, &suite)
            .expect("shared-corridor stage should execute serially");

        assert_eq!(runs.len(), suite.runs.len());
    }
}
