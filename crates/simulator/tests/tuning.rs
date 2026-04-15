use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_olsrv2::OLSRV2_ENGINE_ID;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_simulator::{
    presets, JacquardHostAdapter, JacquardSimulator, ReducedReplayView, ReferenceClientAdapter,
    ScenarioAssertions,
};
use jacquard_traits::RoutingSimulator;

#[test]
fn batman_bellman_decay_window_changes_route_loss_timing() {
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
        .expect_engine_selected(owner, destination.clone(), &BATMAN_BELLMAN_ENGINE_ID)
        .evaluate(&slow)
        .expect("slow BATMAN decay assertions");
    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &BATMAN_BELLMAN_ENGINE_ID)
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
fn olsrv2_decay_window_tuning_scenarios_materialize_routes() {
    let scenarios = presets::olsrv2_decay_tuning();
    assert_eq!(scenarios.len(), 2);
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32]));

    let (slow_replay, _) = simulator
        .run_scenario(&scenarios[0].0, &scenarios[0].1)
        .expect("run slow olsrv2 decay scenario");
    let (fast_replay, _) = simulator
        .run_scenario(&scenarios[1].0, &scenarios[1].1)
        .expect("run fast olsrv2 decay scenario");

    let slow = ReducedReplayView::from_replay(&slow_replay);
    let fast = ReducedReplayView::from_replay(&fast_replay);

    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &OLSRV2_ENGINE_ID)
        .evaluate(&slow)
        .expect("slow olsrv2 decay assertions");
    ScenarioAssertions::new()
        .expect_route_materialized(owner, destination.clone())
        .expect_engine_selected(owner, destination.clone(), &OLSRV2_ENGINE_ID)
        .evaluate(&fast)
        .expect("fast olsrv2 decay assertions");

    let slow_rounds = slow.route_present_rounds(owner, &destination);
    let fast_rounds = fast.route_present_rounds(owner, &destination);
    assert!(
        !slow_rounds.is_empty() && !fast_rounds.is_empty(),
        "slow rounds: {slow_rounds:?}, fast rounds: {fast_rounds:?}"
    );
    assert!(
        !slow.route_stability_scores(owner, &destination).is_empty()
            && !fast.route_stability_scores(owner, &destination).is_empty(),
        "slow rounds: {slow_rounds:?}, fast rounds: {fast_rounds:?}"
    );
}

#[test]
fn batman_classic_decay_window_changes_route_loss_timing() {
    let scenarios = presets::batman_classic_decay_tuning();
    assert_eq!(scenarios.len(), 2);
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32]));

    let (slow_replay, _) = simulator
        .run_scenario(&scenarios[0].0, &scenarios[0].1)
        .expect("run slow BATMAN Classic decay scenario");
    let (fast_replay, _) = simulator
        .run_scenario(&scenarios[1].0, &scenarios[1].1)
        .expect("run fast BATMAN Classic decay scenario");

    let slow = ReducedReplayView::from_replay(&slow_replay);
    let fast = ReducedReplayView::from_replay(&fast_replay);

    let slow_loss = slow.first_round_without_route_after_presence(owner, &destination);
    let fast_loss = fast.first_round_without_route_after_presence(owner, &destination);
    assert!(slow_loss.is_some());
    assert!(fast_loss.is_some());
    assert_ne!(
        slow_loss, fast_loss,
        "expected classic decay windows to change route loss timing; slow={slow_loss:?}, fast={fast_loss:?}"
    );
}

#[test]
fn babel_decay_window_changes_route_loss_timing() {
    let scenarios = presets::babel_decay_tuning();
    assert_eq!(scenarios.len(), 2);
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32]));

    let (slow_replay, _) = simulator
        .run_scenario(&scenarios[0].0, &scenarios[0].1)
        .expect("run slow Babel decay scenario");
    let (fast_replay, _) = simulator
        .run_scenario(&scenarios[1].0, &scenarios[1].1)
        .expect("run fast Babel decay scenario");

    let slow = ReducedReplayView::from_replay(&slow_replay);
    let fast = ReducedReplayView::from_replay(&fast_replay);

    let slow_rounds = slow.route_present_rounds(owner, &destination);
    let fast_rounds = fast.route_present_rounds(owner, &destination);
    let slow_stability = slow.route_stability_scores(owner, &destination);
    let fast_stability = fast.route_stability_scores(owner, &destination);
    assert!(
        slow_rounds != fast_rounds || slow_stability != fast_stability,
        "expected Babel decay windows to change replay-visible behavior; slow rounds={slow_rounds:?}, fast rounds={fast_rounds:?}, slow stability={slow_stability:?}, fast stability={fast_stability:?}"
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
        .expect_engine_selected(owner, destination.clone(), &BATMAN_BELLMAN_ENGINE_ID)
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

#[test]
fn field_bootstrap_evidence_surfaces_in_replay_analysis() {
    let (scenario, environment) = presets::field_bootstrap_multihop();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
    let owner = jacquard_core::NodeId([1; 32]);
    let destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32]));

    let (replay, _) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run field bootstrap scenario");
    let reduced = ReducedReplayView::from_replay(&replay);
    let field_replays = reduced.field_replays_for(owner);
    assert!(
        !field_replays.is_empty(),
        "expected field replay analysis for owner {owner:?}"
    );
    assert!(
        field_replays.iter().any(|summary| {
            summary
                .bundle
                .runtime_search
                .runtime_artifacts
                .iter()
                .any(|artifact| {
                    artifact.destination.as_ref() == Some(&destination)
                        && artifact
                            .router_artifact
                            .as_ref()
                            .is_some_and(|router_artifact| router_artifact.route_support > 0)
                })
        }),
        "expected bootstrap evidence to surface in replay analysis for {destination:?}"
    );
    assert!(field_replays.iter().any(|summary| {
        summary.bootstrap_activation_count
            == summary
                .bundle
                .recovery
                .entries
                .iter()
                .map(|entry| entry.bootstrap_activation_count)
                .max()
                .unwrap_or(0)
    }));
}

#[test]
fn field_bootstrap_multihop_materializes_route_through_router_boundary() {
    let (scenario, _environment) = presets::field_bootstrap_multihop();
    let adapter = ReferenceClientAdapter;
    let mut hosts = adapter.build_hosts(&scenario).expect("build hosts");
    let host = hosts
        .get_mut(&jacquard_core::NodeId([1; 32]))
        .expect("owner host");

    {
        let mut bound = host.bind();
        for round in 1..=3 {
            bound
                .advance_round()
                .unwrap_or_else(|_| panic!("advance field round {round}"));
        }
        let objective = scenario.bound_objectives()[0].objective.clone();
        let route = bound
            .router_mut()
            .activate_route_without_tick(&objective)
            .expect("field bootstrap activation should materialize a route");
        assert_eq!(route.identity.admission.summary.engine, FIELD_ENGINE_ID);
    }
}
