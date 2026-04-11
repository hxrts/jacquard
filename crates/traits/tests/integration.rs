//! Integration tests for jacquard-traits.
//!
//! This test binary collects contract-oriented integration tests that verify
//! the behavioral interfaces defined in `jacquard-traits` are implementable,
//! composable, and semantically correct through stub implementations. Tests
//! here are not tied to any production routing engine.
//!
//! Modules included here:
//! - `effects_component_contract` — transport sender/driver split and
//!   retention.
//! - `proactive_engine_contract` — engines that build private tables on ticks.
//! - `router_middleware_contract` — router-facing middleware trait composition.
//! - `routing_engine_contract` — full candidate-to-materialized-route flow.
//! - `simulator_contract` — pure scenario description and effectful harness.
//! - `world_extension_contract` — typed observation contribution without owning
//!   canonical route state.

pub mod common;
#[path = "integration/effects_component_contract.rs"]
mod effects_component_contract;
#[path = "integration/proactive_engine_contract.rs"]
mod proactive_engine_contract;
#[path = "integration/router_middleware_contract.rs"]
mod router_middleware_contract;
#[path = "integration/routing_engine_contract.rs"]
mod routing_engine_contract;
#[path = "integration/simulator_contract.rs"]
mod simulator_contract;
#[path = "integration/world_extension_contract.rs"]
mod world_extension_contract;
