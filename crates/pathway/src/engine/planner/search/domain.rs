// Search domain for pathway's telltale-search integration.
//
// `PathwaySearchDomain` implements `SearchDomain` over a set of frozen topology
// snapshots keyed by `PathwaySearchEpoch`. Each snapshot holds a precomputed
// successor list and per-node heuristic lower bounds so the search machine can
// query graph structure without holding a live reference to `Configuration`.
//
// `freeze_snapshot_for_search` and `snapshot_id_for_configuration` build and
// content-address those snapshots from an `Observation<Configuration>`.

use std::collections::VecDeque;

use jacquard_core::{Configuration, NodeId, Observation};
use jacquard_traits::{Blake3Hashing, HashDigestBytes, Hashing};
use telltale_search::{SearchDomain, SearchQuery, SearchSelectedResultSemanticsClass};

use super::{
    PathwaySearchEdgeMeta, PathwaySearchEpoch, PathwaySearchHeuristicMode, PathwaySearchSnapshotId,
};

const DOMAIN_TAG_SEARCH_SNAPSHOT: &[u8] = b"pathway-search-snapshot";

type SearchSuccessor = (NodeId, PathwaySearchEdgeMeta, u32);
type SuccessorRows = Vec<(NodeId, Vec<SearchSuccessor>)>;
type HeuristicRows = Vec<(NodeId, Vec<(NodeId, u32)>)>;

#[derive(Clone, Debug, Default)]
pub(super) struct FrozenPathwaySearchSnapshot {
    successors: SuccessorRows,
    heuristic_lower_bounds: HeuristicRows,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PathwaySearchDomain {
    snapshots: Vec<(PathwaySearchEpoch, FrozenPathwaySearchSnapshot)>,
}

impl PathwaySearchDomain {
    #[must_use]
    pub(super) fn new(snapshots: Vec<(PathwaySearchEpoch, FrozenPathwaySearchSnapshot)>) -> Self {
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
            .iter()
            .find_map(|(entry_epoch, snapshot)| (entry_epoch == epoch).then_some(snapshot))
            .ok_or("pathway search snapshot missing")?;
        if let Some(successors) = successor_row_for(&snapshot.successors, node) {
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
        self.snapshots
            .iter()
            .find_map(|(entry_epoch, snapshot)| (entry_epoch == epoch).then_some(snapshot))
            .and_then(|snapshot| heuristic_row_for(&snapshot.heuristic_lower_bounds, goal))
            .and_then(|goal_bounds| bound_for_node(goal_bounds, node))
            .unwrap_or(0)
    }

    fn selected_result_semantics_class(
        &self,
        _query: &SearchQuery<Self::Node>,
    ) -> SearchSelectedResultSemanticsClass {
        SearchSelectedResultSemanticsClass::QueryDerived
    }

    fn snapshot_id(&self, epoch: &Self::GraphEpoch) -> Self::SnapshotId {
        epoch.snapshot_id
    }
}

#[must_use]
pub(super) fn snapshot_id_for_configuration(
    configuration: &Configuration,
) -> PathwaySearchSnapshotId {
    let bytes =
        postcard::to_allocvec(configuration).expect("configuration serialization is stable");
    let digest = Blake3Hashing.hash_tagged(DOMAIN_TAG_SEARCH_SNAPSHOT, &bytes);
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(digest.as_bytes());
    PathwaySearchSnapshotId(jacquard_core::Blake3Digest(raw))
}

#[must_use]
pub(super) fn freeze_snapshot_for_search(
    observation: &Observation<Configuration>,
    successors: SuccessorRows,
    accepted_node_ids: &[NodeId],
    heuristic_mode: PathwaySearchHeuristicMode,
) -> (PathwaySearchEpoch, FrozenPathwaySearchSnapshot) {
    let snapshot_id = snapshot_id_for_configuration(&observation.value);
    let heuristic_lower_bounds = match heuristic_mode {
        PathwaySearchHeuristicMode::Zero => Vec::new(),
        PathwaySearchHeuristicMode::HopLowerBound => {
            let mut unique_goals = accepted_node_ids.to_vec();
            unique_goals.sort_unstable();
            unique_goals.dedup();
            unique_goals
                .into_iter()
                .map(|goal_node_id| (goal_node_id, hop_lower_bounds(&successors, goal_node_id)))
                .collect()
        }
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

fn hop_lower_bounds(successors: &SuccessorRows, goal_node_id: NodeId) -> Vec<(NodeId, u32)> {
    let minimum_edge_cost = successors
        .iter()
        .flat_map(|(_, edges)| edges.iter().map(|(_, _, edge_cost)| *edge_cost))
        .min()
        .unwrap_or(0);
    if minimum_edge_cost == 0 {
        return Vec::new();
    }

    let mut reverse = Vec::<(NodeId, Vec<NodeId>)>::new();
    for (from_node_id, edges) in successors {
        for (to_node_id, _, _) in edges {
            if let Some((_, predecessors)) = reverse
                .iter_mut()
                .find(|(node_id, _)| node_id == to_node_id)
            {
                predecessors.push(*from_node_id);
            } else {
                reverse.push((*to_node_id, vec![*from_node_id]));
            }
        }
    }
    reverse.sort_unstable_by_key(|(node_id, _)| *node_id);
    for (_, predecessors) in &mut reverse {
        predecessors.sort_unstable();
        predecessors.dedup();
    }

    let mut lower_bounds = Vec::new();
    let mut queue = VecDeque::from([(goal_node_id, 0_u32)]);
    while let Some((node_id, hop_distance)) = queue.pop_front() {
        if lower_bounds
            .iter()
            .any(|(seen_node_id, _)| *seen_node_id == node_id)
        {
            continue;
        }
        lower_bounds.push((node_id, hop_distance.saturating_mul(minimum_edge_cost)));
        if let Some(predecessors) = reverse.iter().find_map(|(reverse_node_id, predecessors)| {
            (reverse_node_id == &node_id).then_some(predecessors)
        }) {
            for predecessor in predecessors {
                if !lower_bounds
                    .iter()
                    .any(|(seen_node_id, _)| seen_node_id == predecessor)
                {
                    queue.push_back((*predecessor, hop_distance.saturating_add(1)));
                }
            }
        }
    }
    lower_bounds.sort_unstable_by_key(|(node_id, _)| *node_id);
    lower_bounds
}

fn successor_row_for<'a>(
    successors: &'a SuccessorRows,
    node_id: &NodeId,
) -> Option<&'a Vec<SearchSuccessor>> {
    successors
        .binary_search_by_key(node_id, |(entry_node_id, _)| *entry_node_id)
        .ok()
        .map(|index| &successors[index].1)
}

fn heuristic_row_for<'a>(
    heuristic_lower_bounds: &'a HeuristicRows,
    goal_node_id: &NodeId,
) -> Option<&'a Vec<(NodeId, u32)>> {
    heuristic_lower_bounds
        .binary_search_by_key(goal_node_id, |(entry_goal_id, _)| *entry_goal_id)
        .ok()
        .map(|index| &heuristic_lower_bounds[index].1)
}

fn bound_for_node(goal_bounds: &[(NodeId, u32)], node_id: &NodeId) -> Option<u32> {
    goal_bounds
        .binary_search_by_key(node_id, |(entry_node_id, _)| *entry_node_id)
        .ok()
        .map(|index| goal_bounds[index].1)
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
        let successors = vec![
            (a, vec![(b, stub_edge(a, b), 5)]),
            (b, vec![(c, stub_edge(b, c), 7)]),
            (c, Vec::new()),
        ];

        let (_, frozen) = freeze_snapshot_for_search(
            &empty_observation(),
            successors,
            &[c],
            PathwaySearchHeuristicMode::HopLowerBound,
        );

        assert_eq!(
            heuristic_row_for(&frozen.heuristic_lower_bounds, &c)
                .and_then(|goal| bound_for_node(goal, &c)),
            Some(0),
        );
        assert_eq!(
            heuristic_row_for(&frozen.heuristic_lower_bounds, &c)
                .and_then(|goal| bound_for_node(goal, &b)),
            Some(5),
        );
        assert_eq!(
            heuristic_row_for(&frozen.heuristic_lower_bounds, &c)
                .and_then(|goal| bound_for_node(goal, &a)),
            Some(10),
        );
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
