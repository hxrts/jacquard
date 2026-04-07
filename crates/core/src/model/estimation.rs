//! Family-neutral routing estimates that sit between observation collection and
//! route admission.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    RouteConnectivityProfile, RouteDegradation, RouteEpoch, RouteProtectionClass,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Family-agnostic estimate of what one route candidate is likely to provide.
pub struct RouteEstimate {
    pub estimated_protection: RouteProtectionClass,
    pub estimated_connectivity: RouteConnectivityProfile,
    pub topology_epoch: RouteEpoch,
    pub degradation: RouteDegradation,
}
