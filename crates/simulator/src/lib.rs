//! Deterministic Jacquard simulator harness.
//!
//! This crate reuses the existing Jacquard host bridge, adapter helpers, and
//! in-memory transport surfaces to run deterministic multi-host routing
//! scenarios. It models its top-level integration after Telltale's simulator
//! harness shape:
//! - a pure scenario description
//! - a pure environment model
//! - a host adapter that builds runnable hosts
//! - one effectful harness that executes deterministic rounds and emits replay
//!   artifacts
//!
//! The simulator is intentionally two-lane:
//! - `pathway` scenarios are Telltale-backed in the sense that the engine's own
//!   runtime is choreography-driven
//! - `batman` scenarios stay plain deterministic state machines
//! - mixed scenarios host both lanes under the same router/bridge ownership
//!   model
//!
//! Starter path:
//! 1. Build a [`JacquardScenario`] from one of the presets in [`presets`].
//! 2. Pair it with a [`ScriptedEnvironmentModel`].
//! 3. Run it through [`JacquardSimulator`].

#![forbid(unsafe_code)]

mod assertions;
mod diffusion;
mod environment;
mod experiments;
mod harness;
mod model;
mod reduced_replay;
mod replay;
mod scenario;
mod topology;
mod util;

pub mod presets;

pub use assertions::{AssertionFailure, ScenarioAssertions};
pub use diffusion::{
    diffusion_local_suite, diffusion_smoke_suite, run_diffusion_suite, DiffusionAggregateSummary,
    DiffusionArtifacts, DiffusionBoundarySummary, DiffusionForwardingStyle, DiffusionManifest,
    DiffusionPolicyConfig, DiffusionRegimeDescriptor, DiffusionRunSummary, DiffusionSuite,
};
pub use environment::{
    AppliedEnvironmentHook, EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel,
};
pub use experiments::{
    babel_equivalence_smoke_suite as tuning_babel_equivalence_smoke_suite,
    babel_model_smoke_suite as tuning_babel_model_smoke_suite,
    batman_bellman_model_smoke_suite as tuning_batman_bellman_model_smoke_suite,
    batman_classic_model_smoke_suite as tuning_batman_classic_model_smoke_suite,
    field_model_smoke_suite as tuning_field_model_smoke_suite, local_suite as tuning_local_suite,
    olsrv2_model_smoke_suite as tuning_olsrv2_model_smoke_suite,
    pathway_model_smoke_suite as tuning_pathway_model_smoke_suite, run_suite as run_tuning_suite,
    scatter_model_smoke_suite as tuning_scatter_model_smoke_suite,
    smoke_suite as tuning_smoke_suite, ExperimentAggregateSummary, ExperimentArtifacts,
    ExperimentBreakdownSummary, ExperimentError, ExperimentManifest, ExperimentModelArtifact,
    ExperimentParameterSet, ExperimentRunSummary, ExperimentSuite, RegimeDescriptor,
};
pub use harness::{
    JacquardHostAdapter, JacquardSimulationHarness, JacquardSimulator, ReferenceClientAdapter,
    SimulationCaptureArtifact, SimulationCaptureLevel, SimulationError,
};
pub use model::{
    run_checkpoint_fixture, run_maintenance_transition_fixture, run_planner_fixture,
    run_round_transition_fixture, CheckpointFixture, MaintenanceTransitionFixture, PlannerModelRun,
    PlannerSnapshotFixture, RoundTransitionFixture, SimulationExecutionLane, TransitionModelRun,
};
pub use reduced_replay::{
    ReducedEnvironmentHookCounts, ReducedFailureClassCounts, ReducedReplayRound, ReducedReplayView,
    ReducedRouteKey, ReducedRouteObservation,
};
pub use replay::{
    ActiveRouteSummary, DriverStatusEvent, HostCheckpointSnapshot, HostRoundArtifact,
    HostRoundStatus, IngressBatchBoundary, JacquardCheckpointArtifact, JacquardReplayArtifact,
    JacquardRoundArtifact, JacquardSimulationStats, SimulationFailureSummary,
    TelltaleNativeArtifactRef,
};
pub use scenario::{BoundObjective, EngineLane, FieldBootstrapSummary, HostSpec, JacquardScenario};
