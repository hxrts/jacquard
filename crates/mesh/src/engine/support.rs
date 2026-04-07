use std::{
    collections::{BTreeMap, VecDeque},
    convert::TryFrom,
};

use jacquard_core::{
    BackendRouteId, Belief, Blake3Digest, ByteCount, Configuration, DegradationReason,
    DeterministicOrderKey, Limit, NodeId, OrderStamp, RouteCost, RouteDegradation, RouteId,
};
use jacquard_traits::Hashing;

use super::{
    ActiveMeshRoute, MeshRouteClass, MeshRouteSegment, MESH_HOLD_RESERVED_BYTES,
    MESH_PER_HOP_BYTE_COST,
};
use crate::topology::{adjacent_link_between, adjacent_node_ids};

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

pub(super) fn encode_path_bytes(path: &[NodeId], segments: &[MeshRouteSegment]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for node_id in path {
        bytes.extend_from_slice(&node_id.0);
    }
    for segment in segments {
        bytes.extend_from_slice(&segment.node_id.0);
        bytes.extend_from_slice(&segment.endpoint.mtu_bytes.0.to_le_bytes());
    }
    bytes
}

pub(super) fn encode_backend_token(path: &[NodeId]) -> BackendRouteId {
    let mut bytes = Vec::with_capacity(2 + path.len() * 32);
    bytes.push(1);
    let path_len = u8::try_from(path.len()).expect("mesh backend token path length exceeds u8");
    bytes.push(path_len);
    for node_id in path {
        bytes.extend_from_slice(&node_id.0);
    }
    BackendRouteId(bytes)
}

pub(super) fn decode_backend_token(backend_route_id: &BackendRouteId) -> Option<Vec<NodeId>> {
    let bytes = &backend_route_id.0;
    let (&version, rest) = bytes.split_first()?;
    if version != 1 {
        return None;
    }
    let (&path_len_u8, payload) = rest.split_first()?;
    let path_len = usize::from(path_len_u8);
    if path_len == 0 || payload.len() != path_len.saturating_mul(32) {
        return None;
    }

    let mut path = Vec::with_capacity(path_len);
    for chunk in payload.chunks_exact(32) {
        let mut node_id = [0_u8; 32];
        node_id.copy_from_slice(chunk);
        path.push(NodeId(node_id));
    }
    Some(path)
}

pub(super) fn deterministic_order_key<H: Hashing<Digest = Blake3Digest>>(
    route_id: RouteId,
    hashing: &H,
    path_bytes: &[u8],
) -> DeterministicOrderKey<RouteId> {
    let digest = hashing.hash_tagged(b"mesh-order-key", path_bytes);
    let mut tie_break_bytes = [0_u8; 8];
    tie_break_bytes.copy_from_slice(&digest.0[..8]);
    DeterministicOrderKey {
        stable_key: route_id,
        tie_break: OrderStamp(u64::from_le_bytes(tie_break_bytes)),
    }
}

pub(super) fn confidence_for_segments(
    segments: &[MeshRouteSegment],
    configuration: &Configuration,
) -> jacquard_core::RatioPermille {
    let mut confidence = 1000_u16;
    let mut previous = None;
    for segment in segments {
        if let Some(from) = previous {
            if let Some(link) = adjacent_link_between(&from, &segment.node_id, configuration) {
                confidence = confidence.min(
                    link.state
                        .delivery_confidence_permille
                        .into_estimate()
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
    route_class: &MeshRouteClass,
) -> RouteDegradation {
    if matches!(route_class, MeshRouteClass::DeferredDelivery) {
        RouteDegradation::Degraded(DegradationReason::PartitionRisk)
    } else if configuration.environment.contention_permille.get() > 600 {
        RouteDegradation::Degraded(DegradationReason::CapacityPressure)
    } else if configuration.environment.churn_permille.get() > 600 {
        RouteDegradation::Degraded(DegradationReason::LinkInstability)
    } else {
        RouteDegradation::None
    }
}

pub(super) fn route_cost_for_segments(
    segments: &[MeshRouteSegment],
    route_class: &MeshRouteClass,
) -> RouteCost {
    let hop_count =
        u8::try_from(segments.len()).expect("segment count is bounded by ROUTE_HOP_COUNT_MAX");
    let hold_reserved = match route_class {
        MeshRouteClass::DeferredDelivery => ByteCount(MESH_HOLD_RESERVED_BYTES),
        _ => ByteCount(0),
    };
    RouteCost {
        message_count_max: Limit::Bounded(u32::from(hop_count)),
        byte_count_max: Limit::Bounded(ByteCount(u64::from(hop_count) * MESH_PER_HOP_BYTE_COST)),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(u32::from(hop_count)),
        hold_bytes_reserved: Limit::Bounded(hold_reserved),
        work_step_count_max: Limit::Bounded(u32::from(hop_count) + 1),
    }
}

pub(super) fn checkpoint_bytes(active_route: &ActiveMeshRoute) -> Vec<u8> {
    let mut bytes = active_route.path.route_id.0.to_vec();
    bytes.extend_from_slice(&active_route.current_epoch.0.to_le_bytes());
    bytes.extend_from_slice(&active_route.route_cost.hop_count.to_le_bytes());
    bytes.extend_from_slice(&active_route.repair_steps_remaining.to_le_bytes());
    bytes.push(u8::from(active_route.partition_mode));
    bytes
}

pub(super) fn route_storage_key(route_id: &RouteId) -> Vec<u8> {
    let mut key = b"mesh/route/".to_vec();
    key.extend_from_slice(&route_id.0);
    key
}

pub(super) fn limit_u32(limit: Limit<u32>) -> u32 {
    match limit {
        Limit::Unbounded => u32::MAX,
        Limit::Bounded(value) => value,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MESH_ENGINE_ID;
    use jacquard_core::{
        AdaptiveRoutingProfile, AdmissionAssumptions, AdversaryRegime, ClaimStrength,
        ConnectivityRegime, ContentId, Environment, FailureModelClass, HoldFallbackPolicy,
        LinkEndpoint, MessageFlowAssumptionClass, NodeDensityClass, RatioPermille,
        RouteConnectivityProfile, RouteEpoch, RoutePartitionClass, RouteProtectionClass,
        RouteRepairClass, RouteServiceKind, RouteSummary, RoutingObjective, RuntimeEnvelopeClass,
        Tick, TimeWindow,
    };

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
            destination: jacquard_core::DestinationId::Node(NodeId([3; 32])),
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
            deployment_profile: jacquard_core::DeploymentProfile::FieldPartitionTolerant,
            diversity_floor: 1,
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
            engine: MESH_ENGINE_ID,
            protection,
            connectivity: RouteConnectivityProfile { repair, partition },
            protocol_mix: Vec::new(),
            hop_count_hint: Belief::Estimated(jacquard_core::Estimate {
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
            byte_count_max: Limit::Bounded(ByteCount(1024)),
            hop_count: 1,
            repair_attempt_count_max: Limit::Bounded(1),
            hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
            work_step_count_max: Limit::Bounded(2),
        }
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

    fn link_with_protocol(protocol: jacquard_core::TransportProtocol) -> jacquard_core::Link {
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
