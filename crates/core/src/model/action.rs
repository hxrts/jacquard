//! Policy outputs and selected routing actions.

use jacquard_macros::{id_type, public_model};
use serde::{Deserialize, Serialize};

use crate::{ConnectivityPosture, RouteProtectionClass};

#[id_type]
pub struct DiversityFloor(pub u8);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Output of the local policy engine. Runtime-local, never shared.
pub struct SelectedRoutingParameters {
    pub selected_protection: RouteProtectionClass,
    pub selected_connectivity: ConnectivityPosture,
    pub deployment_profile: OperatingMode,
    pub diversity_floor: DiversityFloor,
    pub routing_engine_fallback_policy: RoutingEngineFallbackPolicy,
    pub route_replacement_policy: RouteReplacementPolicy,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Whether the router may fall back from one routing engine to another.
pub enum RoutingEngineFallbackPolicy {
    Forbidden,
    Allowed,
}

#[public_model]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// Whether a materialized route may be replaced by a newly admitted route.
pub enum RouteReplacementPolicy {
    Forbidden,
    Allowed,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Host-defined name for an extensible deployment profile.
pub struct OperatingModeName(pub String);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Selected deployment posture for the current routing action.
pub enum OperatingMode {
    SparseLowPower,
    DenseInteractive,
    FieldPartitionTolerant,
    RelayAdversarial,
    Custom { name: OperatingModeName },
}
