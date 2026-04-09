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

mod environment;
mod harness;
mod replay;
mod scenario;

pub mod presets;

pub use environment::{
    AppliedEnvironmentHook, EnvironmentHook, ScheduledEnvironmentHook,
    ScriptedEnvironmentModel,
};
pub use harness::{
    JacquardHostAdapter, JacquardSimulationHarness, JacquardSimulator,
    ReferenceClientAdapter, SimulationError,
};
pub use replay::{
    DriverStatusEvent, HostCheckpointSnapshot, HostRoundArtifact, HostRoundStatus,
    IngressBatchBoundary, JacquardCheckpointArtifact, JacquardReplayArtifact,
    JacquardRoundArtifact, JacquardSimulationStats, SimulationFailureSummary,
    TelltaleNativeArtifactRef,
};
pub use scenario::{BoundObjective, EngineLane, HostSpec, JacquardScenario};
