//! Engine-neutral routing estimates that sit between observation and admission.
//!
//! This module defines the shared estimate types that a routing engine produces
//! after collecting and processing world observations but before the admission
//! check has been performed. Estimates are advisory and are not proof-bearing;
//! the proof-bearing counterpart is `RouteAdmission` in `routing/admission.rs`.
//!
//! [`RouteEstimate`] captures the estimated protection class, connectivity
//! posture, topology epoch, and degradation state for one route candidate. It
//! is embedded in `RouteCandidate` as an `Estimate<RouteEstimate>` to carry
//! confidence and observation-tick metadata alongside the estimate value.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{ConnectivityPosture, RouteDegradation, RouteEpoch, RouteProtectionClass};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Engine-agnostic estimate of what one route candidate is likely to provide.
pub struct RouteEstimate {
    pub estimated_protection: RouteProtectionClass,
    pub estimated_connectivity: ConnectivityPosture,
    pub topology_epoch: RouteEpoch,
    pub degradation: RouteDegradation,
}
