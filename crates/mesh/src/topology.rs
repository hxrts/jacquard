//! Mesh-private topology queries and derived estimates.
//!
//! The types below are private mesh-owned interpretations of the shared
//! world schema from `jacquard_core`. `DeterministicMeshTopologyModel`
//! is a pure read-only query surface: every method is a deterministic
//! function of its inputs with no hidden state.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Belief, ByteCount, Configuration, Environment, HealthScore, Link, LinkEndpoint, LinkState,
    Node, NodeId, NodeRelayBudget, RatioPermille, RouteServiceKind, RoutingEngineId,
    RoutingObjective, ServiceDescriptor, ServiceId, ServiceScope, TransportProtocol,
};
use jacquard_traits::MeshTopologyModel;

/// Number of routable service kinds (Discover, Move, Hold) a node must
/// advertise to be considered route-capable for this engine.
pub const MESH_REQUIRED_SERVICE_COUNT: u32 = 3;

/// Upper bound for HealthScore values produced by this crate.
/// Matches the shared `RatioPermille` scale so scores compose cleanly
/// with confidence and loss metrics elsewhere.
pub const HEALTH_SCORE_MAX: u32 = 1000;

/// Multiplier applied to reachable-neighbor counts when scaling them
/// into the HealthScore range in `neighborhood_estimate`.
pub const DENSITY_SCORE_SCALE: u32 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshPeerEstimate {
    pub relay_value_score: HealthScore,
    pub retention_value_score: HealthScore,
    pub stability_score: HealthScore,
    pub service_score: HealthScore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshNeighborhoodEstimate {
    pub density_score: HealthScore,
    pub repair_pressure_score: HealthScore,
    pub partition_risk_score: HealthScore,
    pub service_stability_score: HealthScore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshMediumState {
    pub protocol_counts: BTreeMap<TransportProtocol, u32>,
    pub loss_floor_permille: RatioPermille,
    pub symmetry_floor_permille: RatioPermille,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshNodeIntrinsicState {
    pub available_connection_count: u32,
    pub hold_capacity_available_bytes: ByteCount,
    pub relay_budget: Option<NodeRelayBudget>,
}

#[derive(Clone, Debug, Default)]
pub struct DeterministicMeshTopologyModel;

impl DeterministicMeshTopologyModel {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn node_intrinsic_state(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<MeshNodeIntrinsicState> {
        let node = configuration.nodes.get(local_node_id)?;
        Some(MeshNodeIntrinsicState {
            available_connection_count: belief_u32(node.state.available_connection_count),
            hold_capacity_available_bytes: belief_byte_count(
                node.state.hold_capacity_available_bytes,
            ),
            relay_budget: match &node.state.relay_budget {
                Belief::Absent => None,
                Belief::Estimated(estimate) => Some(estimate.value.clone()),
            },
        })
    }

    #[must_use]
    pub fn medium_state(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> MeshMediumState {
        // Aggregates per-protocol link counts plus the worst-case loss and
        // worst-case symmetry across all adjacent links. `u16::MAX` is a
        // sentinel for "no adjacent links observed"; on that path we
        // publish the most pessimistic defaults (total loss, zero symmetry)
        // so downstream scoring treats an unobserved medium as unusable.
        let mut protocol_counts = BTreeMap::new();
        let mut loss_floor = u16::MAX;
        let mut symmetry_floor = u16::MAX;

        for link in self.adjacent_links(local_node_id, configuration) {
            *protocol_counts.entry(link.endpoint.protocol).or_insert(0) += 1;
            loss_floor = loss_floor.min(link.state.loss_permille.get());
            symmetry_floor = symmetry_floor.min(belief_ratio(link.state.symmetry_permille).get());
        }

        MeshMediumState {
            protocol_counts,
            loss_floor_permille: if loss_floor == u16::MAX {
                RatioPermille(1000)
            } else {
                RatioPermille(loss_floor)
            },
            symmetry_floor_permille: if symmetry_floor == u16::MAX {
                RatioPermille(0)
            } else {
                RatioPermille(symmetry_floor)
            },
        }
    }
}

impl MeshTopologyModel for DeterministicMeshTopologyModel {
    type PeerEstimate = MeshPeerEstimate;
    type NeighborhoodEstimate = MeshNeighborhoodEstimate;

    fn local_node(&self, local_node_id: &NodeId, configuration: &Configuration) -> Option<Node> {
        configuration.nodes.get(local_node_id).cloned()
    }

    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<(NodeId, Node)> {
        let neighbors = adjacent_node_ids(local_node_id, configuration);
        neighbors
            .into_iter()
            .filter_map(|node_id| {
                configuration
                    .nodes
                    .get(&node_id)
                    .cloned()
                    .map(|node| (node_id, node))
            })
            .collect()
    }

    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<LinkEndpoint> {
        let mut endpoints: Vec<LinkEndpoint> = self
            .adjacent_links(local_node_id, configuration)
            .into_iter()
            .map(|link| link.endpoint)
            .collect();
        endpoints.sort();
        endpoints.dedup();
        endpoints
    }

    fn adjacent_links(&self, local_node_id: &NodeId, configuration: &Configuration) -> Vec<Link> {
        let mut links: Vec<Link> = configuration
            .links
            .iter()
            .filter_map(|((left, right), link)| {
                if left == local_node_id || right == local_node_id {
                    Some(link.clone())
                } else {
                    None
                }
            })
            .collect();
        links.sort_by(|left, right| left.endpoint.cmp(&right.endpoint));
        links
    }

    fn peer_estimate(
        &self,
        local_node_id: &NodeId,
        peer_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<Self::PeerEstimate> {
        // Composes four HealthScores from peer state and the adjacent link:
        // relay headroom, retention capacity, link stability, and service surface.
        let peer = configuration.nodes.get(peer_node_id)?;
        let link = adjacent_link_between(local_node_id, peer_node_id, configuration)?;

        let relay_budget = match &peer.state.relay_budget {
            Belief::Absent => HealthScore(0),
            Belief::Estimated(estimate) => {
                // Higher is better, so invert utilization.
                let utilization = u32::from(estimate.value.utilization_permille.get());
                HealthScore(HEALTH_SCORE_MAX.saturating_sub(utilization))
            }
        };

        let retention_capacity = peer
            .state
            .hold_capacity_available_bytes
            .into_estimate()
            .map_or(0, |estimate| clamp_u64_to_u32(estimate.value.0));
        let retention_value = HealthScore(retention_capacity.min(HEALTH_SCORE_MAX));

        let stability = (u32::from(
            link.state
                .delivery_confidence_permille
                .into_estimate()
                .map_or(RatioPermille(0), |estimate| estimate.value)
                .get(),
        ) + u32::from(belief_ratio(link.state.symmetry_permille).get()))
            / 2;
        let service_score = HealthScore(service_surface_score(
            &peer.profile.services,
            &RoutingEngineId::Mesh,
            configuration.epoch,
        ));

        Some(MeshPeerEstimate {
            relay_value_score: relay_budget,
            retention_value_score: retention_value,
            stability_score: HealthScore(stability),
            service_score,
        })
    }

    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<Self::NeighborhoodEstimate> {
        // Density is the larger of the observed neighbor count and the
        // reported reachable count (scaled by 100 so a single neighbor
        // reads as a nontrivial score). Repair pressure tracks churn
        // directly. Partition risk averages churn and contention since
        // either signal alone can predict local isolation.
        let neighbor_count =
            u32::try_from(adjacent_node_ids(local_node_id, configuration).len()).ok()?;
        let Environment {
            reachable_neighbor_count,
            churn_permille,
            contention_permille,
        } = configuration.environment;

        let density_source = reachable_neighbor_count.max(neighbor_count);
        let density_score = HealthScore(density_source.saturating_mul(DENSITY_SCORE_SCALE));
        let repair_pressure_score = HealthScore(u32::from(churn_permille.get()));
        let partition_risk_score = HealthScore(
            u32::from(churn_permille.get()) / 2 + u32::from(contention_permille.get()) / 2,
        );

        let service_stability_score = HealthScore(
            adjacent_node_ids(local_node_id, configuration)
                .into_iter()
                .filter_map(|peer_id| configuration.nodes.get(&peer_id))
                .map(|node| {
                    service_surface_score(
                        &node.profile.services,
                        &RoutingEngineId::Mesh,
                        configuration.epoch,
                    )
                })
                .sum::<u32>()
                .min(HEALTH_SCORE_MAX),
        );

        Some(MeshNeighborhoodEstimate {
            density_score,
            repair_pressure_score,
            partition_risk_score,
            service_stability_score,
        })
    }
}

// A node is route-capable only if it advertises all three routable service
// kinds (Discover, Move, Hold) for this engine under the current epoch.
pub(crate) fn route_capable_for_engine(
    node: &Node,
    engine_id: &RoutingEngineId,
    current_epoch: jacquard_core::RouteEpoch,
) -> bool {
    service_surface_score(&node.profile.services, engine_id, current_epoch)
        >= MESH_REQUIRED_SERVICE_COUNT
}

// Destination matching: a Node destination matches by node-id only; a
// Gateway destination requires a gateway-scoped service; a Service
// destination requires any service of the requested kind on this engine.
// All three forms also require the node to pass basic route-capability.
pub(crate) fn objective_matches_node(
    node_id: &NodeId,
    node: &Node,
    objective: &RoutingObjective,
    engine_id: &RoutingEngineId,
    current_tick: jacquard_core::Tick,
) -> bool {
    if !route_capable_for_engine(node, engine_id, jacquard_core::RouteEpoch(current_tick.0)) {
        return false;
    }

    match &objective.destination {
        jacquard_core::DestinationId::Node(target) => node_id == target,
        jacquard_core::DestinationId::Gateway(target_gateway) => node.profile.services.iter().any(|service| {
            service.service_kind == objective.service_kind
                && service.routing_engines.contains(engine_id)
                && service.valid_for.contains(current_tick)
                && matches!(service.scope, ServiceScope::Gateway(ref gateway) if gateway == target_gateway)
        }),
        jacquard_core::DestinationId::Service(ServiceId(_)) => node.profile.services.iter().any(|service| {
            service.service_kind == objective.service_kind
                && service.routing_engines.contains(engine_id)
                && service.valid_for.contains(current_tick)
        }),
    }
}

// Links are keyed by an ordered node-id pair but modeled as undirected, so
// a lookup must try both orderings.
pub(crate) fn adjacent_link_between<'a>(
    left_node_id: &NodeId,
    right_node_id: &NodeId,
    configuration: &'a Configuration,
) -> Option<&'a Link> {
    configuration
        .links
        .get(&(*left_node_id, *right_node_id))
        .or_else(|| configuration.links.get(&(*right_node_id, *left_node_id)))
}

pub(crate) fn adjacent_node_ids(
    local_node_id: &NodeId,
    configuration: &Configuration,
) -> Vec<NodeId> {
    let mut neighbors = BTreeSet::new();
    for (left, right) in configuration.links.keys() {
        if left == local_node_id {
            neighbors.insert(*right);
        } else if right == local_node_id {
            neighbors.insert(*left);
        }
    }
    neighbors.into_iter().collect()
}

pub(crate) fn service_surface_score(
    services: &[ServiceDescriptor],
    engine_id: &RoutingEngineId,
    current_epoch: jacquard_core::RouteEpoch,
) -> u32 {
    let current_tick = jacquard_core::Tick(current_epoch.0);
    let has_discover = services.iter().any(|service| {
        service.service_kind == RouteServiceKind::Discover
            && service.routing_engines.contains(engine_id)
            && service.valid_for.contains(current_tick)
    });
    let has_move = services.iter().any(|service| {
        service.service_kind == RouteServiceKind::Move
            && service.routing_engines.contains(engine_id)
            && service.valid_for.contains(current_tick)
    });
    let has_hold = services.iter().any(|service| {
        service.service_kind == RouteServiceKind::Hold
            && service.routing_engines.contains(engine_id)
            && service.valid_for.contains(current_tick)
    });

    u32::from(has_discover) + u32::from(has_move) + u32::from(has_hold)
}

pub(crate) fn estimate_hop_link(
    from: &NodeId,
    to: &NodeId,
    configuration: &Configuration,
) -> Option<(LinkEndpoint, LinkState)> {
    adjacent_link_between(from, to, configuration)
        .map(|link| (link.endpoint.clone(), link.state.clone()))
}

fn belief_u32(belief: Belief<u32>) -> u32 {
    belief.into_estimate().map_or(0, |estimate| estimate.value)
}

fn belief_byte_count(belief: Belief<ByteCount>) -> ByteCount {
    belief
        .into_estimate()
        .map_or(ByteCount(0), |estimate| estimate.value)
}

fn belief_ratio(belief: Belief<RatioPermille>) -> RatioPermille {
    belief
        .into_estimate()
        .map_or(RatioPermille(0), |estimate| estimate.value)
}

fn clamp_u64_to_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

trait BeliefExt<T> {
    fn into_estimate(self) -> Option<jacquard_core::Estimate<T>>;
}

impl<T> BeliefExt<T> for Belief<T> {
    fn into_estimate(self) -> Option<jacquard_core::Estimate<T>> {
        match self {
            Belief::Absent => None,
            Belief::Estimated(estimate) => Some(estimate),
        }
    }
}
