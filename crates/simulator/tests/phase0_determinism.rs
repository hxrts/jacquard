use jacquard_simulator::{
    presets, JacquardSimulator, ReducedReplayView, ReferenceClientAdapter,
    SimulationCaptureArtifact, SimulationCaptureLevel,
};

#[test]
fn capture_levels_preserve_shared_metrics_and_reduced_replay() {
    let (scenario, environment) = presets::pathway_line();
    let simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (full_artifact, full_stats) = simulator
        .run_scenario_with_capture(&scenario, &environment, SimulationCaptureLevel::FullReplay)
        .expect("full replay capture should run");
    let (reduced_artifact, reduced_stats) = simulator
        .run_scenario_with_capture(
            &scenario,
            &environment,
            SimulationCaptureLevel::ReducedReplay,
        )
        .expect("reduced replay capture should run");
    let (summary_artifact, summary_stats) = simulator
        .run_scenario_with_capture(&scenario, &environment, SimulationCaptureLevel::SummaryOnly)
        .expect("summary-only capture should run");

    let SimulationCaptureArtifact::FullReplay(full_replay) = full_artifact else {
        panic!("full capture should return a replay artifact");
    };
    let SimulationCaptureArtifact::ReducedReplay(reduced_replay) = reduced_artifact else {
        panic!("reduced capture should return a reduced replay artifact");
    };

    assert_eq!(summary_artifact, SimulationCaptureArtifact::SummaryOnly);
    assert_eq!(full_stats, reduced_stats);
    assert_eq!(reduced_stats, summary_stats);
    assert!(full_stats.executed_round_count > 0);
    assert!(full_stats.checkpoint_count > 0);
    assert_eq!(
        ReducedReplayView::from_replay(&full_replay),
        *reduced_replay
    );
}

#[test]
fn reduced_capture_is_deterministic_across_repeated_runs() {
    let (scenario, environment) = presets::mixed_line();
    let simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (first_reduced, first_stats) = simulator
        .run_scenario_reduced(&scenario, &environment)
        .expect("first reduced replay run should succeed");
    let (second_reduced, second_stats) = simulator
        .run_scenario_reduced(&scenario, &environment)
        .expect("second reduced replay run should succeed");

    assert_eq!(first_reduced, second_reduced);
    assert_eq!(first_stats, second_stats);
    assert!(!first_reduced.rounds.is_empty());
    assert!(first_stats.route_event_count > 0);
}
