//! Error types for routing, transport, retention, and medium operations.
//!
//! This module defines the shared error-enum hierarchy used across all
//! routing, transport, storage, and hold subsystems. The top-level
//! [`RouteError`] composes the domain-specific error enums via `#[from]`
//! conversions. Each domain enum is generated with `define_error_enum!`, which
//! attaches the standard shared-model derives and `thiserror` support.
//!
//! Error enums defined here: [`RouteError`], [`RouteSelectionError`],
//! [`RouteRuntimeError`], [`RoutePolicyError`], [`CapabilityError`],
//! [`TransportError`], [`MediumError`], [`RetentionError`], [`HoldError`],
//! [`StorageError`], [`RouteEventLogError`], [`WorldError`],
//! [`PathSetupError`]. These types are consumed by traits in
//! `jacquard-traits` and implementations in engine and router crates.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::RouteAdmissionRejection;

/// Generates the shared error-enum header: `#[public_model]`, all standard
/// error derives, and the `pub enum` declaration. Variants are passed verbatim
/// so thiserror attributes (`#[error("...")]`, `#[from]`) work normally.
macro_rules! define_error_enum {
    ($name:ident { $($body:tt)* }) => {
        #[public_model]
        #[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
        pub enum $name {
            $($body)*
        }
    };
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RouteError {
    #[error("route selection error: {0}")]
    Selection(#[from] RouteSelectionError),
    #[error("route runtime error: {0}")]
    Runtime(#[from] RouteRuntimeError),
    #[error("route policy error: {0}")]
    Policy(#[from] RoutePolicyError),
    #[error("capability error: {0}")]
    Capability(#[from] CapabilityError),
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
}

define_error_enum!(RouteSelectionError {
    #[error("no candidate route was available")]
    NoCandidate,
    #[error("protection floor was not satisfied")]
    ProtectionFloorUnsatisfied,
    #[error("candidate was inadmissible: {0}")]
    Inadmissible(RouteAdmissionRejection),
    #[error("routing policy conflict")]
    PolicyConflict,
});

define_error_enum!(RouteRuntimeError {
    #[error("route lease expired")]
    LeaseExpired,
    #[error("stale owner attempted a mutation")]
    StaleOwner,
    #[error("route lifecycle event was rejected")]
    LifecycleEventRejected,
    #[error("route maintenance failed")]
    MaintenanceFailed,
    #[error("route operation timed out")]
    TimedOut,
    #[error("route state was invalidated")]
    Invalidated,
});

define_error_enum!(RoutePolicyError {
    #[error("fallback is forbidden")]
    FallbackForbidden,
    #[error("profile is unsupported")]
    ProfileUnsupported,
    #[error("budget exceeded")]
    BudgetExceeded,
});

define_error_enum!(CapabilityError {
    #[error("capability is unsupported")]
    Unsupported,
    #[error("capability was rejected")]
    Rejected,
    #[error("capability budget exceeded")]
    BudgetExceeded,
});

define_error_enum!(TransportError {
    #[error("transport is unavailable")]
    Unavailable,
    #[error("transport operation timed out")]
    TimedOut,
    #[error("transport rejected the operation")]
    Rejected,
});

define_error_enum!(MediumError {
    #[error("medium rejected the frame")]
    Rejected,
    #[error("medium data was corrupted")]
    Corrupted,
    #[error("medium operation timed out")]
    TimedOut,
});

define_error_enum!(RetentionError {
    #[error("retention store is unavailable")]
    Unavailable,
    #[error("retention store is full")]
    Full,
    #[error("retention operation was rejected")]
    Rejected,
});

define_error_enum!(HoldError {
    #[error("hold service is unavailable")]
    Unavailable,
    #[error("held object expired")]
    Expired,
    #[error("hold operation was rejected")]
    Rejected,
});

define_error_enum!(StorageError {
    #[error("storage is unavailable")]
    Unavailable,
    #[error("storage key was missing")]
    Missing,
    #[error("storage write was rejected")]
    Rejected,
});

define_error_enum!(RouteEventLogError {
    #[error("route-event log is unavailable")]
    Unavailable,
    #[error("route-event log entry was rejected")]
    Rejected,
});

define_error_enum!(WorldError {
    #[error("world extension is unavailable")]
    Unavailable,
    #[error("world extension timed out")]
    TimedOut,
    #[error("world observation was rejected")]
    Rejected,
    #[error("world observation was invalid")]
    Invalid,
});

define_error_enum!(PathSetupError {
    #[error("path setup is unsupported")]
    Unsupported,
    #[error("path setup was rejected")]
    Rejected,
    #[error("path setup was invalid")]
    Invalid,
});
