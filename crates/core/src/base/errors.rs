//! Error types for routing, transport, retention, and medium operations.
//!
//! This module defines the shared error-enum hierarchy used across all
//! routing, transport, storage, and hold subsystems. The top-level
//! [`RouteError`] composes the domain-specific error enums via explicit
//! conversions. Each domain enum is generated with `define_error_enum!`, which
//! attaches the standard shared-model derives and portable display support.
//!
//! Error enums defined here: [`RouteError`], [`RouteSelectionError`],
//! [`RouteRuntimeError`], [`RoutePolicyError`], [`CapabilityError`],
//! [`TransportError`], [`MediumError`], [`RetentionError`], [`HoldError`],
//! [`StorageError`], [`RouteEventLogError`], [`WorldError`],
//! [`PathSetupError`]. These types are consumed by traits in
//! `jacquard-traits` and implementations in engine and router crates.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::RouteAdmissionRejection;

/// Generates a shared error enum with portable `Display` and optional
/// `std::error::Error` support.
macro_rules! define_error_enum {
    ($name:ident { $($variant:ident => $message:literal),+ $(,)? }) => {
        #[public_model]
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name {
            $($variant,)+
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $(Self::$variant => f.write_str($message),)+
                }
            }
        }

        #[cfg(feature = "std")]
        impl std::error::Error for $name {}
    };
}

macro_rules! impl_route_error_from {
    ($source:ty, $variant:ident) => {
        impl From<$source> for RouteError {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteError {
    Selection(RouteSelectionError),
    Runtime(RouteRuntimeError),
    Policy(RoutePolicyError),
    Capability(CapabilityError),
    Transport(TransportError),
}

impl core::fmt::Display for RouteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Selection(error) => write!(f, "route selection error: {error}"),
            Self::Runtime(error) => write!(f, "route runtime error: {error}"),
            Self::Policy(error) => write!(f, "route policy error: {error}"),
            Self::Capability(error) => write!(f, "capability error: {error}"),
            Self::Transport(error) => write!(f, "transport error: {error}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RouteError {}

impl_route_error_from!(RouteSelectionError, Selection);
impl_route_error_from!(RouteRuntimeError, Runtime);
impl_route_error_from!(RoutePolicyError, Policy);
impl_route_error_from!(CapabilityError, Capability);
impl_route_error_from!(TransportError, Transport);

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteSelectionError {
    NoCandidate,
    ProtectionFloorUnsatisfied,
    Inadmissible(RouteAdmissionRejection),
    PolicyConflict,
}

impl core::fmt::Display for RouteSelectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoCandidate => f.write_str("no candidate route was available"),
            Self::ProtectionFloorUnsatisfied => f.write_str("protection floor was not satisfied"),
            Self::Inadmissible(error) => write!(f, "candidate was inadmissible: {error}"),
            Self::PolicyConflict => f.write_str("routing policy conflict"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RouteSelectionError {}

define_error_enum!(RouteRuntimeError {
    LeaseExpired => "route lease expired",
    StaleOwner => "stale owner attempted a mutation",
    LifecycleEventRejected => "route lifecycle event was rejected",
    MaintenanceFailed => "route maintenance failed",
    TimedOut => "route operation timed out",
    Invalidated => "route state was invalidated",
});

define_error_enum!(RoutePolicyError {
    FallbackForbidden => "fallback is forbidden",
    ProfileUnsupported => "profile is unsupported",
    BudgetExceeded => "budget exceeded",
});

define_error_enum!(CapabilityError {
    Unsupported => "capability is unsupported",
    Rejected => "capability was rejected",
    BudgetExceeded => "capability budget exceeded",
});

define_error_enum!(TransportError {
    Unavailable => "transport is unavailable",
    TimedOut => "transport operation timed out",
    Rejected => "transport rejected the operation",
});

define_error_enum!(MediumError {
    Rejected => "medium rejected the frame",
    Corrupted => "medium data was corrupted",
    TimedOut => "medium operation timed out",
});

define_error_enum!(RetentionError {
    Unavailable => "retention store is unavailable",
    Full => "retention store is full",
    Rejected => "retention operation was rejected",
});

define_error_enum!(HoldError {
    Unavailable => "hold service is unavailable",
    Expired => "held object expired",
    Rejected => "hold operation was rejected",
});

define_error_enum!(StorageError {
    Unavailable => "storage is unavailable",
    Missing => "storage key was missing",
    Rejected => "storage write was rejected",
});

define_error_enum!(RouteEventLogError {
    Unavailable => "route-event log is unavailable",
    Rejected => "route-event log entry was rejected",
});

define_error_enum!(WorldError {
    Unavailable => "world extension is unavailable",
    TimedOut => "world extension timed out",
    Rejected => "world observation was rejected",
    Invalid => "world observation was invalid",
});

define_error_enum!(PathSetupError {
    Unsupported => "path setup is unsupported",
    Rejected => "path setup was rejected",
    Invalid => "path setup was invalid",
});
