//! Shared test fixtures and stub engines used by the router integration
//! tests. Organized into coherent sub-modules:
//!
//! - `fixtures`: topology, node, link, service, and policy-input sample data
//! - `router_builder`: pre-wired `MultiEngineRouter` builders for tests
//! - `null_engine`: a no-op routing engine stub
//! - `proactive_engine`: a table-backed proactive routing engine stub
//! - `recoverable_engine`: a routing engine stub with shared mutable route
//!   state for recovery tests
//! - `committee_selector`: a configurable committee selector stub

// Each integration-test binary only exercises a subset of these helpers, so
// dead_code and unused_imports warnings are expected on a per-binary basis.
#![allow(dead_code)]
#![allow(unused_imports)]

pub(crate) mod committee_selector;
pub(crate) mod fixtures;
pub(crate) mod null_engine;
pub(crate) mod opaque_engine;
pub(crate) mod proactive_engine;
pub(crate) mod recoverable_engine;
pub(crate) mod router_builder;

pub(crate) use committee_selector::AdvisoryCommitteeSelector;
pub(crate) use fixtures::{
    objective, profile, sample_configuration, sample_policy_inputs, BRIDGE_NODE_ID, FAR_NODE_ID,
    LOCAL_NODE_ID, PEER_NODE_ID,
};
pub(crate) use null_engine::NullCandidateEngine;
pub(crate) use opaque_engine::OpaqueSummaryTestEngine;
pub(crate) use proactive_engine::ProactiveTableTestEngine;
pub(crate) use recoverable_engine::RecoverableTestEngine;
pub(crate) use router_builder::{
    build_router, build_router_with_effects, build_router_with_opaque_engine,
    build_router_with_pathway_and_batman, build_router_with_proactive_engine,
    build_router_with_recoverable_engine, build_router_with_runtime_pair,
    build_router_with_selector, CommitteePathwayEngine, TestPathwayEngine,
};
