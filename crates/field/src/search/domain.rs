//! Search domain for field's telltale-search integration.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use jacquard_core::{Configuration, NodeId, Observation};
use jacquard_traits::{Blake3Hashing, HashDigestBytes, Hashing};
use serde::{Deserialize, Serialize};
use telltale_search::{SearchDomain, SearchQuery, SearchSelectedResultSemanticsClass};

use super::{
    FieldSearchEdgeMeta, FieldSearchEpoch, FieldSearchHeuristicMode, FieldSearchSnapshotId,
};

const DOMAIN_TAG_FIELD_SEARCH_SNAPSHOT: &[u8] = b"field-search-snapshot";

type SearchSuccessor = (NodeId, FieldSearchEdgeMeta, u32);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FrozenFieldSearchSnapshot {
    successors: BTreeMap<NodeId, Vec<SearchSuccessor>>,
    heuristic_lower_bounds: BTreeMap<NodeId, BTreeMap<NodeId, u32>>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FieldSearchDomain {
    snapshots: BTreeMap<FieldSearchEpoch, FrozenFieldSearchSnapshot>,
}

impl FieldSearchDomain {
    #[must_use]
    pub(super) fn new(snapshots: BTreeMap<FieldSearchEpoch, FrozenFieldSearchSnapshot>) -> Self {
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
            .get(epoch)
            .ok_or("field search snapshot missing")?;
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
        self.snapshots
            .get(epoch)
            .and_then(|snapshot| snapshot.heuristic_lower_bounds.get(goal))
            .and_then(|goal_bounds| goal_bounds.get(node).copied())
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
    successors: BTreeMap<NodeId, Vec<SearchSuccessor>>,
    accepted_node_ids: &[NodeId],
    heuristic_mode: FieldSearchHeuristicMode,
) -> (FieldSearchEpoch, FrozenFieldSearchSnapshot) {
    let heuristic_lower_bounds = match heuristic_mode {
        FieldSearchHeuristicMode::Zero => BTreeMap::new(),
        FieldSearchHeuristicMode::HopLowerBound => accepted_node_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|goal_node_id| (goal_node_id, hop_lower_bounds(&successors, goal_node_id)))
            .collect(),
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
