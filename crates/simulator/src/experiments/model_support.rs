//! Model-lane fixture execution for route-visible experiment runs.

use std::collections::BTreeMap;

use jacquard_babel::{
    admit_route_from_snapshot as admit_babel_route_from_snapshot,
    candidate_routes_from_snapshot as babel_candidate_routes_from_snapshot,
    simulator_support::{
        reduce_round_view as reduce_babel_round_view,
        restore_route_view as restore_babel_route_view,
    },
    BabelPlannerSnapshot,
};
use jacquard_batman_bellman::{
    admit_route_from_snapshot as admit_batman_bellman_route_from_snapshot,
    candidate_routes_from_snapshot as batman_bellman_candidate_routes_from_snapshot,
    BatmanBellmanPlannerSnapshot, BestNextHop as BatmanBellmanBestNextHop,
    DecayWindow as BatmanBellmanDecayWindow,
};
use jacquard_batman_classic::{
    admit_route_from_snapshot as admit_batman_classic_route_from_snapshot,
    candidate_routes_from_snapshot as batman_classic_candidate_routes_from_snapshot,
    BatmanClassicPlannerSnapshot, BestNextHop as BatmanClassicBestNextHop,
    DecayWindow as BatmanClassicDecayWindow,
};
use jacquard_core::{
    DestinationId, NodeId, RatioPermille, RouteDegradation, RouteError, RouteSelectionError,
    RoutingTickChange, Tick, TransportError, TransportKind,
};
use jacquard_field::simulator_support::validate_planner_decision as run_field_planner_decision_fixture;
use jacquard_olsrv2::{
    admit_route_from_snapshot as admit_olsr_route_from_snapshot,
    candidate_routes_from_snapshot as olsr_candidate_routes_from_snapshot,
    DecayWindow as OlsrDecayWindow, OlsrBestNextHop, OlsrPlannerSnapshot,
};
use jacquard_pathway::{
    first_hop_node_id_from_backend_route_id, DeterministicPathwayTopologyModel, PathwayEngine,
};
use jacquard_scatter::{ScatterEngine, ScatterEngineConfig};
use jacquard_traits::{
    effect_handler, Blake3Hashing, RoutingEnginePlanner, TimeEffects, TransportSenderEffects,
};

use super::{
    BabelCheckpointRestoreCase, BabelPlannerDecisionCase, BabelRoundRefreshCase,
    BatmanBellmanPlannerDecisionCase, BatmanClassicPlannerDecisionCase, ExperimentError,
    ExperimentModelArtifact, ExperimentModelCase, ExperimentRunSpec, FieldPlannerDecisionCase,
    OlsrPlannerDecisionCase, PathwayPlannerDecisionCase, ScatterPlannerDecisionCase,
};
pub(super) struct VisibilityExpectation {
    pub(super) owner_node_id: NodeId,
    pub(super) destination: DestinationId,
    pub(super) visible_round: u32,
}

pub(super) struct ModelExecution {
    pub(super) artifacts: Vec<ExperimentModelArtifact>,
    pub(super) expected_visibility: Option<VisibilityExpectation>,
}

struct ScatterNullTransport;

#[effect_handler]
impl TransportSenderEffects for ScatterNullTransport {
    fn send_transport(
        &mut self,
        _endpoint: &jacquard_core::LinkEndpoint,
        _payload: &[u8],
    ) -> Result<(), TransportError> {
        Ok(())
    }
}

struct ScatterFixedTime {
    now: Tick,
}

#[effect_handler]
impl TimeEffects for ScatterFixedTime {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

struct PathwayPlannerDecisionFixtureResult {
    candidate_count: usize,
    backend_route_id: jacquard_core::BackendRouteId,
    first_hop_node_id: NodeId,
    admitted: bool,
}

struct ScatterPlannerDecisionFixtureResult {
    candidate_count: usize,
    backend_route_id: jacquard_core::BackendRouteId,
    admitted: bool,
}

struct BatmanBellmanPlannerDecisionFixtureResult {
    candidate_count: usize,
    backend_route_id: jacquard_core::BackendRouteId,
    selected_neighbor: NodeId,
    admitted: bool,
}

struct BatmanClassicPlannerDecisionFixtureResult {
    candidate_count: usize,
    backend_route_id: jacquard_core::BackendRouteId,
    selected_neighbor: NodeId,
    admitted: bool,
}

struct OlsrPlannerDecisionFixtureResult {
    candidate_count: usize,
    backend_route_id: jacquard_core::BackendRouteId,
    selected_neighbor: NodeId,
    admitted: bool,
}

struct BabelPlannerDecisionFixtureResult {
    candidate_count: usize,
    backend_route_id: jacquard_core::BackendRouteId,
    admitted: bool,
}

fn route_id_bytes(destination: NodeId, next_hop: NodeId) -> jacquard_core::BackendRouteId {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(&destination.0);
    bytes.extend_from_slice(&next_hop.0);
    jacquard_core::BackendRouteId(bytes)
}

fn run_babel_planner_decision_fixture(
    snapshot: &BabelPlannerSnapshot,
    objective: &jacquard_core::RoutingObjective,
    profile: &jacquard_core::SelectedRoutingParameters,
    topology: &jacquard_core::Observation<jacquard_core::Configuration>,
) -> Result<BabelPlannerDecisionFixtureResult, RouteError> {
    let candidates = babel_candidate_routes_from_snapshot(snapshot, objective, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission =
        admit_babel_route_from_snapshot(snapshot, objective, profile, &candidate, topology)?;
    Ok(BabelPlannerDecisionFixtureResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

fn run_batman_bellman_planner_decision_fixture(
    local_node_id: NodeId,
    expected_next_hop: NodeId,
    objective: &jacquard_core::RoutingObjective,
    profile: &jacquard_core::SelectedRoutingParameters,
    topology: &jacquard_core::Observation<jacquard_core::Configuration>,
) -> Result<BatmanBellmanPlannerDecisionFixtureResult, RouteError> {
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let snapshot = BatmanBellmanPlannerSnapshot {
        local_node_id,
        stale_after_ticks: BatmanBellmanDecayWindow::default().stale_after_ticks,
        best_next_hops: BTreeMap::from([(
            destination,
            BatmanBellmanBestNextHop {
                originator: destination,
                next_hop: expected_next_hop,
                tq: RatioPermille(950),
                receive_quality: RatioPermille(950),
                hop_count: 1,
                updated_at_tick: topology.observed_at_tick,
                transport_kind: TransportKind::WifiAware,
                degradation: RouteDegradation::None,
                backend_route_id: route_id_bytes(destination, expected_next_hop),
                topology_epoch: topology.value.epoch,
                is_bidirectional: true,
            },
        )]),
    };
    let candidates = batman_bellman_candidate_routes_from_snapshot(&snapshot, objective, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = admit_batman_bellman_route_from_snapshot(
        &snapshot, objective, profile, &candidate, topology,
    )?;
    Ok(BatmanBellmanPlannerDecisionFixtureResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        selected_neighbor: expected_next_hop,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

fn run_batman_classic_planner_decision_fixture(
    local_node_id: NodeId,
    expected_next_hop: NodeId,
    objective: &jacquard_core::RoutingObjective,
    profile: &jacquard_core::SelectedRoutingParameters,
    topology: &jacquard_core::Observation<jacquard_core::Configuration>,
) -> Result<BatmanClassicPlannerDecisionFixtureResult, RouteError> {
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let snapshot = BatmanClassicPlannerSnapshot {
        local_node_id,
        stale_after_ticks: BatmanClassicDecayWindow::default().stale_after_ticks,
        best_next_hops: BTreeMap::from([(
            destination,
            BatmanClassicBestNextHop {
                originator: destination,
                next_hop: expected_next_hop,
                tq: RatioPermille(950),
                receive_quality: RatioPermille(950),
                hop_count: 1,
                updated_at_tick: topology.observed_at_tick,
                transport_kind: TransportKind::WifiAware,
                degradation: RouteDegradation::None,
                backend_route_id: route_id_bytes(destination, expected_next_hop),
                topology_epoch: topology.value.epoch,
                is_bidirectional: true,
            },
        )]),
    };
    let candidates = batman_classic_candidate_routes_from_snapshot(&snapshot, objective, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = admit_batman_classic_route_from_snapshot(
        &snapshot, objective, profile, &candidate, topology,
    )?;
    Ok(BatmanClassicPlannerDecisionFixtureResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        selected_neighbor: expected_next_hop,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

fn run_olsr_planner_decision_fixture(
    local_node_id: NodeId,
    expected_next_hop: NodeId,
    objective: &jacquard_core::RoutingObjective,
    profile: &jacquard_core::SelectedRoutingParameters,
    topology: &jacquard_core::Observation<jacquard_core::Configuration>,
) -> Result<OlsrPlannerDecisionFixtureResult, RouteError> {
    let DestinationId::Node(destination) = objective.destination else {
        return Err(RouteSelectionError::NoCandidate.into());
    };
    let path_cost = 10;
    let snapshot = OlsrPlannerSnapshot {
        local_node_id,
        stale_after_ticks: OlsrDecayWindow::default().stale_after_ticks,
        best_next_hops: BTreeMap::from([(
            destination,
            OlsrBestNextHop {
                destination,
                next_hop: expected_next_hop,
                hop_count: 1,
                path_cost,
                degradation: RouteDegradation::None,
                transport_kind: TransportKind::WifiAware,
                updated_at_tick: topology.observed_at_tick,
                topology_epoch: topology.value.epoch,
                backend_route_id: jacquard_core::BackendRouteId(
                    [
                        destination.0.as_slice(),
                        expected_next_hop.0.as_slice(),
                        path_cost.to_le_bytes().as_slice(),
                    ]
                    .concat(),
                ),
            },
        )]),
    };
    let candidates = olsr_candidate_routes_from_snapshot(&snapshot, objective, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission =
        admit_olsr_route_from_snapshot(&snapshot, objective, profile, &candidate, topology)?;
    Ok(OlsrPlannerDecisionFixtureResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        selected_neighbor: expected_next_hop,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

fn run_pathway_planner_decision_fixture(
    local_node_id: NodeId,
    objective: &jacquard_core::RoutingObjective,
    profile: &jacquard_core::SelectedRoutingParameters,
    topology: &jacquard_core::Observation<jacquard_core::Configuration>,
) -> Result<PathwayPlannerDecisionFixtureResult, RouteError> {
    let engine = PathwayEngine::without_committee_selector(
        local_node_id,
        DeterministicPathwayTopologyModel::new(),
        (),
        (),
        (),
        Blake3Hashing,
    );
    let candidates = engine.candidate_routes(objective, profile, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = engine.admit_route(objective, profile, candidate.clone(), topology)?;
    let first_hop_node_id =
        first_hop_node_id_from_backend_route_id(&candidate.backend_ref.backend_route_id)
            .ok_or(RouteSelectionError::NoCandidate)?;
    Ok(PathwayPlannerDecisionFixtureResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        first_hop_node_id,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
}

fn run_scatter_planner_decision_fixture(
    local_node_id: NodeId,
    objective: &jacquard_core::RoutingObjective,
    profile: &jacquard_core::SelectedRoutingParameters,
    topology: &jacquard_core::Observation<jacquard_core::Configuration>,
) -> Result<ScatterPlannerDecisionFixtureResult, RouteError> {
    let engine = ScatterEngine::with_config(
        local_node_id,
        ScatterNullTransport,
        ScatterFixedTime {
            now: topology.observed_at_tick,
        },
        ScatterEngineConfig::default(),
    );
    let candidates = engine.candidate_routes(objective, profile, topology);
    let candidate = candidates
        .first()
        .cloned()
        .ok_or(RouteSelectionError::NoCandidate)?;
    let admission = engine.admit_route(objective, profile, candidate.clone(), topology)?;
    Ok(ScatterPlannerDecisionFixtureResult {
        candidate_count: candidates.len(),
        backend_route_id: candidate.backend_ref.backend_route_id,
        admitted: matches!(
            admission.admission_check.decision,
            jacquard_core::AdmissionDecision::Admissible
        ),
    })
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
    let expected_backend_route_id = route_id_bytes(case.destination, case.expected_next_hop);
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
    let output = reduce_babel_round_view(&case.prior_state, &case.input);
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
                bytes_to_hex(&route_id_bytes(*destination, *next_hop).0)
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
    let restored = restore_babel_route_view(&case.route).ok_or_else(|| {
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
