use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, Environment, FactSourceClass, NodeId, Observation,
    OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass, Tick,
};
use jacquard_reference_client::{
    topology, BridgeQueueConfig, BridgeRoundProgress, ClientBuilder,
    SharedInMemoryNetwork,
};

fn sample_topology(local_node_id: NodeId) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(
                local_node_id,
                topology::node(1).pathway().build(),
            )]),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(1),
    }
}

#[test]
fn client_builder_constructs_waiting_pathway_bridge() {
    let local_node_id = NodeId([1; 32]);
    let topology = sample_topology(local_node_id);
    let network = SharedInMemoryNetwork::default();
    let mut client =
        ClientBuilder::pathway(local_node_id, topology, network, Tick(1)).build();
    let mut bound = client.bind();

    let progress = bound.advance_round().expect("advance initial round");

    match progress {
        | BridgeRoundProgress::Advanced(report) => {
            assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
        },
        | BridgeRoundProgress::Waiting(_) => {},
    }
}

#[test]
fn client_builder_accepts_explicit_queue_config_and_profile() {
    let local_node_id = NodeId([1; 32]);
    let mut client = ClientBuilder::pathway(
        local_node_id,
        sample_topology(local_node_id),
        SharedInMemoryNetwork::default(),
        Tick(1),
    )
    .with_queue_config(BridgeQueueConfig::new(1, 1))
    .with_batman()
    .build();
    let mut bound = client.bind();

    let progress = bound.advance_round().expect("advance initial round");

    match progress {
        | BridgeRoundProgress::Advanced(report) => {
            assert_eq!(report.router_outcome.topology_epoch, RouteEpoch(1));
        },
        | BridgeRoundProgress::Waiting(_) => {},
    }
}
