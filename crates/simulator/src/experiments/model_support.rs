//! Model-lane fixture execution for route-visible experiment runs.

use jacquard_babel::{
    selected_neighbor_from_backend_route_id as selected_babel_neighbor, BabelEngine,
    BabelPlannerModel, BABEL_ENGINE_ID,
};
use jacquard_batman_bellman::{
    selected_neighbor_from_backend_route_id as selected_batman_bellman_neighbor,
    BatmanBellmanPlannerModel, BATMAN_BELLMAN_ENGINE_ID,
};
use jacquard_batman_classic::{
    selected_neighbor_from_backend_route_id as selected_batman_classic_neighbor,
    BatmanClassicPlannerModel, BATMAN_CLASSIC_ENGINE_ID,
};
use jacquard_core::{
    BackendRouteId, Configuration, DestinationId, NodeId, Observation, RouteAdmission,
    RouteCandidate, RouteError, RouteSelectionError, RoutingEngineId, RoutingObjective,
    RoutingTickChange, SelectedRoutingParameters,
};
use jacquard_olsrv2::{
    selected_neighbor_from_backend_route_id as selected_olsr_neighbor, OlsrPlannerModel,
    OLSRV2_ENGINE_ID,
};
use jacquard_pathway::{
    first_hop_node_id_from_backend_route_id, PathwayPlannerModel, PATHWAY_ENGINE_ID,
};
use jacquard_scatter::{ScatterPlannerModel, SCATTER_ENGINE_ID};
use jacquard_traits::{
    RoutingEngineMaintenanceModel, RoutingEnginePlannerModel, RoutingEngineRestoreModel,
    RoutingEngineRoundModel,
};

use super::{
    BabelCheckpointRestoreCase, BabelMaintenanceCase, BabelRoundRefreshCase, ExperimentError,
    ExperimentModelArtifact, ExperimentModelCase, ExperimentRunSpec, MaintenanceModelCase,
    PlannerDecisionCase, PlannerModelCase, RestoreModelCase, RoundModelCase,
};

pub(super) struct VisibilityExpectation {
    pub(super) engine_id: RoutingEngineId,
    pub(super) owner_node_id: NodeId,
    pub(super) destination: DestinationId,
    pub(super) visible_round: u32,
}

pub(super) struct ModelExecution {
    pub(super) artifacts: Vec<ExperimentModelArtifact>,
    pub(super) expected_visibility: Option<VisibilityExpectation>,
}

struct PlannerCaseView<'a, Seed> {
    fixture_id: &'a str,
    owner_node_id: NodeId,
    destination: NodeId,
    expected_next_hop: Option<NodeId>,
    expected_visible_round: Option<u32>,
    objective: &'a RoutingObjective,
    profile: &'a SelectedRoutingParameters,
    topology: &'a Observation<Configuration>,
    seed: &'a Seed,
}

struct PlannerFixtureResult {
    candidate_count: usize,
    backend_route_id: BackendRouteId,
    next_hop_node_id: Option<NodeId>,
    admitted: bool,
}

struct ModelArtifactData<'a> {
    fixture_id: &'a str,
    artifact_kind: &'a str,
    owner_node_id: Option<NodeId>,
    destination_node_id: Option<NodeId>,
    next_hop_node_id: Option<NodeId>,
    topology_epoch: Option<u64>,
    candidate_count: Option<u32>,
    reducer_route_entry_count: Option<u32>,
    reducer_best_next_hop_count: Option<u32>,
    reducer_change: Option<String>,
    backend_route_id: Option<&'a BackendRouteId>,
    visible_round: Option<u32>,
    equivalence_passed: Option<bool>,
}

pub(super) fn execute_model_case(
    spec: &ExperimentRunSpec,
) -> Result<ModelExecution, ExperimentError> {
    match spec
        .model_case
        .as_ref()
        .ok_or_else(|| ExperimentError::MissingModelCase {
            run_id: spec.run_id.clone(),
        })? {
        ExperimentModelCase::Planner(case) => execute_planner_model_case(spec, case),
        ExperimentModelCase::Round(case) => execute_round_model_case(spec, case),
        ExperimentModelCase::Maintenance(case) => execute_maintenance_model_case(spec, case),
        ExperimentModelCase::Restore(case) => execute_restore_model_case(spec, case),
    }
}

// long-block-exception: planner model dispatch mirrors the shared planner-case
// enum one-to-one, and splitting the match further would obscure that mapping.
fn execute_planner_model_case(
    spec: &ExperimentRunSpec,
    case: &PlannerModelCase,
) -> Result<ModelExecution, ExperimentError> {
    match case {
        PlannerModelCase::BatmanBellman(case) => {
            execute_expected_next_hop_planner_case::<BatmanBellmanPlannerModel, _>(
                spec,
                BATMAN_BELLMAN_ENGINE_ID,
                &expected_next_hop_case_view(case),
                selected_batman_bellman_neighbor,
            )
        }
        PlannerModelCase::BatmanClassic(case) => {
            execute_expected_next_hop_planner_case::<BatmanClassicPlannerModel, _>(
                spec,
                BATMAN_CLASSIC_ENGINE_ID,
                &expected_next_hop_case_view(case),
                selected_batman_classic_neighbor,
            )
        }
        PlannerModelCase::Babel(case) => {
            execute_expected_next_hop_planner_case::<BabelPlannerModel, _>(
                spec,
                BABEL_ENGINE_ID,
                &expected_next_hop_case_view(case),
                selected_babel_neighbor,
            )
        }
        PlannerModelCase::Olsr(case) => {
            execute_expected_next_hop_planner_case::<OlsrPlannerModel, _>(
                spec,
                OLSRV2_ENGINE_ID,
                &expected_next_hop_case_view(case),
                selected_olsr_neighbor,
            )
        }
        PlannerModelCase::Pathway(case) => {
            execute_expected_next_hop_planner_case::<PathwayPlannerModel, _>(
                spec,
                PATHWAY_ENGINE_ID,
                &expected_next_hop_case_view(case),
                first_hop_node_id_from_backend_route_id,
            )
        }
        PlannerModelCase::Scatter(case) => execute_planner_case::<ScatterPlannerModel, _>(
            spec,
            SCATTER_ENGINE_ID,
            &planner_case_view(case),
            |_| None,
        ),
    }
}

fn execute_round_model_case(
    spec: &ExperimentRunSpec,
    case: &RoundModelCase,
) -> Result<ModelExecution, ExperimentError> {
    match case {
        RoundModelCase::Babel(case) => execute_babel_round_case(spec, case),
    }
}

fn execute_restore_model_case(
    spec: &ExperimentRunSpec,
    case: &RestoreModelCase,
) -> Result<ModelExecution, ExperimentError> {
    match case {
        RestoreModelCase::Babel(case) => execute_babel_restore_case(spec, case),
    }
}

fn execute_maintenance_model_case(
    spec: &ExperimentRunSpec,
    case: &MaintenanceModelCase,
) -> Result<ModelExecution, ExperimentError> {
    match case {
        MaintenanceModelCase::Babel(case) => execute_babel_maintenance_case(spec, case),
    }
}

fn execute_expected_next_hop_planner_case<Model, Decode>(
    spec: &ExperimentRunSpec,
    engine_id: RoutingEngineId,
    case: &PlannerCaseView<'_, Model::PlannerSnapshot>,
    decode_next_hop: Decode,
) -> Result<ModelExecution, ExperimentError>
where
    Model: RoutingEnginePlannerModel<
        PlannerCandidate = RouteCandidate,
        PlannerAdmission = RouteAdmission,
    >,
    Decode: Fn(&BackendRouteId) -> Option<NodeId>,
{
    let result = run_planner_fixture::<Model, _>(case, decode_next_hop).map_err(|error| {
        ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: format!("planner fixture failed: {error}"),
        }
    })?;
    let expected_next_hop =
        case.expected_next_hop
            .ok_or_else(|| ExperimentError::ModelExpectationFailed {
                run_id: spec.run_id.clone(),
                detail: "planner fixture is missing an expected next hop".to_string(),
            })?;
    if !result.admitted || result.next_hop_node_id != Some(expected_next_hop) {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "planner fixture produced the wrong next hop".to_string(),
        });
    }
    Ok(planner_execution(spec, engine_id, case, &result))
}

fn execute_planner_case<Model, Decode>(
    spec: &ExperimentRunSpec,
    engine_id: RoutingEngineId,
    case: &PlannerCaseView<'_, Model::PlannerSnapshot>,
    decode_next_hop: Decode,
) -> Result<ModelExecution, ExperimentError>
where
    Model: RoutingEnginePlannerModel<
        PlannerCandidate = RouteCandidate,
        PlannerAdmission = RouteAdmission,
    >,
    Decode: Fn(&BackendRouteId) -> Option<NodeId>,
{
    let result = run_planner_fixture::<Model, _>(case, decode_next_hop).map_err(|error| {
        ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: format!("planner fixture failed: {error}"),
        }
    })?;
    if !result.admitted {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "planner fixture candidate was not admissible".to_string(),
        });
    }
    Ok(planner_execution(spec, engine_id, case, &result))
}

fn run_planner_fixture<Model, Decode>(
    case: &PlannerCaseView<'_, Model::PlannerSnapshot>,
    decode_next_hop: Decode,
) -> Result<PlannerFixtureResult, RouteError>
where
    Model: RoutingEnginePlannerModel<
        PlannerCandidate = RouteCandidate,
        PlannerAdmission = RouteAdmission,
    >,
    Decode: Fn(&BackendRouteId) -> Option<NodeId>,
{
    let candidates = Model::candidate_routes_from_snapshot(
        case.seed,
        case.objective,
        case.profile,
        case.topology,
    );
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = Model::admit_route_from_snapshot(
        case.seed,
        case.objective,
        case.profile,
        &candidate,
        case.topology,
    )?;
    Ok(PlannerFixtureResult {
        candidate_count: candidates.len(),
        next_hop_node_id: decode_next_hop(&candidate.backend_ref.backend_route_id),
        backend_route_id: candidate.backend_ref.backend_route_id,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

fn planner_execution<Seed>(
    spec: &ExperimentRunSpec,
    engine_id: RoutingEngineId,
    case: &PlannerCaseView<'_, Seed>,
    result: &PlannerFixtureResult,
) -> ModelExecution {
    ModelExecution {
        artifacts: vec![build_model_artifact(
            spec,
            ModelArtifactData {
                fixture_id: case.fixture_id,
                artifact_kind: "planner-decision",
                owner_node_id: Some(case.owner_node_id),
                destination_node_id: Some(case.destination),
                next_hop_node_id: result.next_hop_node_id,
                topology_epoch: Some(case.topology.value.epoch.0),
                candidate_count: Some(u32::try_from(result.candidate_count).unwrap_or(u32::MAX)),
                reducer_route_entry_count: None,
                reducer_best_next_hop_count: None,
                reducer_change: None,
                backend_route_id: Some(&result.backend_route_id),
                visible_round: case.expected_visible_round,
                equivalence_passed: None,
            },
        )],
        expected_visibility: case.expected_visible_round.map(|visible_round| {
            VisibilityExpectation {
                engine_id,
                owner_node_id: case.owner_node_id,
                destination: DestinationId::Node(case.destination),
                visible_round,
            }
        }),
    }
}

fn execute_babel_round_case(
    spec: &ExperimentRunSpec,
    case: &BabelRoundRefreshCase,
) -> Result<ModelExecution, ExperimentError> {
    let output = <BabelEngine<(), ()> as RoutingEngineRoundModel>::reduce_round_state(
        &case.prior_state,
        &case.input,
    );
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
    let backend_route_id = observed.first().map(|(destination, next_hop)| {
        jacquard_babel::babel_backend_route_id(*destination, *next_hop)
    });
    Ok(ModelExecution {
        artifacts: vec![build_model_artifact(
            spec,
            ModelArtifactData {
                fixture_id: &case.fixture_id,
                artifact_kind: "round-transition",
                owner_node_id: Some(case.input.local_node_id),
                destination_node_id: observed.first().map(|(destination, _)| *destination),
                next_hop_node_id: observed.first().map(|(_, next_hop)| *next_hop),
                topology_epoch: Some(case.input.topology.value.epoch.0),
                candidate_count: None,
                reducer_route_entry_count: Some(
                    u32::try_from(case.prior_state.route_entries.len()).unwrap_or(u32::MAX),
                ),
                reducer_best_next_hop_count: Some(
                    u32::try_from(output.best_next_hop_count).unwrap_or(u32::MAX),
                ),
                reducer_change: Some(routing_tick_change_label(output.change).to_string()),
                backend_route_id: backend_route_id.as_ref(),
                visible_round: None,
                equivalence_passed: None,
            },
        )],
        expected_visibility: None,
    })
}

fn execute_babel_restore_case(
    spec: &ExperimentRunSpec,
    case: &BabelCheckpointRestoreCase,
) -> Result<ModelExecution, ExperimentError> {
    let restored =
        <BabelEngine<(), ()> as RoutingEngineRestoreModel>::restore_route_runtime(&case.route)
            .ok_or_else(|| ExperimentError::ModelExpectationFailed {
                run_id: spec.run_id.clone(),
                detail: "restore fixture did not reconstruct a Babel route".to_string(),
            })?;
    if restored.next_hop != case.expected_next_hop {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "restore fixture reconstructed the wrong next hop".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![build_model_artifact(
            spec,
            ModelArtifactData {
                fixture_id: &case.fixture_id,
                artifact_kind: "checkpoint-restore",
                owner_node_id: Some(case.owner_node_id),
                destination_node_id: Some(case.destination),
                next_hop_node_id: Some(restored.next_hop),
                topology_epoch: Some(case.route.identity.stamp.topology_epoch.0),
                candidate_count: None,
                reducer_route_entry_count: None,
                reducer_best_next_hop_count: None,
                reducer_change: None,
                backend_route_id: Some(&restored.backend_route_id),
                visible_round: Some(case.expected_visible_round),
                equivalence_passed: None,
            },
        )],
        expected_visibility: Some(VisibilityExpectation {
            engine_id: BABEL_ENGINE_ID,
            owner_node_id: case.owner_node_id,
            destination: DestinationId::Node(case.destination),
            visible_round: case.expected_visible_round,
        }),
    })
}

fn execute_babel_maintenance_case(
    spec: &ExperimentRunSpec,
    case: &BabelMaintenanceCase,
) -> Result<ModelExecution, ExperimentError> {
    let output = <BabelEngine<(), ()> as RoutingEngineMaintenanceModel>::reduce_maintenance_state(
        &case.prior_state,
        &case.input,
    );
    if output.result != case.expected_result {
        return Err(ExperimentError::ModelExpectationFailed {
            run_id: spec.run_id.clone(),
            detail: "maintenance reducer produced the wrong result".to_string(),
        });
    }
    Ok(ModelExecution {
        artifacts: vec![build_model_artifact(
            spec,
            ModelArtifactData {
                fixture_id: &case.fixture_id,
                artifact_kind: "maintenance-transition",
                owner_node_id: None,
                destination_node_id: Some(case.prior_state.active_route.destination),
                next_hop_node_id: Some(case.prior_state.active_route.next_hop),
                topology_epoch: None,
                candidate_count: None,
                reducer_route_entry_count: None,
                reducer_best_next_hop_count: Some(u32::from(
                    case.prior_state.best_next_hop.is_some(),
                )),
                reducer_change: Some(format!("{:?}", output.result.event)),
                backend_route_id: Some(&case.prior_state.active_route.backend_route_id),
                visible_round: None,
                equivalence_passed: None,
            },
        )],
        expected_visibility: None,
    })
}

fn expected_next_hop_case_view<Seed>(
    case: &super::ExpectedNextHopPlannerDecisionCase<Seed>,
) -> PlannerCaseView<'_, Seed> {
    PlannerCaseView {
        fixture_id: &case.fixture_id,
        owner_node_id: case.owner_node_id,
        destination: case.destination,
        expected_next_hop: Some(case.expected_next_hop),
        expected_visible_round: case.expected_visible_round,
        objective: &case.objective,
        profile: &case.profile,
        topology: &case.topology,
        seed: &case.seed,
    }
}

fn planner_case_view<Seed>(case: &PlannerDecisionCase<Seed>) -> PlannerCaseView<'_, Seed> {
    PlannerCaseView {
        fixture_id: &case.fixture_id,
        owner_node_id: case.owner_node_id,
        destination: case.destination,
        expected_next_hop: None,
        expected_visible_round: case.expected_visible_round,
        objective: &case.objective,
        profile: &case.profile,
        topology: &case.topology,
        seed: &case.seed,
    }
}

fn build_model_artifact(
    spec: &ExperimentRunSpec,
    data: ModelArtifactData<'_>,
) -> ExperimentModelArtifact {
    ExperimentModelArtifact {
        run_id: spec.run_id.clone(),
        suite_id: spec.suite_id.clone(),
        engine_family: spec.engine_family.clone(),
        execution_lane: spec.execution_lane.label().to_string(),
        fixture_id: data.fixture_id.to_string(),
        artifact_kind: data.artifact_kind.to_string(),
        owner_node_id: data.owner_node_id.map(node_id_hex),
        destination_node_id: data.destination_node_id.map(node_id_hex),
        next_hop_node_id: data.next_hop_node_id.map(node_id_hex),
        topology_epoch: data.topology_epoch,
        candidate_count: data.candidate_count,
        reducer_route_entry_count: data.reducer_route_entry_count,
        reducer_best_next_hop_count: data.reducer_best_next_hop_count,
        reducer_change: data.reducer_change,
        backend_route_id_hex: data.backend_route_id.map(|route| bytes_to_hex(&route.0)),
        visible_round: data.visible_round,
        equivalence_passed: data.equivalence_passed,
    }
}

fn node_id_hex(node_id: NodeId) -> String {
    bytes_to_hex(&node_id.0)
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
