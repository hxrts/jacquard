//! Substrate layering types for composing routing engines as carriers.
//!
//! This module defines the shared vocabulary used when one routing engine
//! acts as the lower-layer carrier for another. The substrate model lets an
//! upper-layer engine declare what it needs from a carrier, inspect what
//! candidate carriers can provide, and acquire a strong lease binding the
//! chosen substrate for its use.
//!
//! Key types: [`SubstrateRequirements`] (min protection, connectivity, latency
//! budget, MTU floor, and identity assurance floor), [`SubstrateCapabilities`]
//! (what one engine can provide as a carrier), [`SubstrateCandidate`] (the
//! advisory candidate object surfaced to a layering orchestrator), and
//! [`SubstrateLease`] (the strong, must-use-handle lease acquired from the
//! chosen carrier engine). [`LayerParameter`] and [`LayerParameters`] carry
//! per-layer adaptation hints downward from upper-layer or host-level policy
//! engines into a substrate-aware lower-layer engine.

use jacquard_macros::{must_use_handle, public_model};
use serde::{Deserialize, Serialize};

use crate::{
    ByteCount, ConnectivityPosture, DurationMs, IdentityAssuranceClass, Limit, RouteHandle,
    RouteHealth, RouteLease, RouteProtectionClass, RoutingEngineId,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Carrier requirements expressed by one routing engine or host-level policy
/// engine.
pub struct SubstrateRequirements {
    pub min_protection: RouteProtectionClass,
    pub min_connectivity: ConnectivityPosture,
    pub latency_budget_ms: Limit<DurationMs>,
    pub mtu_floor_bytes: ByteCount,
    pub identity_assurance_floor: IdentityAssuranceClass,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// What one routing engine can provide when used as a lower-layer carrier.
pub struct SubstrateCapabilities {
    pub engine: RoutingEngineId,
    pub protection: RouteProtectionClass,
    pub connectivity: ConnectivityPosture,
    pub mtu_bytes: ByteCount,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Advisory substrate candidate surfaced to a layering orchestrator.
pub struct SubstrateCandidate {
    pub capabilities: SubstrateCapabilities,
    pub expected_health: Option<RouteHealth>,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Strong substrate lease acquired from one routing engine for use by another
/// layer.
#[must_use_handle]
pub struct SubstrateLease {
    pub capabilities: SubstrateCapabilities,
    pub handle: RouteHandle,
    pub lease: RouteLease,
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Parameter hints passed from an upper layer to a substrate-aware lower layer,
/// or from a host-level policy engine into a layered routing engine.
pub enum LayerParameter {
    PathLengthHint(u8),
    LatencyBudgetHint(DurationMs),
    MtuFloorHint(ByteCount),
    Custom { name: String, value: Vec<u8> },
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Grouped per-layer adaptation hints.
pub struct LayerParameters {
    pub items: Vec<LayerParameter>,
}
