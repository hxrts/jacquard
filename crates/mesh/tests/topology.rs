//! Integration tests for the deterministic mesh topology model.
//!
//! Unit tests in `topology.rs` cover the private helper and internal
//! topology-derivation surfaces. This file stays focused on the public
//! topology-model contract and candidate-ordering behavior.

mod common;

use std::collections::BTreeMap;

use common::{
    effects::{TestRetentionStore, TestRuntimeEffects, TestTransport},
    engine::{materialization_input, objective, profile, LOCAL_NODE_ID},
    fixtures::{link, node, sample_configuration},
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine, MeshTopologyModel};
use jacquard_traits::{
    jacquard_core::{
        Configuration, DestinationId, Environment, Node, NodeId, Observation,
        RatioPermille, RouteEpoch, RoutingObjective, RoutingTickContext, ServiceId,
        Tick,
    },
    Blake3Hashing, RoutingEngine, RoutingEnginePlanner,
};

#[derive(Clone)]
struct PreferredPeerTopologyModel {
    base: DeterministicMeshTopologyModel,
    preferred_peer: NodeId,
}

impl PreferredPeerTopologyModel {
    fn new(preferred_peer: NodeId) -> Self {
        Self {
            base: DeterministicMeshTopologyModel::new(),
            preferred_peer,
        }
    }
}

impl MeshTopologyModel for PreferredPeerTopologyModel {
    type NeighborhoodEstimate = jacquard_mesh::MeshNeighborhoodEstimate;
    type PeerEstimate = jacquard_mesh::MeshPeerEstimate;

    fn local_node(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Option<Node> {
        self.base.local_node(local_node_id, configuration)
    }

    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Vec<(NodeId, Node)> {
        self.base.neighboring_nodes(local_node_id, configuration)
    }

    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Vec<jacquard_traits::jacquard_core::LinkEndpoint> {
        self.base.reachable_endpoints(local_node_id, configuration)
    }

    fn adjacent_links(
        &self,
        local_node_id: &NodeId,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Vec<jacquard_traits::jacquard_core::Link> {
        self.base.adjacent_links(local_node_id, configuration)
    }

    fn peer_estimate(
        &self,
        local_node_id: &NodeId,
        peer_node_id: &NodeId,
        observed_at_tick: jacquard_traits::jacquard_core::Tick,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Option<Self::PeerEstimate> {
        let mut estimate = self.base.peer_estimate(
            local_node_id,
            peer_node_id,
            observed_at_tick,
            configuration,
        )?;
        if *peer_node_id == self.preferred_peer {
            estimate.relay_value_score =
                Some(jacquard_traits::jacquard_core::HealthScore(1000));
            estimate.service_score =
                Some(jacquard_traits::jacquard_core::HealthScore(1000));
        } else {
            estimate.relay_value_score =
                Some(jacquard_traits::jacquard_core::HealthScore(0));
            estimate.service_score =
                Some(jacquard_traits::jacquard_core::HealthScore(0));
        }
        Some(estimate)
    }

    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        observed_at_tick: jacquard_traits::jacquard_core::Tick,
        configuration: &jacquard_traits::jacquard_core::Configuration,
    ) -> Option<Self::NeighborhoodEstimate> {
        self.base
            .neighborhood_estimate(local_node_id, observed_at_tick, configuration)
    }
}

type PreferredEngine = MeshEngine<
    PreferredPeerTopologyModel,
    TestTransport,
    TestRetentionStore,
    TestRuntimeEffects,
    Blake3Hashing,
>;

fn build_preferred_engine(preferred_peer: NodeId) -> PreferredEngine {
    MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        PreferredPeerTopologyModel::new(preferred_peer),
        TestTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects::with_now(jacquard_traits::jacquard_core::Tick(2)),
        Blake3Hashing,
    )
}

fn service_objective() -> RoutingObjective {
    let mut objective =
        common::engine::objective(DestinationId::Service(ServiceId(vec![9; 16])));
    objective.service_kind = jacquard_traits::jacquard_core::RouteServiceKind::Move;
    objective
}

fn equal_hop_quality_configuration() -> Observation<Configuration> {
    let node_two = NodeId([2; 32]);
    let node_three = NodeId([3; 32]);
    let destination = NodeId([5; 32]);

    Observation {
        value: Configuration {
            epoch: RouteEpoch(9),
            nodes: BTreeMap::from([
                (LOCAL_NODE_ID, node(1)),
                (node_two, node(2)),
                (node_three, node(3)),
                (destination, node(5)),
            ]),
            links: BTreeMap::from([
                ((LOCAL_NODE_ID, node_two), link(2, 950)),
                ((node_two, destination), link(5, 950)),
                ((LOCAL_NODE_ID, node_three), link(3, 650)),
                ((node_three, destination), link(6, 650)),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille: RatioPermille(100),
                contention_permille: RatioPermille(100),
            },
        },
        source_class: jacquard_traits::jacquard_core::FactSourceClass::Local,
        evidence_class:
            jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
        origin_authentication:
            jacquard_traits::jacquard_core::OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(9),
    }
}

#[test]
fn topology_model_estimates_can_change_candidate_ordering_deterministically() {
    let topology = sample_configuration();
    let goal = service_objective();
    let policy = profile();

    let engine_for_node_two = build_preferred_engine(NodeId([2; 32]));
    let engine_for_node_four = build_preferred_engine(NodeId([4; 32]));

    let first_two = engine_for_node_two
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("first candidate with node two preference");
    let first_four = engine_for_node_four
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("first candidate with node four preference");

    assert_ne!(first_two.backend_ref, first_four.backend_ref);
}

#[test]
fn metric_aware_search_prefers_higher_quality_equal_hop_path() {
    let topology = equal_hop_quality_configuration();
    let goal = objective(DestinationId::Node(NodeId([5; 32])));
    let policy = profile();
    let mut engine =
        common::engine::build_engine_for_node_at_tick(LOCAL_NODE_ID, Tick(9));

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate to destination five");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admit route");
    let lease = jacquard_traits::jacquard_core::RouteLease {
        owner_node_id: LOCAL_NODE_ID,
        lease_epoch: RouteEpoch(9),
        valid_for: jacquard_traits::jacquard_core::TimeWindow::new(Tick(9), Tick(20))
            .expect("valid lease"),
    };
    let installation = engine
        .materialize_route(materialization_input(admission, lease))
        .expect("materialize route");

    let route = engine
        .active_route(&installation.materialization_proof.stamp.route_id)
        .expect("active route");
    assert_eq!(route.first_hop_node_id, Some(NodeId([2; 32])));
}
