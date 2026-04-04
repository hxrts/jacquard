//! Routing-visible observations for local node state, links, and neighborhood conditions.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{DurationMs, KnownValue, NodeRoutingIntrinsics, RatioPermille};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRelayBudget {
    pub relay_work_budget: KnownValue<u32>,
    pub utilization_permille: RatioPermille,
    pub retention_horizon_ms: KnownValue<DurationMs>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InformationSummaryEncoding {
    BloomFilter,
    InvertibleBloomLookupTable,
    MinHashSketch,
    Opaque { name: String },
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformationSetSummary {
    pub summary_encoding: InformationSummaryEncoding,
    pub item_count: KnownValue<u32>,
    pub byte_count: KnownValue<u64>,
    pub false_positive_permille: KnownValue<RatioPermille>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRoutingObservation {
    pub intrinsics: NodeRoutingIntrinsics,
    pub relay_budget: NodeRelayBudget,
    pub available_connection_count: KnownValue<u32>,
    pub hold_capacity_available_bytes: KnownValue<u64>,
    pub information_summary: KnownValue<InformationSetSummary>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighborhoodObservation {
    pub reachable_neighbor_count: u32,
    pub churn_permille: RatioPermille,
    pub contention_permille: RatioPermille,
}
