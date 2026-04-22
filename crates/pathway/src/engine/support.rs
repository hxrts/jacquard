//! Pure helper functions shared between `planner` and `runtime`.
//!
//! Every function in this module is a deterministic function of its
//! inputs: no runtime state, no effects, no hidden context. Includes
//! the repair-time shortest-path helper (`repair_shortest_path`), the
//! self-contained `BackendRouteId` encoding and decoding
//! (`encode_backend_token`, `decode_backend_token`), byte encoders used by
//! tagged hashing (`encode_path_bytes`), the `RouteCost` derivation
//! (`route_cost_for_segments`), and small scoring helpers used by both
//! the planner and the scoring sub-module (`link_quality_penalties`,
//! `protocol_diversity_bonus`). The `CommitteeStatus` wrapper and
//! `StorageResultExt`/`MaintenanceResultExt` error-mapping extension
//! traits also live here so planner and runtime can share them without
//! a circular dependency.

use std::{
    collections::{BTreeMap, VecDeque},
    convert::TryFrom,
};

#[allow(unused_imports)]
use jacquard_core::{
    BackendRouteId, Belief, ByteCount, CommitteeSelection, Configuration, DegradationReason,
    DestinationId, DeterministicOrderKey, DiversityFloor, Limit, MaterializedRoute, NodeId,
    OrderStamp, QuorumThreshold, RatioPermille, RouteCost, RouteDegradation, RouteEpoch,
    RouteError, RouteId, RouteRuntimeError, TimeWindow,
};
use jacquard_traits::{HashDigestBytes, Hashing};

/// Extension trait for converting storage errors into
/// `RouteError::Runtime(Invalidated)`.
pub(crate) trait StorageResultExt<T> {
    fn storage_invalid(self) -> Result<T, RouteError>;
}

impl<T, E> StorageResultExt<T> for Result<T, E> {
    fn storage_invalid(self) -> Result<T, RouteError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(RouteError::Runtime(RouteRuntimeError::Invalidated)),
        }
    }
}

/// Extension trait for converting effects errors into
/// `RouteError::Runtime(MaintenanceFailed)`.
pub(crate) trait MaintenanceResultExt<T> {
    fn maintenance_failed(self) -> Result<T, RouteError>;
}

impl<T, E> MaintenanceResultExt<T> for Result<T, E> {
    fn maintenance_failed(self) -> Result<T, RouteError> {
        match self {
            Ok(value) => Ok(value),
            Err(_) => Err(RouteError::Runtime(RouteRuntimeError::MaintenanceFailed)),
        }
    }
}
use serde::{Deserialize, Serialize};

use super::{
    types::{PathwayCommitteeStatus, PathwayRouteCheckpoint},
    ActivePathwayRoute, PathwayRouteClass, PathwayRouteSegment, PATHWAY_ENGINE_ID,
    PATHWAY_HOLD_RESERVED_BYTES, PATHWAY_PER_HOP_BYTE_COST,
};
use crate::topology::{adjacent_link_between, adjacent_node_ids, belief_into_estimate};

/// Per-link quality penalties derived from a single `LinkState`.
pub(crate) struct LinkPenalties {
    pub delivery: u32,
    pub symmetry: u32,
    pub loss: u32,
}

/// Compute delivery, symmetry, and loss penalties from a single link's state.
/// Each penalty is on a 0–1000 scale (higher = worse link quality).
pub(crate) fn link_quality_penalties(state: &jacquard_core::LinkState) -> LinkPenalties {
    let delivery = 1000_u32.saturating_sub(u32::from(
        state
            .delivery_confidence_permille
            .value_or(jacquard_core::RatioPermille(0))
            .get(),
    ));
    let symmetry = 1000_u32.saturating_sub(u32::from(
        state
            .symmetry_permille
            .value_or(jacquard_core::RatioPermille(0))
            .get(),
    ));
    let loss = u32::from(state.loss_permille.get());
    LinkPenalties {
        delivery,
        symmetry,
        loss,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PathwayPlanToken {
    pub(super) epoch: RouteEpoch,
    pub(super) source: NodeId,
    pub(super) destination: DestinationId,
    pub(super) segments: Vec<PathwayRouteSegment>,
    pub(super) valid_for: TimeWindow,
    pub(super) route_class: PathwayRouteClass,
    // Keep committee status self-contained inside the token so cache misses,
    // engine restarts, and materialization re-derivation preserve the exact
    // admission semantics without reintroducing planner cache dependence.
    pub(super) committee_status: PathwayCommitteeStatus,
}

pub(super) use crate::engine::types::PathwayCommitteeStatus as CommitteeStatus;

/// Maps a `Belief<RatioPermille>` to a `u32` health score.
/// `Absent` maps to 0; `Estimated` maps to the inner permille value as `u32`.
pub(crate) fn belief_to_health_score(belief: &Belief<RatioPermille>) -> u32 {
    u32::from(belief.value_or(RatioPermille(0)).get())
}

pub(crate) const DOMAIN_TAG_ROUTE_ID: &[u8] = b"pathway-route-id";
pub(crate) const DOMAIN_TAG_COMMITMENT: &[u8] = b"pathway-commitment";
pub(crate) const DOMAIN_TAG_HANDOFF_RECEIPT: &[u8] = b"pathway-handoff-receipt";
pub(crate) const DOMAIN_TAG_RETENTION: &[u8] = b"pathway-retention";
pub(crate) const DOMAIN_TAG_COMMITTEE_ID: &[u8] = b"pathway-committee-id";
pub(crate) const DOMAIN_TAG_ORDER_KEY: &[u8] = b"pathway-order-key";

const PLAN_TOKEN_ENCODING_VERSION: u8 = 1;
const ROUTE_IDENTITY_ENCODING_VERSION: u8 = 1;
const CHECKPOINT_ENCODING_VERSION: u8 = 1;
const PATH_ENCODING_VERSION: u8 = 1;
const PATHWAY_DEGRADATION_PRESSURE_THRESHOLD_PERMILLE: u16 = 600;

pub(super) fn committee_status(
    result: Result<Option<CommitteeSelection>, jacquard_core::RouteError>,
) -> PathwayCommitteeStatus {
    match result {
        Ok(Some(selection)) => PathwayCommitteeStatus::Selected(selection),
        Ok(None) => PathwayCommitteeStatus::NotApplicable,
        Err(_) => PathwayCommitteeStatus::SelectorFailed,
    }
}

fn encode_versioned<T: Serialize>(version: u8, value: &T) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    bytes.push(version);
    bytes.extend(
        postcard::to_allocvec(value).expect("pathway canonical bytes are always serializable"),
    );
    bytes
}

fn decode_versioned<T: for<'de> Deserialize<'de>>(bytes: &[u8], version: u8) -> Option<T> {
    let (encoded_version, rest) = bytes.split_first()?;
    if *encoded_version != version {
        return None;
    }
    postcard::from_bytes(rest).ok()
}

// Unweighted BFS. Returns the shortest node path from the local node
// to every reachable node using the sorted neighbor order from
// `adjacent_node_ids`. Sorted neighbor order is what makes the result
// deterministic across runs on the same configuration.
pub(super) fn shortest_paths(
    local_node_id: &NodeId,
    configuration: &Configuration,
) -> BTreeMap<NodeId, Vec<NodeId>> {
    let mut visited = BTreeMap::new();
    let mut queue = VecDeque::new();

    visited.insert(*local_node_id, vec![*local_node_id]);
    queue.push_back(*local_node_id);

    while let Some(current) = queue.pop_front() {
        let Some(current_path) = visited.get(&current).cloned() else {
            continue;
        };
        for neighbor in adjacent_node_ids(&current, configuration) {
            if visited.contains_key(&neighbor) {
                continue;
            }
            let mut next_path = current_path.clone();
            next_path.push(neighbor);
            visited.insert(neighbor, next_path);
            queue.push_back(neighbor);
        }
    }

    visited
}

pub(super) fn unique_protocol_mix(
    segments: &[PathwayRouteSegment],
) -> Vec<jacquard_core::TransportKind> {
    let mut protocols = segments
        .iter()
        .map(|segment| segment.endpoint.transport_kind.clone())
        .collect::<Vec<_>>();
    protocols.sort();
    protocols.dedup();
    protocols
}

pub(super) fn encode_path_bytes(path: &[NodeId], segments: &[PathwayRouteSegment]) -> Vec<u8> {
    #[derive(Serialize)]
    struct PathEncoding<'a> {
        path: &'a [NodeId],
        segments: &'a [PathwayRouteSegment],
    }

    encode_versioned(PATH_ENCODING_VERSION, &PathEncoding { path, segments })
}

pub(super) fn node_path_from_plan_token(plan: &PathwayPlanToken) -> Vec<NodeId> {
    let mut path = Vec::with_capacity(plan.segments.len() + 1);
    path.push(plan.source);
    path.extend(plan.segments.iter().map(|segment| segment.node_id));
    path
}

pub(super) fn encode_route_identity_bytes(plan: &PathwayPlanToken) -> Vec<u8> {
    #[derive(Serialize)]
    struct PathwayRouteIdentity<'a> {
        source: &'a NodeId,
        destination: &'a DestinationId,
        segments: &'a [PathwayRouteSegment],
        route_class: &'a PathwayRouteClass,
    }

    // Route ids in v1 pathway are path identities rather than per-epoch instance
    // identities. The epoch stays in the plan token and materialization proof,
    // but the stable route id is derived from source, destination, route class,
    // and the concrete segment path only.
    encode_versioned(
        ROUTE_IDENTITY_ENCODING_VERSION,
        &PathwayRouteIdentity {
            source: &plan.source,
            destination: &plan.destination,
            segments: &plan.segments,
            route_class: &plan.route_class,
        },
    )
}

// Self-contained plan token: a serialized pathway-private route plan
// carrying the path, route class, validity window, and optional
// committee result. Planner cache entries may be dropped; materialize
// and planner cache-miss paths decode this token instead of depending
// on ambient mutable engine state.
pub(super) fn encode_backend_token(plan: &PathwayPlanToken) -> BackendRouteId {
    BackendRouteId(encode_versioned(PLAN_TOKEN_ENCODING_VERSION, plan))
}

// Inverse of `encode_backend_token`. Invalid or hand-crafted bytes fail
// closed with None rather than being partially decoded.
pub(super) fn decode_backend_token(backend_route_id: &BackendRouteId) -> Option<PathwayPlanToken> {
    decode_versioned(&backend_route_id.0, PLAN_TOKEN_ENCODING_VERSION)
}

pub fn first_hop_node_id_from_backend_route_id(
    backend_route_id: &BackendRouteId,
) -> Option<NodeId> {
    let plan = decode_backend_token(backend_route_id)?;
    node_path_from_plan_token(&plan).get(1).copied()
}

/// Extract the first `N` bytes of a digest's byte slice as a fixed-size array.
/// Used to derive typed ID newtypes and tie-break keys from tagged hash
/// outputs.
pub(crate) fn digest_prefix<const N: usize>(digest_bytes: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&digest_bytes[..N]);
    out
}

pub(super) fn deterministic_order_key<H: Hashing>(
    route_id: RouteId,
    hashing: &H,
    path_bytes: &[u8],
) -> DeterministicOrderKey<RouteId> {
    let digest = hashing.hash_tagged(DOMAIN_TAG_ORDER_KEY, path_bytes);
    DeterministicOrderKey {
        stable_key: route_id,
        tie_break: OrderStamp(u64::from_le_bytes(digest_prefix::<8>(digest.as_bytes()))),
    }
}

// Candidate confidence is the worst-hop delivery confidence along the
// path. A single weak link drags the whole route down, which is the
// correct semantic for an ordered hop-by-hop source route.
pub(super) fn confidence_for_segments(
    segments: &[PathwayRouteSegment],
    configuration: &Configuration,
) -> jacquard_core::RatioPermille {
    let mut confidence = 1000_u16;
    let mut previous = None;
    for segment in segments {
        if let Some(from) = previous {
            if let Some(link) = adjacent_link_between(&from, &segment.node_id, configuration) {
                confidence = confidence.min(
                    belief_into_estimate(link.state.delivery_confidence_permille)
                        .map_or(jacquard_core::RatioPermille(0), |estimate| estimate.value)
                        .get(),
                );
            }
        }
        previous = Some(segment.node_id);
    }
    jacquard_core::RatioPermille(confidence)
}

pub(super) fn degradation_for_candidate(
    configuration: &Configuration,
    route_class: &PathwayRouteClass,
) -> RouteDegradation {
    if matches!(route_class, PathwayRouteClass::DeferredDelivery) {
        RouteDegradation::Degraded(DegradationReason::PartitionRisk)
    } else if configuration.environment.contention_permille.get()
        > PATHWAY_DEGRADATION_PRESSURE_THRESHOLD_PERMILLE
    {
        RouteDegradation::Degraded(DegradationReason::CapacityPressure)
    } else if configuration.environment.churn_permille.get()
        > PATHWAY_DEGRADATION_PRESSURE_THRESHOLD_PERMILLE
    {
        RouteDegradation::Degraded(DegradationReason::LinkInstability)
    } else {
        RouteDegradation::None
    }
}

// Segment count is bounded by `ROUTE_HOP_COUNT_MAX` in the planner's
// `derive_segments`, so the cast to u8 is infallible at every call
// site. Shared route cost reflects the chosen path's hop count,
// delivery confidence, symmetry, loss-derived congestion, protocol
// diversity, and deferred-delivery hold reservation.
fn hold_reserved_bytes(route_class: &PathwayRouteClass) -> ByteCount {
    match route_class {
        PathwayRouteClass::DeferredDelivery => ByteCount(PATHWAY_HOLD_RESERVED_BYTES),
        _ => ByteCount(0),
    }
}

fn route_quality_penalties(
    node_path: &[NodeId],
    segments: &[PathwayRouteSegment],
    configuration: &Configuration,
) -> (u32, u32, u32) {
    let confidence = u32::from(confidence_for_segments(segments, configuration).get());
    let delivery_penalty = (1000_u32.saturating_sub(confidence)) / 100;
    let (symmetry_penalty, congestion_penalty) = segments.iter().enumerate().fold(
        (0_u32, 0_u32),
        |(symmetry_penalty, congestion_penalty), (index, segment)| {
            let previous_node = node_path.get(index).copied().unwrap_or(segment.node_id);
            let Some(link) = adjacent_link_between(&previous_node, &segment.node_id, configuration)
            else {
                return (symmetry_penalty, congestion_penalty);
            };
            let symmetry_penalty = symmetry_penalty.saturating_add(
                belief_into_estimate(link.state.symmetry_permille).map_or(10, |estimate| {
                    (1000_u32.saturating_sub(u32::from(estimate.value.get()))) / 100
                }),
            );
            let congestion_penalty =
                congestion_penalty.saturating_add(u32::from(link.state.loss_permille.get()) / 100);
            (symmetry_penalty, congestion_penalty)
        },
    );
    (delivery_penalty, symmetry_penalty, congestion_penalty)
}

pub(crate) fn protocol_diversity_bonus(segments: &[PathwayRouteSegment]) -> u32 {
    let protocol_mix = unique_protocol_mix(segments);
    let u32_max_as_usize = usize::try_from(u32::MAX).expect("u32::MAX fits on supported targets");
    debug_assert!(protocol_mix.len() <= u32_max_as_usize);
    u32::try_from(protocol_mix.len())
        .expect("protocol diversity is bounded by segment count")
        .saturating_sub(1)
}

pub(super) fn route_cost_for_segments(
    node_path: &[NodeId],
    segments: &[PathwayRouteSegment],
    route_class: &PathwayRouteClass,
    configuration: &Configuration,
) -> RouteCost {
    let hop_count =
        u8::try_from(segments.len()).expect("segment count is bounded by ROUTE_HOP_COUNT_MAX");
    let hold_reserved = hold_reserved_bytes(route_class);
    let (delivery_penalty, symmetry_penalty, congestion_penalty) =
        route_quality_penalties(node_path, segments, configuration);
    let diversity_bonus = protocol_diversity_bonus(segments);
    // path_penalty adds slack proportional to link quality across all cost
    // fields. The +1 in work_step_count_max accounts for the local
    // materialization step. Byte budget scales penalty by 128 to stay
    // above per-hop cost.
    let path_penalty = delivery_penalty
        .saturating_add(symmetry_penalty)
        .saturating_add(congestion_penalty)
        .saturating_sub(diversity_bonus);
    RouteCost {
        message_count_max: Limit::Bounded(u32::from(hop_count).saturating_add(path_penalty)),
        byte_count_max: Limit::Bounded(ByteCount(
            u64::from(hop_count) * PATHWAY_PER_HOP_BYTE_COST + u64::from(path_penalty) * 128,
        )),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(u32::from(hop_count).saturating_add(path_penalty)),
        hold_bytes_reserved: Limit::Bounded(hold_reserved),
        work_step_count_max: Limit::Bounded(
            u32::from(hop_count)
                .saturating_add(1)
                .saturating_add(path_penalty),
        ),
    }
}

/// Returns the segment at the current forwarding cursor position, or `None`
/// if the route has been fully forwarded.
pub(crate) fn current_segment(route: &ActivePathwayRoute) -> Option<&PathwayRouteSegment> {
    route
        .path
        .segments
        .get(usize::from(route.forwarding.next_hop_index))
}

pub(super) fn route_checkpoint(active_route: &ActivePathwayRoute) -> PathwayRouteCheckpoint {
    PathwayRouteCheckpoint {
        current_epoch: active_route.current_epoch,
        forwarding: active_route.forwarding.clone(),
        repair: active_route.repair.clone(),
        handoff: active_route.handoff.clone(),
        anti_entropy: active_route.anti_entropy.clone(),
    }
}

pub(super) fn checkpoint_bytes(active_route: &ActivePathwayRoute) -> Vec<u8> {
    encode_versioned(CHECKPOINT_ENCODING_VERSION, &route_checkpoint(active_route))
}

pub(super) fn decode_checkpoint_bytes(bytes: &[u8]) -> Option<PathwayRouteCheckpoint> {
    decode_versioned(bytes, CHECKPOINT_ENCODING_VERSION)
}

pub(super) fn restored_active_route<H: Hashing>(
    route: &MaterializedRoute,
    checkpoint: &PathwayRouteCheckpoint,
    hashing: &H,
) -> Option<ActivePathwayRoute> {
    if route.identity.admission.backend_ref.engine != PATHWAY_ENGINE_ID {
        return None;
    }
    let plan = decode_backend_token(&route.identity.admission.backend_ref.backend_route_id)?;
    let route_id = route.identity.stamp.route_id;
    let path_bytes = encode_path_bytes(&node_path_from_plan_token(&plan), &plan.segments);
    let ordering_key = deterministic_order_key(route_id, hashing, &path_bytes);
    let committee = match plan.committee_status {
        CommitteeStatus::Selected(selection) => Some(selection),
        CommitteeStatus::NotApplicable => None,
        CommitteeStatus::SelectorFailed => return None,
    };
    Some(ActivePathwayRoute {
        path: super::PathwayPath {
            route_id,
            epoch: plan.epoch,
            source: plan.source,
            destination: plan.destination,
            segments: plan.segments,
            valid_for: plan.valid_for,
            route_class: plan.route_class,
        },
        committee,
        current_epoch: checkpoint.current_epoch,
        last_lifecycle_event: route.runtime.last_lifecycle_event,
        route_cost: route.identity.admission.admission_check.route_cost.clone(),
        ordering_key,
        forwarding: checkpoint.forwarding.clone(),
        repair: checkpoint.repair.clone(),
        handoff: checkpoint.handoff.clone(),
        anti_entropy: checkpoint.anti_entropy.clone(),
    })
}

pub(super) fn route_storage_key(local_node_id: &NodeId, route_id: &RouteId) -> Vec<u8> {
    let mut key = b"pathway/".to_vec();
    key.extend_from_slice(&local_node_id.0);
    key.extend_from_slice(b"/route/");
    key.extend_from_slice(&route_id.0);
    key
}

pub(super) fn topology_epoch_storage_key(local_node_id: &NodeId) -> Vec<u8> {
    let mut key = b"pathway/".to_vec();
    key.extend_from_slice(&local_node_id.0);
    key.extend_from_slice(b"/topology-epoch");
    key
}

pub(super) fn limit_u32(limit: Limit<u32>) -> u32 {
    match limit {
        Limit::Unbounded => u32::MAX,
        Limit::Bounded(value) => value,
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        AdmissionAssumptions, AdversaryRegime, Belief, ByteCount, ClaimStrength, CommitteeId,
        CommitteeMember, CommitteeRole, CommitteeSelection, ConnectivityPosture,
        ConnectivityRegime, ContentId, ControllerId, DestinationId, EndpointLocator, Environment,
        Estimate, FailureModelClass, HoldFallbackPolicy, Limit, LinkEndpoint,
        MessageFlowAssumptionClass, NodeDensityClass, RatioPermille, RouteCost, RouteEpoch,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
        RouteSummary, RoutingObjective, RuntimeEnvelopeClass, SelectedRoutingParameters, Tick,
        TransportKind,
    };

    use super::*;
    use crate::{PathwayPath, PATHWAY_ENGINE_ID};

    fn neutral_assumptions() -> AdmissionAssumptions {
        AdmissionAssumptions {
            message_flow_assumption: MessageFlowAssumptionClass::PerRouteSequenced,
            failure_model: FailureModelClass::Benign,
            runtime_envelope: RuntimeEnvelopeClass::Canonical,
            node_density_class: NodeDensityClass::Sparse,
            connectivity_regime: ConnectivityRegime::Stable,
            adversary_regime: AdversaryRegime::BenignUntrusted,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
        }
    }

    fn objective_with_floor(floor: RouteProtectionClass) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(NodeId([3; 32])),
            service_kind: RouteServiceKind::Move,
            target_protection: floor,
            protection_floor: floor,
            target_connectivity: ConnectivityPosture {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::ConnectedOnly,
            },
            hold_fallback_policy: HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Unbounded,
            protection_priority: jacquard_core::PriorityPoints(0),
            connectivity_priority: jacquard_core::PriorityPoints(0),
        }
    }

    fn profile_with(
        repair: RouteRepairClass,
        partition: RoutePartitionClass,
    ) -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture { repair, partition },
            deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
            diversity_floor: DiversityFloor(1),
            routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn summary_with(
        protection: RouteProtectionClass,
        repair: RouteRepairClass,
        partition: RoutePartitionClass,
    ) -> RouteSummary {
        RouteSummary {
            engine: PATHWAY_ENGINE_ID,
            protection,
            connectivity: ConnectivityPosture { repair, partition },
            protocol_mix: Vec::new(),
            hop_count_hint: Belief::Estimated(Estimate {
                value: 1_u8,
                confidence_permille: RatioPermille(1000),
                updated_at_tick: Tick(0),
            }),
            valid_for: TimeWindow::new(Tick(0), Tick(100)).unwrap(),
        }
    }

    fn unit_route_cost() -> RouteCost {
        RouteCost {
            message_count_max: Limit::Bounded(1),
            byte_count_max: Limit::Bounded(jacquard_core::ByteCount(1024)),
            hop_count: 1,
            repair_attempt_count_max: Limit::Bounded(1),
            hold_bytes_reserved: Limit::Bounded(jacquard_core::ByteCount(0)),
            work_step_count_max: Limit::Bounded(2),
        }
    }

    fn opaque_endpoint(
        transport_kind: TransportKind,
        bytes: Vec<u8>,
        mtu_bytes: ByteCount,
    ) -> LinkEndpoint {
        LinkEndpoint::new(transport_kind, EndpointLocator::Opaque(bytes), mtu_bytes)
    }

    fn scoped_endpoint(
        transport_kind: TransportKind,
        scope: &str,
        bytes: Vec<u8>,
        mtu_bytes: ByteCount,
    ) -> LinkEndpoint {
        LinkEndpoint::new(
            transport_kind,
            EndpointLocator::ScopedBytes {
                scope: scope.into(),
                bytes,
            },
            mtu_bytes,
        )
    }

    fn socket_endpoint(
        transport_kind: TransportKind,
        host: &str,
        port: u16,
        mtu_bytes: ByteCount,
    ) -> LinkEndpoint {
        LinkEndpoint::new(
            transport_kind,
            EndpointLocator::Socket {
                host: host.into(),
                port,
            },
            mtu_bytes,
        )
    }

    #[test]
    fn storage_keys_are_scoped_by_local_node_id() {
        let left_node = NodeId([1; 32]);
        let right_node = NodeId([9; 32]);
        let route_id = RouteId([7; 16]);

        assert_ne!(
            route_storage_key(&left_node, &route_id),
            route_storage_key(&right_node, &route_id)
        );
        assert_ne!(
            topology_epoch_storage_key(&left_node),
            topology_epoch_storage_key(&right_node)
        );
    }

    #[test]
    fn shortest_paths_returns_only_local_node_for_singleton_graph() {
        let local = NodeId([1; 32]);
        let configuration = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
            environment: Environment {
                reachable_neighbor_count: 0,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };
        let paths = shortest_paths(&local, &configuration);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths.get(&local).map(Vec::len), Some(1));
    }

    fn link_with_protocol(protocol: TransportKind) -> jacquard_core::Link {
        jacquard_core::Link {
            endpoint: opaque_endpoint(protocol, vec![0], ByteCount(64)),
            profile: jacquard_core::LinkProfile {
                latency_floor_ms: jacquard_core::DurationMs(8),
                repair_capability: jacquard_core::RepairCapability::TransportRetransmit,
                partition_recovery: jacquard_core::PartitionRecoveryClass::LocalReconnect,
            },
            state: jacquard_core::LinkState {
                state: jacquard_core::LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::Absent,
                symmetry_permille: Belief::Absent,
            },
        }
    }

    #[test]
    fn shortest_paths_skips_disconnected_components() {
        let local = NodeId([1; 32]);
        let connected = NodeId([2; 32]);
        let isolated = NodeId([3; 32]);
        let configuration = Configuration {
            epoch: RouteEpoch(0),
            nodes: BTreeMap::new(),
            links: BTreeMap::from([(
                (local, connected),
                link_with_protocol(TransportKind::WifiAware),
            )]),
            environment: Environment {
                reachable_neighbor_count: 1,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        };
        let paths = shortest_paths(&local, &configuration);
        assert!(paths.contains_key(&local));
        assert!(paths.contains_key(&connected));
        assert!(!paths.contains_key(&isolated));
    }

    #[test]
    fn retention_object_id_is_stable_across_calls() {
        let hashing = jacquard_traits::Blake3Hashing;
        let route_a = RouteId([1; 16]);
        let route_b = RouteId([2; 16]);

        let mut tagged_a = route_a.0.to_vec();
        tagged_a.extend_from_slice(b"payload");
        let id_a_first = ContentId {
            digest: hashing.hash_tagged(b"pathway-retention", &tagged_a),
        };
        let id_a_second = ContentId {
            digest: hashing.hash_tagged(b"pathway-retention", &tagged_a),
        };
        assert_eq!(id_a_first, id_a_second);

        let mut tagged_b = route_b.0.to_vec();
        tagged_b.extend_from_slice(b"payload");
        let id_b = ContentId {
            digest: hashing.hash_tagged(b"pathway-retention", &tagged_b),
        };
        assert_ne!(id_a_first, id_b);

        let mut tagged_c = route_a.0.to_vec();
        tagged_c.extend_from_slice(b"different");
        let id_c = ContentId {
            digest: hashing.hash_tagged(b"pathway-retention", &tagged_c),
        };
        assert_ne!(id_a_first, id_c);
    }

    fn sample_plan_token() -> PathwayPlanToken {
        PathwayPlanToken {
            epoch: RouteEpoch(2),
            source: NodeId([1; 32]),
            destination: jacquard_core::DestinationId::Node(NodeId([3; 32])),
            segments: vec![
                PathwayRouteSegment {
                    node_id: NodeId([2; 32]),
                    endpoint: scoped_endpoint(
                        TransportKind::BleGatt,
                        "ble",
                        vec![2; 17],
                        ByteCount(64),
                    ),
                },
                PathwayRouteSegment {
                    node_id: NodeId([3; 32]),
                    endpoint: socket_endpoint(
                        TransportKind::WifiLan,
                        "relay-3",
                        4040,
                        ByteCount(1400),
                    ),
                },
            ],
            valid_for: TimeWindow::new(Tick(2), Tick(14)).unwrap(),
            route_class: PathwayRouteClass::DeferredDelivery,
            committee_status: CommitteeStatus::Selected(CommitteeSelection {
                committee_id: CommitteeId([9; 16]),
                topology_epoch: RouteEpoch(2),
                selected_at_tick: Tick(2),
                valid_for: TimeWindow::new(Tick(2), Tick(10)).unwrap(),
                evidence_basis: jacquard_core::FactBasis::Estimated,
                claim_strength: jacquard_core::ClaimStrength::ConservativeUnderProfile,
                identity_assurance: jacquard_core::IdentityAssuranceClass::ControllerBound,
                quorum_threshold: QuorumThreshold(1),
                members: vec![CommitteeMember {
                    node_id: NodeId([2; 32]),
                    controller_id: ControllerId([2; 32]),
                    role: CommitteeRole::Participant,
                }],
            }),
        }
    }

    fn sample_active_route() -> ActivePathwayRoute {
        let plan = sample_plan_token();
        ActivePathwayRoute {
            path: PathwayPath {
                route_id: RouteId([7; 16]),
                epoch: plan.epoch,
                source: plan.source,
                destination: plan.destination,
                segments: plan.segments,
                valid_for: plan.valid_for,
                route_class: plan.route_class,
            },
            committee: match plan.committee_status {
                CommitteeStatus::Selected(selection) => Some(selection),
                _ => None,
            },
            current_epoch: RouteEpoch(2),
            last_lifecycle_event: jacquard_core::RouteLifecycleEvent::Activated,
            route_cost: unit_route_cost(),
            ordering_key: DeterministicOrderKey {
                stable_key: RouteId([7; 16]),
                tie_break: OrderStamp(17),
            },
            forwarding: super::super::PathwayForwardingState {
                current_owner_node_id: NodeId([1; 32]),
                next_hop_index: 1,
                in_flight_frames: 2,
                last_ack_at_tick: Some(Tick(3)),
            },
            repair: super::super::PathwayRepairState {
                steps_remaining: 3,
                last_repaired_at_tick: Some(Tick(4)),
            },
            handoff: super::super::PathwayHandoffState {
                last_receipt_id: Some(jacquard_core::ReceiptId([5; 16])),
                last_handoff_at_tick: Some(Tick(5)),
            },
            anti_entropy: super::super::PathwayRouteAntiEntropyState {
                partition_mode: true,
                retained_objects: std::iter::once(ContentId {
                    digest: jacquard_core::Blake3Digest([6; 32]),
                })
                .collect(),
                last_refresh_at_tick: Some(Tick(6)),
            },
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn encoding_digest_hex(bytes: &[u8]) -> String {
        let digest =
            jacquard_traits::Blake3Hashing.hash_tagged(b"pathway-canonical-encoding-test", bytes);
        hex(digest.as_bytes())
    }

    #[test]
    fn canonical_encodings_round_trip() {
        let plan = sample_plan_token();
        let checkpointed_route = sample_active_route();
        let backend = encode_backend_token(&plan);
        assert_eq!(decode_backend_token(&backend), Some(plan));
        let checkpoint = checkpoint_bytes(&checkpointed_route);
        assert_eq!(
            decode_checkpoint_bytes(&checkpoint),
            Some(route_checkpoint(&checkpointed_route))
        );
    }

    #[test]
    fn pathway_domain_tags_are_unique() {
        let tags = [
            DOMAIN_TAG_ROUTE_ID,
            DOMAIN_TAG_COMMITMENT,
            DOMAIN_TAG_HANDOFF_RECEIPT,
            DOMAIN_TAG_RETENTION,
            DOMAIN_TAG_COMMITTEE_ID,
            DOMAIN_TAG_ORDER_KEY,
        ];
        let unique = tags.into_iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn canonical_encoding_digests_are_stable() {
        let plan = sample_plan_token();
        let checkpointed_route = sample_active_route();
        let backend_token = encode_backend_token(&plan);
        let route_identity = encode_route_identity_bytes(&plan);
        let checkpoint = checkpoint_bytes(&checkpointed_route);
        let route_id_digest =
            jacquard_traits::Blake3Hashing.hash_tagged(DOMAIN_TAG_ROUTE_ID, &route_identity);
        let route_id = &route_id_digest.as_bytes()[..16];

        assert_eq!(backend_token.0.len(), 265);
        assert_eq!(route_identity.len(), 171);
        assert_eq!(checkpoint.len(), 96);
        assert_eq!(
            encoding_digest_hex(&backend_token.0),
            "a799e1175cf11957094ae35f7bf357d1a47e2daf663caf06ebdb77f139dd3906"
        );
        assert_eq!(
            encoding_digest_hex(&route_identity),
            "794aca5688918110e91000cfc4f1715a4befa79244da3af2f7bae1d36ff5a130"
        );
        assert_eq!(hex(route_id), "ca0b57c68788596c1a166617929531ef");
        assert_eq!(
            encoding_digest_hex(&checkpoint),
            "bcbb29716ee52e0cb1dd0005d0b3a34aadd2c3b9c9f5763be0fb5ff77668a78b"
        );
    }

    #[test]
    fn backend_route_id_size_is_bounded() {
        const BACKEND_ROUTE_ID_BYTES_MAX: usize = 2048;

        let mut plan = sample_plan_token();
        plan.segments = (0..usize::from(jacquard_core::ROUTE_HOP_COUNT_MAX))
            .map(|index| {
                let byte = u8::try_from(index + 1).unwrap_or(u8::MAX);
                PathwayRouteSegment {
                    node_id: NodeId([byte; 32]),
                    endpoint: opaque_endpoint(
                        TransportKind::Custom(format!("transport-{byte}")),
                        vec![byte; 32],
                        ByteCount(1400),
                    ),
                }
            })
            .collect();

        let encoded = encode_backend_token(&plan);
        assert!(encoded.0.len() <= BACKEND_ROUTE_ID_BYTES_MAX);
    }

    #[test]
    fn route_id_is_path_identity_across_epochs() {
        let mut older = sample_plan_token();
        let mut newer = sample_plan_token();
        older.epoch = RouteEpoch(2);
        newer.epoch = RouteEpoch(99);

        let older_identity = encode_route_identity_bytes(&older);
        let newer_identity = encode_route_identity_bytes(&newer);
        assert_eq!(older_identity, newer_identity);
    }

    #[test]
    fn path_bytes_distinguish_transport_and_endpoint_variants() {
        let path = vec![NodeId([1; 32]), NodeId([2; 32])];
        let ble_segments = vec![PathwayRouteSegment {
            node_id: NodeId([2; 32]),
            endpoint: scoped_endpoint(TransportKind::BleGatt, "ble", vec![2; 17], ByteCount(64)),
        }];
        let wifi_segments = vec![PathwayRouteSegment {
            node_id: NodeId([2; 32]),
            endpoint: socket_endpoint(TransportKind::WifiLan, "relay-2", 4040, ByteCount(1400)),
        }];

        assert_ne!(
            encode_path_bytes(&path, &ble_segments),
            encode_path_bytes(&path, &wifi_segments)
        );
    }

    #[test]
    fn support_smoke_values_exist() {
        let _assumptions = neutral_assumptions();
        let _objective = objective_with_floor(RouteProtectionClass::LinkProtected);
        let _profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let _summary = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let _route_cost = unit_route_cost();
    }
}
