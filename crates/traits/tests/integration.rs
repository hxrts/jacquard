//! Integration tests for jacquard-traits.

#[path = "common/mod.rs"]
pub mod common;
#[path = "integration/effects_component_contract.rs"]
mod effects_component_contract;
#[path = "integration/proactive_engine_contract.rs"]
mod proactive_engine_contract;
#[path = "integration/routing_engine_contract.rs"]
mod routing_engine_contract;
#[path = "integration/simulator_contract.rs"]
mod simulator_contract;
#[path = "integration/world_extension_contract.rs"]
mod world_extension_contract;
