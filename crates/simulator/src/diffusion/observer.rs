//! Deterministic observer projections and ambiguity artifacts.

#![allow(dead_code)]

mod projection;

#[allow(unused_imports)]
pub(crate) use projection::{
    project_observer_trace, ObserverEventKind, ObserverProjectionConfig, ObserverProjectionKind,
    ObserverTraceEvent,
};
