//! Optional committee coordination for mesh.
//!
//! `NoCommitteeSelector` is the always-`None` default. `DeterministicCommitteeSelector`
//! is the in-tree implementation used when an engine opts in. Committee output
//! is advisory evidence only: canonical route admission, witnesses, and lease
//! ownership still flow through the shared router boundary. This module also
//! exposes the helpers that derive mesh admission assumptions and health
//! scores from the shared `Configuration`.

use core::cmp::Reverse;

use jacquard_core::{
    AdaptiveRoutingProfile, AdmissionAssumptions, ClaimStrength, CommitteeId, CommitteeMember,
    CommitteeRole, CommitteeSelection, Configuration, ControllerId, FactBasis, HealthScore,
    IdentityAssuranceClass, NodeDensityClass, NodeId, Observation, RouteConnectivityProfile,
    RouteEpoch, RouteError, RoutePartitionClass, RouteRepairClass, RoutingEngineId,
    RoutingObjective, Tick, TimeWindow,
};
use jacquard_traits::{CommitteeSelector, MeshTopologyModel};

use crate::topology::{
    adjacent_node_ids, optional_health_score_value, route_capable_for_engine,
    DeterministicMeshTopologyModel, MeshNeighborhoodEstimate, MeshPeerEstimate,
};

/// Default maximum committee size for `DeterministicCommitteeSelector`.
pub const MESH_COMMITTEE_MEMBERSHIP_CAP: usize = 3;

/// Committee validity window, measured in ticks.
pub const MESH_COMMITTEE_VALIDITY_TICKS: u64 = 8;

/// Minimum reachable neighbor count required before forming a committee.
/// Below this, mesh does not attempt local coordination.
pub const MESH_COMMITTEE_MIN_NEIGHBOR_COUNT: u32 = 2;

/// Churn threshold (permille) at which the admission assumptions flip to
/// `PartitionProne`.
pub const CHURN_PARTITION_PRONE_PERMILLE: u16 = 600;

/// Churn threshold (permille) at which the admission assumptions flip to
/// `HighChurn`.
pub const CHURN_HIGH_CHURN_PERMILLE: u16 = 250;

/// Neighbor count at or above which a node density is classified `Dense`.
pub const DENSITY_DENSE_NEIGHBOR_MIN: u32 = 8;

/// Neighbor count at or above which a node density is classified `Moderate`.
pub const DENSITY_MODERATE_NEIGHBOR_MIN: u32 = 3;

/// Weight multiplier applied to service score so it dominates committee
/// membership ranking. A peer without the required routing services is
/// never a viable committee member regardless of other signals.
const MESH_COMMITTEE_SERVICE_WEIGHT: u32 = 100;

/// Minimum mesh-private service stability required before local coordination
/// is worthwhile. This reads through `MeshNeighborhoodEstimate` so committee
/// gating and candidate ordering use the same topology-model interpretation.
const MESH_COMMITTEE_SERVICE_STABILITY_FLOOR: u32 = 500;

#[derive(Clone, Debug)]
pub struct NoCommitteeSelector;

impl CommitteeSelector for NoCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &RoutingObjective,
        _profile: &AdaptiveRoutingProfile,
        _topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError> {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicCommitteeSelector<Topology = DeterministicMeshTopologyModel> {
    pub local_node_id: NodeId,
    pub engine_id: RoutingEngineId,
    pub membership_cap: usize,
    pub topology_model: Topology,
}

impl DeterministicCommitteeSelector<DeterministicMeshTopologyModel> {
    #[must_use]
    pub fn new(local_node_id: NodeId) -> Self {
        Self::with_topology_model(local_node_id, DeterministicMeshTopologyModel::new())
    }
}

impl<Topology> DeterministicCommitteeSelector<Topology> {
    #[must_use]
    pub fn with_topology_model(local_node_id: NodeId, topology_model: Topology) -> Self {
        Self {
            local_node_id,
            engine_id: RoutingEngineId::Mesh,
            membership_cap: MESH_COMMITTEE_MEMBERSHIP_CAP,
            topology_model,
        }
    }
}

impl<Topology> DeterministicCommitteeSelector<Topology>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
{
    fn membership_score(
        &self,
        peer_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<(u32, ControllerId)> {
        let node = configuration.nodes.get(peer_node_id)?;
        let estimate = self.topology_model.peer_estimate(
            &self.local_node_id,
            peer_node_id,
            observed_at_tick,
            configuration,
        )?;
        let relay_score = optional_health_score_value(estimate.relay_value_score);
        let retention_score = optional_health_score_value(estimate.retention_value_score);
        let stability_score = optional_health_score_value(estimate.stability_score);
        let service_score = optional_health_score_value(estimate.service_score);
        // Service score is weighted by `MESH_COMMITTEE_SERVICE_WEIGHT` so a
        // peer without the required routing services can never outrank a
        // peer that has them, regardless of relay or link quality.
        Some((
            relay_score.saturating_add(retention_score)
                + stability_score
                + service_score.saturating_mul(MESH_COMMITTEE_SERVICE_WEIGHT),
            node.controller_id,
        ))
    }

    fn neighborhood_allows_coordination(
        &self,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> bool {
        let Some(estimate) = self.topology_model.neighborhood_estimate(
            &self.local_node_id,
            observed_at_tick,
            configuration,
        ) else {
            return false;
        };
        let density_score = optional_health_score_value(estimate.density_score);
        let service_stability = optional_health_score_value(estimate.service_stability_score);
        let partition_risk = optional_health_score_value(estimate.partition_risk_score);
        density_score > 0
            && service_stability >= MESH_COMMITTEE_SERVICE_STABILITY_FLOOR
            && service_stability >= partition_risk
    }
}

impl<Topology> CommitteeSelector for DeterministicCommitteeSelector<Topology>
where
    Topology: MeshTopologyModel<
        PeerEstimate = MeshPeerEstimate,
        NeighborhoodEstimate = MeshNeighborhoodEstimate,
    >,
{
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &AdaptiveRoutingProfile,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError> {
        let configuration = &topology.value;
        let current_tick = topology.observed_at_tick;

        // Committees only exist when the profile actually needs coordinated
        // repair and partition tolerance and the neighborhood is not a
        // degenerate 1-peer case. Otherwise there is nothing to coordinate.
        let should_coordinate = profile.selected_connectivity.repair
            == RouteRepairClass::Repairable
            && matches!(
                profile.selected_connectivity.partition,
                RoutePartitionClass::PartitionTolerant
            )
            && configuration.environment.reachable_neighbor_count
                >= MESH_COMMITTEE_MIN_NEIGHBOR_COUNT
            && self.neighborhood_allows_coordination(current_tick, configuration);
        if !should_coordinate {
            return Ok(None);
        }

        let mut ranked: Vec<_> = adjacent_node_ids(&self.local_node_id, configuration)
            .into_iter()
            .filter(|peer_node_id| {
                configuration.nodes.get(peer_node_id).is_some_and(|node| {
                    route_capable_for_engine(node, &self.engine_id, current_tick)
                })
            })
            .filter_map(|peer_node_id| {
                self.membership_score(&peer_node_id, current_tick, configuration)
                    .map(|(score, controller_id)| (Reverse(score), controller_id, peer_node_id))
            })
            .collect();
        ranked.sort();

        if ranked.len() < 2 {
            return Ok(None);
        }

        let members = ranked
            .into_iter()
            .take(self.membership_cap)
            .enumerate()
            .map(
                |(index, (_score, controller_id, node_id))| CommitteeMember {
                    node_id,
                    controller_id,
                    role: if index == 0 {
                        CommitteeRole::Participant
                    } else {
                        CommitteeRole::Witness
                    },
                },
            )
            .collect::<Vec<_>>();

        // Bounded by MESH_COMMITTEE_MEMBERSHIP_CAP above, so the cast is
        // infallible.
        let quorum_threshold = u8::try_from((members.len() / 2) + 1)
            .expect("committee size is bounded by MESH_COMMITTEE_MEMBERSHIP_CAP");
        let validity_end = Tick(current_tick.0 + MESH_COMMITTEE_VALIDITY_TICKS);
        let committee_id = committee_id_for(objective, configuration.epoch);

        Ok(Some(CommitteeSelection {
            committee_id,
            topology_epoch: configuration.epoch,
            selected_at_tick: current_tick,
            valid_for: TimeWindow::new(current_tick, validity_end)
                .expect("committee selection uses a positive validity window"),
            evidence_basis: FactBasis::Estimated,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold,
            members,
        }))
    }
}

// Maps observed environment conditions to the admission-assumption envelope
// the engine reports for every candidate. Churn thresholds drive the
// connectivity regime; repair policy drives claim strength.
#[must_use]
pub fn mesh_admission_assumptions(
    profile: &AdaptiveRoutingProfile,
    configuration: &Configuration,
) -> AdmissionAssumptions {
    let churn = configuration.environment.churn_permille.get();
    AdmissionAssumptions {
        message_flow_assumption: jacquard_core::MessageFlowAssumptionClass::PerRouteSequenced,
        failure_model: jacquard_core::FailureModelClass::Benign,
        runtime_envelope: jacquard_core::RuntimeEnvelopeClass::Canonical,
        node_density_class: density_class(configuration.environment.reachable_neighbor_count),
        connectivity_regime: if churn > CHURN_PARTITION_PRONE_PERMILLE {
            jacquard_core::ConnectivityRegime::PartitionProne
        } else if churn > CHURN_HIGH_CHURN_PERMILLE {
            jacquard_core::ConnectivityRegime::HighChurn
        } else {
            jacquard_core::ConnectivityRegime::Stable
        },
        adversary_regime: jacquard_core::AdversaryRegime::BenignUntrusted,
        claim_strength: if profile.selected_connectivity.repair == RouteRepairClass::Repairable {
            ClaimStrength::ExactUnderAssumptions
        } else {
            ClaimStrength::ConservativeUnderProfile
        },
    }
}

#[must_use]
pub fn mesh_route_connectivity(profile: &AdaptiveRoutingProfile) -> RouteConnectivityProfile {
    RouteConnectivityProfile {
        repair: profile.selected_connectivity.repair,
        partition: profile.selected_connectivity.partition,
    }
}

#[must_use]
pub fn mesh_health_score(configuration: &Configuration) -> HealthScore {
    let reachable = configuration
        .environment
        .reachable_neighbor_count
        .saturating_mul(100);
    let churn_penalty = u32::from(configuration.environment.churn_permille.get());
    let contention_penalty = u32::from(configuration.environment.contention_permille.get());
    HealthScore(reachable.saturating_sub((churn_penalty + contention_penalty) / 2))
}

fn density_class(neighbor_count: u32) -> NodeDensityClass {
    if neighbor_count >= DENSITY_DENSE_NEIGHBOR_MIN {
        NodeDensityClass::Dense
    } else if neighbor_count >= DENSITY_MODERATE_NEIGHBOR_MIN {
        NodeDensityClass::Moderate
    } else {
        NodeDensityClass::Sparse
    }
}

fn committee_id_for(objective: &RoutingObjective, epoch: RouteEpoch) -> CommitteeId {
    let mut bytes = [0_u8; 16];
    let seed = match &objective.destination {
        jacquard_core::DestinationId::Node(node_id) => node_id.0,
        jacquard_core::DestinationId::Service(service_id) => {
            let mut seed = [0_u8; 32];
            for (index, byte) in service_id.0.iter().take(32).enumerate() {
                seed[index] = *byte;
            }
            seed
        }
        jacquard_core::DestinationId::Gateway(gateway_id) => {
            let mut seed = [0_u8; 32];
            seed[..16].copy_from_slice(&gateway_id.0);
            seed
        }
    };
    bytes[..8].copy_from_slice(&seed[..8]);
    bytes[8..].copy_from_slice(&epoch.0.to_le_bytes());
    CommitteeId(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jacquard_core::{
        AdaptiveRoutingProfile, Belief, BleDeviceId, BleProfileId, ByteCount, Configuration,
        ContentId, ControllerId, DeploymentProfile, DestinationId, Environment, Estimate,
        FactSourceClass, HoldFallbackPolicy, Limit, Link, LinkEndpoint, LinkRuntimeState,
        LinkState, Node, NodeProfile, NodeRelayBudget, NodeState, Observation,
        OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteConnectivityProfile,
        RouteEpoch, RoutePartitionClass, RouteProtectionClass, RouteRepairClass,
        RouteReplacementPolicy, RouteServiceKind, RoutingEngineFallbackPolicy,
        RoutingEvidenceClass, RoutingObjective, ServiceDescriptor, ServiceId, ServiceScope, Tick,
        TimeWindow, TransportProtocol,
    };
    use std::collections::BTreeMap;

    fn ble_endpoint(byte: u8) -> LinkEndpoint {
        LinkEndpoint {
            protocol: TransportProtocol::BleGatt,
            address: jacquard_core::EndpointAddress::Ble {
                device_id: BleDeviceId(vec![byte]),
                profile_id: BleProfileId([byte; 16]),
            },
            mtu_bytes: ByteCount(256),
        }
    }

    fn route_capable_services(
        node_id: NodeId,
        controller_id: ControllerId,
    ) -> Vec<ServiceDescriptor> {
        let valid_for = TimeWindow::new(Tick(0), Tick(100)).unwrap();
        [
            RouteServiceKind::Discover,
            RouteServiceKind::Move,
            RouteServiceKind::Hold,
        ]
        .into_iter()
        .map(|kind| ServiceDescriptor {
            provider_node_id: node_id,
            controller_id,
            service_kind: kind,
            endpoints: vec![ble_endpoint(node_id.0[0])],
            routing_engines: vec![RoutingEngineId::Mesh],
            scope: ServiceScope::Discovery(jacquard_core::DiscoveryScopeId([7; 16])),
            valid_for,
            capacity: Belief::Absent,
        })
        .collect()
    }

    fn node(byte: u8) -> Node {
        let node_id = NodeId([byte; 32]);
        let controller_id = ControllerId([byte; 32]);
        Node {
            controller_id,
            profile: NodeProfile {
                services: route_capable_services(node_id, controller_id),
                endpoints: vec![ble_endpoint(byte)],
                connection_count_max: 4,
                neighbor_state_count_max: 4,
                simultaneous_transfer_count_max: 2,
                active_route_count_max: 2,
                relay_work_budget_max: 8,
                maintenance_work_budget_max: 8,
                hold_item_count_max: 4,
                hold_capacity_bytes_max: ByteCount(2048),
            },
            state: NodeState {
                relay_budget: Belief::Estimated(Estimate {
                    value: NodeRelayBudget {
                        relay_work_budget: Belief::Estimated(Estimate {
                            value: 4,
                            confidence_permille: RatioPermille(1000),
                            updated_at_tick: Tick(0),
                        }),
                        utilization_permille: RatioPermille(100),
                        retention_horizon_ms: Belief::Absent,
                    },
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: Tick(0),
                }),
                available_connection_count: Belief::Estimated(Estimate {
                    value: 4,
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: Tick(0),
                }),
                hold_capacity_available_bytes: Belief::Estimated(Estimate {
                    value: ByteCount(2048),
                    confidence_permille: RatioPermille(1000),
                    updated_at_tick: Tick(0),
                }),
                information_summary: Belief::Absent,
            },
        }
    }

    fn link(byte: u8) -> Link {
        Link {
            endpoint: ble_endpoint(byte),
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: jacquard_core::DurationMs(40),
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(20),
                delivery_confidence_permille: Belief::Estimated(Estimate {
                    value: RatioPermille(950),
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

    fn topology_with_neighbor_count(neighbor_count: u32) -> Observation<Configuration> {
        // Always builds a local node (id 1) connected to two route-capable
        // peers, then advertises an environment with `neighbor_count` so we
        // can probe the reachable_neighbor_count guard independently of the
        // adjacency map.
        let local = NodeId([1; 32]);
        let mut nodes = BTreeMap::from([(local, node(1)), (NodeId([2; 32]), node(2))]);
        nodes.insert(NodeId([3; 32]), node(3));
        let links = BTreeMap::from([
            ((local, NodeId([2; 32])), link(2)),
            ((local, NodeId([3; 32])), link(3)),
        ]);
        Observation {
            value: Configuration {
                epoch: RouteEpoch(0),
                nodes,
                links,
                environment: Environment {
                    reachable_neighbor_count: neighbor_count,
                    churn_permille: RatioPermille(100),
                    contention_permille: RatioPermille(100),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(0),
        }
    }

    fn objective_for_service(bytes: Vec<u8>) -> RoutingObjective {
        RoutingObjective {
            destination: DestinationId::Service(ServiceId(bytes)),
            service_kind: RouteServiceKind::Move,
            target_protection: RouteProtectionClass::LinkProtected,
            protection_floor: RouteProtectionClass::LinkProtected,
            target_connectivity: RouteConnectivityProfile {
                repair: RouteRepairClass::Repairable,
                partition: RoutePartitionClass::PartitionTolerant,
            },
            hold_fallback_policy: HoldFallbackPolicy::Allowed,
            latency_budget_ms: Limit::Unbounded,
            protection_priority: PriorityPoints(0),
            connectivity_priority: PriorityPoints(0),
        }
    }

    fn profile_with(
        repair: RouteRepairClass,
        partition: RoutePartitionClass,
    ) -> AdaptiveRoutingProfile {
        AdaptiveRoutingProfile {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: RouteConnectivityProfile { repair, partition },
            deployment_profile: DeploymentProfile::FieldPartitionTolerant,
            diversity_floor: 1,
            routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
            route_replacement_policy: RouteReplacementPolicy::Allowed,
        }
    }

    // The selector returns Some only when the profile actually requires the
    // coordinated capabilities a committee provides. A best-effort repair
    // profile must return None even on a healthy neighborhood.
    #[test]
    fn select_committee_returns_none_when_profile_does_not_require_repair() {
        let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
        let topology = topology_with_neighbor_count(3);
        let objective = objective_for_service(vec![1, 2]);
        let profile = profile_with(
            RouteRepairClass::BestEffort,
            RoutePartitionClass::PartitionTolerant,
        );

        let result = selector
            .select_committee(&objective, &profile, &topology)
            .unwrap();
        assert!(result.is_none());
    }

    // Same property for the partition axis: a connected-only profile has no
    // need for committee-coordinated partition tolerance.
    #[test]
    fn select_committee_returns_none_when_profile_does_not_require_partition_tolerance() {
        let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
        let topology = topology_with_neighbor_count(3);
        let objective = objective_for_service(vec![1, 2]);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::ConnectedOnly,
        );

        let result = selector
            .select_committee(&objective, &profile, &topology)
            .unwrap();
        assert!(result.is_none());
    }

    // Below the minimum neighbor count the selector should bail out, even
    // when the profile is asking for a committee.
    #[test]
    fn select_committee_returns_none_when_neighborhood_too_small() {
        let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
        let topology = topology_with_neighbor_count(1);
        let objective = objective_for_service(vec![1, 2]);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );

        let result = selector
            .select_committee(&objective, &profile, &topology)
            .unwrap();
        assert!(result.is_none());
    }

    // When all guards pass and the neighborhood has multiple route-capable
    // peers, the selector returns Some with a non-empty member list.
    #[test]
    fn select_committee_returns_some_when_all_conditions_met() {
        let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
        let topology = topology_with_neighbor_count(3);
        let objective = objective_for_service(vec![1, 2]);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );

        let result = selector
            .select_committee(&objective, &profile, &topology)
            .unwrap();
        let committee = result.expect("committee should be selected");
        assert!(!committee.members.is_empty());
        assert!(committee.quorum_threshold >= 1);
    }

    // ContentId is unused here but included for parity with the broader
    // mesh tests so the import block stays consistent.
    #[allow(dead_code)]
    fn _content_id_marker(_: ContentId<jacquard_core::Blake3Digest>) {}
}
