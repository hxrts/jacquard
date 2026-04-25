use std::path::PathBuf;

use jacquard_core::SimulationSeed;
use jacquard_simulator::{
    presets, ArtifactSink, ExperimentRunner, ExperimentSuiteSpec, RouteVisibleRunSpec,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (scenario, environment) = presets::batman_line();
    let suite = ExperimentSuiteSpec::route_visible(
        "example-custom-suite",
        vec![RouteVisibleRunSpec::new(
            "example-batman-line",
            "example-connected-line",
            "batman-bellman",
            SimulationSeed(11),
            scenario,
            environment,
        )],
    );
    let output_dir = PathBuf::from("target/jacquard-example-custom-suite");
    let artifacts = ExperimentRunner::default()
        .run_route_visible_suite(&suite, &ArtifactSink::directory(output_dir))?;
    assert_eq!(artifacts.manifest.run_count, 1);
    assert!(artifacts.runs[0].round_count > 0);
    Ok(())
}
