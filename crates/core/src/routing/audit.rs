//! Replay-visible route events and audit records.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Observation, OrderStamp, RouteCommitmentId, RouteCommitmentResolution, RouteHandle,
    RouteHealth, RouteId, RouteMaintenanceResult, RouteMaterializationProof, Tick,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteEvent {
    RouteMaterialized {
        handle: RouteHandle,
        proof: RouteMaterializationProof,
    },
    RouteMaintenanceCompleted {
        route_id: RouteId,
        result: RouteMaintenanceResult,
    },
    RouteCommitmentUpdated {
        route_id: RouteId,
        commitment_id: RouteCommitmentId,
        resolution: RouteCommitmentResolution,
    },
    RouteHealthObserved {
        route_id: RouteId,
        health: Observation<RouteHealth>,
    },
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingAuditEvent {
    pub order_stamp: OrderStamp,
    pub emitted_at_tick: Tick,
    pub event: RouteEvent,
}
