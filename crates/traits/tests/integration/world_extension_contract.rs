//! Verify that world extensions contribute self-describing observations without
//! owning canonical route state.

use jacquard_traits::{
    jacquard_core::{
        Belief, ControllerId, DurationMs, Environment, Link,
        LinkRuntimeState, LinkState, NodeId, NodeRelayBudget, Observation,
        ObservedValue, RatioPermille, RepairCapacitySlots, RoutingEngineId,
        ServiceDescriptor, ServiceScope, Tick, TimeWindow, TransportObservation,
        TransportProtocol, WorldError, WorldObservation,
    },
    LinkWorldExtension, NodeWorldExtension, WorldExtension, WorldExtensionDescriptor,
};

use super::common;

struct StubWorldExtension {
    observations: Vec<WorldObservation>,
}

impl WorldExtensionDescriptor for StubWorldExtension {
    fn extension_id(&self) -> &str {
        "stub-world"
    }

    fn supported_transports(&self) -> Vec<TransportProtocol> {
        vec![TransportProtocol::BleGatt, TransportProtocol::WifiLan]
    }
}

impl WorldExtension<ObservedValue> for StubWorldExtension {
    fn poll_observations(&mut self) -> Result<Vec<WorldObservation>, WorldError> {
        Ok(self.observations.clone())
    }
}

impl NodeWorldExtension for StubWorldExtension {
    fn poll_node_observations(
        &mut self,
    ) -> Result<Vec<jacquard_traits::jacquard_core::NodeObservation>, WorldError> {
        Ok(self
            .observations
            .iter()
            .filter_map(|observation| match &observation.value {
                | ObservedValue::Node(node) => Some(Observation {
                    value: node.clone(),
                    source_class: observation.source_class,
                    evidence_class: observation.evidence_class,
                    origin_authentication: observation.origin_authentication,
                    observed_at_tick: observation.observed_at_tick,
                }),
                | _ => None,
            })
            .collect())
    }
}

impl LinkWorldExtension for StubWorldExtension {
    fn poll_link_observations(
        &mut self,
    ) -> Result<Vec<jacquard_traits::jacquard_core::LinkObservation>, WorldError> {
        Ok(self
            .observations
            .iter()
            .filter_map(|observation| match &observation.value {
                | ObservedValue::Link(link) => Some(Observation {
                    value: link.clone(),
                    source_class: observation.source_class,
                    evidence_class: observation.evidence_class,
                    origin_authentication: observation.origin_authentication,
                    observed_at_tick: observation.observed_at_tick,
                }),
                | _ => None,
            })
            .collect())
    }
}

fn sample_node_observation() -> WorldObservation {
    common::local_observation(ObservedValue::Node(common::sample_node()), Tick(2))
}

fn sample_link_observation() -> WorldObservation {
    common::local_observation(
        ObservedValue::Link(Link {
            endpoint: common::sample_endpoint(),
            profile: jacquard_traits::jacquard_core::LinkProfile {
                latency_floor_ms: DurationMs(2),
                repair_capability:
                    jacquard_traits::jacquard_core::RepairCapability::TransportRetransmit,
                partition_recovery:
                    jacquard_traits::jacquard_core::PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: common::estimated(
                    RatioPermille(950),
                    900,
                    Tick(2),
                ),
                symmetry_permille: common::estimated(RatioPermille(1000), 900, Tick(2)),
            },
        }),
        Tick(2),
    )
}

fn sample_service_observation() -> WorldObservation {
    common::local_observation(
        ObservedValue::Service(ServiceDescriptor {
            provider_node_id: NodeId([8; 32]),
            controller_id: ControllerId([3; 32]),
            service_kind: jacquard_traits::jacquard_core::RouteServiceKind::Discover,
            endpoints: vec![common::sample_endpoint()],
            routing_engines: vec![RoutingEngineId::from_contract_bytes([1; 16])],
            scope: ServiceScope::Introduction { scope_token: vec![9] },
            valid_for: TimeWindow::new(Tick(2), Tick(20))
                .expect("valid service window"),
            capacity: common::estimated(
                jacquard_traits::jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity_slots: common::estimated(
                        RepairCapacitySlots(2),
                        900,
                        Tick(2),
                    ),
                    hold_capacity_bytes: Belief::Absent,
                },
                900,
                Tick(2),
            ),
        }),
        Tick(2),
    )
}

fn sample_transport_observation() -> WorldObservation {
    common::local_observation(
        ObservedValue::Transport(TransportObservation::PayloadReceived {
            from_node_id: NodeId([9; 32]),
            endpoint: common::sample_endpoint(),
            payload: b"hello".to_vec(),
            observed_at_tick: Tick(2),
        }),
        Tick(2),
    )
}

#[test]
// long-block-exception: self-describing observation payload and assertions.
fn world_extensions_publish_self_describing_observations() {
    let mut extension = StubWorldExtension {
        observations: vec![
            sample_node_observation(),
            sample_link_observation(),
            common::local_observation(
                ObservedValue::Environment(Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(100),
                }),
                Tick(2),
            ),
            sample_service_observation(),
            sample_transport_observation(),
        ],
    };

    let observations = extension.poll_observations().expect("observations");

    assert_eq!(extension.extension_id(), "stub-world");
    assert_eq!(
        extension.supported_transports(),
        vec![TransportProtocol::BleGatt, TransportProtocol::WifiLan],
    );
    assert_eq!(observations.len(), 5);
    assert!(matches!(observations[0].value, ObservedValue::Node(_)));
    assert!(matches!(observations[1].value, ObservedValue::Link(_)));
    assert!(matches!(
        observations[2].value,
        ObservedValue::Environment(_)
    ));
    assert!(matches!(observations[3].value, ObservedValue::Service(_)));
    assert!(matches!(observations[4].value, ObservedValue::Transport(_)));

    match &observations[0].value {
        | ObservedValue::Node(node) => assert_eq!(
            node.state.relay_budget,
            common::estimated(
                NodeRelayBudget {
                    relay_work_budget: common::estimated(
                        jacquard_traits::jacquard_core::RelayWorkBudget(8),
                        900,
                        Tick(2),
                    ),
                    utilization_permille: RatioPermille(250),
                    retention_horizon_ms: common::estimated(
                        DurationMs(500),
                        900,
                        Tick(2)
                    ),
                },
                900,
                Tick(2),
            ),
        ),
        | _ => panic!("expected node observation"),
    }

    match &observations[4].value {
        | ObservedValue::Transport(observation) => match observation {
            | TransportObservation::PayloadReceived { payload, .. } => {
                assert_eq!(payload, &b"hello".to_vec());
            },
            | _ => panic!("expected payload transport observation"),
        },
        | _ => panic!("expected transport observation"),
    }
}

#[test]
fn world_extension_facets_can_contribute_nodes_and_links_explicitly() {
    let mut extension = StubWorldExtension {
        observations: vec![sample_node_observation(), sample_link_observation()],
    };

    let node_observations = extension
        .poll_node_observations()
        .expect("node observations");
    let link_observations = extension
        .poll_link_observations()
        .expect("link observations");

    assert_eq!(node_observations.len(), 1);
    assert_eq!(link_observations.len(), 1);
    assert_eq!(
        node_observations[0].value.controller_id,
        ControllerId([3; 32])
    );
    assert_eq!(
        link_observations[0].value.endpoint.protocol,
        TransportProtocol::BleGatt
    );
}
