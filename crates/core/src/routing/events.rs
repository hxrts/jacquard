//! Replay-visible route events and stamped route-event log records.
//!
//! This module defines the shared event types emitted by the routing control
//! plane into the route-event log. Events are stamped with a deterministic
//! `OrderStamp` and an emission tick so that replayers and audit tools can
//! reconstruct causal ordering without wall-clock dependencies.
//!
//! [`RouteEvent`] covers the four significant lifecycle transitions visible
//! to external observers: route materialization (with the full proof),
//! maintenance completion (with the maintenance result), commitment resolution
//! updates, and health observations. [`RouteEventStamped`] wraps any
//! `RouteEvent` with its order stamp and emission tick for log persistence
//! and replay. The event log itself is owned and appended to by the router;
//! engines and pathway layers emit the underlying `RouteEvent` values.

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
pub struct RouteEventStamped {
    pub order_stamp: OrderStamp,
    pub emitted_at_tick: Tick,
    pub event: RouteEvent,
}
