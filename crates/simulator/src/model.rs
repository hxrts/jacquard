//! Model-lane fixture vocabulary for simulator work.
//!
//! These types describe the pure-lane harness inputs the simulator will use as
//! engines adopt explicit planner snapshots and pure reducers. They are generic
//! on engine-private state so the simulator can exercise real engine logic
//! without forcing those internal types into shared crates.

use jacquard_core::{
    Configuration, Observation, RouteEpoch, RoutingObjective, SelectedRoutingParameters,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SimulationExecutionLane {
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "full-stack")]
    FullStack,
    #[serde(rename = "equivalence")]
    Equivalence,
}

impl SimulationExecutionLane {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::FullStack => "full-stack",
            Self::Equivalence => "equivalence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerSnapshotFixture<Snapshot> {
    pub fixture_id: String,
    pub objective: RoutingObjective,
    pub profile: SelectedRoutingParameters,
    pub topology: Observation<Configuration>,
    pub snapshot: Snapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundTransitionFixture<State, Input, Output> {
    pub fixture_id: String,
    pub topology_epoch: RouteEpoch,
    pub prior_state: State,
    pub normalized_input: Input,
    pub expected_output: Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaintenanceTransitionFixture<State, Input, Output> {
    pub fixture_id: String,
    pub topology_epoch: RouteEpoch,
    pub prior_state: State,
    pub normalized_input: Input,
    pub expected_output: Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointFixture<Checkpoint, State, Snapshot> {
    pub fixture_id: String,
    pub checkpoint_state: Checkpoint,
    pub restored_state: State,
    pub restored_snapshot: Snapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerModelRun<Candidate, Score> {
    pub fixture_id: String,
    pub lane: SimulationExecutionLane,
    pub candidates: Vec<Candidate>,
    pub scores: Vec<Score>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionModelRun<Output> {
    pub fixture_id: String,
    pub lane: SimulationExecutionLane,
    pub output: Output,
}

#[must_use = "the planner model run carries candidate and scoring results used by simulator assertions"]
pub fn run_planner_fixture<Snapshot, Candidate, Score>(
    fixture: &PlannerSnapshotFixture<Snapshot>,
    candidate_fn: impl FnOnce(
        &RoutingObjective,
        &SelectedRoutingParameters,
        &Observation<Configuration>,
        &Snapshot,
    ) -> Vec<Candidate>,
    score_fn: impl Fn(&Candidate, &Snapshot) -> Score,
) -> PlannerModelRun<Candidate, Score> {
    let candidates = candidate_fn(
        &fixture.objective,
        &fixture.profile,
        &fixture.topology,
        &fixture.snapshot,
    );
    let scores = candidates
        .iter()
        .map(|candidate| score_fn(candidate, &fixture.snapshot))
        .collect();
    PlannerModelRun {
        fixture_id: fixture.fixture_id.clone(),
        lane: SimulationExecutionLane::Model,
        candidates,
        scores,
    }
}

#[must_use = "the round transition model run carries reducer output used by simulator assertions"]
pub fn run_round_transition_fixture<State, Input, Output>(
    fixture: &RoundTransitionFixture<State, Input, Output>,
    reducer: impl FnOnce(&State, &Input) -> Output,
) -> TransitionModelRun<Output> {
    TransitionModelRun {
        fixture_id: fixture.fixture_id.clone(),
        lane: SimulationExecutionLane::Model,
        output: reducer(&fixture.prior_state, &fixture.normalized_input),
    }
}

#[must_use = "the maintenance transition model run carries reducer output used by simulator assertions"]
pub fn run_maintenance_transition_fixture<State, Input, Output>(
    fixture: &MaintenanceTransitionFixture<State, Input, Output>,
    reducer: impl FnOnce(&State, &Input) -> Output,
) -> TransitionModelRun<Output> {
    TransitionModelRun {
        fixture_id: fixture.fixture_id.clone(),
        lane: SimulationExecutionLane::Model,
        output: reducer(&fixture.prior_state, &fixture.normalized_input),
    }
}

#[must_use = "the checkpoint model run carries restored state used by simulator assertions"]
pub fn run_checkpoint_fixture<Checkpoint, State, Snapshot>(
    fixture: &CheckpointFixture<Checkpoint, State, Snapshot>,
    restore_fn: impl FnOnce(&Checkpoint) -> (State, Snapshot),
) -> TransitionModelRun<(State, Snapshot)> {
    TransitionModelRun {
        fixture_id: fixture.fixture_id.clone(),
        lane: SimulationExecutionLane::Model,
        output: restore_fn(&fixture.checkpoint_state),
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        Configuration, ConnectivityPosture, DestinationId, Environment, FactSourceClass, Limit,
        NodeId, Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteEpoch,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RoutingEvidenceClass, RoutingObjective, SelectedRoutingParameters, Tick,
    };

    use super::{
        run_checkpoint_fixture, run_maintenance_transition_fixture, run_planner_fixture,
        run_round_transition_fixture, CheckpointFixture, MaintenanceTransitionFixture,
        PlannerSnapshotFixture, RoundTransitionFixture, SimulationExecutionLane,
    };

    fn objective() -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(NodeId([9; 32])),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Forbidden,
            latency_budget_ms: Limit::Bounded(jacquard_core::DurationMs(100)),
            protection_priority: PriorityPoints(10),
            connectivity_priority: PriorityPoints(10),
        }
    }

    fn profile() -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            deployment_profile: jacquard_core::OperatingMode::SparseLowPower,
            diversity_floor: jacquard_core::DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn topology() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(7),
                nodes: std::collections::BTreeMap::new(),
                links: std::collections::BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 0,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    #[test]
    fn model_lane_fixture_shapes_are_constructible() {
        let planner = PlannerSnapshotFixture {
            fixture_id: String::from("fixture"),
            objective: objective(),
            profile: profile(),
            topology: topology(),
            snapshot: 4u32,
        };
        let round = RoundTransitionFixture {
            fixture_id: String::from("round"),
            topology_epoch: RouteEpoch(7),
            prior_state: 1u8,
            normalized_input: 2u8,
            expected_output: 3u8,
        };
        let maintenance = MaintenanceTransitionFixture {
            fixture_id: String::from("maintenance"),
            topology_epoch: RouteEpoch(7),
            prior_state: 3u8,
            normalized_input: 4u8,
            expected_output: 5u8,
        };
        let checkpoint = CheckpointFixture {
            fixture_id: String::from("checkpoint"),
            checkpoint_state: 6u8,
            restored_state: 7u8,
            restored_snapshot: 8u8,
        };

        assert_eq!(
            SimulationExecutionLane::Model,
            SimulationExecutionLane::Model
        );
        assert_eq!(planner.snapshot, 4u32);
        assert_eq!(round.expected_output, 3u8);
        assert_eq!(maintenance.expected_output, 5u8);
        assert_eq!(checkpoint.restored_snapshot, 8u8);
    }

    #[test]
    fn planner_model_runner_scores_candidates_from_explicit_snapshot() {
        let fixture = PlannerSnapshotFixture {
            fixture_id: String::from("planner"),
            objective: objective(),
            profile: profile(),
            topology: topology(),
            snapshot: vec![2u8, 4u8],
        };

        let run = run_planner_fixture(
            &fixture,
            |_objective, _profile, _topology, snapshot| snapshot.clone(),
            |candidate, snapshot| {
                u16::from(*candidate) + u16::try_from(snapshot.len()).unwrap_or(0)
            },
        );

        assert_eq!(run.lane, SimulationExecutionLane::Model);
        assert_eq!(run.candidates, vec![2u8, 4u8]);
        assert_eq!(run.scores, vec![4u16, 6u16]);
    }

    #[test]
    fn transition_model_runners_execute_round_maintenance_and_restore_paths() {
        let round = RoundTransitionFixture {
            fixture_id: String::from("round"),
            topology_epoch: RouteEpoch(7),
            prior_state: 3u8,
            normalized_input: 4u8,
            expected_output: 7u8,
        };
        let maintenance = MaintenanceTransitionFixture {
            fixture_id: String::from("maintenance"),
            topology_epoch: RouteEpoch(7),
            prior_state: 5u8,
            normalized_input: 6u8,
            expected_output: 11u8,
        };
        let checkpoint = CheckpointFixture {
            fixture_id: String::from("checkpoint"),
            checkpoint_state: 9u8,
            restored_state: 10u8,
            restored_snapshot: 11u8,
        };

        let round_run = run_round_transition_fixture(&round, |state, input| *state + *input);
        let maintenance_run =
            run_maintenance_transition_fixture(&maintenance, |state, input| *state + *input);
        let checkpoint_run = run_checkpoint_fixture(&checkpoint, |checkpoint_state| {
            (*checkpoint_state + 1, *checkpoint_state + 2)
        });

        assert_eq!(round_run.output, 7u8);
        assert_eq!(maintenance_run.output, 11u8);
        assert_eq!(checkpoint_run.output, (10u8, 11u8));
    }
}
