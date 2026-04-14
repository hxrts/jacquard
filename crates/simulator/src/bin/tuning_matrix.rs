use std::{env, path::PathBuf};

use jacquard_simulator::{
    run_tuning_suite, tuning_local_suite, tuning_smoke_suite, JacquardSimulator,
    ReferenceClientAdapter,
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

    let output_dir =
        output_dir.unwrap_or_else(|| PathBuf::from(format!("artifacts/tuning/{suite}/latest")));

    let suite = match suite.as_str() {
        "local" => tuning_local_suite(),
        _ => tuning_smoke_suite(),
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
    Ok(())
}
