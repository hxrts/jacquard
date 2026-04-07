//! Shared fixtures and helpers for the mesh integration tests.
//!
//! This module is split into three focused submodules. `effects` defines
//! the runtime adapter mocks. `fixtures` defines deterministic node,
//! link, and configuration values. `engine` defines the engine builder
//! and the higher-level helpers that compose admission and
//! materialization into a single call.

#![allow(dead_code)]
#![allow(unreachable_pub)]

pub mod effects;
pub mod engine;
pub mod fixtures;
