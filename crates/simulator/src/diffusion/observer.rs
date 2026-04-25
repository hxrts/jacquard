//! Deterministic observer projections and ambiguity artifacts.
// proc-macro-scope: observer artifact rows use serde derives for replay schema, not shared model macros.

#![allow(dead_code)]

mod artifacts;
mod attacker;
mod metrics;
mod projection;
mod robustness;
mod sweep;

#[allow(unused_imports)]
pub(crate) use artifacts::{observer_artifact_rows, ObserverArtifactBundle, ObserverArtifactRow};
#[allow(unused_imports)]
pub(crate) use attacker::{
    run_observer_attacker, ObserverAttackerConfig, ObserverAttackerHypothesisScore,
    ObserverAttackerResult, ObserverAttackerTarget,
};
#[allow(unused_imports)]
pub(crate) use metrics::{
    ambiguity_cost_frontier_area, observer_metrics_from_result, ObserverAmbiguityMetrics,
    ObserverCostPoint,
};
#[allow(unused_imports)]
pub(crate) use projection::{
    project_observer_trace, ObserverEventKind, ObserverProjectionConfig, ObserverProjectionKind,
    ObserverTraceEvent,
};
#[allow(unused_imports)]
pub(crate) use robustness::{
    run_observer_robustness_summary, ObserverRobustnessScenarioKind, ObserverRobustnessSummary,
};
#[allow(unused_imports)]
pub(crate) use sweep::{
    observer_sweep_cells, run_observer_sweep, ObserverForwardingRandomness, ObserverSweepArtifact,
    ObserverSweepCell,
};
