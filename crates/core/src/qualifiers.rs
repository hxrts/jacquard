//! Explicit wrappers for bounded values and partially known values.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Limit<T> {
    Unlimited,
    Limited(T),
}

#[public_model]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KnownValue<T> {
    Unknown,
    Known(T),
}
