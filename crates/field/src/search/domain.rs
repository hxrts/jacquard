//! Search domain for field's telltale-search integration.

use std::collections::VecDeque;

use jacquard_core::{Configuration, NodeId, Observation};
use jacquard_traits::{Blake3Hashing, HashDigestBytes, Hashing};
use serde::{Deserialize, Serialize};
use telltale_search::{SearchDomain, SearchQuery, SearchSelectedResultSemanticsClass};

use super::{
    FieldSearchEdgeMeta, FieldSearchEpoch, FieldSearchHeuristicMode, FieldSearchSnapshotId,
};

const DOMAIN_TAG_FIELD_SEARCH_SNAPSHOT: &[u8] = b"field-search-snapshot";

type SearchSuccessor = (NodeId, FieldSearchEdgeMeta, u32);
type SuccessorRows = Vec<(NodeId, Vec<SearchSuccessor>)>;
type HeuristicRows = Vec<(NodeId, Vec<(NodeId, u32)>)>;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FrozenFieldSearchSnapshot {
    successors: SuccessorRows,
    heuristic_lower_bounds: HeuristicRows,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FieldSearchDomain {
    snapshots: Vec<(FieldSearchEpoch, FrozenFieldSearchSnapshot)>,
}

impl FieldSearchDomain {
    #[must_use]
    pub(super) fn new(snapshots: Vec<(FieldSearchEpoch, FrozenFieldSearchSnapshot)>) -> Self {
        Self { snapshots }
    }
}

impl SearchDomain for FieldSearchDomain {
    type Node = NodeId;
    type EdgeMeta = FieldSearchEdgeMeta;
    type Cost = u32;
    type GraphEpoch = FieldSearchEpoch;
    type SnapshotId = FieldSearchSnapshotId;
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
            .ok_or("field search snapshot missing")?;
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
pub(super) fn snapshot_id_for_search_snapshot(
    snapshot: &FrozenFieldSearchSnapshot,
) -> FieldSearchSnapshotId {
    let bytes =
        bincode::serialize(snapshot).expect("field search snapshot serialization is stable");
    let digest = Blake3Hashing.hash_tagged(DOMAIN_TAG_FIELD_SEARCH_SNAPSHOT, &bytes);
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(digest.as_bytes());
    FieldSearchSnapshotId(jacquard_core::Blake3Digest(raw))
}

#[must_use]
pub(super) fn freeze_snapshot_for_search(
    observation: &Observation<Configuration>,
    successors: SuccessorRows,
    accepted_node_ids: &[NodeId],
    heuristic_mode: FieldSearchHeuristicMode,
) -> (FieldSearchEpoch, FrozenFieldSearchSnapshot) {
    let heuristic_lower_bounds = match heuristic_mode {
        FieldSearchHeuristicMode::Zero => Vec::new(),
        FieldSearchHeuristicMode::HopLowerBound => {
            let mut unique_goals = accepted_node_ids.to_vec();
            unique_goals.sort_unstable();
            unique_goals.dedup();
            unique_goals
                .into_iter()
                .map(|goal_node_id| (goal_node_id, hop_lower_bounds(&successors, goal_node_id)))
                .collect()
        }
    };
    let snapshot = FrozenFieldSearchSnapshot {
        successors,
        heuristic_lower_bounds,
    };
    let snapshot_id = snapshot_id_for_search_snapshot(&snapshot);
    (
        FieldSearchEpoch {
            route_epoch: observation.value.epoch,
            snapshot_id,
        },
        snapshot,
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
