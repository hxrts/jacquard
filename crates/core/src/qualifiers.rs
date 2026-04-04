//! Explicit wrappers for bounded values and partially known values.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Limit<T> {
    Unlimited,
    Limited(T),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KnownValue<T> {
    Unknown,
    Known(T),
}
