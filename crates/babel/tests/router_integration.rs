//! Router integration test for `jacquard-babel`.

use std::collections::BTreeMap;

use jacquard_babel::{BabelEngine, BABEL_ENGINE_ID};
use jacquard_core::{
    AdmissionDecision, Configuration, Environment, Observation, RatioPermille, Tick,
};
use jacquard_mem_link_profile::{InMemoryRuntimeEffects, SharedInMemoryNetwork};
use jacquard_testkit::{
    homogeneous_router_integration_hosts,
    router_integration::{
        connected_objective, connected_profile, lossy_link, node, route_capable_node,
    },
};

fn topology() -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (node(1), route_capable_node(1, &BABEL_ENGINE_ID, Tick(1))),
                (node(2), route_capable_node(2, &BABEL_ENGINE_ID, Tick(1))),
                (node(3), route_capable_node(3, &BABEL_ENGINE_ID, Tick(1))),
                (node(4), route_capable_node(4, &BABEL_ENGINE_ID, Tick(1))),
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
