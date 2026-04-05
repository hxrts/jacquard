//! Derived routing estimates that sit between observation collection and policy.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, Environment, HealthScore, InformationSetSummary, InformationSummaryEncoding,
    NodeRelayBudget, RatioPermille, RouteConnectivityProfile, RouteDegradation, RouteEpoch,
    RouteProtectionClass,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Estimated overlap and novelty between this node and one peer.
pub struct PeerNoveltyEstimate {
    pub outbound_novel_item_count: Belief<u32>,
    pub inbound_novel_item_count: Belief<u32>,
    pub outbound_novel_byte_count: Belief<ByteCount>,
    pub inbound_novel_byte_count: Belief<ByteCount>,
    pub summary_encoding: InformationSummaryEncoding,
    pub false_positive_permille: Belief<RatioPermille>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Derived routing-facing estimate for one peer.
pub struct PeerRoutingEstimate {
    pub relay_budget: Belief<NodeRelayBudget>,
    pub information_summary: Belief<InformationSetSummary>,
    pub novelty_estimate: Belief<PeerNoveltyEstimate>,
    pub reach_score: Belief<HealthScore>,
    pub underserved_trajectory_score: Belief<HealthScore>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Derived estimate for the local configuration as a whole.
pub struct ConfigurationEstimate {
    pub environment: Environment,
    pub bridging_score: Belief<HealthScore>,
    pub underserved_flow_score: Belief<HealthScore>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Family-agnostic estimate of what one route candidate is likely to provide.
pub struct RouteEstimate {
    pub estimated_protection: RouteProtectionClass,
    pub estimated_connectivity: RouteConnectivityProfile,
    pub topology_epoch: RouteEpoch,
    pub degradation: RouteDegradation,
}
