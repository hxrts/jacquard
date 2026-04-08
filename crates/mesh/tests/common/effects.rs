//! Shared in-memory runtime adapters used by the mesh integration tests.
//!
//! Control flow intuition: mesh tests should exercise the same mock transport,
//! retention, and runtime-effect crate that router/device integration uses.
//! This module keeps the old local names only as thin aliases so the test
//! suite does not maintain a second parallel harness.

pub use jacquard_mock_transport::{
    InMemoryMeshTransport as TestTransport,
    InMemoryRetentionStore as TestRetentionStore,
    InMemoryRuntimeEffects as TestRuntimeEffects,
};
