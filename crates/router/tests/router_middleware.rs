mod common;

use common::{build_router, objective, sample_configuration, FAR_NODE_ID};
use jacquard_core::{DestinationId, RouterCanonicalMutation, RoutingTickChange, Tick};
use jacquard_traits::{Router, RoutingControlPlane};

#[test]
fn middleware_replaces_topology_and_policy_inputs() {
    let mut router = build_router(Tick(2));
    let mut topology = sample_configuration();
    topology.value.environment.reachable_neighbor_count = 5;
    let mut policy_inputs = common::sample_policy_inputs(&topology);
    policy_inputs.routing_engine_count = 3;

    router.replace_topology(topology.clone());
    router.replace_policy_inputs(policy_inputs.clone());

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation after middleware updates");

    assert_eq!(route.identity.handle.topology_epoch, topology.value.epoch);
}

#[test]
fn middleware_tracks_registered_capabilities() {
    let router = build_router(Tick(2));

    assert_eq!(
        router.registered_engine_ids(),
        vec![jacquard_mesh::MESH_ENGINE_ID]
    );
    let capabilities = router
        .registered_engine_capabilities(&jacquard_mesh::MESH_ENGINE_ID)
        .expect("mesh capabilities");

    assert_eq!(capabilities.engine, jacquard_mesh::MESH_ENGINE_ID);
}

#[test]
fn anti_entropy_tick_reports_shared_router_outcome() {
    let mut router = build_router(Tick(2));

    let outcome = router.anti_entropy_tick().expect("anti-entropy tick");

    assert_eq!(outcome.topology_epoch, sample_configuration().value.epoch);
    assert_eq!(
        outcome.engine_change,
        RoutingTickChange::PrivateStateUpdated
    );
    assert_eq!(outcome.canonical_mutation, RouterCanonicalMutation::None);
}
