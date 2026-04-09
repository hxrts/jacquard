//! Integration tests for proactive routing engine support in the router.
//!
//! These tests verify that `MultiEngineRouter` correctly handles engines that
//! operate with `RouteShapeVisibility::AggregatePath` and
//! `RouteShapeVisibility::NextHopOnly` without requiring the router to inspect
//! engine-private forwarding tables. The `ProactiveTableTestEngine` stub
//! rebuilds its next-hop table on each `engine_tick` and serves candidates from
//! that private state.
//!
//! Key behaviors covered:
//! - Activation succeeds and the materialized route's admission summary names
//!   the proactive engine when it is the highest-priority candidate source.
//! - The router correctly records and surfaces `RouteShapeVisibility` from the
//!   registered engine capabilities.
//! - Engine-private periodic work (`advance_round` before any activation)
//!   completes without error and the router correctly reflects the resulting
//!   `RoutingTickChange::PrivateStateUpdated` hint to the host.

mod common;

use common::{
    build_router_with_proactive_engine, objective, FAR_NODE_ID, LOCAL_NODE_ID,
};
use jacquard_core::{
    DestinationId, RouteShapeVisibility, RoutingEngineId, RoutingTickChange,
    RoutingTickHint, Tick,
};
use jacquard_traits::{Router, RoutingControlPlane};

fn aggregate_engine_id() -> RoutingEngineId {
    RoutingEngineId::from_contract_bytes(*b"router.proactv.1")
}

fn next_hop_engine_id() -> RoutingEngineId {
    RoutingEngineId::from_contract_bytes(*b"router.proactv.2")
}

#[test]
fn router_activates_route_from_aggregate_path_proactive_engine() {
    let mut router = build_router_with_proactive_engine(
        Tick(2),
        aggregate_engine_id(),
        RouteShapeVisibility::AggregatePath,
    );

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("aggregate-path activation");

    assert_eq!(
        route.identity.admission.summary.engine,
        aggregate_engine_id()
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&aggregate_engine_id())
            .expect("registered engine")
            .route_shape_visibility,
        RouteShapeVisibility::AggregatePath
    );
}

#[test]
fn router_activates_route_from_next_hop_only_proactive_engine() {
    let mut router = build_router_with_proactive_engine(
        Tick(2),
        next_hop_engine_id(),
        RouteShapeVisibility::NextHopOnly,
    );

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("next-hop-only activation");

    assert_eq!(
        route.identity.admission.summary.engine,
        next_hop_engine_id()
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&next_hop_engine_id())
            .expect("registered engine")
            .route_shape_visibility,
        RouteShapeVisibility::NextHopOnly
    );
}

#[test]
fn router_tolerates_engine_private_periodic_work_before_activation() {
    let mut router = build_router_with_proactive_engine(
        Tick(2),
        aggregate_engine_id(),
        RouteShapeVisibility::AggregatePath,
    );

    let outcome = router.advance_round().expect("router round");

    assert_eq!(outcome.topology_epoch, jacquard_core::RouteEpoch(2));
    assert_eq!(
        outcome.engine_change,
        RoutingTickChange::PrivateStateUpdated
    );
    assert_eq!(outcome.next_round_hint, RoutingTickHint::Immediate);

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("route activation after proactive work");

    assert_eq!(route.identity.lease.owner_node_id, LOCAL_NODE_ID);
}
