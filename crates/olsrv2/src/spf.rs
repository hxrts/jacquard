//! Deterministic shortest-path derivation for the OLSRv2 topology database.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{DegradationReason, NodeId, RouteDegradation, Tick, TransportKind};

use crate::public_state::{NeighborLinkState, OlsrBestNextHop, SelectedOlsrRoute, TopologyTuple};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RouteState {
    cost: u32,
    hop_count: u8,
    first_hop: NodeId,
}

#[must_use]
pub(crate) fn derive_routes(
    local_node_id: NodeId,
    neighbors: &BTreeMap<NodeId, NeighborLinkState>,
    topology_tuples: &BTreeMap<(NodeId, NodeId), TopologyTuple>,
    topology_epoch: jacquard_core::RouteEpoch,
    now: Tick,
) -> (
    BTreeMap<NodeId, SelectedOlsrRoute>,
    BTreeMap<NodeId, OlsrBestNextHop>,
) {
    let adjacency = adjacency(local_node_id, neighbors, topology_tuples);
    let route_states = shortest_paths(local_node_id, &adjacency);
    let mut selected_routes = BTreeMap::new();
    let mut best_next_hops = BTreeMap::new();

    for (destination, state) in route_states {
        if destination == local_node_id {
            continue;
        }
        let transport_kind = neighbors
            .get(&state.first_hop)
            .map(|neighbor| neighbor.transport_kind.clone())
            .unwrap_or_else(|| TransportKind::Custom("unknown".into()));
        let degradation = degradation_for(state.cost, state.hop_count);
        let backend_route_id = backend_route_id_for(destination, state.first_hop, state.cost);

        selected_routes.insert(
            destination,
            SelectedOlsrRoute {
                destination,
                next_hop: state.first_hop,
                hop_count: state.hop_count,
                path_cost: state.cost,
                degradation,
                transport_kind: transport_kind.clone(),
                observed_at_tick: now,
            },
        );
        best_next_hops.insert(
            destination,
            OlsrBestNextHop {
                destination,
                next_hop: state.first_hop,
                hop_count: state.hop_count,
                path_cost: state.cost,
                degradation,
                transport_kind,
                updated_at_tick: now,
                topology_epoch,
                backend_route_id,
            },
        );
    }

    (selected_routes, best_next_hops)
}

fn adjacency(
    local_node_id: NodeId,
    neighbors: &BTreeMap<NodeId, NeighborLinkState>,
    topology_tuples: &BTreeMap<(NodeId, NodeId), TopologyTuple>,
) -> BTreeMap<NodeId, Vec<(NodeId, u32)>> {
    let mut adjacency = BTreeMap::new();

    adjacency.insert(
        local_node_id,
        neighbors
            .iter()
            .filter(|(_, state)| state.is_symmetric)
            .map(|(neighbor, state)| (*neighbor, state.link_cost.max(1)))
            .collect(),
    );

    for tuple in topology_tuples.values() {
        adjacency
            .entry(tuple.originator)
            .or_insert_with(Vec::new)
            .push((tuple.advertised_neighbor, 1));
    }

    for edges in adjacency.values_mut() {
        edges.sort_by_key(|(neighbor, cost)| (*cost, *neighbor));
        edges.dedup();
    }

    adjacency
}

fn shortest_paths(
    local_node_id: NodeId,
    adjacency: &BTreeMap<NodeId, Vec<(NodeId, u32)>>,
) -> BTreeMap<NodeId, RouteState> {
    let mut unsettled_nodes = BTreeSet::from([local_node_id]);
    let mut states = BTreeMap::from([(
        local_node_id,
        RouteState {
            cost: 0,
            hop_count: 0,
            first_hop: local_node_id,
        },
    )]);
    let mut settled_nodes = BTreeSet::new();

    while let Some(current) = unsettled_nodes
        .iter()
        .copied()
        .min_by(|left, right| compare_route_state(left, right, &states))
    {
        unsettled_nodes.remove(&current);
        if !settled_nodes.insert(current) {
            continue;
        }
        let Some(current_state) = states.get(&current).copied() else {
            continue;
        };
        let Some(edges) = adjacency.get(&current) else {
            continue;
        };
        for (neighbor, edge_cost) in edges {
            let candidate = RouteState {
                cost: current_state.cost.saturating_add(*edge_cost),
                hop_count: current_state.hop_count.saturating_add(1),
                first_hop: if current == local_node_id {
                    *neighbor
                } else {
                    current_state.first_hop
                },
            };
            let replace = states
                .get(neighbor)
                .map(|known| route_state_less_than(candidate, *known))
                .unwrap_or(true);
            if replace {
                states.insert(*neighbor, candidate);
                unsettled_nodes.insert(*neighbor);
            }
        }
    }

    states
}

fn compare_route_state(
    left: &NodeId,
    right: &NodeId,
    states: &BTreeMap<NodeId, RouteState>,
) -> std::cmp::Ordering {
    let left_state = states.get(left).expect("left state");
    let right_state = states.get(right).expect("right state");
    left_state
        .cost
        .cmp(&right_state.cost)
        .then_with(|| left_state.hop_count.cmp(&right_state.hop_count))
        .then_with(|| left_state.first_hop.cmp(&right_state.first_hop))
        .then_with(|| left.cmp(right))
}

fn route_state_less_than(left: RouteState, right: RouteState) -> bool {
    left.cost < right.cost
        || (left.cost == right.cost
            && (left.hop_count < right.hop_count
                || (left.hop_count == right.hop_count && (left.first_hop < right.first_hop))))
}

fn degradation_for(path_cost: u32, hop_count: u8) -> RouteDegradation {
    if path_cost > 12 {
        RouteDegradation::Degraded(DegradationReason::LinkInstability)
    } else if hop_count > 3 {
        RouteDegradation::Degraded(DegradationReason::SparseTopology)
    } else {
        RouteDegradation::None
    }
}

fn backend_route_id_for(
    destination: NodeId,
    next_hop: NodeId,
    path_cost: u32,
) -> jacquard_core::BackendRouteId {
    let mut bytes = Vec::with_capacity(68);
    bytes.extend_from_slice(&destination.0);
    bytes.extend_from_slice(&next_hop.0);
    bytes.extend_from_slice(&path_cost.to_le_bytes());
    jacquard_core::BackendRouteId(bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use jacquard_core::{RouteEpoch, Tick, TransportKind};

    use super::*;
    use crate::public_state::{HoldWindow, NeighborLinkState, TopologyTuple};

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn neighbor(cost: u32) -> NeighborLinkState {
        NeighborLinkState {
            neighbor: node(1),
            latest_sequence: 1,
            hold_window: HoldWindow {
                last_observed_at_tick: Tick(1),
                stale_after_ticks: 8,
            },
            is_symmetric: true,
            is_mpr_selector: false,
            advertised_symmetric_neighbors: BTreeSet::new(),
            advertised_mprs: BTreeSet::new(),
            link_cost: cost,
            transport_kind: TransportKind::Custom("test".into()),
        }
    }

    #[test]
    fn shortest_path_prefers_lower_cost_then_first_hop() {
        let neighbors = BTreeMap::from([(node(2), neighbor(1)), (node(3), neighbor(1))]);
        let tuples = BTreeMap::from([
            (
                (node(2), node(4)),
                TopologyTuple {
                    originator: node(2),
                    advertised_neighbor: node(4),
                    seqno: 1,
                    observed_at_tick: Tick(1),
                },
            ),
            (
                (node(3), node(5)),
                TopologyTuple {
                    originator: node(3),
                    advertised_neighbor: node(5),
                    seqno: 1,
                    observed_at_tick: Tick(1),
                },
            ),
            (
                (node(5), node(4)),
                TopologyTuple {
                    originator: node(5),
                    advertised_neighbor: node(4),
                    seqno: 1,
                    observed_at_tick: Tick(1),
                },
            ),
        ]);

        let (_, best) = derive_routes(node(1), &neighbors, &tuples, RouteEpoch(2), Tick(2));

        assert_eq!(best[&node(4)].next_hop, node(2));
        assert_eq!(best[&node(4)].hop_count, 2);
    }
}
