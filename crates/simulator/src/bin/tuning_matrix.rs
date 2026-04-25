//! CLI binary for running tuning and diffusion experiment suites.

use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::Command,
};

use jacquard_simulator::{
    aggregate_diffusion_runs, aggregate_tuning_runs,
    builtin_suites::{
        diffusion_local_stage_suite, diffusion_local_suite, diffusion_smoke_suite,
        tuning_babel_equivalence_smoke_suite, tuning_babel_model_smoke_suite,
        tuning_batman_bellman_model_smoke_suite, tuning_batman_classic_model_smoke_suite,
        tuning_field_model_smoke_suite, tuning_local_stage_suite,
        tuning_local_stage_suite_with_seeds_and_config, tuning_olsrv2_model_smoke_suite,
        tuning_pathway_model_smoke_suite, tuning_scatter_model_smoke_suite, tuning_smoke_suite,
    },
    summarize_diffusion_boundaries, summarize_tuning_breakdowns, ArtifactSink, DiffusionManifest,
    DiffusionRunSummary, ExperimentManifest, ExperimentModelArtifact, ExperimentRunSummary,
    ExperimentRunner, ExperimentSuite, JacquardSimulator, ReferenceClientAdapter,
    DIFFUSION_ARTIFACT_SCHEMA_VERSION, ROUTE_VISIBLE_ARTIFACT_SCHEMA_VERSION,
};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Eq, PartialEq)]
struct CliArgs {
    suite: String,
    output_dir: Option<PathBuf>,
    jobs: Option<usize>,
    seed: Option<u64>,
    config_id: Option<String>,
}

fn parse_args() -> Result<CliArgs, String> {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from<I>(args: I) -> Result<CliArgs, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let mut suite = None;
    let mut output_dir = None;
    let mut jobs = None;
    let mut seed = None;
    let mut config_id = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --output".to_string())?;
                output_dir = Some(PathBuf::from(value));
            }
            "--jobs" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --jobs".to_string())?;
                jobs = Some(parse_positive_usize("--jobs", &value)?);
            }
            "--seed" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --seed".to_string())?;
                seed = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| format!("invalid value '{value}' for --seed"))?,
                );
            }
            "--config-id" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --config-id".to_string())?;
                config_id = Some(value);
            }
            _ if arg.starts_with("--") => {
                return Err(format!("unrecognized argument '{arg}'"));
            }
            _ => {
                if suite.replace(arg.clone()).is_some() {
                    return Err(format!("unexpected extra suite argument '{arg}'"));
                }
            }
        }
    }

    Ok(CliArgs {
        suite: suite.unwrap_or_else(|| "smoke".to_string()),
        output_dir,
        jobs,
        seed,
        config_id,
    })
}

fn parse_positive_usize(flag: &str, raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|_| format!("invalid value '{raw}' for {flag}: expected a positive integer"))?;
    if value == 0 {
        return Err(format!(
            "invalid value '{raw}' for {flag}: expected a positive integer"
        ));
    }
    Ok(value)
}

fn default_parallel_jobs() -> usize {
    let detected = std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1);
    detected.saturating_add(3).saturating_div(4).clamp(1, 4)
}

fn resolve_parallel_jobs(cli_jobs: Option<usize>) -> Result<usize, String> {
    if let Some(jobs) = cli_jobs {
        return Ok(jobs);
    }
    if let Ok(raw) = env::var("JACQUARD_TUNING_JOBS") {
        return parse_positive_usize("JACQUARD_TUNING_JOBS", &raw);
    }
    if let Ok(raw) = env::var("RAYON_NUM_THREADS") {
        return parse_positive_usize("RAYON_NUM_THREADS", &raw);
    }
    Ok(default_parallel_jobs())
}

fn configure_parallel_jobs(jobs: usize) -> Result<(), Box<dyn std::error::Error>> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build_global()?;
    println!(
        "Using up to {jobs} worker(s) for matrix execution. Override with --jobs or JACQUARD_TUNING_JOBS."
    );
    Ok(())
}

fn print_tuning_summary(artifacts: &jacquard_simulator::ExperimentArtifacts) {
    println!(
        "Tuning suite '{}' wrote {} runs, {} aggregates, {} breakdowns, and {} model artifacts to {}",
        artifacts.manifest.suite_id,
        artifacts.manifest.run_count,
        artifacts.manifest.aggregate_count,
        artifacts.manifest.breakdown_count,
        artifacts.manifest.model_artifact_count,
        artifacts.output_dir.display()
    );
}

fn print_diffusion_summary(artifacts: &jacquard_simulator::DiffusionArtifacts) {
    println!(
        "Diffusion suite '{}' wrote {} runs, {} aggregates, {} boundaries to {}",
        artifacts.manifest.suite_id,
        artifacts.manifest.run_count,
        artifacts.manifest.aggregate_count,
        artifacts.manifest.boundary_count,
        artifacts.output_dir.display()
    );
}

fn run_single_tuning_suite(
    suite: &jacquard_simulator::ExperimentSuite,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let artifacts = ExperimentRunner::default().run_tuning_suite(
        &mut simulator,
        suite,
        &ArtifactSink::directory(output_dir),
    )?;
    print_tuning_summary(&artifacts);
    remove_report_dir(output_dir);
    update_latest_symlink(output_dir);
    Ok(())
}

fn run_single_diffusion_suite(
    suite: &jacquard_simulator::DiffusionSuite,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = ExperimentRunner::default()
        .run_diffusion_suite(suite, &ArtifactSink::directory(output_dir))?;
    print_diffusion_summary(&artifacts);
    update_latest_symlink(output_dir);
    Ok(())
}

fn run_tuning_mode(
    suite: &jacquard_simulator::ExperimentSuite,
    diffusion_suite: &jacquard_simulator::DiffusionSuite,
    output_dir: &Path,
    generate_report: bool,
    jobs: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let artifacts = ExperimentRunner::default().run_tuning_suite(
        &mut simulator,
        suite,
        &ArtifactSink::directory(output_dir),
    )?;
    print_tuning_summary(&artifacts);
    let diffusion_artifacts = ExperimentRunner::default()
        .run_diffusion_suite(diffusion_suite, &ArtifactSink::directory(output_dir))?;
    print_diffusion_summary(&diffusion_artifacts);
    update_latest_symlink(output_dir);
    if generate_report {
        run_analysis_report(output_dir, jobs);
    } else {
        remove_report_dir(output_dir);
    }
    Ok(())
}

const LOCAL_TUNING_STAGE_IDS: &[&str] = &[
    "local-batman-bellman",
    "local-batman-classic",
    "local-babel",
    "local-olsrv2",
    "local-scatter",
    "local-mercator",
    "local-pathway",
    "local-comparison-stage-1",
    "local-comparison-stage-2",
    "local-comparison-multi-flow-shared-corridor",
    "local-comparison-multi-flow-asymmetric-demand",
    "local-comparison-multi-flow-detour-choice",
    "local-comparison-stale-observation-delay",
    "local-comparison-stale-asymmetric-region",
    "local-comparison-stale-recovery-window",
    "local-head-to-head-stage-1",
    "local-head-to-head-stage-2",
    "local-head-to-head-multi-flow-shared-corridor",
    "local-head-to-head-multi-flow-asymmetric-demand",
    "local-head-to-head-multi-flow-detour-choice",
    "local-head-to-head-stale-observation-delay",
    "local-head-to-head-stale-asymmetric-region",
    "local-head-to-head-stale-recovery-window",
];

const LOCAL_DIFFUSION_STAGE_IDS: &[&str] = &[
    "diffusion-local-stage-1",
    "diffusion-local-stage-2",
    "diffusion-local-stage-3",
    "diffusion-local-stage-4",
];

const LOCAL_STAGE_SEEDS: &[u64] = &[41, 43, 47, 53];
const LOCAL_COMPARISON_CONFIG_IDS: &[&str] = &[
    "comparison-b4-2-p3-zero",
    "comparison-b6-3-p4-hop-lower-bound",
];
const LOCAL_HEAD_TO_HEAD_CONFIG_IDS: &[&str] = &[
    "head-to-head-batman-bellman-1-1",
    "head-to-head-batman-classic-4-2",
    "head-to-head-babel-4-2",
    "head-to-head-olsrv2-4-2",
    "head-to-head-scatter",
    "head-to-head-mercator",
    "head-to-head-pathway-6-hop-lower-bound",
    "head-to-head-pathway-batman-b6-3-p6-hop-lower-bound",
];

fn run_local_staged_mode(output_dir: &Path, jobs: usize) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    remove_report_dir(output_dir);
    let stage_root = output_dir.join("stages");
    if stage_root.exists() {
        std::fs::remove_dir_all(&stage_root)?;
    }
    std::fs::create_dir_all(&stage_root)?;

    let mut completed_tuning_stage_dirs = Vec::new();
    for stage_id in LOCAL_TUNING_STAGE_IDS {
        completed_tuning_stage_dirs.extend(run_stage_with_optional_seed_split(
            stage_id,
            &stage_root,
            jobs,
        )?);
    }
    let mut completed_diffusion_stage_dirs = Vec::new();
    for stage_id in LOCAL_DIFFUSION_STAGE_IDS {
        completed_diffusion_stage_dirs.extend(run_stage_with_optional_seed_split(
            stage_id,
            &stage_root,
            jobs,
        )?);
    }

    merge_tuning_stage_outputs(output_dir, &completed_tuning_stage_dirs)?;
    merge_diffusion_stage_outputs(output_dir, &completed_diffusion_stage_dirs)?;
    update_latest_symlink(output_dir);
    run_analysis_report(output_dir, jobs);
    Ok(())
}

fn run_stage_with_optional_seed_split(
    stage_id: &str,
    stage_root: &Path,
    jobs: usize,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    if let Some(config_ids) = stage_seed_split_config_ids(stage_id) {
        let mut stage_dirs = Vec::new();
        for seed in LOCAL_STAGE_SEEDS {
            for config_id in config_ids {
                let stage_output_dir =
                    stage_root.join(format!("{stage_id}-seed-{seed}-{config_id}"));
                run_child_stage_suite(
                    stage_id,
                    &stage_output_dir,
                    jobs,
                    Some(*seed),
                    Some(config_id),
                )?;
                stage_dirs.push(stage_output_dir);
            }
        }
        return Ok(stage_dirs);
    }
    let stage_output_dir = stage_root.join(stage_id);
    run_child_stage_suite(stage_id, &stage_output_dir, jobs, None, None)?;
    Ok(vec![stage_output_dir])
}

fn stage_seed_split_config_ids(stage_id: &str) -> Option<&'static [&'static str]> {
    if stage_id.starts_with("local-comparison-")
        && (stage_id.contains("multi-flow") || stage_id.contains("stale-"))
    {
        return Some(LOCAL_COMPARISON_CONFIG_IDS);
    }
    if stage_id.starts_with("local-head-to-head-")
        && (stage_id.contains("multi-flow") || stage_id.contains("stale-"))
    {
        return Some(LOCAL_HEAD_TO_HEAD_CONFIG_IDS);
    }
    None
}

fn run_child_stage_suite(
    suite: &str,
    output_dir: &Path,
    jobs: usize,
    seed: Option<u64>,
    config_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let mut command = Command::new(env::current_exe()?);
    command
        .arg(suite)
        .arg("--jobs")
        .arg(jobs.to_string())
        .arg("--output")
        .arg(output_dir);
    if let Some(seed) = seed {
        command.arg("--seed").arg(seed.to_string());
    }
    if let Some(config_id) = config_id {
        command.arg("--config-id").arg(config_id);
    }
    println!(
        "Running staged suite '{suite}'{}{}...",
        seed.map_or_else(String::new, |value| format!(" for seed {value}")),
        config_id.map_or_else(String::new, |value| format!(" config {value}"))
    );
    let status = command.status()?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "staged suite '{suite}' exited with status {}",
        status.code().unwrap_or(-1)
    )
    .into())
}

fn merge_tuning_stage_outputs(
    output_dir: &Path,
    stage_dirs: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut runs = Vec::<ExperimentRunSummary>::new();
    let mut model_artifacts = Vec::<ExperimentModelArtifact>::new();
    for stage_dir in stage_dirs {
        runs.extend(read_jsonl::<ExperimentRunSummary>(
            &stage_dir.join("runs.jsonl"),
        )?);
        let model_path = stage_dir.join("model_artifacts.jsonl");
        if model_path.exists() {
            model_artifacts.extend(read_jsonl::<ExperimentModelArtifact>(&model_path)?);
        }
    }
    let aggregates = aggregate_tuning_runs(&runs);
    let breakdowns = summarize_tuning_breakdowns(&aggregates);
    let manifest = ExperimentManifest {
        schema_version: ROUTE_VISIBLE_ARTIFACT_SCHEMA_VERSION,
        suite_id: "local".to_string(),
        generated_at_unix_seconds: 0,
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        breakdown_count: u32::try_from(breakdowns.len()).unwrap_or(u32::MAX),
        model_artifact_count: u32::try_from(model_artifacts.len()).unwrap_or(u32::MAX),
    };
    write_jsonl(output_dir.join("runs.jsonl"), &runs)?;
    if model_artifacts.is_empty() {
        let model_path = output_dir.join("model_artifacts.jsonl");
        if model_path.exists() {
            std::fs::remove_file(model_path)?;
        }
    } else {
        write_jsonl(output_dir.join("model_artifacts.jsonl"), &model_artifacts)?;
    }
    write_pretty_json(output_dir.join("manifest.json"), &manifest)?;
    write_pretty_json(output_dir.join("aggregates.json"), &aggregates)?;
    write_pretty_json(output_dir.join("breakdowns.json"), &breakdowns)?;
    print_merged_tuning_summary(output_dir, &manifest);
    Ok(())
}

fn merge_diffusion_stage_outputs(
    output_dir: &Path,
    stage_dirs: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut runs = Vec::<DiffusionRunSummary>::new();
    for stage_dir in stage_dirs {
        runs.extend(read_jsonl::<DiffusionRunSummary>(
            &stage_dir.join("diffusion_runs.jsonl"),
        )?);
    }
    let aggregates = aggregate_diffusion_runs(&runs);
    let boundaries = summarize_diffusion_boundaries(&aggregates);
    let manifest = DiffusionManifest {
        schema_version: DIFFUSION_ARTIFACT_SCHEMA_VERSION,
        suite_id: "diffusion-local".to_string(),
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        boundary_count: u32::try_from(boundaries.len()).unwrap_or(u32::MAX),
    };
    write_jsonl(output_dir.join("diffusion_runs.jsonl"), &runs)?;
    write_pretty_json(output_dir.join("diffusion_manifest.json"), &manifest)?;
    write_pretty_json(output_dir.join("diffusion_aggregates.json"), &aggregates)?;
    write_pretty_json(output_dir.join("diffusion_boundaries.json"), &boundaries)?;
    print_merged_diffusion_summary(output_dir, &manifest);
    Ok(())
}

fn print_merged_tuning_summary(output_dir: &Path, manifest: &ExperimentManifest) {
    println!(
        "Tuning suite '{}' wrote {} runs, {} aggregates, {} breakdowns, and {} model artifacts to {}",
        manifest.suite_id,
        manifest.run_count,
        manifest.aggregate_count,
        manifest.breakdown_count,
        manifest.model_artifact_count,
        output_dir.display()
    );
}

fn print_merged_diffusion_summary(output_dir: &Path, manifest: &DiffusionManifest) {
    println!(
        "Diffusion suite '{}' wrote {} runs, {} aggregates, {} boundaries to {}",
        manifest.suite_id,
        manifest.run_count,
        manifest.aggregate_count,
        manifest.boundary_count,
        output_dir.display()
    );
}

fn read_jsonl<T>(path: &Path) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    T: DeserializeOwned,
{
    let reader = BufReader::new(File::open(path)?);
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        rows.push(serde_json::from_str(&line)?);
    }
    Ok(rows)
}

fn write_jsonl<T>(path: PathBuf, rows: &[T]) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
{
    let mut writer = BufWriter::new(File::create(path)?);
    for row in rows {
        serde_json::to_writer(&mut writer, row)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn write_pretty_json<T>(path: PathBuf, value: &T) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
{
    serde_json::to_writer_pretty(File::create(path)?, value)?;
    Ok(())
}

fn resolve_tuning_stage_suite(
    suite: &str,
    seed: Option<u64>,
    config_id: Option<&str>,
) -> Option<ExperimentSuite> {
    match (seed, config_id) {
        (Some(seed), config_id) => {
            tuning_local_stage_suite_with_seeds_and_config(suite, &[seed], config_id)
        }
        (None, Some(config_id)) => tuning_local_stage_suite_with_seeds_and_config(
            suite,
            LOCAL_STAGE_SEEDS,
            Some(config_id),
        ),
        (None, None) => tuning_local_stage_suite(suite),
    }
}

// long-block-exception: one CLI dispatch table keeps the maintained suite ids,
// output handling, and report-generation behavior aligned in one place.
fn run_selected_suite(
    suite: &str,
    output_dir: &Path,
    jobs: usize,
    seed: Option<u64>,
    config_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match suite {
        "diffusion-local" => run_single_diffusion_suite(&diffusion_local_suite(), output_dir),
        "diffusion-smoke" => run_single_diffusion_suite(&diffusion_smoke_suite(), output_dir),
        "local" => run_local_staged_mode(output_dir, jobs),
        "babel-model-smoke" => {
            run_single_tuning_suite(&tuning_babel_model_smoke_suite(), output_dir)
        }
        "babel-equivalence-smoke" => {
            run_single_tuning_suite(&tuning_babel_equivalence_smoke_suite(), output_dir)
        }
        "field-model-smoke" => {
            run_single_tuning_suite(&tuning_field_model_smoke_suite(), output_dir)
        }
        "batman-bellman-model-smoke" => {
            run_single_tuning_suite(&tuning_batman_bellman_model_smoke_suite(), output_dir)
        }
        "batman-classic-model-smoke" => {
            run_single_tuning_suite(&tuning_batman_classic_model_smoke_suite(), output_dir)
        }
        "olsrv2-model-smoke" => {
            run_single_tuning_suite(&tuning_olsrv2_model_smoke_suite(), output_dir)
        }
        "pathway-model-smoke" => {
            run_single_tuning_suite(&tuning_pathway_model_smoke_suite(), output_dir)
        }
        "scatter-model-smoke" => {
            run_single_tuning_suite(&tuning_scatter_model_smoke_suite(), output_dir)
        }
        "smoke" => run_tuning_mode(
            &tuning_smoke_suite(),
            &diffusion_smoke_suite(),
            output_dir,
            false,
            jobs,
        ),
        _ if resolve_tuning_stage_suite(suite, seed, config_id).is_some() => {
            let stage_suite =
                resolve_tuning_stage_suite(suite, seed, config_id).expect("checked is_some above");
            run_single_tuning_suite(&stage_suite, output_dir)
        }
        _ if diffusion_local_stage_suite(suite).is_some() => run_single_diffusion_suite(
            &diffusion_local_stage_suite(suite).expect("checked is_some above"),
            output_dir,
        ),
        _ => Err(std::io::Error::other(format!("unknown suite '{suite}'")).into()),
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let jobs = resolve_parallel_jobs(args.jobs).map_err(std::io::Error::other)?;
    configure_parallel_jobs(jobs)?;

    let output_dir = args
        .output_dir
        .unwrap_or_else(|| default_output_dir(&args.suite));
    run_selected_suite(
        &args.suite,
        &output_dir,
        jobs,
        args.seed,
        args.config_id.as_deref(),
    )
}

fn main() -> std::process::ExitCode {
    match try_main() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            let mut source = error.source();
            while let Some(cause) = source {
                eprintln!("caused by: {cause}");
                source = cause.source();
            }
            std::process::ExitCode::FAILURE
        }
    }
}

fn default_output_dir(suite: &str) -> PathBuf {
    let base = PathBuf::from(format!("artifacts/analysis/{suite}"));
    std::fs::create_dir_all(&base).expect("create analysis base output directory");
    let next_index = std::fs::read_dir(&base)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
                .filter_map(|name| name.strip_prefix("run-").map(str::to_owned))
                .filter_map(|suffix| suffix.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
                .saturating_add(1)
        })
        .unwrap_or(1);
    let run_dir = base.join(format!("run-{next_index:04}"));
    std::fs::create_dir_all(&run_dir).expect("create analysis output directory");
    run_dir
}

fn run_analysis_report(artifact_dir: &Path, jobs: usize) {
    let canonical = artifact_dir
        .canonicalize()
        .unwrap_or_else(|_| artifact_dir.to_path_buf());
    println!("Generating analysis report...");
    let jobs_str = jobs.to_string();
    // Try python3 directly first, fall back to nix develop --command.
    let status = Command::new("python3")
        .env("POLARS_MAX_THREADS", &jobs_str)
        .env("RAYON_NUM_THREADS", &jobs_str)
        .args(["-m", "analysis.report"])
        .arg(&canonical)
        .status()
        .and_then(|s| {
            if s.success() {
                Ok(s)
            } else {
                Err(std::io::Error::other("python3 failed"))
            }
        })
        .or_else(|_| {
            Command::new("nix")
                .env("POLARS_MAX_THREADS", &jobs_str)
                .env("RAYON_NUM_THREADS", &jobs_str)
                .args(["develop", "--command", "python3", "-m", "analysis.report"])
                .arg(&canonical)
                .status()
        });
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!(
            "warning: analysis report exited with status {}",
            s.code().unwrap_or(-1)
        ),
        Err(e) => eprintln!("warning: could not run analysis report: {e}"),
    }
}

fn update_latest_symlink(output_dir: &Path) {
    let Some(base) = output_dir.parent() else {
        return;
    };
    let Some(run_name) = output_dir.file_name() else {
        return;
    };
    let Some(run_name_str) = run_name.to_str() else {
        return;
    };
    if !run_name_str.starts_with("run-") {
        return;
    }
    let latest = base.join("latest");
    if latest.is_symlink() || latest.exists() {
        if let Err(error) = std::fs::remove_file(&latest) {
            let acceptable = matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::IsADirectory
            );
            if !acceptable {
                eprintln!(
                    "warning: could not remove stale latest file at {}: {error}",
                    latest.display()
                );
            }
        }
        if let Err(error) = std::fs::remove_dir_all(&latest) {
            let acceptable = matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            );
            if !acceptable {
                eprintln!(
                    "warning: could not remove stale latest directory at {}: {error}",
                    latest.display()
                );
            }
        }
    }
    #[cfg(unix)]
    {
        if let Err(error) = std::os::unix::fs::symlink(run_name, &latest) {
            eprintln!(
                "warning: could not update latest symlink at {}: {error}",
                latest.display()
            );
        }
    }
}

fn remove_report_dir(output_dir: &Path) {
    let report_dir = output_dir.join("report");
    if report_dir.exists() {
        match std::fs::remove_dir_all(&report_dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                eprintln!(
                    "warning: could not remove stale smoke report directory at {}: {error}",
                    report_dir.display()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_parallel_jobs, parse_args_from, parse_positive_usize, resolve_parallel_jobs,
        run_selected_suite, CliArgs,
    };
    use std::path::PathBuf;

    #[test]
    fn parse_args_accepts_flags_before_suite() {
        let args = parse_args_from(
            ["--jobs", "2", "--output", "artifacts/tmp", "local"]
                .into_iter()
                .map(str::to_string),
        )
        .expect("cli args should parse");

        assert_eq!(
            args,
            CliArgs {
                suite: "local".to_string(),
                output_dir: Some(PathBuf::from("artifacts/tmp")),
                jobs: Some(2),
                seed: None,
                config_id: None,
            }
        );
    }

    #[test]
    fn parse_args_defaults_to_smoke_suite() {
        let args = parse_args_from(std::iter::empty::<String>()).expect("empty args should parse");
        assert_eq!(
            args,
            CliArgs {
                suite: "smoke".to_string(),
                output_dir: None,
                jobs: None,
                seed: None,
                config_id: None,
            }
        );
    }

    #[test]
    fn parse_args_accepts_seed_and_config_filters() {
        let args = parse_args_from(
            [
                "local-comparison-multi-flow-shared-corridor",
                "--seed",
                "41",
                "--config-id",
                "comparison-b4-2-p3-zero",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("cli args should parse");

        assert_eq!(
            args,
            CliArgs {
                suite: "local-comparison-multi-flow-shared-corridor".to_string(),
                output_dir: None,
                jobs: None,
                seed: Some(41),
                config_id: Some("comparison-b4-2-p3-zero".to_string()),
            }
        );
    }

    #[test]
    fn parse_positive_usize_rejects_zero() {
        let error = parse_positive_usize("--jobs", "0").expect_err("zero should be rejected");
        assert!(error.contains("positive integer"));
    }

    #[test]
    fn resolve_parallel_jobs_uses_cli_override() {
        let jobs = resolve_parallel_jobs(Some(3)).expect("cli jobs should win");
        assert_eq!(jobs, 3);
    }

    #[test]
    fn default_parallel_jobs_stays_bounded() {
        let jobs = default_parallel_jobs();
        assert!((1..=4).contains(&jobs));
    }

    #[test]
    fn run_selected_suite_rejects_unknown_suite_ids() {
        let output_dir = std::env::temp_dir().join("jacquard-unknown-suite-rejects");
        let error = run_selected_suite("definitely-not-a-suite", &output_dir, 1, None, None)
            .expect_err("unknown suite ids should be rejected");
        assert!(error.to_string().contains("unknown suite"));
    }
}
