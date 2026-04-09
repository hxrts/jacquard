mod common;

use common::{build_router, objective, sample_configuration, FAR_NODE_ID};
use jacquard_core::{
    DestinationId, RouterCanonicalMutation, RoutingTickChange, RoutingTickHint, Tick,
};
use jacquard_traits::{Router, RoutingControlPlane};

#[test]
fn middleware_ingests_topology_and_policy_inputs() {
    let mut router = build_router(Tick(2));
    let mut topology = sample_configuration();
    topology.value.environment.reachable_neighbor_count = 5;
    let mut policy_inputs = common::sample_policy_inputs(&topology);
    policy_inputs.routing_engine_count = 3;

    router.ingest_topology_observation(topology.clone());
    router.ingest_policy_inputs(policy_inputs.clone());

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation after middleware updates");

    assert_eq!(route.identity.stamp.topology_epoch, topology.value.epoch);
}

#[test]
fn middleware_tracks_registered_capabilities() {
    let router = build_router(Tick(2));

    assert_eq!(
        router.registered_engine_ids(),
        vec![jacquard_pathway::PATHWAY_ENGINE_ID]
    );
    let capabilities = router
        .registered_engine_capabilities(&jacquard_pathway::PATHWAY_ENGINE_ID)
        .expect("mesh capabilities");

    assert_eq!(capabilities.engine, jacquard_pathway::PATHWAY_ENGINE_ID);
}

#[test]
fn advance_round_reports_shared_router_outcome() {
    let mut router = build_router(Tick(2));

    let outcome = router.advance_round().expect("advance round");

    assert_eq!(outcome.topology_epoch, sample_configuration().value.epoch);
    assert_eq!(
        outcome.engine_change,
        RoutingTickChange::PrivateStateUpdated
    );
    assert_eq!(
        outcome.next_round_hint,
        RoutingTickHint::WithinTicks(Tick(1))
    );
    assert_eq!(outcome.canonical_mutation, RouterCanonicalMutation::None);
}
