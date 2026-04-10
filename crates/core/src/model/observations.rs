//! Observation-layer support types, relay budget facts, and world observation
//! aliases.
//!
//! This module provides the shared types used by world extensions to surface
//! node-level observations and relay budget facts into the routing pipeline.
//! Budget types: [`RelayWorkBudget`], [`MaintenanceWorkBudget`],
//! [`HoldItemCount`], [`NodeRelayBudget`] (the full relay budget snapshot
//! including utilization and retention horizon), and [`InformationSetSummary`]
//! (the bloom-filter or sketch summary of a node's retained information).
//!
//! Observation type aliases make provenance explicit: [`NodeObservation`],
//! [`LinkObservation`], [`EnvironmentObservation`], [`ServiceObservation`],
//! [`ConfigurationObservation`], and [`WorldObservation`] each wrap the
//! underlying value in `Observation<T>` with full source, evidence, and
//! authentication provenance. [`ObservedValue`] is the self-describing payload
//! enum that world extensions emit.

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, Configuration, DurationMs, Environment, Link, Node, RatioPermille,
    ServiceDescriptor, Tick, TransportObservation,
};

#[id_type]
pub struct RelayWorkBudget(pub u32);

#[id_type]
pub struct MaintenanceWorkBudget(pub u32);

#[id_type]
pub struct HoldItemCount(pub u32);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Summary encoding used to approximate a node's retained information set.
pub enum InformationSummaryEncoding {
    BloomFilter,
    InvertibleBloomLookupTable,
    MinHashSketch,
    Opaque { name: String },
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Summary of the retained information set observed at one node.
pub struct InformationSetSummary {
    pub summary_encoding: InformationSummaryEncoding,
    pub item_count: Belief<HoldItemCount>,
    pub byte_count: Belief<ByteCount>,
    pub false_positive_permille: Belief<RatioPermille>,
}

impl InformationSetSummary {
    #[must_use]
    pub fn bloom_filter(
        item_count: HoldItemCount,
        byte_count: ByteCount,
        false_positive_permille: RatioPermille,
        updated_at_tick: Tick,
    ) -> Self {
        Self {
            summary_encoding: InformationSummaryEncoding::BloomFilter,
            item_count: Belief::certain(item_count, updated_at_tick),
            byte_count: Belief::certain(byte_count, updated_at_tick),
            false_positive_permille: Belief::certain(false_positive_permille, updated_at_tick),
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Local forwarding and retention budget currently observed for one node.
pub struct NodeRelayBudget {
    pub relay_work_budget: Belief<RelayWorkBudget>,
    pub utilization_permille: RatioPermille,
    pub retention_horizon_ms: Belief<DurationMs>,
}

impl NodeRelayBudget {
    #[must_use]
    pub fn observed(
        relay_work_budget: RelayWorkBudget,
        utilization_permille: RatioPermille,
        retention_horizon_ms: DurationMs,
        updated_at_tick: Tick,
    ) -> Self {
        Self {
            relay_work_budget: Belief::certain(relay_work_budget, updated_at_tick),
            utilization_permille,
            retention_horizon_ms: Belief::certain(retention_horizon_ms, updated_at_tick),
        }
    }
}

/// Observation wrapper for one instantiated node.
pub type NodeObservation = crate::Observation<Node>;

/// Observation wrapper for one instantiated link.
pub type LinkObservation = crate::Observation<Link>;

/// Observation wrapper for one instantiated environment object.
pub type EnvironmentObservation = crate::Observation<Environment>;

/// Observation wrapper for one shared service descriptor.
pub type ServiceObservation = crate::Observation<ServiceDescriptor>;

/// Observation wrapper for one instantiated configuration.
pub type ConfigurationObservation = crate::Observation<Configuration>;

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Self-describing observed payload surfaced by a world extension.
///
/// Higher-level runtime layers may later wrap these observations into batches,
/// diffs, partial snapshots, or other update shapes without changing what the
/// extension boundary means.
pub enum ObservedValue {
    Node(Node),
    Link(Link),
    Environment(Environment),
    Service(ServiceDescriptor),
    Transport(TransportObservation),
}

/// World observation type emitted by world extensions.
pub type WorldObservation = crate::Observation<ObservedValue>;
