use jacquard_simulator::{
    presets, JacquardSimulator, ReducedReplayView, ReferenceClientAdapter, ScenarioAssertions,
};
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
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_absent(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .evaluate(&reduced)
        .expect("pathway replay assertions");
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
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_distinct_engine_count(1)
        .evaluate(&reduced)
        .expect("batman replay assertions");
}

#[test]
fn babel_scenario_runs_through_deterministic_round_lane() {
    let (scenario, environment) = presets::babel_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run babel scenario");

    assert!(stats.executed_round_count > 0);
    assert!(!replay.rounds.is_empty());
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_distinct_engine_count(1)
        .evaluate(&reduced)
        .expect("babel replay assertions");
}

#[test]
fn olsrv2_scenario_runs_through_deterministic_round_lane() {
    let (scenario, environment) = presets::olsrv2_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run olsrv2 scenario");

    assert!(stats.executed_round_count > 0);
    assert!(!replay.rounds.is_empty());
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_distinct_engine_count(1)
        .evaluate(&reduced)
        .expect("olsrv2 replay assertions");
}

#[test]
fn batman_classic_scenario_runs_through_deterministic_round_lane() {
    let (scenario, environment) = presets::batman_classic_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run batman-classic scenario");

    assert!(stats.executed_round_count > 0);
    assert!(!replay.rounds.is_empty());
    // batman-classic has no bootstrap shortcut: routes emerge only after the
    // OGM receive window has filled AND echo-based bidirectionality has been
    // confirmed. A successful run without panics is sufficient for the smoke
    // test; the tuning experiment families verify materialization properties.
}

#[test]
fn field_scenario_runs() {
    let (scenario, environment) = presets::field_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run field scenario");

    assert!(stats.executed_round_count > 0);
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .evaluate(&reduced)
        .expect("field replay assertions");
}

#[test]
fn all_engines_scenario_runs() {
    let (scenario, environment) = presets::all_engines_line();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run all-engines scenario");

    assert!(stats.executed_round_count > 0);
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_route_materialized(
            jacquard_core::NodeId([2; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32])),
        )
        .evaluate(&reduced)
        .expect("all-engines replay assertions");
}

#[test]
fn all_engines_ring_scenario_runs() {
    let (scenario, environment) = presets::all_engines_ring();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run all-engines ring scenario");

    assert!(stats.executed_round_count > 0);
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .evaluate(&reduced)
        .expect("all-engines ring replay assertions");
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
    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_route_materialized(
            jacquard_core::NodeId([2; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32])),
        )
        .evaluate(&reduced)
        .expect("mixed replay assertions");
    assert!(
        reduced
            .rounds
            .iter()
            .flat_map(|round| round.environment_hooks.iter())
            .count()
            > 0
    );
}
