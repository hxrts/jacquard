//! Verify that world extensions contribute self-describing observations without
//! owning canonical route state.

use jacquard_traits::{
    jacquard_core::{
        Belief, ByteCount, ControllerId, DurationMs, EndpointAddress, Environment,
        FactSourceClass, InformationSetSummary, Link, LinkEndpoint, LinkRuntimeState,
        LinkState, Node, NodeId, NodeProfile, NodeRelayBudget, NodeState, Observation,
        ObservedValue, OriginAuthenticationClass, RatioPermille, RoutingEngineId,
        RoutingEvidenceClass, ServiceDescriptor, ServiceScope, Tick, TimeWindow,
        TransportObservation, TransportProtocol, WorldError, WorldObservation,
    },
    LinkWorldExtension, NodeWorldExtension, WorldExtension, WorldExtensionDescriptor,
};

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

impl WorldExtension for StubWorldExtension {
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

fn sample_endpoint() -> LinkEndpoint {
    LinkEndpoint {
        protocol: TransportProtocol::BleGatt,
        address: EndpointAddress::Opaque(vec![1, 2, 3]),
        mtu_bytes: ByteCount(512),
    }
}

fn sample_node() -> Node {
    Node {
        controller_id: ControllerId([3; 32]),
        profile: NodeProfile {
            services: Vec::new(),
            endpoints: vec![sample_endpoint()],
            connection_count_max: 4,
            neighbor_state_count_max: 8,
            simultaneous_transfer_count_max: 2,
            active_route_count_max: 4,
            relay_work_budget_max: 16,
            maintenance_work_budget_max: 8,
            hold_item_count_max: 8,
            hold_capacity_bytes_max: ByteCount(1024),
        },
        state: NodeState {
            relay_budget: Belief::Estimated(jacquard_traits::jacquard_core::Estimate {
                value:               NodeRelayBudget {
                    relay_work_budget:    Belief::Estimated(
                        jacquard_traits::jacquard_core::Estimate {
                            value:               8,
                            confidence_permille: RatioPermille(900),
                            updated_at_tick:     Tick(2),
                        },
                    ),
                    utilization_permille: RatioPermille(250),
                    retention_horizon_ms: Belief::Estimated(
                        jacquard_traits::jacquard_core::Estimate {
                            value:               DurationMs(500),
                            confidence_permille: RatioPermille(900),
                            updated_at_tick:     Tick(2),
                        },
                    ),
                },
                confidence_permille: RatioPermille(900),
                updated_at_tick:     Tick(2),
            }),
            available_connection_count: Belief::Absent,
            hold_capacity_available_bytes: Belief::Absent,
            information_summary: Belief::Estimated(jacquard_traits::jacquard_core::Estimate {
                value: InformationSetSummary {
                    summary_encoding:
                        jacquard_traits::jacquard_core::InformationSummaryEncoding::BloomFilter,
                    item_count: Belief::Absent,
                    byte_count: Belief::Absent,
                    false_positive_permille: Belief::Absent,
                },
                confidence_permille: RatioPermille(900),
                updated_at_tick: Tick(2),
            }),
        },
    }
}

fn sample_node_observation() -> WorldObservation {
    Observation {
        value: ObservedValue::Node(sample_node()),
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn sample_link_observation() -> WorldObservation {
    Observation {
        value: ObservedValue::Link(Link {
            endpoint: sample_endpoint(),
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: DurationMs(7),
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::Estimated(
                    jacquard_traits::jacquard_core::Estimate {
                        value: RatioPermille(950),
                        confidence_permille: RatioPermille(900),
                        updated_at_tick: Tick(2),
                    },
                ),
                symmetry_permille: Belief::Estimated(
                    jacquard_traits::jacquard_core::Estimate {
                        value: RatioPermille(1000),
                        confidence_permille: RatioPermille(900),
                        updated_at_tick: Tick(2),
                    },
                ),
            },
        }),
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn sample_service_observation() -> WorldObservation {
    Observation {
        value: ObservedValue::Service(ServiceDescriptor {
            provider_node_id: NodeId([8; 32]),
            controller_id: ControllerId([3; 32]),
            service_kind: jacquard_traits::jacquard_core::RouteServiceKind::Discover,
            endpoints: vec![sample_endpoint()],
            routing_engines: vec![RoutingEngineId::Mesh],
            scope: ServiceScope::Introduction { scope_token: vec![9] },
            valid_for: TimeWindow::new(Tick(2), Tick(20))
                .expect("valid service window"),
            capacity: Belief::Estimated(jacquard_traits::jacquard_core::Estimate {
                value: jacquard_traits::jacquard_core::CapacityHint {
                    saturation_permille: RatioPermille(100),
                    repair_capacity: Belief::Estimated(
                        jacquard_traits::jacquard_core::Estimate {
                            value: 2,
                            confidence_permille: RatioPermille(900),
                            updated_at_tick: Tick(2),
                        },
                    ),
                    hold_capacity_bytes: Belief::Absent,
                },
                confidence_permille: RatioPermille(900),
                updated_at_tick: Tick(2),
            }),
        }),
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn sample_transport_observation() -> WorldObservation {
    Observation {
        value: ObservedValue::Transport(TransportObservation::PayloadReceived {
            from_node_id: NodeId([9; 32]),
            endpoint: sample_endpoint(),
            payload: b"hello".to_vec(),
            observed_at_tick: Tick(2),
        }),
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

#[test]
// long-block-exception: self-describing observation payload and assertions.
fn world_extensions_publish_self_describing_observations() {
    let mut extension = StubWorldExtension {
        observations: vec![
            sample_node_observation(),
            sample_link_observation(),
            Observation {
                value: ObservedValue::Environment(Environment {
                    reachable_neighbor_count: 1,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(100),
                }),
                source_class: FactSourceClass::Local,
                evidence_class: RoutingEvidenceClass::DirectObservation,
                origin_authentication: OriginAuthenticationClass::Controlled,
                observed_at_tick: Tick(2),
            },
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
            Belief::Estimated(jacquard_traits::jacquard_core::Estimate {
                value: NodeRelayBudget {
                    relay_work_budget: Belief::Estimated(
                        jacquard_traits::jacquard_core::Estimate {
                            value: 8,
                            confidence_permille: RatioPermille(900),
                            updated_at_tick: Tick(2),
                        },
                    ),
                    utilization_permille: RatioPermille(250),
                    retention_horizon_ms: Belief::Estimated(
                        jacquard_traits::jacquard_core::Estimate {
                            value: DurationMs(500),
                            confidence_permille: RatioPermille(900),
                            updated_at_tick: Tick(2),
                        },
                    ),
                },
                confidence_permille: RatioPermille(900),
                updated_at_tick: Tick(2),
            }),
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
