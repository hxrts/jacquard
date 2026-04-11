mod common;

use common::{
    engine::{
        admit_first_candidate, build_engine, build_engine_with_config, objective, profile,
        tick_and_get_candidates, LOCAL_NODE_ID,
    },
    fixtures::{link, node, sample_configuration},
};
use jacquard_pathway::{
    PathwaySearchConfig, PathwaySearchHeuristicMode, PathwaySearchTransitionClass,
};
use jacquard_traits::{
    jacquard_core::{DestinationId, NodeId, Observation, RouteEpoch, ServiceId},
    RoutingEnginePlanner,
};
use telltale_search::{
    compare_observations, replay_observation, theorem_backed_observables, ObservationRelation,
    ReplayExpectation, SearchDeterminismMode, SearchQuery, SearchReseedingPolicy,
    SearchSchedulerProfile,
};

fn same_epoch_updated_topology() -> Observation<jacquard_traits::jacquard_core::Configuration> {
    let mut topology = sample_configuration();
    let destination_node_id = NodeId([3; 32]);
    let local_node_id = NodeId([1; 32]);
    topology
        .value
        .links
        .insert((local_node_id, destination_node_id), link(9, 990));
    topology
}

fn new_epoch_updated_topology() -> Observation<jacquard_traits::jacquard_core::Configuration> {
    let mut topology = same_epoch_updated_topology();
    topology.value.epoch = RouteEpoch(3);
    let gateway_node_id = NodeId([5; 32]);
    topology.value.nodes.insert(gateway_node_id, node(5));
    topology
}

#[test]
fn search_record_replays_into_the_final_observation() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidates = engine.candidate_routes(&goal, &policy, &topology);
    assert_eq!(candidates.len(), 1);

    let record = engine.last_search_record().expect("search record");
    assert_eq!(
        record.query,
        Some(SearchQuery::single_goal(LOCAL_NODE_ID, NodeId([3; 32]))),
    );
    let run = record.run.as_ref().expect("one search run");
    assert_eq!(
        run.topology_transition,
        PathwaySearchTransitionClass::InitialSnapshot
    );
    assert_eq!(run.report.observation, run.replay.final_observation);

    let expectation = ReplayExpectation {
        expected_epochs: run.replay.epoch_trace.clone(),
        expected_snapshots: run
            .replay
            .rounds
            .iter()
            .map(|round| round.snapshot_id)
            .collect(),
        expected_phases: run.replay.rounds.iter().map(|round| round.phase).collect(),
        expected_batch_nodes: run
            .replay
            .rounds
            .iter()
            .map(|round| round.batch_nodes.clone())
            .collect(),
        required_fairness: run.replay.fairness_assumptions.clone(),
    };
    let replayed = replay_observation(&run.replay, &expectation).expect("replay observation");
    assert_eq!(replayed, run.report.observation);
}

#[test]
// long-block-exception: epoch-transition assertions are clearer in one sequence.
fn topology_transition_classification_and_reconfiguration_are_explicit() {
    let engine = build_engine();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let first_topology = sample_configuration();
    let second_topology = same_epoch_updated_topology();
    let third_topology = new_epoch_updated_topology();

    std::mem::drop(engine.candidate_routes(&goal, &policy, &first_topology));
    let first = engine.last_search_record().expect("first search record");
    let first_run = first.run.as_ref().expect("first search run");
    assert_eq!(
        first_run.topology_transition,
        PathwaySearchTransitionClass::InitialSnapshot,
    );
    assert!(first_run.reconfiguration.is_none());

    std::mem::drop(engine.candidate_routes(&goal, &policy, &second_topology));
    let second = engine.last_search_record().expect("second search record");
    let second_run = second.run.as_ref().expect("second search run");
    assert_eq!(
        second_run.topology_transition,
        PathwaySearchTransitionClass::SameEpochNewSnapshot,
    );
    let second_reconfiguration = second_run
        .reconfiguration
        .as_ref()
        .expect("same epoch update reconfigures");
    assert_eq!(
        second_reconfiguration.transition_class,
        PathwaySearchTransitionClass::SameEpochNewSnapshot,
    );
    assert_eq!(
        second_reconfiguration.reseeding_policy,
        SearchReseedingPolicy::PreserveOpenAndIncons,
    );
    assert!(
        second_run.replay.epoch_trace.len() >= 2,
        "reconfigured runs should carry the prior and current snapshot epochs",
    );

    std::mem::drop(engine.candidate_routes(&goal, &policy, &second_topology));
    let stable = engine.last_search_record().expect("stable search record");
    let stable_run = stable.run.as_ref().expect("stable search run");
    assert_eq!(
        stable_run.topology_transition,
        PathwaySearchTransitionClass::SameEpochSameSnapshot,
    );
    assert!(stable_run.reconfiguration.is_none());

    std::mem::drop(engine.candidate_routes(&goal, &policy, &third_topology));
    let third = engine.last_search_record().expect("third search record");
    let third_run = third.run.as_ref().expect("third search run");
    assert_eq!(
        third_run.topology_transition,
        PathwaySearchTransitionClass::NewRouteEpoch,
    );
    assert_eq!(
        third_run
            .reconfiguration
            .as_ref()
            .expect("new epoch reconfigures")
            .transition_class,
        PathwaySearchTransitionClass::NewRouteEpoch,
    );
}

#[test]
fn multi_goal_query_runs_once_and_still_publishes_all_service_candidates() {
    let mut serial_engine = build_engine_with_config(PathwaySearchConfig::canonical_serial());
    let mut threaded_engine =
        build_engine_with_config(PathwaySearchConfig::threaded_exact_single_lane());
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![1, 2, 3])));
    let policy = profile();

    let serial_candidates = tick_and_get_candidates(&mut serial_engine, &topology, &goal, &policy);
    let threaded_candidates =
        tick_and_get_candidates(&mut threaded_engine, &topology, &goal, &policy);
    assert_eq!(serial_candidates, threaded_candidates);
    assert_eq!(serial_candidates.len(), 3);

    let expected_query = SearchQuery::try_multi_goal(
        LOCAL_NODE_ID,
        vec![NodeId([2; 32]), NodeId([3; 32]), NodeId([4; 32])],
    )
    .expect("non-empty multi-goal query");

    let serial_record = serial_engine.last_search_record().expect("serial record");
    let threaded_record = threaded_engine
        .last_search_record()
        .expect("threaded record");
    assert_eq!(serial_record.query, Some(expected_query.clone()));
    assert_eq!(threaded_record.query, Some(expected_query));
    assert!(serial_record.run.is_some());
    assert!(threaded_record.run.is_some());
    assert_eq!(serial_record.candidate_node_paths().len(), 3);
    assert_eq!(threaded_record.candidate_node_paths().len(), 3);
}

#[test]
fn threaded_exact_matches_canonical_serial_for_theorem_backed_observables() {
    let mut serial_engine = build_engine_with_config(PathwaySearchConfig::canonical_serial());
    let mut threaded_engine =
        build_engine_with_config(PathwaySearchConfig::threaded_exact_single_lane());
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![1, 2, 3])));
    let policy = profile();

    let serial_candidates = tick_and_get_candidates(&mut serial_engine, &topology, &goal, &policy);
    let threaded_candidates =
        tick_and_get_candidates(&mut threaded_engine, &topology, &goal, &policy);
    assert_eq!(serial_candidates, threaded_candidates);

    let serial_record = serial_engine.last_search_record().expect("serial record");
    let threaded_record = threaded_engine
        .last_search_record()
        .expect("threaded record");
    let serial_run = serial_record.run.as_ref().expect("serial run");
    let threaded_run = threaded_record.run.as_ref().expect("threaded run");

    let required = theorem_backed_observables(SearchSchedulerProfile::ThreadedExactSingleLane)
        .into_iter()
        .collect::<Vec<_>>();
    assert!(serial_run.report.progress.total_scheduler_steps > 0);
    assert!(threaded_run.report.progress.total_scheduler_steps > 0);
    let comparison = compare_observations(
        &serial_run.report.observation,
        &threaded_run.report.observation,
        SearchDeterminismMode::Full,
        &required,
    );
    assert_eq!(comparison.relation, ObservationRelation::Exact);
    assert!(comparison.mismatches.is_empty());
}

#[test]
fn hop_lower_bound_heuristic_preserves_candidate_output() {
    let zero_engine = build_engine_with_config(PathwaySearchConfig::canonical_serial());
    let heuristic_engine = build_engine_with_config(
        PathwaySearchConfig::canonical_serial()
            .with_heuristic_mode(PathwaySearchHeuristicMode::HopLowerBound),
    );
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![1, 2, 3])));
    let policy = profile();

    let zero_candidates = zero_engine.candidate_routes(&goal, &policy, &topology);
    let heuristic_candidates = heuristic_engine.candidate_routes(&goal, &policy, &topology);
    assert_eq!(zero_candidates, heuristic_candidates);

    let zero_record = zero_engine
        .last_search_record()
        .expect("zero heuristic record");
    let heuristic_record = heuristic_engine
        .last_search_record()
        .expect("hop heuristic record");
    let zero_run = zero_record.run.as_ref().expect("zero heuristic run");
    let heuristic_run = heuristic_record.run.as_ref().expect("hop heuristic run");
    assert_eq!(
        zero_run.selected_node_path,
        heuristic_run.selected_node_path
    );
    assert_eq!(
        zero_run.report.observation.selected_result_cost,
        heuristic_run.report.observation.selected_result_cost,
    );
}

#[test]
fn serial_and_threaded_exact_preserve_router_visible_route_behavior() {
    let mut serial_engine = build_engine_with_config(PathwaySearchConfig::canonical_serial());
    let mut threaded_engine =
        build_engine_with_config(PathwaySearchConfig::threaded_exact_single_lane());
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let serial_candidates = serial_engine.candidate_routes(&goal, &policy, &topology);
    let threaded_candidates = threaded_engine.candidate_routes(&goal, &policy, &topology);
    assert_eq!(serial_candidates, threaded_candidates);

    let (serial_route_id, serial_admission) = admit_first_candidate(
        &mut serial_engine,
        &topology,
        &goal,
        &policy,
        serial_candidates,
    );
    let (threaded_route_id, threaded_admission) = admit_first_candidate(
        &mut threaded_engine,
        &topology,
        &goal,
        &policy,
        threaded_candidates,
    );
    assert_eq!(serial_route_id, threaded_route_id);
    assert_eq!(serial_admission, threaded_admission);
}

#[test]
fn pathway_search_source_avoids_compat_and_removed_aliases() {
    let sources = [
        include_str!("../src/engine/planner/search/mod.rs"),
        include_str!("../src/engine/planner/search/runner.rs"),
        include_str!("../src/engine/planner/publishing.rs"),
    ];

    for source in sources {
        assert!(!source.contains("telltale_search::compat"));
        assert!(!source.contains("incumbent_"));
        assert!(!source.contains("route_bound"));
        assert!(!source.contains("single-goal loop"));
        assert!(!source.contains("TODO remove after v13"));
    }
}
