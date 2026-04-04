//! Routing objectives, adaptive profiles, and policy enums.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    DestinationId, DurationMs, HealthScore, KnownValue, Limit, PenaltyPoints, PriorityPoints,
    RatioPermille, ServiceFamily,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Hard and soft route requirements for one operation.
/// The adaptive controller may move privacy from target toward floor
/// when connectivity pressure rises.
pub struct RoutingObjective {
    pub destination: DestinationId,
    pub service_family: ServiceFamily,
    /// Preferred privacy posture before adaptation.
    pub target_privacy: RoutePrivacyClass,
    /// Hard lower bound. Admission fails if no family can satisfy this.
    pub privacy_floor: RoutePrivacyClass,
    pub target_connectivity: RouteConnectivityClass,
    pub hold_fallback_policy: HoldFallbackPolicy,
    pub latency_budget_ms: Limit<DurationMs>,
    pub privacy_priority: PriorityPoints,
    pub connectivity_priority: PriorityPoints,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HoldFallbackPolicy {
    Forbidden,
    Allowed,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutePrivacyClass {
    None,
    LinkConfidential,
    TopologyObscured,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteConnectivityClass {
    BestEffort,
    Repairable,
    PartitionTolerant,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingObservations {
    pub reachable_neighbor_count: u32,
    pub route_family_count: u32,
    pub median_rtt_ms: DurationMs,
    pub loss_permille: RatioPermille,
    pub topology_churn_permille: RatioPermille,
    pub congestion_penalty_points: PenaltyPoints,
    pub partition_risk_permille: RatioPermille,
    pub direct_reachability_score: HealthScore,
    pub available_hold_capacity_bytes: KnownValue<u64>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Output of the local adaptive controller. Runtime-local, never shared.
pub struct AdaptiveRoutingProfile {
    pub selected_privacy: RoutePrivacyClass,
    pub selected_connectivity: RouteConnectivityClass,
    pub deployment_profile: DeploymentProfileId,
    pub diversity_floor: u8,
    pub family_fallback_policy: FamilyFallbackPolicy,
    pub route_replacement_policy: RouteReplacementPolicy,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FamilyFallbackPolicy {
    Forbidden,
    Allowed,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RouteReplacementPolicy {
    Forbidden,
    Allowed,
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeploymentProfileId {
    SparseLowPower,
    DenseInteractive,
    PartitionTolerantField,
    HostileRelay,
}
