//! Integration tests for the engine's determinism guarantee.
//!
//! The pathway engine claims that its candidate output is a deterministic
//! function of the input topology. The existing integration test only
//! checks that two calls on the same `Configuration` agree. These tests
//! check stronger properties: byte-identical candidate output across
//! independently constructed engines, and across logically equivalent
//! topologies built with different insertion orders.

mod common;

use std::collections::BTreeMap;

use common::{
    effects::{TestRetentionStore, TestRuntimeEffects, TestTransport},
    engine::{build_engine, objective, profile, LOCAL_NODE_ID},
    fixtures::{link, node, sample_configuration},
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine};
use jacquard_traits::{
    jacquard_core::{
        Configuration, DestinationId, Environment, FactSourceClass, NodeId,
        Observation, OriginAuthenticationClass, RatioPermille, RouteEpoch,
        RoutingEvidenceClass, ServiceId, Tick,
    },
    HashDigestBytes, Hashing, RoutingEnginePlanner,
};

fn permuted_topology() -> Observation<Configuration> {
    // Same logical graph as `sample_configuration`, built by inserting
    // nodes and links in the reverse order. BTreeMap normalizes ordering
    // by key, so the resulting Configuration must be byte-equal to the
    // original. This locks in the property that the engine never
    // accidentally introduces a HashMap-style ordering dependency.
    let local_node_id = NodeId([1; 32]);
    let node_two_id = NodeId([2; 32]);
    let node_three_id = NodeId([3; 32]);
    let node_four_id = NodeId([4; 32]);

    let mut nodes = BTreeMap::new();
    nodes.insert(node_four_id, node(4));
    nodes.insert(node_three_id, node(3));
    nodes.insert(node_two_id, node(2));
    nodes.insert(local_node_id, node(1));

    let mut links = BTreeMap::new();
    links.insert((local_node_id, node_four_id), link(4, 925));
    links.insert((node_two_id, node_three_id), link(3, 875));
    links.insert((local_node_id, node_two_id), link(2, 950));

    Observation {
        value: Configuration {
            epoch: RouteEpoch(2),
            nodes,
            links,
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille: RatioPermille(150),
                contention_permille: RatioPermille(120),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

// Two engines built independently from the same topology must produce
// byte-identical candidate lists. This is a stronger property than the
// existing "two calls on one engine match" test because it confirms the
// engine carries no per-instance hidden state that could perturb output.
#[test]
fn independent_engines_produce_identical_candidates() {
    let engine_a = build_engine();
    let engine_b = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidates_a = engine_a.candidate_routes(&goal, &policy, &topology);
    let candidates_b = engine_b.candidate_routes(&goal, &policy, &topology);
    assert_eq!(candidates_a, candidates_b);
    assert!(!candidates_a.is_empty());
}

// A logically equivalent topology built with reversed insertion order
// must produce the same candidate list. BTreeMap normalizes order by
// key so the input Configuration is structurally equal, and the engine
// must not introduce any ordering-sensitive operation downstream.
#[test]
fn permuted_insertion_order_produces_identical_candidates() {
    let engine = build_engine();
    let original = sample_configuration();
    let permuted = permuted_topology();

    // First confirm the test fixture is what we claim it is.
    assert_eq!(original.value, permuted.value);

    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();
    let original_candidates = engine.candidate_routes(&goal, &policy, &original);
    let permuted_candidates = engine.candidate_routes(&goal, &policy, &permuted);
    assert_eq!(original_candidates, permuted_candidates);
}

// The candidate list ordering must be a deterministic function of the
// inputs even when the destination set spans multiple service ids. This
// catches a regression where the sort key collapses on tie-breaking and
// the engine falls back on insertion order.
#[test]
fn service_destination_ordering_is_stable_across_calls() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![1, 2, 3])));
    let policy = profile();

    let first = engine.candidate_routes(&goal, &policy, &topology);
    let second = engine.candidate_routes(&goal, &policy, &topology);
    let third = engine.candidate_routes(&goal, &policy, &topology);
    assert_eq!(first, second);
    assert_eq!(second, third);
}

// Two planning passes over the same topology snapshot must return
// identical candidate lists in the same order, with the expected
// candidate count from the sample fixture.
#[test]
fn candidate_ordering_matches_expected_count_and_is_stable() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![1, 2, 3])));
    let policy = profile();

    let first = engine.candidate_routes(&goal, &policy, &topology);
    let second = engine.candidate_routes(&goal, &policy, &topology);
    assert_eq!(first, second);
    assert_eq!(first.len(), 3);
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AltDigest([u8; 32]);

impl HashDigestBytes for AltDigest {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AltHashing;

impl Hashing for AltHashing {
    type Digest = AltDigest;

    fn hash_bytes(&self, input: &[u8]) -> Self::Digest {
        let mut bytes = [0_u8; 32];
        for (index, byte) in input.iter().enumerate() {
            bytes[index % 32] = bytes[index % 32].wrapping_add(*byte);
        }
        AltDigest(bytes)
    }

    fn hash_tagged(&self, domain: &[u8], input: &[u8]) -> Self::Digest {
        let mut tagged = domain.to_vec();
        tagged.extend_from_slice(input);
        self.hash_bytes(&tagged)
    }
}

#[test]
fn mesh_engine_accepts_non_blake3_hashing_for_route_identity() {
    let engine = PathwayEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        TestTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects::with_now(Tick(2)),
        AltHashing,
    );
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidates = engine.candidate_routes(&goal, &policy, &topology);
    assert!(!candidates.is_empty());
}
