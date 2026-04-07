//! Integration tests for the deterministic mesh topology model.
//!
//! Unit tests in `topology.rs` cover the small private helpers that
//! drive route capability and adjacency logic. This file exercises the
//! public `DeterministicMeshTopologyModel` query surface against the
//! standard sample fixture and confirms the mesh-private intrinsic,
//! medium, and neighborhood estimates are derived as expected.

mod common;

use common::sample_configuration;
use jacquard_mesh::DeterministicMeshTopologyModel;
use jacquard_traits::{
    jacquard_core::{NodeId, TransportProtocol},
    MeshTopologyModel,
};

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
        .neighborhood_estimate(&NodeId([1; 32]), &topology.value)
        .expect("neighborhood estimate");

    assert_eq!(intrinsic.available_connection_count, 4);
    assert_eq!(
        medium.protocol_counts.get(&TransportProtocol::BleGatt),
        Some(&2)
    );
    assert!(neighborhood.density_score.0 > 0);
}
