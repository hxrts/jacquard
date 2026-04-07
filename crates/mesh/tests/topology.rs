//! Integration tests for the deterministic mesh topology model.
//!
//! Unit tests in `topology.rs` cover the small private helpers that
//! drive route capability and adjacency logic. This file exercises the
//! public `DeterministicMeshTopologyModel` query surface against the
//! standard sample fixture and confirms the mesh-private intrinsic,
//! medium, and neighborhood estimates are derived as expected.

mod common;

use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_traits::{
    jacquard_core::{DestinationId, Node, NodeId, RoutingObjective, ServiceId, TransportProtocol},
    Blake3Hashing, MeshTopologyModel, RoutingEnginePlanner,
};

use common::effects::{TestRetentionStore, TestRuntimeEffects, TestTransport};
use common::engine::{profile, LOCAL_NODE_ID};
use common::fixtures::sample_configuration;

// The deterministic topology model must surface mesh-private intrinsic
// node state, per-protocol medium counts, and a non-trivial
// neighborhood density estimate from a shared `Configuration`.
#[test]
fn topology_model_exposes_medium_and_node_intrinsic_support() {
    let topology = sample_configuration();
    let model = DeterministicMeshTopologyModel::new();

    let intrinsic = model
        .node_intrinsic_state(&NodeId([1; 32]), &topology.value)
        .expect("local node intrinsic state");
    let medium = model.medium_state(&NodeId([1; 32]), &topology.value);
    let neighborhood = model
        .neighborhood_estimate(&NodeId([1; 32]), topology.observed_at_tick, &topology.value)
        .expect("neighborhood estimate");

    assert_eq!(intrinsic.available_connection_count, 4);
    assert_eq!(
        medium.protocol_counts.get(&TransportProtocol::BleGatt),
        Some(&2)
    );
    assert!(neighborhood.density_score.expect("density score").0 > 0);
}

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
    type PeerEstimate = jacquard_mesh::MeshPeerEstimate;
    type NeighborhoodEstimate = jacquard_mesh::MeshNeighborhoodEstimate;

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
            estimate.relay_value_score = Some(jacquard_traits::jacquard_core::HealthScore(1000));
            estimate.service_score = Some(jacquard_traits::jacquard_core::HealthScore(1000));
        } else {
            estimate.relay_value_score = Some(jacquard_traits::jacquard_core::HealthScore(0));
            estimate.service_score = Some(jacquard_traits::jacquard_core::HealthScore(0));
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
        TestRuntimeEffects {
            now: jacquard_traits::jacquard_core::Tick(2),
            ..Default::default()
        },
        Blake3Hashing,
    )
}

fn service_objective() -> RoutingObjective {
    let mut objective = common::engine::objective(DestinationId::Service(ServiceId(vec![9; 16])));
    objective.service_kind = jacquard_traits::jacquard_core::RouteServiceKind::Move;
    objective
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
