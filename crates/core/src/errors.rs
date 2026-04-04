use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::RouteAdmissionRejection;

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

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RouteSelectionError {
    #[error("no candidate route was available")]
    NoCandidate,
    #[error("privacy floor was not satisfied")]
    PrivacyFloorUnsatisfied,
    #[error("candidate was inadmissible: {0}")]
    Inadmissible(RouteAdmissionRejection),
    #[error("routing policy conflict")]
    PolicyConflict,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RouteRuntimeError {
    #[error("route lease expired")]
    LeaseExpired,
    #[error("stale owner attempted a mutation")]
    StaleOwner,
    #[error("route transition was rejected")]
    TransitionRejected,
    #[error("route maintenance failed")]
    MaintenanceFailed,
    #[error("route operation timed out")]
    TimedOut,
    #[error("route state was invalidated")]
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RoutePolicyError {
    #[error("fallback is forbidden")]
    FallbackForbidden,
    #[error("profile is unsupported")]
    ProfileUnsupported,
    #[error("budget exceeded")]
    BudgetExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum CapabilityError {
    #[error("capability is unsupported")]
    Unsupported,
    #[error("capability was rejected")]
    Rejected,
    #[error("capability budget exceeded")]
    BudgetExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum TransportError {
    #[error("transport is unavailable")]
    Unavailable,
    #[error("transport operation timed out")]
    TimedOut,
    #[error("transport rejected the operation")]
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum MediumError {
    #[error("medium rejected the frame")]
    Rejected,
    #[error("medium data was corrupted")]
    Corrupted,
    #[error("medium operation timed out")]
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum CustodyError {
    #[error("custody store is unavailable")]
    Unavailable,
    #[error("custody store is full")]
    Full,
    #[error("custody operation was rejected")]
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum HoldError {
    #[error("hold service is unavailable")]
    Unavailable,
    #[error("held object expired")]
    Expired,
    #[error("hold operation was rejected")]
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum PathSetupError {
    #[error("path setup is unsupported")]
    Unsupported,
    #[error("path setup was rejected")]
    Rejected,
    #[error("path setup was invalid")]
    Invalid,
}
