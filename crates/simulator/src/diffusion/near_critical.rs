//! Simulator-local near-critical control and potential accounting.
// proc-macro-scope: near-critical artifact rows use serde derives for replay schema, not shared model macros.

#![allow(dead_code)]

mod artifacts;
mod controller;
mod potential;
mod reproduction;
mod sweep;
mod theory;

#[allow(unused_imports)]
pub(crate) use artifacts::{
    near_critical_artifact_rows, NearCriticalArtifactBundle, NearCriticalRoundArtifact,
    NearCriticalSummaryArtifact,
};
#[allow(unused_imports)]
pub(crate) use controller::{
    decide_near_critical_controller, NearCriticalCapState, NearCriticalControllerConfig,
    NearCriticalControllerDecision, NearCriticalControllerError, NearCriticalControllerMode,
    NearCriticalOpportunityState, NearCriticalResourceUsage,
};
#[allow(unused_imports)]
pub(crate) use potential::{
    compute_w_diff, compute_w_infer, summarize_potential_trace, DiffusionPotentialInput,
    DiffusionPotentialRecord, DiffusionPotentialWeights, InferencePotentialInput,
    InferencePotentialRecord, InferencePotentialWeights, PotentialTraceSummary,
};
#[allow(unused_imports)]
pub(crate) use reproduction::{
    reproduction_pressure_from_trace, ReproductionPressureEvent, ReproductionPressureSummary,
    RollingReproductionPressure,
};
#[allow(unused_imports)]
pub(crate) use sweep::{
    run_near_critical_sweep, ControllerModeKind, NearCriticalSweepArtifact, NearCriticalSweepCell,
    NearCriticalSweepRegion,
};
#[allow(unused_imports)]
pub(crate) use theory::{
    run_near_critical_theory_fixtures, NearCriticalTheoryFixture, NearCriticalTheoryFixtureKind,
};
