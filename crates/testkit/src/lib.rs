#![forbid(unsafe_code)]

//! Shared integration and end-to-end test support for Jacquard crates.

pub mod router_integration;
pub mod topology;

#[cfg(feature = "reference-client-e2e")]
pub mod reference_client_scenarios;
