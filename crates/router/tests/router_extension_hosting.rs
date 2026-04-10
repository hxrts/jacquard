//! Integration tests for external and mixed-engine hosting in the router.
//!
//! These tests harden the router's extension boundary:
//! - one opaque external-engine double proves the router can activate routes
//!   without inspecting path visibility or hop-count summaries
//! - one real mixed-engine router proves `pathway` and `batman` can coexist in
//!   the same router while remaining private-state isolated behind the engine
//!   boundary

mod common;

use common::{
    build_router_with_opaque_engine, build_router_with_pathway_and_batman, objective, FAR_NODE_ID,
    LOCAL_NODE_ID,
};
use jacquard_batman::BATMAN_ENGINE_ID;
use jacquard_core::{
    Belief, DestinationId, RouteShapeVisibility, RoutingEngineId, RoutingTickChange,
    RoutingTickHint, Tick,
};
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_traits::{Router, RoutingControlPlane};

fn opaque_engine_id() -> RoutingEngineId {
    RoutingEngineId::from_contract_bytes(*b"router.opaque.01")
}

#[test]
fn router_activates_route_from_opaque_external_engine() {
    let mut router = build_router_with_opaque_engine(Tick(2), opaque_engine_id());

    let route = Router::activate_route(&mut router, objective(DestinationId::Node(FAR_NODE_ID)))
        .expect("opaque external engine activation");

    assert_eq!(route.identity.admission.summary.engine, opaque_engine_id());
    assert_eq!(
        route.identity.admission.summary.hop_count_hint,
        Belief::Absent
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&opaque_engine_id())
            .expect("registered opaque engine")
            .route_shape_visibility,
        RouteShapeVisibility::Opaque
    );
}

#[test]
fn router_can_host_real_pathway_and_batman_engines_together() {
    let mut router = build_router_with_pathway_and_batman(Tick(2));

    let round = router.advance_round().expect("initial mixed-engine round");
    assert_eq!(round.engine_change, RoutingTickChange::PrivateStateUpdated);
    assert_eq!(round.next_round_hint, RoutingTickHint::Immediate);

    let batman_route =
        Router::activate_route(&mut router, objective(DestinationId::Node(FAR_NODE_ID)))
            .expect("batman-backed activation");
    assert_eq!(
        batman_route.identity.admission.summary.engine,
        BATMAN_ENGINE_ID
    );
    assert_eq!(batman_route.identity.lease.owner_node_id, LOCAL_NODE_ID);
    assert_eq!(
        router.registered_engine_ids(),
        vec![BATMAN_ENGINE_ID, PATHWAY_ENGINE_ID]
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&PATHWAY_ENGINE_ID)
            .expect("registered pathway engine")
            .route_shape_visibility,
        RouteShapeVisibility::ExplicitPath
    );
    assert_eq!(
        router
            .registered_engine_capabilities(&BATMAN_ENGINE_ID)
            .expect("registered batman engine")
            .route_shape_visibility,
        RouteShapeVisibility::NextHopOnly
    );

    let follow_up = router
        .advance_round()
        .expect("follow-up mixed-engine round");
    assert_eq!(follow_up.topology_epoch, jacquard_core::RouteEpoch(2));
}
