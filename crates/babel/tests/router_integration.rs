//! Router integration test for `jacquard-babel`.

use std::collections::BTreeMap;

use jacquard_babel::{BabelEngine, BABEL_ENGINE_ID};
use jacquard_core::{
    AdmissionDecision, Configuration, Environment, Observation, RatioPermille, Tick,
};
use jacquard_core::{LinkEndpoint, TransportError};
use jacquard_mem_link_profile::{InMemoryRuntimeEffects, SharedInMemoryNetwork};
use jacquard_testkit::{
    homogeneous_router_integration_hosts,
    router_integration::{
        build_router, connected_objective, connected_profile, fixture_route_node, lossy_link, node,
    },
};
use jacquard_traits::{
    effect_handler, RoutingControlPlane, RoutingDataPlane, TransportSenderEffects,
};

#[derive(Default)]
struct RecordingSender {
    sent_frames: Vec<(LinkEndpoint, Vec<u8>)>,
}

#[effect_handler]
impl TransportSenderEffects for RecordingSender {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.sent_frames.push((endpoint.clone(), payload.to_vec()));
        Ok(())
    }
}

fn topology() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (node(1), fixture_route_node(1, &BABEL_ENGINE_ID, Tick(1))),
                (node(2), fixture_route_node(2, &BABEL_ENGINE_ID, Tick(1))),
                (node(3), fixture_route_node(3, &BABEL_ENGINE_ID, Tick(1))),
                (node(4), fixture_route_node(4, &BABEL_ENGINE_ID, Tick(1))),
            ]),
            links: BTreeMap::from([
                (
                    (node(1), node(2)),
                    lossy_link(2, Tick(1), RatioPermille(970)),
                ),
                (
                    (node(2), node(1)),
                    lossy_link(1, Tick(1), RatioPermille(970)),
                ),
                (
                    (node(2), node(4)),
                    lossy_link(4, Tick(1), RatioPermille(960)),
                ),
                (
                    (node(4), node(2)),
                    lossy_link(2, Tick(1), RatioPermille(960)),
                ),
                (
                    (node(1), node(3)),
                    lossy_link(3, Tick(1), RatioPermille(700)),
                ),
                (
                    (node(3), node(1)),
                    lossy_link(1, Tick(1), RatioPermille(700)),
                ),
                (
                    (node(3), node(4)),
                    lossy_link(4, Tick(1), RatioPermille(690)),
                ),
                (
                    (node(4), node(3)),
                    lossy_link(3, Tick(1), RatioPermille(690)),
                ),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(40),
                contention_permille: RatioPermille(20),
            },
        },
        source_class: jacquard_core::FactSourceClass::Local,
        evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

fn decode_next_hop(backend_route_id: &jacquard_core::BackendRouteId) -> jacquard_core::NodeId {
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(&backend_route_id.0[32..64]);
    jacquard_core::NodeId(bytes)
}

#[test]
fn babel_selects_etx_favored_route_and_admits_it_within_bound() {
    let objective = connected_objective(node(4));

    let network = SharedInMemoryNetwork::default();
    let mut hosts = homogeneous_router_integration_hosts!(
        network,
        topology,
        connected_profile(),
        1,
        [1, 2, 3, 4],
        |local_node_id, sender, now| {
            Box::new(BabelEngine::new(
                local_node_id,
                sender,
                InMemoryRuntimeEffects {
                    now,
                    ..Default::default()
                },
            ))
        }
    );

    for _ in 0..8 {
        for host in hosts.values_mut() {
            host.advance_round();
        }
    }

    let route = jacquard_traits::RoutingControlPlane::activate_route(
        hosts.get_mut(&node(1)).expect("babel host 1").router_mut(),
        objective.clone(),
    )
    .unwrap_or_else(|err| panic!("expected babel route within 8 rounds: {err}"));
    assert_eq!(route.identity.admission.backend_ref.engine, BABEL_ENGINE_ID);
    assert_eq!(
        route.identity.admission.admission_check.decision,
        AdmissionDecision::Admissible
    );
    assert_eq!(
        decode_next_hop(&route.identity.admission.backend_ref.backend_route_id),
        node(2)
    );
}

#[test]
fn babel_router_checkpoint_round_trip_restores_forwarding() {
    let objective = connected_objective(node(4));
    let network = SharedInMemoryNetwork::default();
    let mut hosts = homogeneous_router_integration_hosts!(
        network,
        topology,
        connected_profile(),
        1,
        [1, 2, 3, 4],
        |local_node_id, sender, now| {
            Box::new(BabelEngine::new(
                local_node_id,
                sender,
                InMemoryRuntimeEffects {
                    now,
                    ..Default::default()
                },
            ))
        }
    );

    for _ in 0..8 {
        for host in hosts.values_mut() {
            host.advance_round();
        }
    }

    let host = hosts.get_mut(&node(1)).expect("babel host 1");
    let route = RoutingControlPlane::activate_route(host.router_mut(), objective)
        .expect("activate route before checkpoint recovery");
    let persisted_effects = host.router_mut().effects().clone();

    let mut recovered = build_router(node(1), &topology(), connected_profile(), Tick(1), 1);
    *recovered.effects_mut() = persisted_effects;
    recovered
        .register_engine(Box::new(BabelEngine::new(
            node(1),
            RecordingSender::default(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
        )))
        .expect("register recovered babel engine");

    let restored = recovered
        .recover_checkpointed_routes()
        .expect("recover checkpointed babel route");
    assert_eq!(restored, 1);
    recovered
        .forward_payload(&route.identity.stamp.route_id, b"restored")
        .expect("forward on recovered router");
    assert!(recovered
        .active_route(&route.identity.stamp.route_id)
        .is_some());
}
