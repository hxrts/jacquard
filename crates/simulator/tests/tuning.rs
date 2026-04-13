use jacquard_batman::BATMAN_ENGINE_ID;
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_simulator::{
    presets, JacquardSimulator, ReducedReplayView, ReferenceClientAdapter, ScenarioAssertions,
};
use jacquard_traits::RoutingSimulator;

#[test]
fn batman_decay_window_changes_route_loss_timing() {
    let scenarios = presets::batman_decay_tuning();
    assert_eq!(scenarios.len(), 2);
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32]));

    let (slow_replay, _) = simulator
        .run_scenario(&scenarios[0].0, &scenarios[0].1)
        .expect("run slow BATMAN decay scenario");
    let (fast_replay, _) = simulator
        .run_scenario(&scenarios[1].0, &scenarios[1].1)
        .expect("run fast BATMAN decay scenario");

    let slow = ReducedReplayView::from_replay(&slow_replay);
    let fast = ReducedReplayView::from_replay(&fast_replay);

    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &BATMAN_ENGINE_ID)
        .evaluate(&slow)
        .expect("slow BATMAN decay assertions");
    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &BATMAN_ENGINE_ID)
        .evaluate(&fast)
        .expect("fast BATMAN decay assertions");

    let slow_rounds = slow.route_present_rounds(owner, &destination);
    let fast_rounds = fast.route_present_rounds(owner, &destination);
    let slow_stability = slow.route_stability_scores(owner, &destination);
    let fast_stability = fast.route_stability_scores(owner, &destination);
    assert!(
        !slow_rounds.is_empty() && !fast_rounds.is_empty(),
        "slow rounds: {slow_rounds:?}, fast rounds: {fast_rounds:?}"
    );
    assert!(
        !slow_stability.is_empty() && !fast_stability.is_empty(),
        "slow stability: {slow_stability:?}, fast stability: {fast_stability:?}"
    );
    assert!(
        fast_stability != slow_stability,
        "expected decay-window stability difference; fast stability: {fast_stability:?}, slow stability: {slow_stability:?}"
    );
    assert!(
        slow.first_round_without_route_after_presence(owner, &destination).is_some()
            && fast.first_round_without_route_after_presence(owner, &destination).is_some(),
        "expected both BATMAN decay scenarios to lose the route after partition; slow rounds: {slow_rounds:?}, fast rounds: {fast_rounds:?}"
    );
}

#[test]
fn routing_profile_changes_selected_engine() {
    let scenarios = presets::profile_driven_engine_selection();
    assert_eq!(scenarios.len(), 2);
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32]));

    let (connected_replay, _) = simulator
        .run_scenario(&scenarios[0].0, &scenarios[0].1)
        .expect("run connected-profile scenario");
    let (partition_replay, _) = simulator
        .run_scenario(&scenarios[1].0, &scenarios[1].1)
        .expect("run partition-tolerant profile scenario");

    let connected = ReducedReplayView::from_replay(&connected_replay);
    let partition = ReducedReplayView::from_replay(&partition_replay);

    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &BATMAN_ENGINE_ID)
        .evaluate(&connected)
        .expect("connected-profile BATMAN selection");
    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &FIELD_ENGINE_ID)
        .evaluate(&partition)
        .expect("partition-tolerant field selection");
}

#[test]
fn pathway_search_budget_changes_service_route_presence() {
    let scenarios = presets::pathway_search_budget_tuning();
    assert_eq!(scenarios.len(), 2);
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Service(jacquard_core::ServiceId(vec![9; 16]));

    let (low_budget_replay, _) = simulator
        .run_scenario(&scenarios[0].0, &scenarios[0].1)
        .expect("run low-budget pathway search scenario");
    let (high_budget_replay, _) = simulator
        .run_scenario(&scenarios[1].0, &scenarios[1].1)
        .expect("run high-budget pathway search scenario");

    let low_budget = ReducedReplayView::from_replay(&low_budget_replay);
    let high_budget = ReducedReplayView::from_replay(&high_budget_replay);

    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &PATHWAY_ENGINE_ID)
        .evaluate(&low_budget)
        .expect("low-budget pathway route materializes before partition");
    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &PATHWAY_ENGINE_ID)
        .evaluate(&high_budget)
        .expect("high-budget pathway route materializes");

    assert!(
        low_budget.route_absent_after_round(owner, &destination, 3),
        "low-budget search should lose the service route after partition: {:?}",
        low_budget.route_present_rounds(owner, &destination)
    );
    assert!(
        !low_budget.failure_summaries.is_empty(),
        "low-budget search should record a replay-visible failure summary"
    );
    assert!(
        !high_budget.route_absent_after_round(owner, &destination, 3),
        "high-budget search should preserve or recover service routing after partition: {:?}",
        high_budget.route_present_rounds(owner, &destination)
    );
}
