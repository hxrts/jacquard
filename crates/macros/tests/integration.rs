//! Integration test harness for `jacquard-macros`.
//!
//! Collects all integration test modules and runs them as a single binary.
//! Each submodule targets a specific macro or cross-macro interaction surface.
//! Integration tests in this tree verify that proc-macro expansions produce
//! the correct types, constants, and trait impls when applied to real Rust
//! items — as opposed to the regression tests which verify that invalid
//! annotations are rejected with the expected compiler errors.

#[path = "integration/annotation_contract.rs"]
mod annotation_contract;
