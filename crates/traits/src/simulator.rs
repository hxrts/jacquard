//! Simulator-facing contract boundaries.
//!
//! Effect boundary:
//! - `RoutingScenario` is a pure scenario-description surface.
//! - `RoutingEnvironmentModel` is a pure deterministic environment-evolution
//!   surface over shared routing world objects.
//! - `RoutingSimulator` is the effectful harness boundary that executes,
//!   checkpoints, resumes, and emits replay-visible artifacts.
//! - `RoutingReplayView` is a read-only artifact inspection surface.

use jacquard_core::{
    Configuration, DeploymentProfile, Observation, RouteEvent, RoutingAuditEvent, RoutingObjective,
    Tick,
};

/// Pure scenario description for a deterministic routing run.
///
/// Pure deterministic boundary.
pub trait RoutingScenario {
    fn name(&self) -> &str;

    fn seed(&self) -> u64;

    fn deployment_profile(&self) -> &DeploymentProfile;

    fn initial_configuration(&self) -> &Observation<Configuration>;

    fn objectives(&self) -> &[RoutingObjective];
}

/// Pure deterministic environment evolution over the shared world model.
///
/// Pure deterministic boundary.
pub trait RoutingEnvironmentModel {
    type EnvironmentArtifact;

    fn advance_environment(
        &self,
        configuration: &Configuration,
        at_tick: Tick,
    ) -> (Observation<Configuration>, Vec<Self::EnvironmentArtifact>);
}

/// Effectful routing simulation harness.
///
/// Effectful runtime boundary.
pub trait RoutingSimulator {
    type Scenario: RoutingScenario;
    type EnvironmentModel: RoutingEnvironmentModel;
    type ReplayArtifact;
    type SimulationStats;
    type Error;

    fn run_scenario(
        &mut self,
        scenario: &Self::Scenario,
        environment: &Self::EnvironmentModel,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error>;

    fn resume_replay(
        &mut self,
        replay: &Self::ReplayArtifact,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error>;
}

/// Read-only inspection surface for replay-visible simulator artifacts.
///
/// Read-only deterministic boundary.
pub trait RoutingReplayView {
    type ReplayArtifact;

    fn route_events<'a>(&self, replay: &'a Self::ReplayArtifact) -> &'a [RouteEvent];

    fn audit_events<'a>(&self, replay: &'a Self::ReplayArtifact) -> &'a [RoutingAuditEvent];
}
