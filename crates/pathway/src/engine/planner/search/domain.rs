// Search domain for pathway's telltale-search integration.
//
// `PathwaySearchDomain` implements `SearchDomain` over a set of frozen topology
// snapshots keyed by `PathwaySearchEpoch`. Each snapshot holds a precomputed
// successor list and per-node heuristic lower bounds so the search machine can
// query graph structure without holding a live reference to `Configuration`.
//
// `freeze_snapshot_for_search` and `snapshot_id_for_configuration` build and
// content-address those snapshots from an `Observation<Configuration>`.

use std::collections::{BTreeMap, VecDeque};

use jacquard_core::{Configuration, NodeId, Observation};
use jacquard_traits::{Blake3Hashing, HashDigestBytes, Hashing};
use telltale_search::SearchDomain;

use super::{
    PathwaySearchEdgeMeta, PathwaySearchEpoch, PathwaySearchHeuristicMode, PathwaySearchSnapshotId,
};

const DOMAIN_TAG_SEARCH_SNAPSHOT: &[u8] = b"pathway-search-snapshot";

type SearchSuccessor = (NodeId, PathwaySearchEdgeMeta, u32);

#[derive(Clone, Debug, Default)]
pub(super) struct FrozenPathwaySearchSnapshot {
    successors: BTreeMap<NodeId, Vec<SearchSuccessor>>,
    heuristic_lower_bounds: BTreeMap<NodeId, u32>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PathwaySearchDomain {
    snapshots: BTreeMap<PathwaySearchEpoch, FrozenPathwaySearchSnapshot>,
}

impl PathwaySearchDomain {
    #[must_use]
    pub(super) fn new(
        snapshots: BTreeMap<PathwaySearchEpoch, FrozenPathwaySearchSnapshot>,
    ) -> Self {
        Self { snapshots }
    }
}

impl SearchDomain for PathwaySearchDomain {
    type Node = NodeId;
    type EdgeMeta = PathwaySearchEdgeMeta;
    type Cost = u32;
    type GraphEpoch = PathwaySearchEpoch;
    type SnapshotId = PathwaySearchSnapshotId;
    type Error = &'static str;

    fn successors(
        &self,
        epoch: &Self::GraphEpoch,
        node: &Self::Node,
        out: &mut Vec<(Self::Node, Self::EdgeMeta, Self::Cost)>,
    ) -> Result<(), Self::Error> {
        let snapshot = self
            .snapshots
            .get(epoch)
            .ok_or("pathway search snapshot missing")?;
        if let Some(successors) = snapshot.successors.get(node) {
            out.extend(successors.iter().cloned());
        }
        Ok(())
    }

    fn heuristic(
        &self,
        epoch: &Self::GraphEpoch,
        node: &Self::Node,
        goal: &Self::Node,
    ) -> Self::Cost {
        let _ = goal;
        self.snapshots
            .get(epoch)
            .and_then(|snapshot| snapshot.heuristic_lower_bounds.get(node).copied())
            .unwrap_or(0)
    }

    fn snapshot_id(&self, epoch: &Self::GraphEpoch) -> Self::SnapshotId {
        epoch.snapshot_id
    }
}

#[must_use]
pub(super) fn snapshot_id_for_configuration(
    configuration: &Configuration,
) -> PathwaySearchSnapshotId {
    let bytes = bincode::serialize(configuration).expect("configuration serialization is stable");
    let digest = Blake3Hashing.hash_tagged(DOMAIN_TAG_SEARCH_SNAPSHOT, &bytes);
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(digest.as_bytes());
    PathwaySearchSnapshotId(jacquard_core::Blake3Digest(raw))
}

#[must_use]
pub(super) fn freeze_snapshot_for_search(
    observation: &Observation<Configuration>,
    successors: BTreeMap<NodeId, Vec<SearchSuccessor>>,
    goal_node_id: NodeId,
    heuristic_mode: PathwaySearchHeuristicMode,
) -> (PathwaySearchEpoch, FrozenPathwaySearchSnapshot) {
    let snapshot_id = snapshot_id_for_configuration(&observation.value);
    let heuristic_lower_bounds = match heuristic_mode {
        PathwaySearchHeuristicMode::Zero => BTreeMap::new(),
        PathwaySearchHeuristicMode::HopLowerBound => hop_lower_bounds(&successors, goal_node_id),
    };
    (
        PathwaySearchEpoch {
            route_epoch: observation.value.epoch,
            snapshot_id,
        },
        FrozenPathwaySearchSnapshot {
            successors,
            heuristic_lower_bounds,
        },
    )
}

fn hop_lower_bounds(
    successors: &BTreeMap<NodeId, Vec<SearchSuccessor>>,
    goal_node_id: NodeId,
) -> BTreeMap<NodeId, u32> {
    let minimum_edge_cost = successors
        .values()
        .flat_map(|edges| edges.iter().map(|(_, _, edge_cost)| *edge_cost))
        .min()
        .unwrap_or(0);
    if minimum_edge_cost == 0 {
        return BTreeMap::new();
    }

    let mut reverse = BTreeMap::<NodeId, Vec<NodeId>>::new();
    for (from_node_id, edges) in successors {
        for (to_node_id, _, _) in edges {
            reverse.entry(*to_node_id).or_default().push(*from_node_id);
        }
    }

    let mut lower_bounds = BTreeMap::new();
    let mut queue = VecDeque::from([(goal_node_id, 0_u32)]);
    while let Some((node_id, hop_distance)) = queue.pop_front() {
        if lower_bounds.contains_key(&node_id) {
            continue;
        }
        lower_bounds.insert(node_id, hop_distance.saturating_mul(minimum_edge_cost));
        if let Some(predecessors) = reverse.get(&node_id) {
            for predecessor in predecessors {
                if !lower_bounds.contains_key(predecessor) {
                    queue.push_back((*predecessor, hop_distance.saturating_add(1)));
                }
            }
        }
    }
    lower_bounds
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Configuration, Environment, NodeId, Observation, RatioPermille, RouteEpoch, Tick,
    };

    use super::*;

    fn empty_observation() -> Observation<Configuration> {
        Observation {
            value: Configuration {
                epoch: RouteEpoch(7),
                nodes: BTreeMap::new(),
                links: BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 0,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: jacquard_core::FactSourceClass::Local,
            evidence_class: jacquard_core::RoutingEvidenceClass::DirectObservation,
            origin_authentication: jacquard_core::OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        }
    }

    #[test]
    fn snapshot_id_is_deterministic() {
        let observation = empty_observation();
        assert_eq!(
            snapshot_id_for_configuration(&observation.value),
            snapshot_id_for_configuration(&observation.value),
        );
    }

    #[test]
    fn hop_lower_bound_is_derived_from_reverse_distance() {
        let a = NodeId([1; 32]);
        let b = NodeId([2; 32]);
        let c = NodeId([3; 32]);
        let successors = BTreeMap::from([
            (a, vec![(b, stub_edge(a, b), 5)]),
            (b, vec![(c, stub_edge(b, c), 7)]),
            (c, Vec::new()),
        ]);

        let (_, frozen) = freeze_snapshot_for_search(
            &empty_observation(),
            successors,
            c,
            PathwaySearchHeuristicMode::HopLowerBound,
        );

        assert_eq!(frozen.heuristic_lower_bounds.get(&c), Some(&0));
        assert_eq!(frozen.heuristic_lower_bounds.get(&b), Some(&5));
        assert_eq!(frozen.heuristic_lower_bounds.get(&a), Some(&10));
    }

    fn stub_edge(from_node_id: NodeId, to_node_id: NodeId) -> PathwaySearchEdgeMeta {
        PathwaySearchEdgeMeta {
            from_node_id,
            to_node_id,
            endpoint: jacquard_core::LinkEndpoint::new(
                jacquard_core::TransportKind::WifiAware,
                jacquard_core::EndpointLocator::Opaque(vec![1]),
                jacquard_core::ByteCount(16),
            ),
        }
    }
}
