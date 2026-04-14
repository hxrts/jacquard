use std::{env, path::PathBuf, process::Command};

use jacquard_simulator::{
    diffusion_local_suite, diffusion_smoke_suite, run_diffusion_suite, run_tuning_suite,
    tuning_local_suite, tuning_smoke_suite, JacquardSimulator, ReferenceClientAdapter,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let suite = args.next().unwrap_or_else(|| "smoke".to_string());
    let mut output_dir = None;

    while let Some(arg) = args.next() {
        if arg == "--output" {
            output_dir = args.next().map(PathBuf::from);
        }
    }

    let output_dir = output_dir.unwrap_or_else(|| {
        let base = PathBuf::from(format!("artifacts/analysis/{suite}"));
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let run_dir = base.join(format!("{timestamp}"));
        std::fs::create_dir_all(&run_dir).expect("create analysis output directory");
        let latest = base.join("latest");
        // Remove existing symlink or directory named "latest"
        if latest.is_symlink() || latest.exists() {
            let _ = std::fs::remove_file(&latest);
            let _ = std::fs::remove_dir_all(&latest);
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(format!("{timestamp}"), &latest).expect("create latest symlink");
        run_dir
    });

    match suite.as_str() {
        "diffusion-local" => {
            let artifacts = run_diffusion_suite(&diffusion_local_suite(), &output_dir)?;
            println!(
                "Diffusion suite '{}' wrote {} runs, {} aggregates, {} boundaries to {}",
                artifacts.manifest.suite_id,
                artifacts.manifest.run_count,
                artifacts.manifest.aggregate_count,
                artifacts.manifest.boundary_count,
                artifacts.output_dir.display()
            );
        }
        "diffusion-smoke" => {
            let artifacts = run_diffusion_suite(&diffusion_smoke_suite(), &output_dir)?;
            println!(
                "Diffusion suite '{}' wrote {} runs, {} aggregates, {} boundaries to {}",
                artifacts.manifest.suite_id,
                artifacts.manifest.run_count,
                artifacts.manifest.aggregate_count,
                artifacts.manifest.boundary_count,
                artifacts.output_dir.display()
            );
        }
        "local" | "smoke" => {
            let suite = if suite == "local" {
                tuning_local_suite()
            } else {
                tuning_smoke_suite()
            };
            let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
            let artifacts = run_tuning_suite(&mut simulator, &suite, &output_dir)?;
            println!(
                "Tuning suite '{}' wrote {} runs, {} aggregates, {} breakdowns to {}",
                artifacts.manifest.suite_id,
                artifacts.manifest.run_count,
                artifacts.manifest.aggregate_count,
                artifacts.manifest.breakdown_count,
                artifacts.output_dir.display()
            );
            let diffusion_suite = if suite.suite_id() == "local" {
                diffusion_local_suite()
            } else {
                diffusion_smoke_suite()
            };
            let diffusion_artifacts = run_diffusion_suite(&diffusion_suite, &output_dir)?;
            println!(
                "Diffusion suite '{}' wrote {} runs, {} aggregates, {} boundaries to {}",
                diffusion_artifacts.manifest.suite_id,
                diffusion_artifacts.manifest.run_count,
                diffusion_artifacts.manifest.aggregate_count,
                diffusion_artifacts.manifest.boundary_count,
                diffusion_artifacts.output_dir.display()
            );
            run_analysis_report(&output_dir);
        }
        _ => {
            let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
            let artifacts = run_tuning_suite(&mut simulator, &tuning_smoke_suite(), &output_dir)?;
            println!(
                "Tuning suite '{}' wrote {} runs, {} aggregates, {} breakdowns to {}",
                artifacts.manifest.suite_id,
                artifacts.manifest.run_count,
                artifacts.manifest.aggregate_count,
                artifacts.manifest.breakdown_count,
                artifacts.output_dir.display()
            );
            run_analysis_report(&output_dir);
        }
    }
    Ok(())
}

fn run_analysis_report(artifact_dir: &PathBuf) {
    let canonical = artifact_dir
        .canonicalize()
        .unwrap_or_else(|_| artifact_dir.clone());
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
