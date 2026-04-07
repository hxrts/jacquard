//! Pure helper functions shared between `planner` and `runtime`.
//!
//! Every function in this module is a deterministic function of its
//! inputs: no runtime state, no effects, no hidden context. Includes
//! the repair-time shortest-path helper, the self-contained
//! `BackendRouteId` encoding and decoding, byte encoders used by
//! tagged hashing, and the `RouteCost` derivation.

use std::{
    collections::{BTreeMap, VecDeque},
    convert::TryFrom,
};

use bincode::Options;
use jacquard_core::{
    BackendRouteId, ByteCount, CommitteeSelection, Configuration, DegradationReason,
    DestinationId, DeterministicOrderKey, Limit, NodeId, OrderStamp, RouteCost,
    RouteDegradation, RouteEpoch, RouteId, TimeWindow,
};
use jacquard_traits::{HashDigestBytes, Hashing};
use serde::{Deserialize, Serialize};

use super::{
    ActiveMeshRoute, MeshCommitteeStatus, MeshRouteClass, MeshRouteSegment,
    MESH_HOLD_RESERVED_BYTES, MESH_PER_HOP_BYTE_COST,
};
use crate::topology::{adjacent_link_between, adjacent_node_ids, belief_into_estimate};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MeshPlanToken {
    pub(super) epoch: RouteEpoch,
    pub(super) source: NodeId,
    pub(super) destination: DestinationId,
    pub(super) segments: Vec<MeshRouteSegment>,
    pub(super) valid_for: TimeWindow,
    pub(super) route_class: MeshRouteClass,
    // Keep committee status self-contained inside the token so cache misses,
    // engine restarts, and materialization re-derivation preserve the exact
    // admission semantics without reintroducing planner cache dependence.
    pub(super) committee_status: MeshCommitteeStatus,
}

pub(super) use super::MeshCommitteeStatus as CommitteeStatus;

pub(crate) const DOMAIN_TAG_ROUTE_ID: &[u8] = b"mesh-route-id";
pub(crate) const DOMAIN_TAG_COMMITMENT: &[u8] = b"mesh-commitment";
pub(crate) const DOMAIN_TAG_HANDOFF_RECEIPT: &[u8] = b"mesh-handoff-receipt";
pub(crate) const DOMAIN_TAG_RETENTION: &[u8] = b"mesh-retention";
pub(crate) const DOMAIN_TAG_COMMITTEE_ID: &[u8] = b"mesh-committee-id";
pub(crate) const DOMAIN_TAG_ORDER_KEY: &[u8] = b"mesh-order-key";

const PLAN_TOKEN_ENCODING_VERSION: u8 = 1;
const ROUTE_IDENTITY_ENCODING_VERSION: u8 = 1;
const CHECKPOINT_ENCODING_VERSION: u8 = 1;
const PATH_ENCODING_VERSION: u8 = 1;
const MESH_DEGRADATION_PRESSURE_THRESHOLD_PERMILLE: u16 = 600;

pub(super) fn committee_status(
    result: Result<Option<CommitteeSelection>, jacquard_core::RouteError>,
) -> MeshCommitteeStatus {
    match result {
        | Ok(Some(selection)) => MeshCommitteeStatus::Selected(selection),
        | Ok(None) => MeshCommitteeStatus::NotApplicable,
        | Err(_) => MeshCommitteeStatus::SelectorFailed,
    }
}

// `allow_trailing_bytes` is required because decode_versioned strips the
// leading version byte before passing the rest to bincode, which would
// otherwise reject the slice as oversized.
fn canonical_options() -> impl Options {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
}

fn encode_versioned<T: Serialize>(version: u8, value: &T) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    bytes.push(version);
    bytes.extend(
        canonical_options()
            .serialize(value)
            .expect("mesh canonical bytes are always serializable"),
    );
    bytes
}

fn decode_versioned<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    version: u8,
) -> Option<T> {
    let (encoded_version, rest) = bytes.split_first()?;
    if *encoded_version != version {
        return None;
    }
    canonical_options().deserialize(rest).ok()
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
    segments: &[MeshRouteSegment],
) -> Vec<jacquard_core::TransportProtocol> {
    let mut protocols = segments
        .iter()
        .map(|segment| segment.endpoint.protocol.clone())
        .collect::<Vec<_>>();
    protocols.sort();
    protocols.dedup();
    protocols
}

pub(super) fn encode_path_bytes(
    path: &[NodeId],
    segments: &[MeshRouteSegment],
) -> Vec<u8> {
    #[derive(Serialize)]
    struct PathEncoding<'a> {
        path: &'a [NodeId],
        segments: &'a [MeshRouteSegment],
    }

    encode_versioned(PATH_ENCODING_VERSION, &PathEncoding { path, segments })
}

pub(super) fn node_path_from_plan_token(plan: &MeshPlanToken) -> Vec<NodeId> {
    let mut path = Vec::with_capacity(plan.segments.len() + 1);
    path.push(plan.source);
    path.extend(plan.segments.iter().map(|segment| segment.node_id));
    path
}

pub(super) fn encode_route_identity_bytes(plan: &MeshPlanToken) -> Vec<u8> {
    #[derive(Serialize)]
    struct MeshRouteIdentity<'a> {
        source: &'a NodeId,
        destination: &'a DestinationId,
        segments: &'a [MeshRouteSegment],
        route_class: &'a MeshRouteClass,
    }

    // Route ids in v1 mesh are path identities rather than per-epoch instance
    // identities. The epoch stays in the plan token and materialization proof,
    // but the stable route id is derived from source, destination, route class,
    // and the concrete segment path only.
    encode_versioned(
        ROUTE_IDENTITY_ENCODING_VERSION,
        &MeshRouteIdentity {
            source: &plan.source,
            destination: &plan.destination,
            segments: &plan.segments,
            route_class: &plan.route_class,
        },
    )
}

// Self-contained plan token: a serialized mesh-private route plan
// carrying the path, route class, validity window, and optional
// committee result. Planner cache entries may be dropped; materialize
// and planner cache-miss paths decode this token instead of depending
// on ambient mutable engine state.
pub(super) fn encode_backend_token(plan: &MeshPlanToken) -> BackendRouteId {
    BackendRouteId(encode_versioned(PLAN_TOKEN_ENCODING_VERSION, plan))
}

// Inverse of `encode_backend_token`. Invalid or hand-crafted bytes fail
// closed with None rather than being partially decoded.
pub(super) fn decode_backend_token(
    backend_route_id: &BackendRouteId,
) -> Option<MeshPlanToken> {
    decode_versioned(&backend_route_id.0, PLAN_TOKEN_ENCODING_VERSION)
}

pub(super) fn deterministic_order_key<H: Hashing>(
    route_id: RouteId,
    hashing: &H,
    path_bytes: &[u8],
) -> DeterministicOrderKey<RouteId> {
    let digest = hashing.hash_tagged(DOMAIN_TAG_ORDER_KEY, path_bytes);
    let mut tie_break_bytes = [0_u8; 8];
    tie_break_bytes.copy_from_slice(&digest.as_bytes()[..8]);
    DeterministicOrderKey {
        stable_key: route_id,
        tie_break: OrderStamp(u64::from_le_bytes(tie_break_bytes)),
    }
}

// Candidate confidence is the worst-hop delivery confidence along the
// path. A single weak link drags the whole route down, which is the
// correct semantic for an ordered hop-by-hop source route.
pub(super) fn confidence_for_segments(
    segments: &[MeshRouteSegment],
    configuration: &Configuration,
) -> jacquard_core::RatioPermille {
    let mut confidence = 1000_u16;
    let mut previous = None;
    for segment in segments {
        if let Some(from) = previous {
            if let Some(link) =
                adjacent_link_between(&from, &segment.node_id, configuration)
            {
                confidence = confidence.min(
                    belief_into_estimate(link.state.delivery_confidence_permille)
                        .map_or(jacquard_core::RatioPermille(0), |estimate| {
                            estimate.value
                        })
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
    route_class: &MeshRouteClass,
) -> RouteDegradation {
    if matches!(route_class, MeshRouteClass::DeferredDelivery) {
        RouteDegradation::Degraded(DegradationReason::PartitionRisk)
    } else if configuration.environment.contention_permille.get()
        > MESH_DEGRADATION_PRESSURE_THRESHOLD_PERMILLE
    {
        RouteDegradation::Degraded(DegradationReason::CapacityPressure)
    } else if configuration.environment.churn_permille.get()
        > MESH_DEGRADATION_PRESSURE_THRESHOLD_PERMILLE
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
fn hold_reserved_bytes(route_class: &MeshRouteClass) -> ByteCount {
    match route_class {
        | MeshRouteClass::DeferredDelivery => ByteCount(MESH_HOLD_RESERVED_BYTES),
        | _ => ByteCount(0),
    }
}

fn route_quality_penalties(
    node_path: &[NodeId],
    segments: &[MeshRouteSegment],
    configuration: &Configuration,
) -> (u32, u32, u32) {
    let confidence = u32::from(confidence_for_segments(segments, configuration).get());
    let delivery_penalty = (1000_u32.saturating_sub(confidence)) / 100;
    let (symmetry_penalty, congestion_penalty) = segments.iter().enumerate().fold(
        (0_u32, 0_u32),
        |(symmetry_penalty, congestion_penalty), (index, segment)| {
            let previous_node =
                node_path.get(index).copied().unwrap_or(segment.node_id);
            let Some(link) =
                adjacent_link_between(&previous_node, &segment.node_id, configuration)
            else {
                return (symmetry_penalty, congestion_penalty);
            };
            let symmetry_penalty = symmetry_penalty.saturating_add(
                belief_into_estimate(link.state.symmetry_permille).map_or(
                    10,
                    |estimate| {
                        (1000_u32.saturating_sub(u32::from(estimate.value.get()))) / 100
                    },
                ),
            );
            let congestion_penalty = congestion_penalty
                .saturating_add(u32::from(link.state.loss_permille.get()) / 100);
            (symmetry_penalty, congestion_penalty)
        },
    );
    (delivery_penalty, symmetry_penalty, congestion_penalty)
}

fn protocol_diversity_bonus(segments: &[MeshRouteSegment]) -> u32 {
    let protocol_mix = unique_protocol_mix(segments);
    let u32_max_as_usize =
        usize::try_from(u32::MAX).expect("u32::MAX fits on supported targets");
    debug_assert!(protocol_mix.len() <= u32_max_as_usize);
    u32::try_from(protocol_mix.len())
        .expect("protocol diversity is bounded by segment count")
        .saturating_sub(1)
}

pub(super) fn route_cost_for_segments(
    node_path: &[NodeId],
    segments: &[MeshRouteSegment],
    route_class: &MeshRouteClass,
    configuration: &Configuration,
) -> RouteCost {
    let hop_count = u8::try_from(segments.len())
        .expect("segment count is bounded by ROUTE_HOP_COUNT_MAX");
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
        message_count_max: Limit::Bounded(
            u32::from(hop_count).saturating_add(path_penalty),
        ),
        byte_count_max: Limit::Bounded(ByteCount(
            u64::from(hop_count) * MESH_PER_HOP_BYTE_COST
                + u64::from(path_penalty) * 128,
        )),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(
            u32::from(hop_count).saturating_add(path_penalty),
        ),
        hold_bytes_reserved: Limit::Bounded(hold_reserved),
        work_step_count_max: Limit::Bounded(
            u32::from(hop_count)
                .saturating_add(1)
                .saturating_add(path_penalty),
        ),
    }
}

pub(super) fn checkpoint_bytes(active_route: &ActiveMeshRoute) -> Vec<u8> {
    encode_versioned(CHECKPOINT_ENCODING_VERSION, active_route)
}

pub(super) fn decode_checkpoint_bytes(bytes: &[u8]) -> Option<ActiveMeshRoute> {
    decode_versioned(bytes, CHECKPOINT_ENCODING_VERSION)
}

pub(super) fn route_storage_key(local_node_id: &NodeId, route_id: &RouteId) -> Vec<u8> {
    let mut key = b"mesh/".to_vec();
    key.extend_from_slice(&local_node_id.0);
    key.extend_from_slice(b"/route/");
    key.extend_from_slice(&route_id.0);
    key
}

pub(super) fn topology_epoch_storage_key(local_node_id: &NodeId) -> Vec<u8> {
    let mut key = b"mesh/".to_vec();
    key.extend_from_slice(&local_node_id.0);
    key.extend_from_slice(b"/topology-epoch");
    key
}

pub(super) fn limit_u32(limit: Limit<u32>) -> u32 {
    match limit {
        | Limit::Unbounded => u32::MAX,
        | Limit::Bounded(value) => value,
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        AdaptiveRoutingProfile, AdmissionAssumptions, AdversaryRegime, Belief,
        ClaimStrength, CommitteeId, CommitteeMember, CommitteeRole,
        CommitteeSelection, ConnectivityRegime, ContentId, ControllerId,
        DestinationId, Environment, Estimate, FailureModelClass,
        HoldFallbackPolicy, HostName, Limit, LinkEndpoint,
        MessageFlowAssumptionClass, NetworkHost, NodeDensityClass,
        RatioPermille, RouteConnectivityProfile, RouteCost, RouteEpoch,
        RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
        RouteServiceKind, RouteSummary, RoutingObjective, RuntimeEnvelopeClass,
        Tick,
    };

    use super::*;
    use crate::{MeshPath, MESH_ENGINE_ID};

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

    fn objective_with_floor(
        floor: RouteProtectionClass,
    ) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Node(NodeId([3; 32])),
            service_kind: RouteServiceKind::Move,
            target_protection: floor,
            protection_floor: floor,
            target_connectivity: RouteConnectivityProfile {
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
    ) -> AdaptiveRoutingProfile {
        AdaptiveRoutingProfile {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: RouteConnectivityProfile { repair, partition },
            deployment_profile:
                jacquard_core::DeploymentProfile::FieldPartitionTolerant,
            diversity_floor: 1,
            routing_engine_fallback_policy:
                jacquard_core::RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy:
                jacquard_core::RouteReplacementPolicy::Allowed,
        }
    }

    fn summary_with(
        protection: RouteProtectionClass,
        repair: RouteRepairClass,
        partition: RoutePartitionClass,
    ) -> RouteSummary {
        RouteSummary {
            engine: MESH_ENGINE_ID,
            protection,
            connectivity: RouteConnectivityProfile { repair, partition },
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

    fn link_with_protocol(
        protocol: jacquard_core::TransportProtocol,
    ) -> jacquard_core::Link {
        jacquard_core::Link {
            endpoint: LinkEndpoint {
                protocol,
                address: jacquard_core::EndpointAddress::Ble {
                    device_id: jacquard_core::BleDeviceId(vec![0]),
                    profile_id: jacquard_core::BleProfileId([0; 16]),
                },
                mtu_bytes: ByteCount(256),
            },
            state: jacquard_core::LinkState {
                state: jacquard_core::LinkRuntimeState::Active,
                median_rtt_ms: jacquard_core::DurationMs(40),
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
                link_with_protocol(jacquard_core::TransportProtocol::BleGatt),
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
            digest: hashing.hash_tagged(b"mesh-retention", &tagged_a),
        };
        let id_a_second = ContentId {
            digest: hashing.hash_tagged(b"mesh-retention", &tagged_a),
        };
        assert_eq!(id_a_first, id_a_second);

        let mut tagged_b = route_b.0.to_vec();
        tagged_b.extend_from_slice(b"payload");
        let id_b = ContentId {
            digest: hashing.hash_tagged(b"mesh-retention", &tagged_b),
        };
        assert_ne!(id_a_first, id_b);

        let mut tagged_c = route_a.0.to_vec();
        tagged_c.extend_from_slice(b"different");
        let id_c = ContentId {
            digest: hashing.hash_tagged(b"mesh-retention", &tagged_c),
        };
        assert_ne!(id_a_first, id_c);
    }

    fn sample_plan_token() -> MeshPlanToken {
        MeshPlanToken {
            epoch: RouteEpoch(2),
            source: NodeId([1; 32]),
            destination: jacquard_core::DestinationId::Node(NodeId([3; 32])),
            segments: vec![
                MeshRouteSegment {
                    node_id: NodeId([2; 32]),
                    endpoint: LinkEndpoint {
                        protocol: jacquard_core::TransportProtocol::BleGatt,
                        address: jacquard_core::EndpointAddress::Ble {
                            device_id: jacquard_core::BleDeviceId(vec![2]),
                            profile_id: jacquard_core::BleProfileId([2; 16]),
                        },
                        mtu_bytes: ByteCount(256),
                    },
                },
                MeshRouteSegment {
                    node_id: NodeId([3; 32]),
                    endpoint: LinkEndpoint {
                        protocol: jacquard_core::TransportProtocol::WifiLan,
                        address: jacquard_core::EndpointAddress::Ip {
                            host: NetworkHost::Name(HostName("relay-3".into())),
                            port: 4040,
                        },
                        mtu_bytes: ByteCount(1400),
                    },
                },
            ],
            valid_for: TimeWindow::new(Tick(2), Tick(14)).unwrap(),
            route_class: MeshRouteClass::DeferredDelivery,
            committee_status: CommitteeStatus::Selected(CommitteeSelection {
                committee_id: CommitteeId([9; 16]),
                topology_epoch: RouteEpoch(2),
                selected_at_tick: Tick(2),
                valid_for: TimeWindow::new(Tick(2), Tick(10)).unwrap(),
                evidence_basis: jacquard_core::FactBasis::Estimated,
                claim_strength: jacquard_core::ClaimStrength::ConservativeUnderProfile,
                identity_assurance:
                    jacquard_core::IdentityAssuranceClass::ControllerBound,
                quorum_threshold: 1,
                members: vec![CommitteeMember {
                    node_id: NodeId([2; 32]),
                    controller_id: ControllerId([2; 32]),
                    role: CommitteeRole::Participant,
                }],
            }),
        }
    }

    fn sample_active_route() -> ActiveMeshRoute {
        let plan = sample_plan_token();
        ActiveMeshRoute {
            path: MeshPath {
                route_id: RouteId([7; 16]),
                epoch: plan.epoch,
                source: plan.source,
                destination: plan.destination,
                segments: plan.segments,
                valid_for: plan.valid_for,
                route_class: plan.route_class,
            },
            committee: match plan.committee_status {
                | CommitteeStatus::Selected(selection) => Some(selection),
                | _ => None,
            },
            current_epoch: RouteEpoch(2),
            last_lifecycle_event: jacquard_core::RouteLifecycleEvent::Activated,
            route_cost: unit_route_cost(),
            ordering_key: DeterministicOrderKey {
                stable_key: RouteId([7; 16]),
                tie_break: OrderStamp(17),
            },
            forwarding: super::super::MeshForwardingState {
                current_owner_node_id: NodeId([1; 32]),
                next_hop_index: 1,
                in_flight_frames: 2,
                last_ack_at_tick: Some(Tick(3)),
            },
            repair: super::super::MeshRepairState {
                steps_remaining: 3,
                last_repaired_at_tick: Some(Tick(4)),
            },
            handoff: super::super::MeshHandoffState {
                last_receipt_id: Some(jacquard_core::ReceiptId([5; 16])),
                last_handoff_at_tick: Some(Tick(5)),
            },
            anti_entropy: super::super::MeshRouteAntiEntropyState {
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

    #[test]
    fn canonical_encodings_round_trip() {
        let plan = sample_plan_token();
        let checkpointed_route = sample_active_route();
        let backend = encode_backend_token(&plan);
        assert_eq!(decode_backend_token(&backend), Some(plan));
        let checkpoint = checkpoint_bytes(&checkpointed_route);
        assert_eq!(
            decode_checkpoint_bytes(&checkpoint),
            Some(checkpointed_route)
        );
    }

    #[test]
    fn mesh_domain_tags_are_unique() {
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
    fn canonical_bytes_snapshot_values() {
        let plan = sample_plan_token();
        let checkpointed_route = sample_active_route();
        let route_identity = encode_route_identity_bytes(&plan);
        let route_id_digest = jacquard_traits::Blake3Hashing
            .hash_tagged(DOMAIN_TAG_ROUTE_ID, &route_identity);
        let route_id = &route_id_digest.as_bytes()[..16];
        assert_eq!(
            hex(&encode_backend_token(&plan).0),
            "01020000000000000001010101010101010101010101010101010101010101010101010101010101010000000003030303030303030303030303030303030303030303030303030303030303030200000000000000020202020202020202020202020202020202020202020202020202020202020200000000000000000100000000000000020202020202020202020202020202020200010000000000000303030303030303030303030303030303030303030303030303030303030303030000000100000001000000070000000000000072656c61792d33c80f780500000000000002000000000000000e000000000000000300000001000000090909090909090909090909090909090200000000000000020000000000000002000000000000000a000000000000000100000001000000010000000101000000000000000202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020200000000"
        );
        assert_eq!(
            hex(&route_identity),
            "0101010101010101010101010101010101010101010101010101010101010101010000000003030303030303030303030303030303030303030303030303030303030303030200000000000000020202020202020202020202020202020202020202020202020202020202020200000000000000000100000000000000020202020202020202020202020202020200010000000000000303030303030303030303030303030303030303030303030303030303030303030000000100000001000000070000000000000072656c61792d33c80f780500000000000003000000"
        );
        assert_eq!(hex(route_id), "f98f7e44a7904e4f3b3d7ec88a1feafb");
        assert_eq!(
            hex(&checkpoint_bytes(&checkpointed_route)),
            "0107070707070707070707070707070707020000000000000001010101010101010101010101010101010101010101010101010101010101010000000003030303030303030303030303030303030303030303030303030303030303030200000000000000020202020202020202020202020202020202020202020202020202020202020200000000000000000100000000000000020202020202020202020202020202020200010000000000000303030303030303030303030303030303030303030303030303030303030303030000000100000001000000070000000000000072656c61792d33c80f780500000000000002000000000000000e000000000000000300000001090909090909090909090909090909090200000000000000020000000000000002000000000000000a00000000000000010000000100000001000000010100000000000000020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020202020000000002000000000000000000000001000000010000000100000000040000000000000101000000010000000100000000000000000000000100000002000000070707070707070707070707070707071100000000000000010101010101010101010101010101010101010101010101010101010101010101020000000103000000000000000300000001040000000000000001050505050505050505050505050505050105000000000000000101000000000000000606060606060606060606060606060606060606060606060606060606060606010600000000000000"
        );
    }

    #[test]
    fn backend_route_id_size_is_bounded() {
        let mut plan = sample_plan_token();
        plan.segments = (0..usize::from(jacquard_core::ROUTE_HOP_COUNT_MAX))
            .map(|index| {
                let byte = u8::try_from(index + 1).unwrap_or(u8::MAX);
                MeshRouteSegment {
                    node_id: NodeId([byte; 32]),
                    endpoint: LinkEndpoint {
                        protocol: jacquard_core::TransportProtocol::Custom(format!(
                            "mesh-{byte}"
                        )),
                        address: jacquard_core::EndpointAddress::Opaque(vec![byte; 32]),
                        mtu_bytes: ByteCount(1400),
                    },
                }
            })
            .collect();

        let encoded = encode_backend_token(&plan);
        assert!(encoded.0.len() <= crate::engine::MESH_BACKEND_ROUTE_ID_BYTES_MAX);
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
        let ble_segments = vec![MeshRouteSegment {
            node_id: NodeId([2; 32]),
            endpoint: LinkEndpoint {
                protocol: jacquard_core::TransportProtocol::BleGatt,
                address: jacquard_core::EndpointAddress::Ble {
                    device_id: jacquard_core::BleDeviceId(vec![2]),
                    profile_id: jacquard_core::BleProfileId([2; 16]),
                },
                mtu_bytes: ByteCount(256),
            },
        }];
        let wifi_segments = vec![MeshRouteSegment {
            node_id: NodeId([2; 32]),
            endpoint: LinkEndpoint {
                protocol: jacquard_core::TransportProtocol::WifiLan,
                address: jacquard_core::EndpointAddress::Ip {
                    host: NetworkHost::Name(HostName("relay-2".into())),
                    port: 4040,
                },
                mtu_bytes: ByteCount(1400),
            },
        }];

        assert_ne!(
            encode_path_bytes(&path, &ble_segments),
            encode_path_bytes(&path, &wifi_segments)
        );
    }

    #[test]
    fn support_smoke_values_exist() {
        let _ = neutral_assumptions();
        let _ = objective_with_floor(RouteProtectionClass::LinkProtected);
        let _ = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let _ = summary_with(
            RouteProtectionClass::LinkProtected,
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );
        let _ = unit_route_cost();
    }
}
