//! Derived routing estimates that sit between observation collection and policy.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    HealthScore, InformationSetSummary, KnownValue, NeighborhoodObservation, NodeRelayBudget,
    RatioPermille, RouteConnectivityClass, RouteDegradation, RouteEpoch, RoutePrivacyClass,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerNoveltyEstimate {
    pub outbound_novel_item_count: KnownValue<u32>,
    pub inbound_novel_item_count: KnownValue<u32>,
    pub outbound_novel_byte_count: KnownValue<u64>,
    pub inbound_novel_byte_count: KnownValue<u64>,
    pub summary_encoding: crate::InformationSummaryEncoding,
    pub false_positive_permille: KnownValue<RatioPermille>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerRoutingEstimate {
    pub relay_budget: KnownValue<NodeRelayBudget>,
    pub information_summary: KnownValue<InformationSetSummary>,
    pub novelty_estimate: KnownValue<PeerNoveltyEstimate>,
    pub reach_score: KnownValue<HealthScore>,
    pub underserved_trajectory_score: KnownValue<HealthScore>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighborhoodEstimate {
    pub observation: NeighborhoodObservation,
    pub bridging_score: KnownValue<HealthScore>,
    pub underserved_flow_score: KnownValue<HealthScore>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEstimate {
    pub estimated_privacy: RoutePrivacyClass,
    pub estimated_connectivity: RouteConnectivityClass,
    pub topology_epoch: RouteEpoch,
    pub degradation: RouteDegradation,
}
