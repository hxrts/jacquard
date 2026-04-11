//! Route-shape derivation and connectivity classification for Pathway planning.
//!
//! The search frontier itself now lives in the Telltale-backed `search`
//! submodule. This file keeps the path-to-route logic that remains
//! Pathway-specific after that extraction: route-class derivation, repair
//! slack detection, connectivity posture, candidate path scoring, and segment
//! derivation from one chosen node path.

use std::collections::BTreeSet;

use jacquard_core::{
    Belief, Configuration, ConnectivityPosture, DestinationId, Estimate, NodeId, Observation,
    RoutePartitionClass, RouteRepairClass, RouteServiceKind, RoutingObjective, Tick,
    ROUTE_HOP_COUNT_MAX,
};

use super::{
    super::support::protocol_diversity_bonus, PathwayEngine, PATH_METRIC_BASE_HOP_COST,
    PATH_METRIC_DEFERRED_DELIVERY_BONUS, PATH_METRIC_DIVERSITY_BONUS,
    PATH_METRIC_PROTOCOL_REPEAT_PENALTY,
};
use crate::{
    topology::estimate_hop_link, PathwayNeighborhoodEstimateAccess, PathwayPeerEstimateAccess,
    PathwayRouteClass, PathwayRouteSegment, PATHWAY_ENGINE_ID,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HoldCapability {
    Available,
    Unavailable,
}

impl<Topology, Transport, Retention, Effects, Hasher, Selector>
    PathwayEngine<Topology, Transport, Retention, Effects, Hasher, Selector>
where
    Topology: super::super::PathwayTopologyBounds,
    Topology::PeerEstimate: PathwayPeerEstimateAccess,
    Topology::NeighborhoodEstimate: PathwayNeighborhoodEstimateAccess,
{
    pub(super) fn path_metric_score(
        &self,
        objective: &RoutingObjective,
        topology: &Observation<Configuration>,
        node_path: &[NodeId],
        segments: &[PathwayRouteSegment],
        route_class: &PathwayRouteClass,
    ) -> u32 {
        let mut protocol_mix = Vec::new();
        let mut score = 0_u32;

        for (index, segment) in segments.iter().enumerate() {
            let from_node_id = node_path.get(index).copied().unwrap_or(self.local_node_id);
            score = score.saturating_add(
                self.edge_metric_score(objective, topology, &from_node_id, &segment.node_id)
                    .unwrap_or(PATH_METRIC_BASE_HOP_COST.saturating_mul(4)),
            );
            if protocol_mix.contains(&segment.endpoint.transport_kind) {
                score = score.saturating_add(PATH_METRIC_PROTOCOL_REPEAT_PENALTY);
            } else {
                protocol_mix.push(segment.endpoint.transport_kind.clone());
            }
        }

        let diversity_bonus =
            protocol_diversity_bonus(segments).saturating_mul(PATH_METRIC_DIVERSITY_BONUS);
        score = score.saturating_sub(diversity_bonus);

        if matches!(route_class, PathwayRouteClass::DeferredDelivery) {
            score = score.saturating_sub(PATH_METRIC_DEFERRED_DELIVERY_BONUS);
        }

        score
    }
    pub(super) fn determine_route_class(
        &self,
        objective: &RoutingObjective,
        hop_count: usize,
        hold_capability: HoldCapability,
    ) -> PathwayRouteClass {
        if matches!(objective.destination, DestinationId::Gateway(_)) {
            PathwayRouteClass::Gateway
        } else if matches!(hold_capability, HoldCapability::Available)
            && objective.hold_fallback_policy == jacquard_core::HoldFallbackPolicy::Allowed
            && hop_count > 1
        {
            PathwayRouteClass::DeferredDelivery
        } else if hop_count <= 1 {
            PathwayRouteClass::Direct
        } else {
            PathwayRouteClass::MultiHop
        }
    }

    // Three cases by path length: (1) 1-hop — only a shared neighbor can
    // bridge; (2) multi-hop with alternate first-hop — repair is possible;
    // (3) multi-hop — scan each segment pair for a bypass node.
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
        route_class: &PathwayRouteClass,
    ) -> ConnectivityPosture {
        let repair = if self.local_repair_slack(&topology.value, node_path) {
            RouteRepairClass::Repairable
        } else {
            RouteRepairClass::BestEffort
        };
        match route_class {
            PathwayRouteClass::DeferredDelivery => ConnectivityPosture {
                repair,
                partition: if objective.hold_fallback_policy
                    == jacquard_core::HoldFallbackPolicy::Allowed
                {
                    RoutePartitionClass::PartitionTolerant
                } else {
                    RoutePartitionClass::ConnectedOnly
                },
            },
            _ => ConnectivityPosture {
                repair,
                partition: RoutePartitionClass::ConnectedOnly,
            },
        }
    }

    pub(in crate::engine) fn derive_segments(
        &self,
        configuration: &Configuration,
        node_path: &[NodeId],
    ) -> Option<Vec<PathwayRouteSegment>> {
        let mut segments = Vec::with_capacity(node_path.len().saturating_sub(1));
        for pair in node_path.windows(2) {
            let (endpoint, _) = estimate_hop_link(&pair[0], &pair[1], configuration)?;
            segments.push(PathwayRouteSegment {
                node_id: pair[1],
                endpoint,
            });
        }
        // Empty segments means node_path had one entry (local node is the
        // destination) — not a valid route. Also reject paths past the cap.
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
                && service.routing_engines.contains(&PATHWAY_ENGINE_ID)
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
