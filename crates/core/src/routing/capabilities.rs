//! Capability tokens for route admission, ownership, evidence, and transitions.

use jacquard_macros::{id_type, must_use_handle};
use serde::{Deserialize, Serialize};

#[must_use_handle]
#[id_type]
pub struct RouteAdmissionCapability(pub u64);

#[must_use_handle]
#[id_type]
pub struct RouteOwnershipCapability(pub u64);

#[must_use_handle]
#[id_type]
pub struct RouteEvidenceCapability(pub u64);

#[must_use_handle]
#[id_type]
pub struct RouteTransitionCapability(pub u64);
