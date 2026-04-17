//! Suite execution and artifact writing for route-visible experiment runs.

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use jacquard_babel::simulator::{
    run_checkpoint_restore_fixture as run_babel_checkpoint_restore_fixture,
    run_planner_decision_fixture as run_babel_planner_decision_fixture,
    run_round_refresh_fixture as run_babel_round_refresh_fixture,
};
use jacquard_batman_bellman::simulator::run_planner_decision_fixture as run_batman_bellman_planner_decision_fixture;
use jacquard_batman_classic::simulator::run_planner_decision_fixture as run_batman_classic_planner_decision_fixture;
use jacquard_core::{DestinationId, NodeId, RoutingTickChange};
use jacquard_field::simulator::run_planner_decision_fixture as run_field_planner_decision_fixture;
use jacquard_olsrv2::simulator::run_planner_decision_fixture as run_olsr_planner_decision_fixture;
use jacquard_pathway::simulator::run_planner_decision_fixture as run_pathway_planner_decision_fixture;
use jacquard_scatter::simulator::run_planner_decision_fixture as run_scatter_planner_decision_fixture;
use jacquard_traits::{RoutingScenario, RoutingSimulator};
use rayon::prelude::*;

use super::{
    aggregate_runs, summarize_breakdowns, summarize_run, BabelCheckpointRestoreCase,
    BabelPlannerDecisionCase, BabelRoundRefreshCase, BatmanBellmanPlannerDecisionCase,
    BatmanClassicPlannerDecisionCase, ExperimentArtifacts, ExperimentError, ExperimentManifest,
    ExperimentModelArtifact, ExperimentModelCase, ExperimentRunSpec, ExperimentRunSummary,
    ExperimentSuite, FieldPlannerDecisionCase, JacquardHostAdapter, JacquardSimulator,
    OlsrPlannerDecisionCase, PathwayPlannerDecisionCase, ScatterPlannerDecisionCase,
};
use crate::SimulationExecutionLane;

struct ExecutedRun {
    summary: ExperimentRunSummary,
    model_artifacts: Vec<ExperimentModelArtifact>,
}

struct VisibilityExpectation {
    owner_node_id: NodeId,
    destination: DestinationId,
    visible_round: u32,
}

struct ModelExecution {
    artifacts: Vec<ExperimentModelArtifact>,
    expected_visibility: Option<VisibilityExpectation>,
}

#[cfg(test)]
pub(super) fn execute_suite_runs_serial<A>(
    adapter: &A,
    suite: &ExperimentSuite,
) -> Result<Vec<ExperimentRunSummary>, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    suite
        .runs
        .iter()
        .map(|spec| {
            let simulator = JacquardSimulator::new(adapter.clone());
            execute_run(&simulator, spec).map(|executed| executed.summary)
        })
        .collect()
}

fn execute_run<A>(
    simulator: &JacquardSimulator<A>,
    spec: &ExperimentRunSpec,
) -> Result<ExecutedRun, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    match spec.execution_lane {
        SimulationExecutionLane::FullStack => execute_full_stack_run(simulator, spec),
        SimulationExecutionLane::Model => execute_model_run(spec),
        SimulationExecutionLane::Equivalence => execute_equivalence_run(simulator, spec),
    }
}

fn execute_full_stack_run<A>(
    simulator: &JacquardSimulator<A>,
    spec: &ExperimentRunSpec,
) -> Result<ExecutedRun, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    let (reduced, _) = simulator
        .run_scenario_reduced(&spec.scenario, &spec.environment)
        .map_err(|source| ExperimentError::SimulationRun {
            run_id: spec.run_id.clone(),
            source,
        })?;
    Ok(ExecutedRun {
        summary: summarize_run(spec, &reduced),
        model_artifacts: Vec::new(),
    })
}

fn execute_model_run(spec: &ExperimentRunSpec) -> Result<ExecutedRun, ExperimentError> {
    let execution = execute_model_case(spec)?;
    let mut summary = summarize_run(spec, &empty_reduced_view(spec));
    summary.model_artifact_count = u32::try_from(execution.artifacts.len()).unwrap_or(u32::MAX);
    Ok(ExecutedRun {
        summary,
        model_artifacts: execution.artifacts,
    })
}

// long-block-exception: equivalence execution keeps the model-lane run,
// full-stack replay, comparison, and artifact recording in one flow.
fn execute_equivalence_run<A>(
    simulator: &JacquardSimulator<A>,
    spec: &ExperimentRunSpec,
) -> Result<ExecutedRun, ExperimentError>
where
    A: JacquardHostAdapter + Clone,
{
    let reduced = match spec.model_case.as_ref() {
        Some(ExperimentModelCase::BabelCheckpointRestore(_)) => {
            let mut resumed_simulator = JacquardSimulator::new(simulator.host_adapter().clone());
            let (replay, _) = resumed_simulator
                .run_scenario(&spec.scenario, &spec.environment)
                .map_err(|source| ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                })?;
            let (resumed, _) = resumed_simulator.resume_replay(&replay).map_err(|source| {
                ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                }
            })?;
            crate::ReducedReplayView::from_replay(&resumed)
        }
        _ => {
            simulator
                .run_scenario_reduced(&spec.scenario, &spec.environment)
                .map_err(|source| ExperimentError::SimulationRun {
                    run_id: spec.run_id.clone(),
                    source,
                })?
                .0
        }
    };
    let mut execution = execute_model_case(spec)?;
    let expectation = execution.expected_visibility.take().ok_or_else(|| {
        ExperimentError::EquivalenceMismatch {
            run_id: spec.run_id.clone(),
            detail: "equivalence run is missing a visibility expectation".to_string(),
        }
    })?;
    let visible = reduced.rounds.iter().any(|round| {
        round.round_index >= expectation.visible_round
            && round.active_routes.iter().any(|route| {
                route.owner_node_id == expectation.owner_node_id
                    && route.destination == expectation.destination
                    && route.engine_id == jacquard_babel::BABEL_ENGINE_ID
            })
    });
    if !visible {
        return Err(ExperimentError::EquivalenceMismatch {
            run_id: spec.run_id.clone(),
            detail: format!(
                "full-stack replay never showed a Babel route for {:?} after round {}",
                expectation.destination, expectation.visible_round
            ),
        });
    }
    execution.artifacts.push(ExperimentModelArtifact {
        run_id: spec.run_id.clone(),
        suite_id: spec.suite_id.clone(),
        engine_family: spec.engine_family.clone(),
        execution_lane: spec.execution_lane.label().to_string(),
        fixture_id: format!("{}-equivalence", spec.run_id),
        artifact_kind: "equivalence-check".to_string(),
        owner_node_id: Some(node_id_hex(expectation.owner_node_id)),
        destination_node_id: destination_hex(&expectation.destination),
        next_hop_node_id: None,
        topology_epoch: None,
        candidate_count: None,
        reducer_route_entry_count: None,
        reducer_best_next_hop_count: None,
        reducer_change: None,
        backend_route_id_hex: None,
        visible_round: Some(expectation.visible_round),
        equivalence_passed: Some(true),
    });
    let mut summary = summarize_run(spec, &reduced);
    summary.model_artifact_count = u32::try_from(execution.artifacts.len()).unwrap_or(u32::MAX);
    summary.equivalence_passed = Some(true);
    Ok(ExecutedRun {
        summary,
        model_artifacts: execution.artifacts,
    })
}

fn execute_model_case(spec: &ExperimentRunSpec) -> Result<ModelExecution, ExperimentError> {
    match spec
        .model_case
        .as_ref()
        .ok_or_else(|| ExperimentError::MissingModelCase {
            run_id: spec.run_id.clone(),
        })? {
        ExperimentModelCase::BatmanBellmanPlannerDecision(case) => {
            execute_batman_bellman_planner_case(spec, case)
        }
        ExperimentModelCase::BatmanClassicPlannerDecision(case) => {
            execute_batman_classic_planner_case(spec, case)
        }
        ExperimentModelCase::BabelPlannerDecision(case) => execute_babel_planner_case(spec, case),
        ExperimentModelCase::BabelRoundRefresh(case) => execute_babel_round_case(spec, case),
        ExperimentModelCase::BabelCheckpointRestore(case) => {
            execute_babel_checkpoint_case(spec, case.as_ref())
        }
        ExperimentModelCase::FieldPlannerDecision(case) => execute_field_planner_case(spec, case),
        ExperimentModelCase::OlsrPlannerDecision(case) => execute_olsr_planner_case(spec, case),
        ExperimentModelCase::PathwayPlannerDecision(case) => {
            execute_pathway_planner_case(spec, case)
        }
        ExperimentModelCase::ScatterPlannerDecision(case) => {
            execute_scatter_planner_case(spec, case)
        }
    }
}

fn execute_batman_bellman_planner_case(
    spec: &ExperimentRunSpec,
    case: &BatmanBellmanPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_batman_bellman_planner_decision_fixture(
        case.owner_node_id,
        case.expected_next_hop,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("batman bellman planner fixture failed: {error}"),
    })?;
    if !result.admitted || result.selected_neighbor != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "batman bellman planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(result.selected_neighbor)),
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

fn execute_batman_classic_planner_case(
    spec: &ExperimentRunSpec,
    case: &BatmanClassicPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_batman_classic_planner_decision_fixture(
        case.owner_node_id,
        case.expected_next_hop,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("batman classic planner fixture failed: {error}"),
    })?;
    if !result.admitted || result.selected_neighbor != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "batman classic planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(result.selected_neighbor)),
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

fn execute_babel_planner_case(
    spec: &ExperimentRunSpec,
    case: &BabelPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_babel_planner_decision_fixture(
        &case.snapshot,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("babel planner fixture failed: {error}"),
    })?;
    let expected_backend_route_id =
        jacquard_babel::simulator::backend_route_id(case.destination, case.expected_next_hop);
    if !result.admitted || result.backend_route_id != expected_backend_route_id {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(case.expected_next_hop)),
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: Some(case.expected_visible_round),
            equivalence_passed: None,
        }],
        expected_visibility: Some(VisibilityExpectation {
            owner_node_id: case.owner_node_id,
            destination: DestinationId::Node(case.destination),
            visible_round: case.expected_visible_round,
        }),
    })
}

fn execute_babel_round_case(
    spec: &ExperimentRunSpec,
    case: &BabelRoundRefreshCase,
) -> Result<ModelExecution, ExperimentError> {
    let output = run_babel_round_refresh_fixture(&case.prior_state, &case.input);
    if output.change != case.expected_change {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: format!(
                "round reducer reported {:?} instead of {:?}",
                output.change, case.expected_change
            ),
        });
    }
    let observed = output
        .planner_snapshot
        .choices
        .iter()
        .map(|choice| (choice.destination, choice.next_hop))
        .collect::<Vec<_>>();
    if observed != case.expected_destinations {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "round reducer produced the wrong destination-to-next-hop mapping".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "round-transition".to_string(),
            owner_node_id: Some(node_id_hex(case.input.local_node_id)),
            destination_node_id: observed
                .first()
                .map(|(destination, _)| node_id_hex(*destination)),
            next_hop_node_id: observed.first().map(|(_, next_hop)| node_id_hex(*next_hop)),
            topology_epoch: Some(case.input.topology.value.epoch.0),
            candidate_count: None,
            reducer_route_entry_count: Some(
                u32::try_from(case.prior_state.route_entries.len()).unwrap_or(u32::MAX),
            ),
            reducer_best_next_hop_count: Some(
                u32::try_from(output.best_next_hop_count).unwrap_or(u32::MAX),
            ),
            reducer_change: Some(routing_tick_change_label(output.change).to_string()),
            backend_route_id_hex: observed.first().map(|(destination, next_hop)| {
                bytes_to_hex(
                    &jacquard_babel::simulator::backend_route_id(*destination, *next_hop).0,
                )
            }),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

fn execute_babel_checkpoint_case(
    spec: &ExperimentRunSpec,
    case: &BabelCheckpointRestoreCase,
) -> Result<ModelExecution, ExperimentError> {
    let restored = run_babel_checkpoint_restore_fixture(&case.route).ok_or_else(|| {
        ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "checkpoint restore fixture did not reconstruct a Babel route".to_string(),
        }
    })?;
    if restored.next_hop != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "checkpoint restore fixture reconstructed the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "checkpoint-restore".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(restored.next_hop)),
            topology_epoch: Some(case.route.identity.stamp.topology_epoch.0),
            candidate_count: None,
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&restored.backend_route_id.0)),
            visible_round: Some(case.expected_visible_round),
            equivalence_passed: None,
        }],
        expected_visibility: Some(VisibilityExpectation {
            owner_node_id: case.owner_node_id,
            destination: DestinationId::Node(case.destination),
            visible_round: case.expected_visible_round,
        }),
    })
}

fn execute_field_planner_case(
    spec: &ExperimentRunSpec,
    case: &FieldPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_field_planner_decision_fixture(
        case.owner_node_id,
        case.expected_next_hop,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("field planner fixture failed: {error}"),
    })?;
    if !result.admitted {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "field planner fixture candidate was not admissible".to_string(),
        });
    }
    if result.selected_neighbor != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "field planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(result.selected_neighbor)),
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

fn execute_pathway_planner_case(
    spec: &ExperimentRunSpec,
    case: &PathwayPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_pathway_planner_decision_fixture(
        case.owner_node_id,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("pathway planner fixture failed: {error}"),
    })?;
    if !result.admitted {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "pathway planner fixture candidate was not admissible".to_string(),
        });
    }
    if result.first_hop_node_id != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "pathway planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(result.first_hop_node_id)),
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

fn execute_olsr_planner_case(
    spec: &ExperimentRunSpec,
    case: &OlsrPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_olsr_planner_decision_fixture(
        case.owner_node_id,
        case.expected_next_hop,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("olsrv2 planner fixture failed: {error}"),
    })?;
    if !result.admitted || result.selected_neighbor != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "olsrv2 planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: Some(node_id_hex(result.selected_neighbor)),
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

fn execute_scatter_planner_case(
    spec: &ExperimentRunSpec,
    case: &ScatterPlannerDecisionCase,
) -> Result<ModelExecution, ExperimentError> {
    let result = run_scatter_planner_decision_fixture(
        case.owner_node_id,
        &case.objective,
        &case.profile,
        &case.topology,
    )
    .map_err(|error| ExperimentError::ModelExpectationFailed {
        run_id: spec.run_id.clone(),
        detail: format!("scatter planner fixture failed: {error}"),
    })?;
    if !result.admitted {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "scatter planner fixture candidate was not admissible".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![ExperimentModelArtifact {
            run_id: spec.run_id.clone(),
            suite_id: spec.suite_id.clone(),
            engine_family: spec.engine_family.clone(),
            execution_lane: spec.execution_lane.label().to_string(),
            fixture_id: case.fixture_id.clone(),
            artifact_kind: "planner-decision".to_string(),
            owner_node_id: Some(node_id_hex(case.owner_node_id)),
            destination_node_id: Some(node_id_hex(case.destination)),
            next_hop_node_id: None,
            topology_epoch: Some(case.topology.value.epoch.0),
            candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
            reducer_route_entry_count: None,
            reducer_best_next_hop_count: None,
            reducer_change: None,
            backend_route_id_hex: Some(bytes_to_hex(&result.backend_route_id.0)),
            visible_round: None,
            equivalence_passed: None,
        }],
        expected_visibility: None,
    })
}

pub(super) fn execute_suite_runs_parallel<A>(
    adapter: &A,
    suite: &ExperimentSuite,
) -> Result<(Vec<ExperimentRunSummary>, Vec<ExperimentModelArtifact>), ExperimentError>
where
    A: JacquardHostAdapter + Clone + Send + Sync,
{
    let mut indexed = suite
        .runs
        .par_iter()
        .enumerate()
        .map(|(index, spec)| {
            let simulator = JacquardSimulator::new(adapter.clone());
            execute_run(&simulator, spec).map(|executed| (index, executed))
        })
        .collect::<Vec<_>>();
    let mut runs = Vec::with_capacity(indexed.len());
    let mut model_artifacts = Vec::new();
    indexed.sort_by_key(|result| match result {
        Ok((index, _)) => *index,
        Err(_) => usize::MAX,
    });
    for result in indexed {
        let (_, executed) = result?;
        runs.push(executed.summary);
        model_artifacts.extend(executed.model_artifacts);
    }
    Ok((runs, model_artifacts))
}

pub fn run_suite<A>(
    simulator: &mut JacquardSimulator<A>,
    suite: &ExperimentSuite,
    output_dir: &Path,
) -> Result<ExperimentArtifacts, ExperimentError>
where
    A: JacquardHostAdapter + Clone + Send + Sync,
{
    fs::create_dir_all(output_dir)?;
    let (runs, model_artifacts) = execute_suite_runs_parallel(simulator.host_adapter(), suite)?;
    let run_path = output_dir.join("runs.jsonl");
    let mut writer = BufWriter::new(File::create(&run_path)?);

    for summary in &runs {
        serde_json::to_writer(&mut writer, summary)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;

    if !model_artifacts.is_empty() {
        let model_path = output_dir.join("model_artifacts.jsonl");
        let mut model_writer = BufWriter::new(File::create(&model_path)?);
        for artifact in &model_artifacts {
            serde_json::to_writer(&mut model_writer, artifact)?;
            model_writer.write_all(b"\n")?;
        }
        model_writer.flush()?;
    }

    let aggregates = aggregate_runs(&runs);
    let breakdowns = summarize_breakdowns(&aggregates);
    let manifest = ExperimentManifest {
        suite_id: suite.suite_id.clone(),
        generated_at_unix_seconds: 0,
        run_count: u32::try_from(runs.len()).unwrap_or(u32::MAX),
        aggregate_count: u32::try_from(aggregates.len()).unwrap_or(u32::MAX),
        breakdown_count: u32::try_from(breakdowns.len()).unwrap_or(u32::MAX),
        model_artifact_count: u32::try_from(model_artifacts.len()).unwrap_or(u32::MAX),
    };

    serde_json::to_writer_pretty(File::create(output_dir.join("manifest.json"))?, &manifest)?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("aggregates.json"))?,
        &aggregates,
    )?;
    serde_json::to_writer_pretty(
        File::create(output_dir.join("breakdowns.json"))?,
        &breakdowns,
    )?;

    Ok(ExperimentArtifacts {
        output_dir: output_dir.to_path_buf(),
        manifest,
        runs,
        aggregates,
        breakdowns,
        model_artifacts,
    })
}

fn empty_reduced_view(spec: &ExperimentRunSpec) -> crate::ReducedReplayView {
    crate::ReducedReplayView {
        scenario_name: spec.scenario.name().to_string(),
        round_count: 0,
        rounds: Vec::new(),
        distinct_engine_ids: Vec::new(),
        driver_status_events: Vec::new(),
        failure_summaries: Vec::new(),
    }
}

fn node_id_hex(node_id: NodeId) -> String {
    bytes_to_hex(&node_id.0)
}

fn destination_hex(destination: &DestinationId) -> Option<String> {
    match destination {
        DestinationId::Node(node_id) => Some(node_id_hex(*node_id)),
        DestinationId::Service(service_id) => Some(bytes_to_hex(&service_id.0)),
        DestinationId::Gateway(gateway_id) => Some(bytes_to_hex(&gateway_id.0)),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn routing_tick_change_label(change: RoutingTickChange) -> &'static str {
    match change {
        RoutingTickChange::NoChange => "no-change",
        RoutingTickChange::PrivateStateUpdated => "private-state-updated",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::{
        tuning_babel_equivalence_smoke_suite, tuning_babel_model_smoke_suite,
        tuning_batman_bellman_model_smoke_suite, tuning_batman_classic_model_smoke_suite,
        tuning_field_model_smoke_suite, tuning_olsrv2_model_smoke_suite,
        tuning_pathway_model_smoke_suite, tuning_scatter_model_smoke_suite, ReferenceClientAdapter,
    };

    static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn temp_output_dir(label: &str) -> std::path::PathBuf {
        let suffix = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("jacquard-{label}-{}-{suffix}", std::process::id()))
    }

    fn remove_temp_output_dir(output_dir: &std::path::Path) {
        // allow-ignored-result: temp test-artifact cleanup must not hide model-lane assertion failures.
        let _ = std::fs::remove_dir_all(output_dir);
    }

    #[test]
    fn babel_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("babel-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_babel_model_smoke_suite(),
            &output_dir,
        )
        .expect("babel model smoke suite should run");

        assert!(artifacts.manifest.model_artifact_count >= 3);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model", "model", "model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn babel_equivalence_smoke_suite_records_passing_equivalence() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("babel-equivalence-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_babel_equivalence_smoke_suite(),
            &output_dir,
        )
        .expect("babel equivalence smoke suite should run");

        assert!(artifacts.manifest.model_artifact_count >= 4);
        assert!(artifacts
            .runs
            .iter()
            .all(|run| run.equivalence_passed == Some(true)));

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn field_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("field-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_field_model_smoke_suite(),
            &output_dir,
        )
        .expect("field model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn pathway_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("pathway-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_pathway_model_smoke_suite(),
            &output_dir,
        )
        .expect("pathway model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn batman_bellman_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("batman-bellman-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_batman_bellman_model_smoke_suite(),
            &output_dir,
        )
        .expect("batman bellman model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn batman_classic_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("batman-classic-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_batman_classic_model_smoke_suite(),
            &output_dir,
        )
        .expect("batman classic model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn olsrv2_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("olsrv2-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_olsrv2_model_smoke_suite(),
            &output_dir,
        )
        .expect("olsrv2 model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }

    #[test]
    fn scatter_model_smoke_suite_writes_model_artifacts() {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let output_dir = temp_output_dir("scatter-model-smoke");
        let artifacts = run_suite(
            &mut simulator,
            &tuning_scatter_model_smoke_suite(),
            &output_dir,
        )
        .expect("scatter model smoke suite should run");

        assert_eq!(artifacts.manifest.model_artifact_count, 1);
        assert_eq!(
            artifacts
                .runs
                .iter()
                .map(|run| run.execution_lane.as_str())
                .collect::<Vec<_>>(),
            vec!["model"]
        );

        remove_temp_output_dir(&output_dir);
    }
}
