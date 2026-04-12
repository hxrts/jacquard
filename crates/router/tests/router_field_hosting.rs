//! Integration tests for hosting the real field engine behind the router's
//! shared engine boundary.
//!
//! These tests verify that:
//! - the router can register and activate `jacquard-field` through the shared
//!   traits without learning anything about field-private corridor belief or
//!   choreography sessions
//! - the router surfaces only the shared `CorridorEnvelope` capability and
//!   standard round hints while field-private protocol activity remains opaque
//! - maintenance fails closed when topology changes invalidate the field
//!   frontier

mod common;

use common::{build_router_with_field, objective, LOCAL_NODE_ID, PEER_NODE_ID};
use jacquard_core::{
    DestinationId, Environment, FactSourceClass, Observation, OriginAuthenticationClass,
    RatioPermille, RouteMaintenanceTrigger, RouteShapeVisibility, RouterCanonicalMutation,
    RoutingEvidenceClass, RoutingTickChange, Tick,
};
use jacquard_reference_client::topology;
use jacquard_traits::{Router, RoutingControlPlane, RoutingDataPlane};

fn topology_without_direct_link(
    observed_at_tick: Tick,
) -> Observation<jacquard_core::Configuration> {
    Observation {
        value: jacquard_core::Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: std::collections::BTreeMap::from([
                (
                    LOCAL_NODE_ID,
                    topology::node(1)
                        .for_engine(&jacquard_field::FIELD_ENGINE_ID)
                        .build(),
                ),
                (
                    PEER_NODE_ID,
                    topology::node(2)
                        .for_engine(&jacquard_field::FIELD_ENGINE_ID)
                        .build(),
                ),
            ]),
            links: std::collections::BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(250),
                contention_permille: RatioPermille(300),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick,
    }
}

#[test]
fn router_activates_field_route_with_corridor_envelope_visibility() {
    let mut router = build_router_with_field(Tick(2));

    let route = Router::activate_route(&mut router, objective(DestinationId::Node(PEER_NODE_ID)))
        .expect("field-backed activation");

    assert_eq!(
        route.identity.admission.summary.engine,
        jacquard_field::FIELD_ENGINE_ID
    );
    assert_eq!(route.identity.lease.owner_node_id, LOCAL_NODE_ID);
    assert_eq!(
        router
            .registered_engine_capabilities(&jacquard_field::FIELD_ENGINE_ID)
            .expect("registered field engine")
            .route_shape_visibility,
        RouteShapeVisibility::CorridorEnvelope
    );
    assert!(route.identity.admission.summary.hop_count_hint.value_or(0) >= 1);
    let commitments = router
        .route_commitments(&route.identity.stamp.route_id)
        .expect("field commitments");
    assert_eq!(commitments.len(), 1);
    assert_eq!(
        commitments[0].resolution,
        jacquard_core::RouteCommitmentResolution::Pending
    );
    assert_eq!(
        commitments[0].route_binding,
        jacquard_core::RouteBinding::Bound(route.identity.stamp.route_id)
    );
    router
        .forward_payload(&route.identity.stamp.route_id, b"field-data")
        .expect("router forwards via field engine");
}

#[test]
fn advance_round_hosts_field_private_updates_without_exposing_session_state() {
    let mut router = build_router_with_field(Tick(2));

    let outcome = router.advance_round().expect("initial field round");

    assert_eq!(
        router.registered_engine_ids(),
        vec![jacquard_field::FIELD_ENGINE_ID]
    );
    assert_eq!(outcome.topology_epoch, jacquard_core::RouteEpoch(2));
    assert_eq!(
        outcome.engine_change,
        RoutingTickChange::PrivateStateUpdated
    );
    assert_eq!(outcome.canonical_mutation, RouterCanonicalMutation::None);
}

#[test]
fn router_expires_field_route_fail_closed_after_frontier_disappears() {
    let mut router = build_router_with_field(Tick(2));
    let route = Router::activate_route(&mut router, objective(DestinationId::Node(PEER_NODE_ID)))
        .expect("field route activation");

    router.ingest_topology_observation(topology_without_direct_link(Tick(10)));
    let round = router.advance_round().expect("advance after link removal");
    assert_eq!(round.engine_change, RoutingTickChange::PrivateStateUpdated);

    let maintenance = router
        .maintain_route(
            &route.identity.stamp.route_id,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("field maintenance after link removal");

    assert_eq!(
        maintenance.canonical_mutation,
        RouterCanonicalMutation::RouteExpired {
            route_id: route.identity.stamp.route_id,
        }
    );
    assert!(router
        .active_route(&route.identity.stamp.route_id)
        .is_none());
}
