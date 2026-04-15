//! Pure helpers shared between `planner` and `runtime`.

use std::collections::BTreeSet;

use bincode::Options;
use jacquard_core::{
    AdmissionAssumptions, AdversaryRegime, BackendRouteId, BackendRouteRef, Belief, ByteCount,
    ClaimStrength, Configuration, ConnectivityPosture, ConnectivityRegime, DegradationReason,
    DestinationId, Estimate, FailureModelClass, Limit, Link, LinkRuntimeState,
    MessageFlowAssumptionClass, Node, NodeDensityClass, NodeId, ObjectiveVsDelivered, Observation,
    RouteAdmission, RouteAdmissionCheck, RouteCandidate, RouteCost, RouteDegradation, RouteEpoch,
    RouteEstimate, RouteProtectionClass, RouteSelectionError, RouteSummary, RouteWitness,
    RoutingEngineId, RoutingObjective, RuntimeEnvelopeClass, SelectedRoutingParameters,
    ServiceScope, Tick, TimeWindow, TransportKind,
};
use serde::{Deserialize, Serialize};

use crate::{
    public_state::{
        ScatterAction, ScatterBudgetPolicy, ScatterEngineConfig, ScatterExpiryPolicy,
        ScatterLocalSummary, ScatterRegime, ScatterRouteProgress, ScatterSizeClass,
        ScatterUrgencyClass,
    },
    SCATTER_CAPABILITIES, SCATTER_ENGINE_ID,
};

const BACKEND_TOKEN_ENCODING_VERSION: u8 = 1;
const SCATTER_WIRE_PACKET_ENCODING_VERSION: u8 = 1;
const DOMAIN_TAG_ROUTE_ID_PREFIX: &[u8] = b"scatter-route";
const SCATTER_PACKET_MAGIC: &[u8] = b"scatter/v1";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct ScatterMessageId(pub [u8; 16]);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ScatterBackendToken {
    pub destination: DestinationId,
    pub service_kind: jacquard_core::RouteServiceKind,
    pub topology_epoch: RouteEpoch,
    pub partition_tolerant: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ScatterWirePacket {
    pub message_id: ScatterMessageId,
    pub destination: DestinationId,
    pub service_kind: jacquard_core::RouteServiceKind,
    pub created_tick: Tick,
    pub expiry_after_ms: jacquard_core::DurationMs,
    pub copy_budget: u8,
    pub urgency_class: ScatterUrgencyClass,
    pub size_class: ScatterSizeClass,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveScatterRoute {
    pub destination: DestinationId,
    pub service_kind: jacquard_core::RouteServiceKind,
    pub backend_route_id: BackendRouteId,
    pub installed_at_tick: Tick,
    pub progress: ScatterRouteProgress,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredScatterMessage {
    pub packet: ScatterWirePacket,
    pub known_holder_nodes: BTreeSet<NodeId>,
    pub injected_by_route_id: Option<jacquard_core::RouteId>,
    pub last_action: ScatterAction,
    pub last_progress_at_tick: Tick,
    pub preferential_handoff_target: Option<NodeId>,
    pub delivered_locally: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PeerObservationState {
    pub encounter_count: u32,
    pub last_seen_tick: Option<Tick>,
    pub recent_novelty_count: u32,
}

pub(crate) fn direct_neighbors(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
) -> Vec<(NodeId, &Link)> {
    let mut neighbors = topology
        .value
        .links
        .iter()
        .filter_map(|((left, right), link)| {
            if *left == local_node_id {
                Some((*right, link))
            } else if *right == local_node_id {
                Some((*left, link))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    neighbors.sort_by_key(|(neighbor, _)| *neighbor);
    neighbors
}

pub(crate) fn link_is_usable(link: &Link) -> bool {
    !matches!(
        link.state.state,
        LinkRuntimeState::Suspended | LinkRuntimeState::Faulted
    )
}

pub(crate) fn objective_matches_node(
    node_id: &NodeId,
    node: &Node,
    objective: &RoutingObjective,
    engine_id: &RoutingEngineId,
    current_tick: Tick,
) -> bool {
    match &objective.destination {
        DestinationId::Node(target) => {
            if node_id != target {
                return false;
            }
            node.profile.services.iter().any(|service| {
                service.service_kind == objective.service_kind
                    && service.routing_engines.contains(engine_id)
                    && service.valid_for.contains(current_tick)
            })
        }
        DestinationId::Gateway(target_gateway) => node.profile.services.iter().any(|service| {
            service.service_kind == objective.service_kind
                && service.routing_engines.contains(engine_id)
                && service.valid_for.contains(current_tick)
                && matches!(service.scope, ServiceScope::Gateway(ref gateway) if gateway == target_gateway)
        }),
        DestinationId::Service(_) => node.profile.services.iter().any(|service| {
            service.service_kind == objective.service_kind
                && service.routing_engines.contains(engine_id)
                && service.valid_for.contains(current_tick)
        }),
    }
}

pub(crate) fn objective_supported(
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    current_tick: Tick,
) -> bool {
    topology.value.nodes.iter().any(|(node_id, node)| {
        objective_matches_node(node_id, node, objective, &SCATTER_ENGINE_ID, current_tick)
    })
}

pub(crate) fn local_objective_match(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    destination: &DestinationId,
    service_kind: jacquard_core::RouteServiceKind,
    current_tick: Tick,
) -> bool {
    let Some(node) = topology.value.nodes.get(&local_node_id) else {
        return false;
    };
    let objective = RoutingObjective {
        destination: destination.clone(),
        service_kind,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: SCATTER_CAPABILITIES.max_connectivity,
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: Limit::Bounded(jacquard_core::DurationMs(1)),
        protection_priority: jacquard_core::PriorityPoints(0),
        connectivity_priority: jacquard_core::PriorityPoints(0),
    };
    objective_matches_node(
        &local_node_id,
        node,
        &objective,
        &SCATTER_ENGINE_ID,
        current_tick,
    )
}

pub(crate) fn has_direct_match(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    objective: &RoutingObjective,
) -> bool {
    direct_neighbors(topology, local_node_id)
        .into_iter()
        .filter(|(_, link)| link_is_usable(link))
        .any(|(neighbor, _)| {
            topology.value.nodes.get(&neighbor).is_some_and(|node| {
                objective_matches_node(
                    &neighbor,
                    node,
                    objective,
                    &SCATTER_ENGINE_ID,
                    topology.observed_at_tick,
                )
            })
        })
}

pub(crate) fn hold_capacity_available_bytes(node: &Node) -> u32 {
    u32::try_from(
        node.state
            .hold_capacity_available_bytes
            .value_or(ByteCount(0))
            .0,
    )
    .unwrap_or(u32::MAX)
}

pub(crate) fn relay_utilization_permille(node: &Node) -> u16 {
    node.state
        .relay_budget
        .value()
        .map(|budget| budget.utilization_permille.get())
        .unwrap_or(0)
}

pub(crate) fn diversity_score(topology: &Observation<Configuration>, local_node_id: NodeId) -> u32 {
    let protocols = direct_neighbors(topology, local_node_id)
        .into_iter()
        .filter(|(_, link)| link_is_usable(link))
        .map(|(_, link)| link.endpoint.transport_kind.clone())
        .collect::<BTreeSet<_>>();
    u32::try_from(protocols.len()).unwrap_or(u32::MAX)
}

pub(crate) fn classify_regime(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    local_node: &Node,
    peer_observations: &std::collections::BTreeMap<NodeId, PeerObservationState>,
    config: &ScatterEngineConfig,
) -> (ScatterRegime, ScatterLocalSummary) {
    let neighbors = direct_neighbors(topology, local_node_id)
        .into_iter()
        .filter(|(_, link)| link_is_usable(link))
        .collect::<Vec<_>>();
    let distinct_peer_rate = peer_observations
        .values()
        .filter_map(|state| state.last_seen_tick)
        .filter(|tick| {
            topology.observed_at_tick.0.saturating_sub(tick.0) <= config.regime.history_window_ticks
        })
        .count();
    let novelty_rate = peer_observations
        .values()
        .map(|state| state.recent_novelty_count)
        .sum::<u32>();
    let resource_pressure_permille = if local_node.profile.hold_capacity_bytes_max.0 == 0 {
        relay_utilization_permille(local_node)
    } else {
        let remaining = u64::from(hold_capacity_available_bytes(local_node));
        let max = local_node.profile.hold_capacity_bytes_max.0.max(1);
        let used = max.saturating_sub(remaining.min(max));
        u16::try_from(used.saturating_mul(1000) / max).unwrap_or(1000)
    };
    let summary = ScatterLocalSummary {
        contact_rate: u32::try_from(neighbors.len()).unwrap_or(u32::MAX),
        distinct_peer_rate: u32::try_from(distinct_peer_rate).unwrap_or(u32::MAX),
        novelty_rate,
        diversity_score: diversity_score(topology, local_node_id),
        resource_pressure_permille,
        encounter_rate: u32::try_from(peer_observations.len()).unwrap_or(u32::MAX),
        scope_encounter_rate: 0,
        bridge_score: diversity_score(topology, local_node_id),
        scope_bridge_score: diversity_score(topology, local_node_id),
        mobility_score: u32::from(topology.value.environment.churn_permille.get()),
    };
    if u64::from(hold_capacity_available_bytes(local_node))
        <= config.regime.constrained_hold_capacity_floor_bytes.0
        || relay_utilization_permille(local_node)
            >= config.regime.constrained_relay_utilization_floor_permille
    {
        return (ScatterRegime::Constrained, summary);
    }
    if summary.diversity_score >= config.regime.bridging_diversity_floor
        && summary.contact_rate > config.regime.sparse_neighbor_count_max
    {
        return (ScatterRegime::Bridging, summary);
    }
    if summary.contact_rate <= config.regime.sparse_neighbor_count_max {
        return (ScatterRegime::Sparse, summary);
    }
    if summary.contact_rate >= config.regime.dense_neighbor_count_min {
        return (ScatterRegime::Dense, summary);
    }
    (ScatterRegime::Dense, summary)
}

pub(crate) fn candidate_summary(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    objective: &RoutingObjective,
    config: &ScatterEngineConfig,
) -> Result<RouteSummary, RouteSelectionError> {
    let start_tick = topology.observed_at_tick;
    let end_tick = Tick(
        start_tick
            .0
            .saturating_add(config.bounds.validity_window_ticks.max(1)),
    );
    let valid_for =
        TimeWindow::new(start_tick, end_tick).map_err(|_| RouteSelectionError::PolicyConflict)?;
    let protocols = direct_neighbors(topology, local_node_id)
        .into_iter()
        .filter(|(_, link)| link_is_usable(link))
        .map(|(_, link)| link.endpoint.transport_kind.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let direct = has_direct_match(topology, local_node_id, objective);
    Ok(RouteSummary {
        engine: SCATTER_ENGINE_ID,
        protection: RouteProtectionClass::LinkProtected,
        connectivity: ConnectivityPosture {
            repair: jacquard_core::RouteRepairClass::BestEffort,
            partition: if direct {
                jacquard_core::RoutePartitionClass::ConnectedOnly
            } else {
                jacquard_core::RoutePartitionClass::PartitionTolerant
            },
        },
        protocol_mix: if protocols.is_empty() {
            vec![TransportKind::WifiAware]
        } else {
            protocols
        },
        hop_count_hint: Belief::certain(if direct { 1 } else { 2 }, topology.observed_at_tick),
        valid_for,
    })
}

pub(crate) fn candidate_for(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    objective: &RoutingObjective,
    config: &ScatterEngineConfig,
) -> Result<RouteCandidate, RouteSelectionError> {
    let direct = has_direct_match(topology, local_node_id, objective);
    let token = ScatterBackendToken {
        destination: objective.destination.clone(),
        service_kind: objective.service_kind,
        topology_epoch: topology.value.epoch,
        partition_tolerant: !direct,
    };
    let backend_route_id = encode_backend_token(&token);
    let summary = candidate_summary(topology, local_node_id, objective, config)?;
    Ok(RouteCandidate {
        route_id: route_id_for_backend(&backend_route_id),
        summary,
        estimate: Estimate::certain(
            RouteEstimate {
                estimated_protection: RouteProtectionClass::LinkProtected,
                estimated_connectivity: if direct {
                    ConnectivityPosture {
                        repair: jacquard_core::RouteRepairClass::BestEffort,
                        partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
                    }
                } else {
                    SCATTER_CAPABILITIES.max_connectivity
                },
                topology_epoch: topology.value.epoch,
                degradation: if direct {
                    RouteDegradation::None
                } else {
                    RouteDegradation::Degraded(DegradationReason::PartitionRisk)
                },
            },
            topology.observed_at_tick,
        ),
        backend_ref: BackendRouteRef {
            engine: SCATTER_ENGINE_ID,
            backend_route_id,
        },
    })
}

pub(crate) fn admission_for(
    topology: &Observation<Configuration>,
    objective: &RoutingObjective,
    profile: &SelectedRoutingParameters,
    candidate: RouteCandidate,
    config: &ScatterEngineConfig,
) -> RouteAdmission {
    let delivered_connectivity = candidate.summary.connectivity;
    let degradation = if delivered_connectivity.partition < objective.target_connectivity.partition
    {
        RouteDegradation::Degraded(DegradationReason::PartitionRisk)
    } else {
        RouteDegradation::None
    };
    let density = match topology.value.environment.reachable_neighbor_count {
        0..=1 => NodeDensityClass::Sparse,
        2..=4 => NodeDensityClass::Moderate,
        _ => NodeDensityClass::Dense,
    };
    let admission_profile = AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
        failure_model: FailureModelClass::Benign,
        runtime_envelope: RuntimeEnvelopeClass::Canonical,
        node_density_class: density,
        connectivity_regime: if delivered_connectivity.partition
            == jacquard_core::RoutePartitionClass::PartitionTolerant
        {
            ConnectivityRegime::PartitionProne
        } else {
            ConnectivityRegime::Stable
        },
        adversary_regime: AdversaryRegime::Cooperative,
        claim_strength: ClaimStrength::ConservativeUnderProfile,
    };
    RouteAdmission {
        backend_ref: candidate.backend_ref,
        objective: objective.clone(),
        profile: profile.clone(),
        admission_check: RouteAdmissionCheck {
            decision: jacquard_core::AdmissionDecision::Admissible,
            profile: admission_profile.clone(),
            productive_step_bound: Limit::Bounded(1),
            total_step_bound: Limit::Bounded(config.bounds.work_step_count_max),
            route_cost: RouteCost {
                message_count_max: Limit::Bounded(config.bounds.message_count_max),
                byte_count_max: Limit::Bounded(config.bounds.byte_count_max),
                hop_count: if delivered_connectivity.partition
                    == jacquard_core::RoutePartitionClass::ConnectedOnly
                {
                    1
                } else {
                    2
                },
                repair_attempt_count_max: Limit::Bounded(1),
                hold_bytes_reserved: Limit::Bounded(config.bounds.hold_bytes_reserved),
                work_step_count_max: Limit::Bounded(config.bounds.work_step_count_max),
            },
        },
        summary: candidate.summary,
        witness: RouteWitness {
            protection: ObjectiveVsDelivered {
                objective: objective.target_protection,
                delivered: RouteProtectionClass::LinkProtected,
            },
            connectivity: ObjectiveVsDelivered {
                objective: profile.selected_connectivity,
                delivered: delivered_connectivity,
            },
            admission_profile,
            topology_epoch: topology.value.epoch,
            degradation,
        },
    }
}

pub(crate) fn expiry_for_urgency(
    expiry: ScatterExpiryPolicy,
    urgency: ScatterUrgencyClass,
) -> jacquard_core::DurationMs {
    match urgency {
        ScatterUrgencyClass::Emergency => expiry.emergency_expiry_ms,
        ScatterUrgencyClass::Normal => expiry.normal_expiry_ms,
        ScatterUrgencyClass::Background => expiry.background_expiry_ms,
    }
}

pub(crate) fn initial_budget_for_urgency(
    budget: ScatterBudgetPolicy,
    urgency: ScatterUrgencyClass,
) -> u8 {
    match urgency {
        ScatterUrgencyClass::Emergency => budget.emergency_copy_budget,
        ScatterUrgencyClass::Normal => budget.normal_copy_budget,
        ScatterUrgencyClass::Background => budget.background_copy_budget,
    }
}

pub(crate) fn size_class_for_payload(payload: &[u8]) -> ScatterSizeClass {
    match payload.len() {
        0..=128 => ScatterSizeClass::Small,
        129..=512 => ScatterSizeClass::Medium,
        _ => ScatterSizeClass::Large,
    }
}

pub(crate) fn route_id_for_backend(backend_route_id: &BackendRouteId) -> jacquard_core::RouteId {
    let mut bytes = [0u8; 16];
    for (index, byte) in DOMAIN_TAG_ROUTE_ID_PREFIX
        .iter()
        .chain(backend_route_id.0.iter())
        .take(16)
        .enumerate()
    {
        bytes[index] = *byte;
    }
    jacquard_core::RouteId(bytes)
}

pub(crate) fn encode_backend_token(token: &ScatterBackendToken) -> BackendRouteId {
    let mut bytes = Vec::with_capacity(32);
    bytes.push(BACKEND_TOKEN_ENCODING_VERSION);
    bytes.extend(
        canonical_options()
            .serialize(token)
            .expect("serialize backend token"),
    );
    BackendRouteId(bytes)
}

pub(crate) fn decode_backend_token(
    backend_route_id: &BackendRouteId,
) -> Option<ScatterBackendToken> {
    decode_versioned(&backend_route_id.0, BACKEND_TOKEN_ENCODING_VERSION)
}

pub(crate) fn encode_packet(packet: &ScatterWirePacket) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(packet.payload.len().saturating_add(32));
    bytes.extend_from_slice(SCATTER_PACKET_MAGIC);
    bytes.push(SCATTER_WIRE_PACKET_ENCODING_VERSION);
    bytes.extend(
        canonical_options()
            .serialize(packet)
            .expect("serialize packet"),
    );
    bytes
}

pub(crate) fn decode_packet(payload: &[u8]) -> Option<ScatterWirePacket> {
    if !payload.starts_with(SCATTER_PACKET_MAGIC) {
        return None;
    }
    let version = *payload.get(SCATTER_PACKET_MAGIC.len())?;
    if version != SCATTER_WIRE_PACKET_ENCODING_VERSION {
        return None;
    }
    canonical_options()
        .deserialize(&payload[SCATTER_PACKET_MAGIC.len().saturating_add(1)..])
        .ok()
}

pub(crate) fn packet_expired(packet: &ScatterWirePacket, current_tick: Tick) -> bool {
    let expiry_tick = packet
        .created_tick
        .0
        .saturating_add(u64::from(packet.expiry_after_ms.0.max(1)));
    current_tick.0 >= expiry_tick
}

pub(crate) fn contact_supports_payload(
    link: &Link,
    payload_len: usize,
    config: &ScatterEngineConfig,
) -> bool {
    if usize::try_from(link.endpoint.mtu_bytes.0)
        .ok()
        .is_some_and(|mtu| payload_len > mtu)
    {
        return false;
    }
    let transfer_rate = link.state.transfer_rate_bytes_per_sec.value_or(0);
    if transfer_rate < config.transport.min_transfer_rate_bytes_per_sec {
        return u32::try_from(payload_len)
            .ok()
            .is_some_and(|len| u64::from(len) <= config.transport.low_rate_payload_bytes_max.0);
    }
    let stability_horizon_ms = link
        .state
        .stability_horizon_ms
        .value_or(jacquard_core::DurationMs(0));
    if stability_horizon_ms < config.transport.min_stability_horizon_ms {
        return false;
    }
    let feasible_bytes =
        u64::from(transfer_rate).saturating_mul(u64::from(stability_horizon_ms.0)) / 1000;
    feasible_bytes >= u64::try_from(payload_len).unwrap_or(u64::MAX)
}

pub(crate) fn peer_score(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    peer_node_id: NodeId,
    destination: &DestinationId,
    service_kind: jacquard_core::RouteServiceKind,
) -> i32 {
    let Some(peer) = topology.value.nodes.get(&peer_node_id) else {
        return i32::MIN / 2;
    };
    let Some(local) = topology.value.nodes.get(&local_node_id) else {
        return 0;
    };
    let direct_destination_bonus = if local_objective_match(
        topology,
        peer_node_id,
        destination,
        service_kind,
        topology.observed_at_tick,
    ) {
        500
    } else {
        0
    };
    let hold_bonus = i32::try_from(hold_capacity_available_bytes(peer) / 32).unwrap_or(i32::MAX);
    let relay_headroom = i32::from(1000_u16.saturating_sub(relay_utilization_permille(peer)));
    let local_storage_penalty =
        i32::try_from(hold_capacity_available_bytes(local) / 64).unwrap_or(i32::MAX);
    direct_destination_bonus + hold_bonus + relay_headroom - local_storage_penalty
}

pub(crate) fn diversity_gate(
    topology: &Observation<Configuration>,
    local_node_id: NodeId,
    peer_node_id: NodeId,
) -> bool {
    let local_transports = direct_neighbors(topology, local_node_id)
        .into_iter()
        .map(|(_, link)| link.endpoint.transport_kind.clone())
        .collect::<BTreeSet<_>>();
    let peer_transports = topology
        .value
        .nodes
        .get(&peer_node_id)
        .map(|peer| {
            peer.profile
                .endpoints
                .iter()
                .map(|endpoint| endpoint.transport_kind.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    !peer_transports.is_subset(&local_transports)
}

pub(crate) fn action_for_delta(
    regime: ScatterRegime,
    delta: i32,
    packet: &ScatterWirePacket,
    config: &ScatterEngineConfig,
) -> ScatterAction {
    if packet.copy_budget > 0 && delta >= config.decision.preferential_handoff_delta_floor {
        return ScatterAction::PreferentialHandoff;
    }
    let floor = match regime {
        ScatterRegime::Sparse => config.decision.sparse_delta_floor,
        ScatterRegime::Dense => config.decision.dense_delta_floor,
        ScatterRegime::Bridging => config.decision.bridging_delta_floor,
        ScatterRegime::Constrained => config.decision.constrained_delta_floor,
    };
    if delta < floor {
        return ScatterAction::KeepCarrying;
    }
    if packet.copy_budget >= 2 {
        ScatterAction::Replicate
    } else if delta >= config.decision.preferential_handoff_delta_floor {
        ScatterAction::PreferentialHandoff
    } else {
        ScatterAction::KeepCarrying
    }
}

pub(crate) fn urgency_from_payload_len(payload_len: usize) -> ScatterUrgencyClass {
    if payload_len <= 64 {
        ScatterUrgencyClass::Emergency
    } else if payload_len <= 256 {
        ScatterUrgencyClass::Normal
    } else {
        ScatterUrgencyClass::Background
    }
}

fn canonical_options() -> impl Options {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
}

fn decode_versioned<T: for<'de> Deserialize<'de>>(bytes: &[u8], version: u8) -> Option<T> {
    let (encoded_version, rest) = bytes.split_first()?;
    if *encoded_version != version {
        return None;
    }
    canonical_options().deserialize(rest).ok()
}
