use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use jacquard_simulator::{
    diffusion_local_suite, diffusion_smoke_suite, run_diffusion_suite, run_tuning_suite,
    tuning_local_suite, tuning_smoke_suite, JacquardSimulator, ReferenceClientAdapter,
};

fn parse_args() -> (String, Option<PathBuf>) {
    let mut args = env::args().skip(1);
    let suite = args.next().unwrap_or_else(|| "smoke".to_string());
    let mut output_dir = None;

    while let Some(arg) = args.next() {
        if arg == "--output" {
            output_dir = args.next().map(PathBuf::from);
        }
    }
    (suite, output_dir)
}

fn print_tuning_summary(artifacts: &jacquard_simulator::ExperimentArtifacts) {
    println!(
        "Tuning suite '{}' wrote {} runs, {} aggregates, {} breakdowns to {}",
        artifacts.manifest.suite_id,
        artifacts.manifest.run_count,
        artifacts.manifest.aggregate_count,
        artifacts.manifest.breakdown_count,
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

fn run_single_diffusion_suite(
    suite: jacquard_simulator::DiffusionSuite,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = run_diffusion_suite(&suite, output_dir)?;
    print_diffusion_summary(&artifacts);
    update_latest_symlink(output_dir);
    Ok(())
}

fn run_tuning_mode(
    suite: jacquard_simulator::ExperimentSuite,
    diffusion_suite: jacquard_simulator::DiffusionSuite,
    output_dir: &Path,
    generate_report: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let artifacts = run_tuning_suite(&mut simulator, &suite, output_dir)?;
    print_tuning_summary(&artifacts);
    let diffusion_artifacts = run_diffusion_suite(&diffusion_suite, output_dir)?;
    print_diffusion_summary(&diffusion_artifacts);
    update_latest_symlink(output_dir);
    if generate_report {
        run_analysis_report(output_dir);
    } else {
        remove_report_dir(output_dir);
    }
    Ok(())
}

fn run_default_smoke_tuning(output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let artifacts = run_tuning_suite(&mut simulator, &tuning_smoke_suite(), output_dir)?;
    print_tuning_summary(&artifacts);
    remove_report_dir(output_dir);
    update_latest_symlink(output_dir);
    Ok(())
}

fn run_selected_suite(suite: &str, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    match suite {
        "diffusion-local" => run_single_diffusion_suite(diffusion_local_suite(), output_dir),
        "diffusion-smoke" => run_single_diffusion_suite(diffusion_smoke_suite(), output_dir),
        "local" => run_tuning_mode(
            tuning_local_suite(),
            diffusion_local_suite(),
            output_dir,
            true,
        ),
        "smoke" => run_tuning_mode(
            tuning_smoke_suite(),
            diffusion_smoke_suite(),
            output_dir,
            false,
        ),
        _ => run_default_smoke_tuning(output_dir),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (suite, output_dir) = parse_args();

    let output_dir = output_dir.unwrap_or_else(|| default_output_dir(&suite));
    run_selected_suite(&suite, &output_dir)
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

fn run_analysis_report(artifact_dir: &Path) {
    let canonical = artifact_dir
        .canonicalize()
        .unwrap_or_else(|_| artifact_dir.to_path_buf());
    println!("Generating analysis report...");
    // Try python3 directly first, fall back to nix develop --command.
    let status = Command::new("python3")
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
