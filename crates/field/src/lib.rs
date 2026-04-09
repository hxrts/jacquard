//! First-party deterministic field-style routing engine for Jacquard.
//!
//! `FieldEngine` implements the shared planner/runtime contracts while keeping
//! its corridor belief state private. The engine publishes
//! `RouteShapeVisibility::CorridorEnvelope`: it can make conservative
//! end-to-end claims without claiming an explicit hop-by-hop path.
//!
//! The implementation is intentionally split into thin modules so the private
//! observer/controller model can evolve without changing the shared engine
//! surface:
//! - `engine` defines the engine type, identity, and baseline capabilities.
//! - `planner` implements the shared planning surface.
//! - `runtime` implements materialization, maintenance, and forwarding hooks.
//!
//! At this stage the crate only locks the public contract. The richer field
//! data model is added incrementally in later phases.

#![forbid(unsafe_code)]

mod attractor;
mod choreography;
mod control;
mod engine;
mod observer;
mod planner;
mod route;
mod runtime;
mod state;
mod summary;

pub use engine::{FieldEngine, FIELD_CAPABILITIES, FIELD_ENGINE_ID};
