//! Deterministic MPR selection.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::NodeId;

use crate::public_state::{MprSelection, NeighborLinkState, TwoHopReachability};

#[must_use]
pub(crate) fn select_mprs(
    neighbors: &BTreeMap<NodeId, NeighborLinkState>,
    two_hop_reachability: &BTreeMap<NodeId, TwoHopReachability>,
    observed_at_tick: jacquard_core::Tick,
) -> MprSelection {
    let symmetric_neighbors: BTreeSet<NodeId> = neighbors
        .iter()
        .filter_map(|(neighbor, state)| state.is_symmetric.then_some(*neighbor))
        .collect();
    let mut selected_relays = BTreeSet::new();
    let mut covered_two_hops = BTreeSet::new();

    for (two_hop, reachability) in two_hop_reachability {
        let candidate_vias: BTreeSet<NodeId> = reachability
            .via_neighbors
            .intersection(&symmetric_neighbors)
            .copied()
            .collect();
        if candidate_vias.len() == 1 {
            let only = *candidate_vias.first().expect("singleton candidate set");
            selected_relays.insert(only);
        }
        if !candidate_vias.is_empty() && candidate_vias.len() == 1 {
            covered_two_hops.insert(*two_hop);
        }
    }

    for relay in &selected_relays {
        cover_two_hops_via(*relay, two_hop_reachability, &mut covered_two_hops);
    }

    let all_two_hops: BTreeSet<NodeId> = two_hop_reachability.keys().copied().collect();
    while covered_two_hops != all_two_hops {
        let best = symmetric_neighbors
            .iter()
            .filter(|candidate| !selected_relays.contains(candidate))
            .filter_map(|candidate| {
                let newly_covered: BTreeSet<NodeId> = two_hop_reachability
                    .iter()
                    .filter(|(two_hop, reachability)| {
                        !covered_two_hops.contains(two_hop)
                            && reachability.via_neighbors.contains(candidate)
                    })
                    .map(|(two_hop, _)| *two_hop)
                    .collect();
                if newly_covered.is_empty() {
                    return None;
                }
                let link_cost = neighbors
                    .get(candidate)
                    .map(|state| state.link_cost)
                    .unwrap_or(u32::MAX);
                Some((*candidate, newly_covered.len(), link_cost))
            })
            .max_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| right.2.cmp(&left.2))
                    .then_with(|| right.0.cmp(&left.0))
            });
        let Some((relay, _, _)) = best else {
            break;
        };
        selected_relays.insert(relay);
        cover_two_hops_via(relay, two_hop_reachability, &mut covered_two_hops);
    }

    MprSelection {
        selected_relays,
        covered_two_hops,
        observed_at_tick: Some(observed_at_tick),
    }
}

fn cover_two_hops_via(
    relay: NodeId,
    two_hop_reachability: &BTreeMap<NodeId, TwoHopReachability>,
    covered_two_hops: &mut BTreeSet<NodeId>,
) {
    for (two_hop, reachability) in two_hop_reachability {
        if reachability.via_neighbors.contains(&relay) {
            covered_two_hops.insert(*two_hop);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use jacquard_core::{Tick, TransportKind};

    use super::*;
    use crate::public_state::{HoldWindow, NeighborLinkState, TwoHopReachability};

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn neighbor(link_cost: u32) -> NeighborLinkState {
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
            link_cost,
            transport_kind: TransportKind::Custom("test".into()),
        }
    }

    #[test]
    fn mpr_selection_picks_unique_cover_and_cheapest_tie_break() {
        let neighbors = BTreeMap::from([
            (node(2), neighbor(5)),
            (node(3), neighbor(3)),
            (node(4), neighbor(4)),
        ]);
        let two_hops = BTreeMap::from([
            (
                node(8),
                TwoHopReachability {
                    two_hop: node(8),
                    via_neighbors: BTreeSet::from([node(2)]),
                },
            ),
            (
                node(9),
                TwoHopReachability {
                    two_hop: node(9),
                    via_neighbors: BTreeSet::from([node(3), node(4)]),
                },
            ),
        ]);

        let selection = select_mprs(&neighbors, &two_hops, Tick(3));

        assert_eq!(
            selection.selected_relays,
            BTreeSet::from([node(2), node(3)])
        );
        assert_eq!(
            selection.covered_two_hops,
            BTreeSet::from([node(8), node(9)])
        );
    }
}
