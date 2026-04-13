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

#[test]
fn field_client_exposes_recovery_replay_continuity_end_to_end() {
    let topology = asymmetric_topology(Tick(2));
    let network = SharedInMemoryNetwork::default();
    let mut client = FieldClientBuilder::new(node(1), topology, network, Tick(2)).build();

    let route = client
        .activate_route(&default_objective(DestinationId::Node(node(2))))
        .expect("initial field activation");

    assert!(client
        .suspend_route_runtime(&route.identity.stamp.route_id)
        .expect("suspend runtime"));
    let suspended = client.exported_replay_bundle();
    let suspended_entry = suspended
        .recovery
        .entries
        .iter()
        .find(|entry| entry.route_id == route.identity.stamp.route_id)
        .expect("recovery entry");
    assert!(suspended_entry.checkpoint_available);
    assert_eq!(suspended_entry.checkpoint_capture_count, 1);
    assert_eq!(
        suspended_entry.last_outcome.as_deref(),
        Some("CheckpointStored")
    );

    assert!(client
        .restore_route_runtime(&route.identity.stamp.route_id)
        .expect("restore runtime"));
    let restored = client.exported_replay_bundle();
    let restored_entry = restored
        .recovery
        .entries
        .iter()
        .find(|entry| entry.route_id == route.identity.stamp.route_id)
        .expect("recovery entry");
    assert!(!restored_entry.checkpoint_available);
    assert_eq!(restored_entry.checkpoint_capture_count, 1);
    assert_eq!(restored_entry.checkpoint_restore_count, 1);
    assert_eq!(
        restored_entry.last_outcome.as_deref(),
        Some("CheckpointRestored")
    );
}

#[test]
fn field_client_exported_replay_bundle_records_checkpoint_restore_protocol_cause() {
    let topology = asymmetric_topology(Tick(2));
    let network = SharedInMemoryNetwork::default();
    let mut client = FieldClientBuilder::new(node(1), topology, network, Tick(2)).build();

    let route = client
        .activate_route(&default_objective(DestinationId::Node(node(2))))
        .expect("initial field activation");
    client
        .suspend_route_runtime(&route.identity.stamp.route_id)
        .expect("suspend runtime");
    client
        .restore_route_runtime(&route.identity.stamp.route_id)
        .expect("restore runtime");

    let bundle = client.exported_replay_bundle();
    assert!(bundle
        .protocol
        .reconfigurations
        .iter()
        .any(|reconfiguration| {
            reconfiguration.route_id == Some(route.identity.stamp.route_id)
                && reconfiguration.cause == "CheckpointRestore"
        }));
    let runtime_search = client.reduced_runtime_search_replay();
    assert!(runtime_search.search.is_some());
    assert_eq!(client.reduced_protocol_replay().schema_version, 1);
}
