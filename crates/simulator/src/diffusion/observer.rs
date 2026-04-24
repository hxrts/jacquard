//! Deterministic observer projections and ambiguity artifacts.

#![allow(dead_code)]

mod attacker;
mod projection;

#[allow(unused_imports)]
pub(crate) use attacker::{
    run_observer_attacker, ObserverAttackerConfig, ObserverAttackerHypothesisScore,
    ObserverAttackerResult, ObserverAttackerTarget,
};
#[allow(unused_imports)]
pub(crate) use projection::{
    project_observer_trace, ObserverEventKind, ObserverProjectionConfig, ObserverProjectionKind,
    ObserverTraceEvent,
};
