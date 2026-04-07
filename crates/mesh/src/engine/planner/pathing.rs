//! Path search, route class, and connectivity derivation for mesh planning.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
};

use jacquard_core::{
    Belief, Configuration, DestinationId, Estimate, NodeId, Observation, RouteConnectivityProfile,
    RoutePartitionClass, RouteRepairClass, RouteServiceKind, RoutingObjective, Tick,
    ROUTE_HOP_COUNT_MAX,
};
use jacquard_traits::{MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess};

use super::{
    MeshEngine, PATH_METRIC_BASE_HOP_COST, PATH_METRIC_DEFERRED_DELIVERY_BONUS,
    PATH_METRIC_DIVERSITY_BONUS, PATH_METRIC_PROTOCOL_REPEAT_PENALTY,
};
use crate::topology::estimate_hop_link;
use crate::{MeshRouteClass, MeshRouteSegment, MESH_ENGINE_ID};

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    MeshEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::MeshTopologyBounds,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
{
    pub(super) fn path_metric_score(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        segments: &[MeshRouteSegment],
        route_class: &MeshRouteClass,
    ) -> u32 {
        let mut protocol_mix = Vec::new();
        let mut score = 0_u32;

        for (index, segment) in segments.iter().enumerate() {
            let from_node_id = node_path.get(index).copied().unwrap_or(self.local_node_id);
            score = score.saturating_add(
                self.edge_metric_score(objective, topology, &from_node_id, &segment.node_id)
                    .unwrap_or(PATH_METRIC_BASE_HOP_COST.saturating_mul(4)),
            );
            if protocol_mix.contains(&segment.endpoint.protocol) {
                score = score.saturating_add(PATH_METRIC_PROTOCOL_REPEAT_PENALTY);
            } else {
                protocol_mix.push(segment.endpoint.protocol.clone());
            }
        }

        let diversity_count = u32::try_from(protocol_mix.len()).unwrap_or(u32::MAX);
        let diversity_bonus = diversity_count
            .saturating_sub(1)
            .saturating_mul(PATH_METRIC_DIVERSITY_BONUS);
        score = score.saturating_sub(diversity_bonus);

        if matches!(route_class, MeshRouteClass::DeferredDelivery) {
            score = score.saturating_sub(PATH_METRIC_DEFERRED_DELIVERY_BONUS);
        }

        score
    }

    pub(super) fn weighted_paths(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
    ) -> Vec<(u32, Vec<NodeId>)> {
        let configuration = &topology.value;
        let mut best_paths = BTreeMap::<NodeId, (u32, Vec<NodeId>)>::new();
        let mut frontier = BinaryHeap::new();
        frontier.push(Reverse((0_u32, vec![self.local_node_id])));

        while let Some(Reverse((score, path))) = frontier.pop() {
            let current = *path.last().expect("weighted path frontier is never empty");
            if let Some((best_score, best_path)) = best_paths.get(&current) {
                if score > *best_score || (score == *best_score && path > *best_path) {
                    continue;
                }
            }
            best_paths.insert(current, (score, path.clone()));

            if path.len().saturating_sub(1) >= usize::from(ROUTE_HOP_COUNT_MAX) {
                continue;
            }

            for neighbor in crate::topology::adjacent_node_ids(&current, configuration) {
                if path.contains(&neighbor) {
                    continue;
                }
                let Some(edge_score) =
                    self.edge_metric_score(objective, topology, &current, &neighbor)
                else {
                    continue;
                };
                let mut next_path = path.clone();
                next_path.push(neighbor);
                let next_score = score.saturating_add(edge_score);
                if let Some((best_score, best_path)) = best_paths.get(&neighbor) {
                    if next_score > *best_score
                        || (next_score == *best_score && next_path >= *best_path)
                    {
                        continue;
                    }
                }
                frontier.push(Reverse((next_score, next_path)));
            }
        }

        best_paths.into_values().collect()
    }

    pub(super) fn determine_route_class(
        &self,
        objective: &RoutingObjective,
        hop_count: usize,
        hold_capable: bool,
    ) -> MeshRouteClass {
        if matches!(objective.destination, DestinationId::Gateway(_)) {
            MeshRouteClass::Gateway
        } else if hold_capable
            && objective.hold_fallback_policy == jacquard_core::HoldFallbackPolicy::Allowed
            && hop_count > 1
        {
            MeshRouteClass::DeferredDelivery
        } else if hop_count <= 1 {
            MeshRouteClass::Direct
        } else {
            MeshRouteClass::MultiHop
        }
    }

    pub(super) fn local_repair_slack(
        &self,
        configuration: &Configuration,
        node_path: &[NodeId],
    ) -> bool {
        if node_path.len() <= 2 {
            let Some(destination_node_id) = node_path.last().copied() else {
                return false;
            };
            return crate::topology::adjacent_node_ids(&self.local_node_id, configuration)
                .into_iter()
                .filter(|candidate| *candidate != destination_node_id)
                .any(|candidate| {
                    estimate_hop_link(&self.local_node_id, &candidate, configuration).is_some()
                        && estimate_hop_link(&candidate, &destination_node_id, configuration)
                            .is_some()
                });
        }

        let next_hop = node_path.get(1).copied();
        let source_has_alternate_neighbor =
            crate::topology::adjacent_node_ids(&self.local_node_id, configuration)
                .into_iter()
                .any(|candidate| Some(candidate) != next_hop);
        if source_has_alternate_neighbor {
            return true;
        }

        for pair in node_path.windows(2) {
            let from_node_id = pair[0];
            let to_node_id = pair[1];
            let path_nodes = node_path.iter().copied().collect::<BTreeSet<_>>();
            let has_bypass = crate::topology::adjacent_node_ids(&from_node_id, configuration)
                .into_iter()
                .filter(|candidate| !path_nodes.contains(candidate))
                .any(|candidate| {
                    estimate_hop_link(&from_node_id, &candidate, configuration).is_some()
                        && estimate_hop_link(&candidate, &to_node_id, configuration).is_some()
                });
            if has_bypass {
                return true;
            }
        }
        false
    }

    pub(super) fn route_connectivity_for_path(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        route_class: &MeshRouteClass,
    ) -> RouteConnectivityProfile {
        let repair = if self.local_repair_slack(&topology.value, node_path) {
            RouteRepairClass::Repairable
        } else {
            RouteRepairClass::BestEffort
        };
        match route_class {
            MeshRouteClass::DeferredDelivery => RouteConnectivityProfile {
                repair,
                partition: if objective.hold_fallback_policy
                    == jacquard_core::HoldFallbackPolicy::Allowed
                {
                    RoutePartitionClass::PartitionTolerant
                } else {
                    RoutePartitionClass::ConnectedOnly
                },
            },
            _ => RouteConnectivityProfile {
                repair,
                partition: RoutePartitionClass::ConnectedOnly,
            },
        }
    }

    pub(in crate::engine) fn derive_segments(
        &self,
        configuration: &Configuration,
        node_path: &[NodeId],
    ) -> Option<Vec<MeshRouteSegment>> {
        let mut segments = Vec::with_capacity(node_path.len().saturating_sub(1));
        for pair in node_path.windows(2) {
            let (endpoint, _) = estimate_hop_link(&pair[0], &pair[1], configuration)?;
            segments.push(MeshRouteSegment {
                node_id: pair[1],
                endpoint,
            });
        }
        if segments.is_empty() || segments.len() > usize::from(ROUTE_HOP_COUNT_MAX) {
            return None;
        }
        Some(segments)
    }

    pub(super) fn hold_capable_for_destination(
        &self,
        destination_node: &jacquard_core::Node,
        observed_at_tick: Tick,
    ) -> bool {
        let service_advertised = destination_node.profile.services.iter().any(|service| {
            service.service_kind == RouteServiceKind::Hold
                && service.routing_engines.contains(&MESH_ENGINE_ID)
                && service.valid_for.contains(observed_at_tick)
                && matches!(
                    service.capacity,
                    Belief::Estimated(Estimate {
                        value: jacquard_core::CapacityHint {
                            hold_capacity_bytes: Belief::Estimated(Estimate { value, .. }),
                            ..
                        },
                        ..
                    }) if value.0 > 0
                )
        });
        let state_ready = matches!(
            destination_node.state.hold_capacity_available_bytes,
            Belief::Estimated(Estimate { value, .. }) if value.0 > 0
        );
        service_advertised && state_ready
    }
}
