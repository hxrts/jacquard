//! Routing objectives, policy inputs, and policy vocabulary.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    DestinationId, DurationMs, Environment, HealthScore, IdentityAssuranceClass, Limit,
    Node, Observation, PriorityPoints, RatioPermille, RouteServiceKind,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Hard and soft route requirements for one operation.
/// The policy engine may move route protection from target toward floor
/// when connectivity pressure rises.
pub struct RoutingObjective {
    pub destination:           DestinationId,
    pub service_kind:          RouteServiceKind,
    /// Preferred protection posture before adaptation.
    pub target_protection:     RouteProtectionClass,
    /// Hard lower bound. Admission fails if no routing engine can satisfy this.
    pub protection_floor:      RouteProtectionClass,
    pub target_connectivity:   RouteConnectivityProfile,
    pub hold_fallback_policy:  HoldFallbackPolicy,
    pub latency_budget_ms:     Limit<DurationMs>,
    pub protection_priority:   PriorityPoints,
    pub connectivity_priority: PriorityPoints,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Whether an operation may fall back to deferred delivery.
pub enum HoldFallbackPolicy {
    Forbidden,
    Allowed,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Abstract protection level that a routing engine may provide.
pub enum RouteProtectionClass {
    None,
    LinkProtected,
    TopologyProtected,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Connectivity repair posture for one route.
pub enum RouteRepairClass {
    BestEffort,
    Repairable,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Partition posture for one route.
pub enum RoutePartitionClass {
    ConnectedOnly,
    PartitionTolerant,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Abstract connectivity profile that a routing engine may provide.
pub struct RouteConnectivityProfile {
    pub repair:    RouteRepairClass,
    pub partition: RoutePartitionClass,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Policy-stage inputs derived from local state and neighborhood estimates.
pub struct RoutingPolicyInputs {
    pub local_node:                  Observation<Node>,
    pub local_environment:           Observation<Environment>,
    pub routing_engine_count:        u32,
    pub median_rtt_ms:               DurationMs,
    pub loss_permille:               RatioPermille,
    pub partition_risk_permille:     RatioPermille,
    pub adversary_pressure_permille: RatioPermille,
    pub identity_assurance:          IdentityAssuranceClass,
    pub direct_reachability_score:   HealthScore,
}
