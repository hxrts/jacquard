//! Shared fixtures and helpers for the mesh integration tests.
//!
//! This module is split into three focused submodules. `effects` defines
//! the runtime adapter mocks. `fixtures` defines deterministic node,
//! link, and configuration values. `engine` defines the engine builder
//! and the higher-level helpers that compose admission and
//! materialization into a single call.

#![allow(dead_code)]
#![allow(unreachable_pub)]

pub mod admission_fixtures;
pub mod effects;
pub mod engine;
pub mod fixtures;

use jacquard_core::{DiscoveryScopeId, NodeId, RatioPermille};

/// Standard "local" node used in most mesh integration tests.
pub const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);
/// Standard peer node used in most mesh integration tests.
pub const PEER_NODE_ID: NodeId = NodeId([2; 32]);
/// Standard far node used in most mesh integration tests.
pub const FAR_NODE_ID: NodeId = NodeId([3; 32]);
/// Standard bridge node used in most mesh integration tests.
pub const BRIDGE_NODE_ID: NodeId = NodeId([4; 32]);
/// Standard discovery scope used in most mesh integration tests.
pub const DISCOVERY_SCOPE_ID: DiscoveryScopeId = DiscoveryScopeId([7; 16]);
/// Confidence value representing 100% certainty.
pub const MAX_CONFIDENCE: RatioPermille = RatioPermille(1000);
