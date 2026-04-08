//! Mesh-private topology queries and derived estimates.
//!
//! The types below are private mesh-owned interpretations of the shared
//! world schema from `jacquard_core`. `DeterministicMeshTopologyModel`
//! is a pure read-only query surface: every method is a deterministic
//! function of its inputs with no hidden state.

// long-file-exception: cohesive topology model, estimates, and fixture tests.

use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{
    Belief, ByteCount, Configuration, Environment, HealthScore, Link, LinkEndpoint,
    LinkState, Node, NodeId, NodeRelayBudget, RatioPermille, RouteServiceKind,
    RoutingEngineId, RoutingObjective, ServiceDescriptor, ServiceId, ServiceScope,
    Tick, TransportProtocol,
};
use jacquard_traits::{
    MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess, MeshTopologyModel,
};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshServiceRequirements {
    pub discover: bool,
    pub activate: bool,
    pub move_: bool,
    pub repair: bool,
    pub hold: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshPeerEstimate {
    pub relay_value_score: Option<HealthScore>,
    pub retention_value_score: Option<HealthScore>,
    pub stability_score: Option<HealthScore>,
    pub service_score: Option<HealthScore>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshNeighborhoodEstimate {
    pub density_score: Option<HealthScore>,
    pub repair_pressure_score: Option<HealthScore>,
    pub partition_risk_score: Option<HealthScore>,
    pub service_stability_score: Option<HealthScore>,
}

impl MeshPeerEstimateAccess for MeshPeerEstimate {
    fn relay_value_score(&self) -> Option<HealthScore> {
        self.relay_value_score
    }

    fn retention_value_score(&self) -> Option<HealthScore> {
        self.retention_value_score
    }

    fn stability_score(&self) -> Option<HealthScore> {
        self.stability_score
    }

    fn service_score(&self) -> Option<HealthScore> {
        self.service_score
    }
}

impl MeshNeighborhoodEstimateAccess for MeshNeighborhoodEstimate {
    fn density_score(&self) -> Option<HealthScore> {
        self.density_score
    }

    fn repair_pressure_score(&self) -> Option<HealthScore> {
        self.repair_pressure_score
    }

    fn partition_risk_score(&self) -> Option<HealthScore> {
        self.partition_risk_score
    }

    fn service_stability_score(&self) -> Option<HealthScore> {
        self.service_stability_score
    }
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
            available_connection_count: belief_u32(
                node.state.available_connection_count,
            ),
            hold_capacity_available_bytes: belief_byte_count(
                node.state.hold_capacity_available_bytes,
            ),
            relay_budget: match &node.state.relay_budget {
                | Belief::Absent => None,
                | Belief::Estimated(estimate) => Some(estimate.value.clone()),
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
            symmetry_floor = symmetry_floor.min(
                belief_ratio(link.state.symmetry_permille)
                    .map_or(0, |value| value.get()),
            );
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

pub(crate) fn bounded_health_score(value: u32) -> HealthScore {
    HealthScore(value.min(HEALTH_SCORE_MAX))
}

impl MeshTopologyModel for DeterministicMeshTopologyModel {
    type NeighborhoodEstimate = MeshNeighborhoodEstimate;
    type PeerEstimate = MeshPeerEstimate;

    fn local_node(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<Node> {
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

    fn adjacent_links(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<Link> {
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
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::PeerEstimate> {
        // Composes four HealthScores from peer state and the adjacent link:
        // relay headroom, retention capacity, link stability, and service surface.
        let peer = configuration.nodes.get(peer_node_id)?;
        let link = adjacent_link_between(local_node_id, peer_node_id, configuration)?;

        let relay_budget = match &peer.state.relay_budget {
            | Belief::Absent => None,
            | Belief::Estimated(estimate) => {
                // Higher is better, so invert utilization.
                let utilization = u32::from(estimate.value.utilization_permille.get());
                Some(bounded_health_score(
                    HEALTH_SCORE_MAX.saturating_sub(utilization),
                ))
            },
        };

        let retention_capacity = belief_into_estimate(
            peer.state.hold_capacity_available_bytes,
        )
        .map(|estimate| bounded_health_score(clamp_u64_to_u32(estimate.value.0)));

        let confidence = belief_into_estimate(link.state.delivery_confidence_permille)
            .map(|estimate| u32::from(estimate.value.get()));
        let symmetry = belief_ratio(link.state.symmetry_permille)
            .map(|value| u32::from(value.get()));
        let stability = mean_score(confidence, symmetry).map(HealthScore);
        let service_score = Some(bounded_health_score(service_surface_health_score(
            &peer.profile.services,
            &RoutingEngineId::Mesh,
            observed_at_tick,
        )));

        Some(MeshPeerEstimate {
            relay_value_score: relay_budget,
            retention_value_score: retention_capacity,
            stability_score: stability,
            service_score,
        })
    }

    fn neighborhood_estimate(
        &self,
        local_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<Self::NeighborhoodEstimate> {
        // Density is the larger of the observed neighbor count and the
        // reported reachable count (scaled by 100 so a single neighbor
        // reads as a nontrivial score). Repair pressure tracks churn
        // directly. Partition risk averages churn and contention since
        // either signal alone can predict local isolation.
        let neighbor_ids = adjacent_node_ids(local_node_id, configuration);
        let neighbor_count = u32::try_from(neighbor_ids.len()).ok()?;
        let Environment {
            reachable_neighbor_count,
            churn_permille,
            contention_permille,
        } = configuration.environment;

        // Take the larger of topology-observed links and the self-reported
        // reachable count: indirect neighbor knowledge can exceed local
        // link map entries, and density must not be understated.
        let density_source = reachable_neighbor_count.max(neighbor_count);
        let density_score = Some(bounded_health_score(
            density_source.saturating_mul(DENSITY_SCORE_SCALE),
        ));
        let repair_pressure_score =
            Some(bounded_health_score(u32::from(churn_permille.get())));
        let partition_risk_score = Some(bounded_health_score(
            u32::from(churn_permille.get()) / 2
                + u32::from(contention_permille.get()) / 2,
        ));

        // Sum (not average) across neighbors then clamp. Sum rewards
        // having more service-capable neighbors: a dense neighborhood
        // saturates the cap faster than a sparse one with equal per-node
        // score.
        let service_stability_score = Some(bounded_health_score(
            neighbor_ids
                .into_iter()
                .filter_map(|peer_id| configuration.nodes.get(&peer_id))
                .map(|node| {
                    service_surface_health_score(
                        &node.profile.services,
                        &RoutingEngineId::Mesh,
                        observed_at_tick,
                    )
                })
                .sum::<u32>(),
        ));

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
    current_tick: Tick,
) -> bool {
    service_surface_score(&node.profile.services, engine_id, current_tick)
        >= MESH_REQUIRED_SERVICE_COUNT
}

pub(crate) fn service_requirements_for_objective(
    objective: &RoutingObjective,
    require_hold: bool,
) -> MeshServiceRequirements {
    let mut requirements = MeshServiceRequirements::default();
    match objective.service_kind {
        | RouteServiceKind::Discover => requirements.discover = true,
        | RouteServiceKind::Activate => requirements.activate = true,
        | RouteServiceKind::Move => requirements.move_ = true,
        | RouteServiceKind::Repair => requirements.repair = true,
        | RouteServiceKind::Hold => requirements.hold = true,
    }
    requirements.hold |= require_hold;
    requirements
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
    let requirements = service_requirements_for_objective(objective, false);
    if !services_meet_requirements(
        &node.profile.services,
        engine_id,
        current_tick,
        requirements,
    ) {
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
    current_tick: Tick,
) -> u32 {
    service_surface_score_for_requirements(
        services,
        engine_id,
        current_tick,
        MeshServiceRequirements {
            discover: true,
            move_: true,
            hold: true,
            ..MeshServiceRequirements::default()
        },
    )
}

pub(crate) fn service_surface_score_for_requirements(
    services: &[ServiceDescriptor],
    engine_id: &RoutingEngineId,
    current_tick: Tick,
    requirements: MeshServiceRequirements,
) -> u32 {
    let has_kind = |kind: RouteServiceKind| {
        services.iter().any(|service| {
            service.service_kind == kind
                && service.routing_engines.contains(engine_id)
                && service.valid_for.contains(current_tick)
        })
    };

    u32::from(requirements.discover && has_kind(RouteServiceKind::Discover))
        + u32::from(requirements.activate && has_kind(RouteServiceKind::Activate))
        + u32::from(requirements.move_ && has_kind(RouteServiceKind::Move))
        + u32::from(requirements.repair && has_kind(RouteServiceKind::Repair))
        + u32::from(requirements.hold && has_kind(RouteServiceKind::Hold))
}

pub(crate) fn services_meet_requirements(
    services: &[ServiceDescriptor],
    engine_id: &RoutingEngineId,
    current_tick: Tick,
    requirements: MeshServiceRequirements,
) -> bool {
    service_surface_score_for_requirements(
        services,
        engine_id,
        current_tick,
        requirements,
    ) == required_service_count(requirements)
}

pub(crate) fn service_surface_health_score(
    services: &[ServiceDescriptor],
    engine_id: &RoutingEngineId,
    current_tick: Tick,
) -> u32 {
    service_surface_health_score_for_requirements(
        services,
        engine_id,
        current_tick,
        MeshServiceRequirements {
            discover: true,
            move_: true,
            hold: true,
            ..MeshServiceRequirements::default()
        },
    )
}

pub(crate) fn service_surface_health_score_for_requirements(
    services: &[ServiceDescriptor],
    engine_id: &RoutingEngineId,
    current_tick: Tick,
    requirements: MeshServiceRequirements,
) -> u32 {
    let required = required_service_count(requirements);
    if required == 0 {
        return HEALTH_SCORE_MAX;
    }
    let service_count = service_surface_score_for_requirements(
        services,
        engine_id,
        current_tick,
        requirements,
    );
    if service_count >= required {
        HEALTH_SCORE_MAX
    } else {
        service_count.saturating_mul(HEALTH_SCORE_MAX / required)
    }
}

fn required_service_count(requirements: MeshServiceRequirements) -> u32 {
    u32::from(requirements.discover)
        + u32::from(requirements.activate)
        + u32::from(requirements.move_)
        + u32::from(requirements.repair)
        + u32::from(requirements.hold)
}

pub(crate) fn optional_health_score_value(score: Option<HealthScore>) -> u32 {
    score.map_or(0, |score| score.0)
}

pub(crate) fn belief_into_estimate<T>(
    belief: Belief<T>,
) -> Option<jacquard_core::Estimate<T>> {
    match belief {
        | Belief::Absent => None,
        | Belief::Estimated(estimate) => Some(estimate),
    }
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
    belief_into_estimate(belief).map_or(0, |estimate| estimate.value)
}

fn belief_byte_count(belief: Belief<ByteCount>) -> ByteCount {
    belief_into_estimate(belief).map_or(ByteCount(0), |estimate| estimate.value)
}

fn belief_ratio(belief: Belief<RatioPermille>) -> Option<RatioPermille> {
    belief_into_estimate(belief).map(|estimate| estimate.value)
}

fn clamp_u64_to_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn mean_score(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    match (left, right) {
        | (Some(left), Some(right)) => Some((left + right) / 2),
        | (Some(left), None) => Some(left),
        | (None, Some(right)) => Some(right),
        | (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        BleDeviceId, BleProfileId, ControllerId, DestinationId, Estimate,
        LinkRuntimeState, NodeProfile, NodeState, RouteEpoch, ServiceDescriptor, Tick,
        TimeWindow,
    };

    use super::*;

    fn empty_node_state() -> NodeState {
        NodeState {
            relay_budget: Belief::Absent,
            available_connection_count: Belief::Absent,
            hold_capacity_available_bytes: Belief::Absent,
            information_summary: Belief::Absent,
        }
    }

    fn empty_node_profile() -> NodeProfile {
        NodeProfile {
            services: Vec::new(),
            endpoints: Vec::new(),
            connection_count_max: 0,
            neighbor_state_count_max: 0,
            simultaneous_transfer_count_max: 0,
            active_route_count_max: 0,
            relay_work_budget_max: 0,
            maintenance_work_budget_max: 0,
            hold_item_count_max: 0,
            hold_capacity_bytes_max: ByteCount(0),
        }
    }

    fn service(
        kind: RouteServiceKind,
        engine: RoutingEngineId,
        validity: TimeWindow,
    ) -> ServiceDescriptor {
        ServiceDescriptor {
            provider_node_id: NodeId([0; 32]),
            controller_id: ControllerId([0; 32]),
            service_kind: kind,
            endpoints: Vec::new(),
            routing_engines: vec![engine],
            scope: ServiceScope::Discovery(jacquard_core::DiscoveryScopeId([0; 16])),
            valid_for: validity,
            capacity: Belief::Absent,
        }
    }

    fn node_with_services(services: Vec<ServiceDescriptor>) -> Node {
        Node {
            controller_id: ControllerId([0; 32]),
            profile: NodeProfile { services, ..empty_node_profile() },
            state: empty_node_state(),
        }
    }

    fn active_link(byte: u8, confidence: u16) -> Link {
        Link {
            endpoint: LinkEndpoint {
                protocol: TransportProtocol::BleGatt,
                address: jacquard_core::EndpointAddress::Ble {
                    device_id: BleDeviceId(vec![byte]),
                    profile_id: BleProfileId([byte; 16]),
                },
                mtu_bytes: ByteCount(256),
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: jacquard_core::DurationMs(20),
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::Estimated(Estimate {
                    value: RatioPermille(confidence),
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: Tick(0),
                }),
                symmetry_permille: Belief::Estimated(Estimate {
                    value: RatioPermille(900),
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: Tick(0),
                }),
            },
        }
    }

    // A node must advertise all three of Discover, Move, and Hold to count
    // as route-capable. Two services is one short and should be rejected.
    #[test]
    fn route_capable_requires_all_three_services() {
        let validity = TimeWindow::new(Tick(0), Tick(100)).unwrap();
        let two_of_three = node_with_services(vec![
            service(RouteServiceKind::Discover, RoutingEngineId::Mesh, validity),
            service(RouteServiceKind::Move, RoutingEngineId::Mesh, validity),
        ]);
        assert!(!route_capable_for_engine(
            &two_of_three,
            &RoutingEngineId::Mesh,
            Tick(0),
        ));

        let all_three = node_with_services(vec![
            service(RouteServiceKind::Discover, RoutingEngineId::Mesh, validity),
            service(RouteServiceKind::Move, RoutingEngineId::Mesh, validity),
            service(RouteServiceKind::Hold, RoutingEngineId::Mesh, validity),
        ]);
        assert!(route_capable_for_engine(
            &all_three,
            &RoutingEngineId::Mesh,
            Tick(0),
        ));
    }

    // Service descriptors carry an engine id list. A node that lists all
    // three services for a different engine must not be treated as
    // route-capable for mesh.
    #[test]
    fn route_capable_filters_by_engine_id() {
        let validity = TimeWindow::new(Tick(0), Tick(100)).unwrap();
        let other_engine = RoutingEngineId::External {
            name: "external-test".into(),
            contract_id: jacquard_core::RoutingEngineContractId([1; 16]),
        };
        let foreign = node_with_services(vec![
            service(RouteServiceKind::Discover, other_engine.clone(), validity),
            service(RouteServiceKind::Move, other_engine.clone(), validity),
            service(RouteServiceKind::Hold, other_engine, validity),
        ]);
        assert!(!route_capable_for_engine(
            &foreign,
            &RoutingEngineId::Mesh,
            Tick(0),
        ));
    }

    // long-block-exception: dense peer-estimate fixture and assertions.
    #[test]
    fn peer_estimate_preserves_unknown_component_scores() {
        let local = NodeId([1; 32]);
        let peer = NodeId([2; 32]);
        let validity = TimeWindow::new(Tick(0), Tick(100)).unwrap();
        let configuration = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::from([
                (local, node_with_services(vec![])),
                (
                    peer,
                    node_with_services(vec![
                        service(
                            RouteServiceKind::Discover,
                            RoutingEngineId::Mesh,
                            validity,
                        ),
                        service(
                            RouteServiceKind::Move,
                            RoutingEngineId::Mesh,
                            validity,
                        ),
                        service(
                            RouteServiceKind::Hold,
                            RoutingEngineId::Mesh,
                            validity,
                        ),
                    ]),
                ),
            ]),
            links: BTreeMap::from([(
                (local, peer),
                Link {
                    endpoint: LinkEndpoint {
                        protocol: TransportProtocol::BleGatt,
                        address: jacquard_core::EndpointAddress::Ble {
                            device_id: BleDeviceId(vec![2]),
                            profile_id: BleProfileId([2; 16]),
                        },
                        mtu_bytes: ByteCount(256),
                    },
                    state: LinkState {
                        state: LinkRuntimeState::Active,
                        median_rtt_ms: jacquard_core::DurationMs(20),
                        transfer_rate_bytes_per_sec: Belief::Absent,
                        stability_horizon_ms: Belief::Absent,
                        loss_permille: RatioPermille(0),
                        delivery_confidence_permille: Belief::Absent,
                        symmetry_permille: Belief::Absent,
                    },
                },
            )]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };
        let model = DeterministicMeshTopologyModel::new();
        let estimate = model
            .peer_estimate(&local, &peer, Tick(0), &configuration)
            .expect("peer estimate");

        assert!(estimate.relay_value_score.is_none());
        assert!(estimate.retention_value_score.is_none());
        assert!(estimate.stability_score.is_none());
        assert_eq!(estimate.service_score, Some(HealthScore(HEALTH_SCORE_MAX)));
    }

    // Services advertise a validity window. Once the current tick falls
    // outside that window the service must be ignored, even if the kind
    // and engine id match.
    #[test]
    fn route_capable_rejects_expired_service_windows() {
        let expired = TimeWindow::new(Tick(0), Tick(5)).unwrap();
        let stale = node_with_services(vec![
            service(RouteServiceKind::Discover, RoutingEngineId::Mesh, expired),
            service(RouteServiceKind::Move, RoutingEngineId::Mesh, expired),
            service(RouteServiceKind::Hold, RoutingEngineId::Mesh, expired),
        ]);
        // Tick 10 is outside the [0, 5) window.
        assert!(!route_capable_for_engine(
            &stale,
            &RoutingEngineId::Mesh,
            Tick(10),
        ));
    }

    #[test]
    fn route_capable_depends_on_tick_even_when_epoch_is_constant() {
        let short_validity = TimeWindow::new(Tick(0), Tick(5)).unwrap();
        let node = node_with_services(vec![
            service(
                RouteServiceKind::Discover,
                RoutingEngineId::Mesh,
                short_validity,
            ),
            service(
                RouteServiceKind::Move,
                RoutingEngineId::Mesh,
                short_validity,
            ),
            service(
                RouteServiceKind::Hold,
                RoutingEngineId::Mesh,
                short_validity,
            ),
        ]);

        assert!(route_capable_for_engine(
            &node,
            &RoutingEngineId::Mesh,
            Tick(1),
        ));
        assert!(!route_capable_for_engine(
            &node,
            &RoutingEngineId::Mesh,
            Tick(10),
        ));
    }

    #[test]
    fn peer_service_score_depends_on_tick_not_epoch() {
        let local = NodeId([1; 32]);
        let peer = NodeId([2; 32]);
        let long_validity = TimeWindow::new(Tick(0), Tick(100)).unwrap();
        let mut configuration = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::from([
                (local, node_with_services(vec![])),
                (
                    peer,
                    node_with_services(vec![
                        service(
                            RouteServiceKind::Discover,
                            RoutingEngineId::Mesh,
                            long_validity,
                        ),
                        service(
                            RouteServiceKind::Move,
                            RoutingEngineId::Mesh,
                            long_validity,
                        ),
                        service(
                            RouteServiceKind::Hold,
                            RoutingEngineId::Mesh,
                            long_validity,
                        ),
                    ]),
                ),
            ]),
            links: BTreeMap::from([((local, peer), active_link(2, 950))]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };
        let model = DeterministicMeshTopologyModel::new();
        let score_at_epoch_zero = model
            .peer_estimate(&local, &peer, Tick(1), &configuration)
            .expect("peer estimate at epoch zero")
            .service_score;
        configuration.epoch = RouteEpoch(77);
        let score_at_epoch_seventy_seven = model
            .peer_estimate(&local, &peer, Tick(1), &configuration)
            .expect("peer estimate at later epoch")
            .service_score;

        assert_eq!(score_at_epoch_zero, score_at_epoch_seventy_seven);
    }

    #[test]
    fn neighborhood_density_score_is_clamped_to_health_score_max() {
        let local = NodeId([1; 32]);
        let configuration = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::from([(local, node_with_services(vec![]))]),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 99,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };
        let model = DeterministicMeshTopologyModel::new();
        let estimate = model
            .neighborhood_estimate(&local, Tick(0), &configuration)
            .expect("neighborhood estimate");

        assert_eq!(estimate.density_score, Some(HealthScore(HEALTH_SCORE_MAX)));
    }

    // A Node destination matches strictly by node-id. A non-matching id
    // must be rejected even if the candidate node is otherwise route-capable.
    #[test]
    fn objective_matches_node_destination_requires_exact_id() {
        let validity = TimeWindow::new(Tick(0), Tick(100)).unwrap();
        let candidate = node_with_services(vec![
            service(RouteServiceKind::Discover, RoutingEngineId::Mesh, validity),
            service(RouteServiceKind::Move, RoutingEngineId::Mesh, validity),
            service(RouteServiceKind::Hold, RoutingEngineId::Mesh, validity),
        ]);
        let objective = RoutingObjective {
            destination: DestinationId::Node(NodeId([7; 32])),
            service_kind: RouteServiceKind::Move,
            target_protection: jacquard_core::RouteProtectionClass::LinkProtected,
            protection_floor: jacquard_core::RouteProtectionClass::LinkProtected,
            target_connectivity: jacquard_core::ConnectivityPosture {
                repair: jacquard_core::RouteRepairClass::Repairable,
                partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
            latency_budget_ms: jacquard_core::Limit::Unbounded,
            protection_priority: jacquard_core::PriorityPoints(0),
            connectivity_priority: jacquard_core::PriorityPoints(0),
        };

        assert!(objective_matches_node(
            &NodeId([7; 32]),
            &candidate,
            &objective,
            &RoutingEngineId::Mesh,
            Tick(1),
        ));
        assert!(!objective_matches_node(
            &NodeId([8; 32]),
            &candidate,
            &objective,
            &RoutingEngineId::Mesh,
            Tick(1),
        ));
    }

    // Links are stored as ordered key pairs but modeled as undirected, so
    // a lookup must succeed regardless of which node id is supplied first.
    #[test]
    fn adjacent_link_between_handles_both_orderings() {
        let left = NodeId([1; 32]);
        let right = NodeId([2; 32]);
        let endpoint = LinkEndpoint {
            protocol: TransportProtocol::BleGatt,
            address: jacquard_core::EndpointAddress::Ble {
                device_id: jacquard_core::BleDeviceId(vec![1]),
                profile_id: jacquard_core::BleProfileId([1; 16]),
            },
            mtu_bytes: ByteCount(256),
        };
        let link = Link {
            endpoint,
            state: LinkState {
                state: jacquard_core::LinkRuntimeState::Active,
                median_rtt_ms: jacquard_core::DurationMs(40),
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::Absent,
                symmetry_permille: Belief::Absent,
            },
        };
        let configuration = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::new(),
            links: BTreeMap::from([((left, right), link)]),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };

        assert!(adjacent_link_between(&left, &right, &configuration).is_some());
        assert!(adjacent_link_between(&right, &left, &configuration).is_some());
    }

    // Querying neighbors of an absent node or an empty graph must return
    // an empty list rather than panicking.
    #[test]
    fn adjacent_node_ids_returns_empty_for_missing_node_or_empty_graph() {
        let empty = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };
        assert!(adjacent_node_ids(&NodeId([1; 32]), &empty).is_empty());
        assert!(adjacent_node_ids(&NodeId([99; 32]), &empty).is_empty());
    }
}
