//! Observation-layer support types, shared observed payloads, and observation
//! aliases over world objects.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, Configuration, DurationMs, Environment, Link, Node,
    RatioPermille, ServiceDescriptor, TransportObservation,
};

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
    pub summary_encoding:        InformationSummaryEncoding,
    pub item_count:              Belief<u32>,
    pub byte_count:              Belief<ByteCount>,
    pub false_positive_permille: Belief<RatioPermille>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Local forwarding and retention budget currently observed for one node.
pub struct NodeRelayBudget {
    pub relay_work_budget:    Belief<u32>,
    pub utilization_permille: RatioPermille,
    pub retention_horizon_ms: Belief<DurationMs>,
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
