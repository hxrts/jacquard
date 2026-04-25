//! Deterministic Jacquard simulator harness.
//!
//! This crate reuses the existing Jacquard host bridge, host-support helpers, and
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
mod cast;
mod diffusion;
mod environment;
mod experiments;
mod external;
mod harness;
mod model;
mod reduced_replay;
mod replay;
mod scenario;
mod topology;
mod util;

pub(crate) const LEGACY_FIELD_ENGINE_ID: jacquard_core::RoutingEngineId =
    jacquard_core::RoutingEngineId::from_contract_bytes(*b"jacquard.field..");

pub mod presets;

pub mod builtin_suites {
    pub use crate::diffusion::{
        diffusion_local_stage_suite, diffusion_local_suite, diffusion_smoke_suite,
    };
    pub use crate::experiments::{
        babel_equivalence_smoke_suite as tuning_babel_equivalence_smoke_suite,
        babel_model_smoke_suite as tuning_babel_model_smoke_suite,
        batman_bellman_model_smoke_suite as tuning_batman_bellman_model_smoke_suite,
        batman_classic_model_smoke_suite as tuning_batman_classic_model_smoke_suite,
        local_stage_suite as tuning_local_stage_suite,
        local_stage_suite_with_seeds as tuning_local_stage_suite_with_seeds,
        local_stage_suite_with_seeds_and_config as tuning_local_stage_suite_with_seeds_and_config,
        local_suite as tuning_local_suite,
        olsrv2_model_smoke_suite as tuning_olsrv2_model_smoke_suite,
        pathway_model_smoke_suite as tuning_pathway_model_smoke_suite,
        scatter_model_smoke_suite as tuning_scatter_model_smoke_suite,
        smoke_suite as tuning_smoke_suite,
    };
}

pub use assertions::{AssertionFailure, ScenarioAssertions};
pub use cast::{
    broadcast_cast_evidence_scenario, cast_report_surface_decision,
    multicast_cast_evidence_scenario, unicast_cast_evidence_scenario, CastEvidenceScenarioKind,
    CastEvidenceScenarioOutcome,
};
pub use diffusion::{
    active_belief_artifact_contract, aggregate_diffusion_runs, diffusion_local_stage_suite,
    diffusion_local_suite, diffusion_smoke_suite, run_diffusion_suite,
    summarize_diffusion_boundaries, CustomDiffusionRunSpec, CustomDiffusionScenarioSpec,
    DiffusionAggregateSummary, DiffusionArtifacts, DiffusionBoundarySummary,
    DiffusionForwardingStyle, DiffusionManifest, DiffusionMessageMode, DiffusionMobilityProfile,
    DiffusionNodeSpec, DiffusionPolicyConfig, DiffusionRegimeDescriptor, DiffusionRunSummary,
    DiffusionSuite, DiffusionSuiteBuildError, DiffusionTransportKind,
    PaperExperimentArtifactContract, ACTIVE_BELIEF_REQUIRED_CSV_FILES,
    DIFFUSION_ARTIFACT_SCHEMA_VERSION,
};
pub use environment::{
    AppliedEnvironmentHook, EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel,
};
pub use experiments::{
    aggregate_runs as aggregate_tuning_runs,
    babel_equivalence_smoke_suite as tuning_babel_equivalence_smoke_suite,
    babel_model_smoke_suite as tuning_babel_model_smoke_suite,
    batman_bellman_model_smoke_suite as tuning_batman_bellman_model_smoke_suite,
    batman_classic_model_smoke_suite as tuning_batman_classic_model_smoke_suite,
    local_stage_suite as tuning_local_stage_suite,
    local_stage_suite_with_seeds as tuning_local_stage_suite_with_seeds,
    local_stage_suite_with_seeds_and_config as tuning_local_stage_suite_with_seeds_and_config,
    local_suite as tuning_local_suite, olsrv2_model_smoke_suite as tuning_olsrv2_model_smoke_suite,
    pathway_model_smoke_suite as tuning_pathway_model_smoke_suite, run_suite as run_tuning_suite,
    scatter_model_smoke_suite as tuning_scatter_model_smoke_suite,
    smoke_suite as tuning_smoke_suite, summarize_breakdowns as summarize_tuning_breakdowns,
    ExperimentAggregateSummary, ExperimentArtifacts, ExperimentBreakdownSummary, ExperimentError,
    ExperimentManifest, ExperimentModelArtifact, ExperimentParameterSet, ExperimentRunSummary,
    ExperimentSuite, RegimeDescriptor, ROUTE_VISIBLE_ARTIFACT_SCHEMA_VERSION,
};
pub use external::{
    ArtifactSink, EngineRegistry, EngineRegistryEntry, EngineRouteShape, ExperimentRunner,
    ExperimentSuiteSpec, ExternalExperimentError, ExternalExperimentManifest,
    RouteVisibleArtifacts, RouteVisibleRunSpec, RouteVisibleRunSummary, SimulatorConfig,
    EXTERNAL_ROUTE_ARTIFACT_SCHEMA_VERSION,
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
pub use scenario::{BoundObjective, EngineLane, HostSpec, JacquardScenario};
