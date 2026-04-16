//! Suite execution and artifact writing for route-visible experiment runs.

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use rayon::prelude::*;

use super::{
    aggregate_runs, summarize_breakdowns, summarize_run, ExperimentArtifacts, ExperimentError,
    ExperimentManifest, ExperimentRunSummary, ExperimentSuite, JacquardHostAdapter,
    JacquardSimulator,
};

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

pub(super) fn execute_suite_runs_parallel<A>(
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
