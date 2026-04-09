use jacquard_simulator::{presets, JacquardSimulator, ReferenceClientAdapter};
use jacquard_traits::{RoutingScenario, RoutingSimulator};

#[test]
fn pathway_scenario_runs_and_replays_deterministically() {
    let (scenario, environment) = presets::pathway_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run pathway scenario");
    let (resumed, resumed_stats) = simulator
        .resume_replay(&replay)
        .expect("resume pathway replay");

    assert!(stats.executed_round_count > 0);
    assert_eq!(
        stats.executed_round_count,
        resumed_stats.executed_round_count
    );
    assert_eq!(
        replay.rounds.last().map(|round| &round.topology),
        resumed.rounds.last().map(|round| &round.topology)
    );
    assert_eq!(replay.telltale_native, resumed.telltale_native);
    assert!(!replay.checkpoints.is_empty());
    assert!(replay.telltale_native.is_some());
    assert!(!replay.failure_summaries.is_empty());
}

#[test]
fn batman_scenario_runs_through_deterministic_round_lane() {
    let (scenario, environment) = presets::batman_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run batman scenario");

    assert!(stats.executed_round_count > 0);
    assert!(!replay.rounds.is_empty());
    assert!(replay
        .rounds
        .iter()
        .flat_map(|round| round.host_rounds.iter())
        .any(|round| matches!(
            round.status,
            jacquard_simulator::HostRoundStatus::Advanced { .. }
        )));
}

#[test]
fn mixed_engine_scenario_runs_under_shared_harness() {
    let (scenario, environment) = presets::mixed_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run mixed scenario");

    assert!(stats.route_event_count > 0);
    assert_eq!(replay.scenario.name(), "mixed-line");
    assert!(
        replay
            .rounds
            .iter()
            .flat_map(|round| round.environment_artifacts.iter())
            .count()
            > 0
    );
}
