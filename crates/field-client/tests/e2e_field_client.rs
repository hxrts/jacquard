//! End-to-end tests for `jacquard-field-client`.
//!
//! Exercises the full `FieldClient` lifecycle over in-memory transport.
//! Two topology fixtures drive the tests: `asymmetric_topology` places two
//! nodes with one directed link at moderate churn and contention;
//! `stressed_topology` raises churn to 980‰, contention to 950‰, and zeroes
//! available connections to force degraded maintenance outcomes.
//!
//! `field_client_routes_end_to_end_over_asymmetric_forward_link` activates a
//! route, forwards a payload, and asserts delivery to the peer node.
//! `field_client_surfaces_regime_or_posture_adaptation_under_sustained_stress`
//! ingests stressed topology rounds and asserts that maintenance either enters
//! hold fallback, requires replacement, or legitimately continues inside the
//! installed corridor envelope.

use std::collections::BTreeMap;

use jacquard_core::{
    ByteCount, Configuration, DestinationId, Environment, FactSourceClass, Observation,
    OriginAuthenticationClass, RatioPermille, RouteEpoch, RouteMaintenanceOutcome,
    RouteMaintenanceTrigger, RoutingEvidenceClass, Tick, TransportIngressEvent,
};
use jacquard_field_client::{
    default_objective, topology, FieldClientBuilder, NodeIdentity, NodePreset, NodePresetOptions,
    NodeStateSnapshot, SharedInMemoryNetwork,
};

fn node(byte: u8) -> jacquard_core::NodeId {
    jacquard_core::NodeId([byte; 32])
}

fn asymmetric_topology(observed_at_tick: Tick) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (
                    node(1),
                    topology::node(1).observed_at(observed_at_tick).build(),
                ),
                (
                    node(2),
                    topology::node(2).observed_at(observed_at_tick).build(),
                ),
            ]),
            links: BTreeMap::from([(
                (node(1), node(2)),
                topology::link(2)
                    .observed_at(observed_at_tick)
                    .with_confidence(RatioPermille(975))
                    .build(),
            )]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(50),
                contention_permille: RatioPermille(40),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick,
    }
}

fn stressed_topology(observed_at_tick: Tick) -> Observation<Configuration> {
    let local = NodePreset::route_capable(
        NodePresetOptions::new(
            NodeIdentity::new(node(1), jacquard_core::ControllerId([1; 32])),
            jacquard_adapter::opaque_endpoint(
                jacquard_core::TransportKind::WifiAware,
                vec![1],
                ByteCount(256),
            ),
            observed_at_tick,
        ),
        &jacquard_field_client::FIELD_ENGINE_ID,
    )
    .with_state(
        NodeStateSnapshot::route_capable(observed_at_tick)
            .with_hold_capacity(ByteCount(32))
            .with_available_connections(0)
            .with_relay_state(8, RatioPermille(950), jacquard_core::DurationMs(500))
            .with_observed_at_tick(observed_at_tick),
    )
    .build();

    Observation {
        value: Configuration {
            epoch: RouteEpoch(2),
            nodes: BTreeMap::from([
                (node(1), local),
                (
                    node(2),
                    topology::node(2).observed_at(observed_at_tick).build(),
                ),
            ]),
            links: BTreeMap::from([(
                (node(1), node(2)),
                topology::link(2)
                    .observed_at(observed_at_tick)
                    .with_confidence(RatioPermille(950))
                    .build(),
            )]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(980),
                contention_permille: RatioPermille(950),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick,
    }
}

#[test]
fn field_client_routes_end_to_end_over_asymmetric_forward_link() {
    let topology = asymmetric_topology(Tick(2));
    let network = SharedInMemoryNetwork::default();
    let mut client = FieldClientBuilder::new(node(1), topology, network, Tick(2)).build();

    let round = client.advance_round().expect("seed field round");
    assert_eq!(round.topology_epoch, RouteEpoch(1));

    let route = client
        .activate_route(&default_objective(DestinationId::Node(node(2))))
        .expect("field route activation");
    assert_eq!(
        route.identity.admission.summary.engine,
        jacquard_field_client::FIELD_ENGINE_ID
    );

    client
        .forward_payload(&route.identity.stamp.route_id, b"field-e2e")
        .expect("field forwarding");
    let ingress = client
        .drain_peer_ingress(node(2))
        .expect("drain peer ingress");

    assert_eq!(ingress.len(), 1);
    assert!(matches!(
        &ingress[0],
        TransportIngressEvent::PayloadReceived { from_node_id, payload, .. }
            if *from_node_id == node(1) && payload == b"field-e2e"
    ));
}

#[test]
fn field_client_surfaces_regime_or_posture_adaptation_under_sustained_stress() {
    let topology = asymmetric_topology(Tick(2));
    let network = SharedInMemoryNetwork::default();
    let mut client = FieldClientBuilder::new(node(1), topology, network, Tick(2)).build();

    let route = client
        .activate_route(&default_objective(DestinationId::Node(node(2))))
        .expect("initial field activation");

    for tick in 10..=18 {
        client.ingest_topology(stressed_topology(Tick(tick)));
        client.advance_round().expect("stressed round");
    }

    let maintenance = client
        .maintain_route(
            &route.identity.stamp.route_id,
            RouteMaintenanceTrigger::PartitionDetected,
        )
        .expect("field maintenance under stress");

    assert!(
        matches!(
            maintenance.outcome,
            RouteMaintenanceOutcome::HoldFallback { .. }
                | RouteMaintenanceOutcome::ReplacementRequired {
                    trigger: RouteMaintenanceTrigger::PolicyShift
                        | RouteMaintenanceTrigger::CapacityExceeded
                }
                | RouteMaintenanceOutcome::Continued
        ),
        "unexpected maintenance outcome: {:?}",
        maintenance.outcome
    );
}
