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
