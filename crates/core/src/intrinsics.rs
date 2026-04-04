//! Stable routing-facing node limits derived from local device or policy constraints.

use contour_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::KnownValue;

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRoutingIntrinsics {
    pub connection_count_max: KnownValue<u32>,
    pub neighbor_state_count_max: KnownValue<u32>,
    pub simultaneous_transfer_count_max: KnownValue<u32>,
    pub active_route_count_max: KnownValue<u32>,
    pub relay_work_budget_max: KnownValue<u32>,
    pub maintenance_work_budget_max: KnownValue<u32>,
    pub hold_item_count_max: KnownValue<u32>,
    pub hold_capacity_bytes_max: KnownValue<u64>,
}
