use jacquard_batman::BATMAN_ENGINE_ID;
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_simulator::{
    presets, JacquardSimulator, ReducedReplayView, ReferenceClientAdapter, ScenarioAssertions,
};
use jacquard_traits::RoutingSimulator;

#[test]
fn composition_explicit_path_preferred_selects_pathway() {
    let (scenario, environment) = presets::composition_explicit_path_preferred();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, _) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run explicit-path composition scenario");

    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_engine_selected(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
            &PATHWAY_ENGINE_ID,
        )
        .evaluate(&reduced)
        .expect("explicit-path composition assertions");
}

#[test]
fn composition_next_hop_only_viable_selects_batman() {
    let (scenario, environment) = presets::composition_next_hop_only_viable();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, _) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run next-hop-only composition scenario");

    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32])),
        )
        .expect_engine_selected(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([3; 32])),
            &BATMAN_ENGINE_ID,
        )
        .evaluate(&reduced)
        .expect("next-hop-only composition assertions");
}

#[test]
fn composition_corridor_preferred_selects_field() {
    let (scenario, environment) = presets::composition_corridor_preferred();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, _) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run corridor composition scenario");

    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_route_materialized(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
        )
        .expect_engine_selected(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
            &FIELD_ENGINE_ID,
        )
        .evaluate(&reduced)
        .expect("corridor composition assertions");
}

#[test]
fn composition_concurrent_objectives_select_distinct_engines() {
    let (scenario, environment) = presets::composition_concurrent_objectives();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, _) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run concurrent composition scenario");

    let reduced = ReducedReplayView::from_replay(&replay);
    ScenarioAssertions::new()
        .expect_engine_selected(
            jacquard_core::NodeId([1; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32])),
            &BATMAN_ENGINE_ID,
        )
        .expect_engine_selected(
            jacquard_core::NodeId([3; 32]),
            jacquard_core::DestinationId::Node(jacquard_core::NodeId([4; 32])),
            &FIELD_ENGINE_ID,
        )
        .expect_distinct_engine_count(2)
        .evaluate(&reduced)
        .expect("concurrent composition assertions");
}

#[test]
fn composition_cascade_partition_eliminates_route() {
    let (scenario, environment) = presets::composition_cascade_partition_eliminates_route();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, _) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run cascade partition composition scenario");

    let reduced = ReducedReplayView::from_replay(&replay);
    let batman_owner = jacquard_core::NodeId([1; 32]);
    let batman_destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([2; 32]));
    let field_owner = jacquard_core::NodeId([3; 32]);
    let field_destination = jacquard_core::DestinationId::Node(jacquard_core::NodeId([4; 32]));
    ScenarioAssertions::new()
        .expect_route_materialized(batman_owner, batman_destination.clone())
        .expect_route_materialized(field_owner, field_destination.clone())
        .expect_engine_selected(batman_owner, batman_destination.clone(), &BATMAN_ENGINE_ID)
        .expect_engine_selected(field_owner, field_destination.clone(), &FIELD_ENGINE_ID)
        .evaluate(&reduced)
        .expect("cascade partition initial assertions");
    assert_eq!(
        reduced.first_round_with_route(batman_owner, &batman_destination),
        Some(0)
    );
    assert_eq!(
        reduced.first_round_with_route(field_owner, &field_destination),
        Some(0)
    );
    assert!(
        reduced.route_absent_after_round(batman_owner, &batman_destination, 5),
        "batman route rounds: {:?}",
        reduced.route_present_rounds(batman_owner, &batman_destination)
    );
    assert!(
        reduced.route_absent_after_round(field_owner, &field_destination, 5),
        "field route rounds: {:?}",
        reduced.route_present_rounds(field_owner, &field_destination)
    );
}
