//! Optional committee coordination for mesh.
//!
//! `NoCommitteeSelector` is the always-`None` default.
//! `DeterministicCommitteeSelector` is the in-tree implementation used when an
//! engine opts in. Committee output is advisory evidence only: canonical route
//! admission, witnesses, and lease ownership still flow through the shared
//! router boundary. This module also exposes the helpers that derive mesh
//! admission assumptions and health scores from the shared `Configuration`.

use core::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use bincode::Options;
use jacquard_core::{
    SelectedRoutingParameters, AdmissionAssumptions, ClaimStrength, CommitteeId,
    CommitteeMember, CommitteeRole, CommitteeSelection, Configuration, ControllerId,
    FactBasis, HealthScore, IdentityAssuranceClass, NodeDensityClass, NodeId,
    Observation, PenaltyPoints, ConnectivityPosture, RouteEpoch, RouteError,
    RoutePartitionClass, RouteRepairClass, RoutingEngineId, RoutingObjective,
    ServiceScope, Tick, TimeWindow,
};
use jacquard_traits::{
    Blake3Hashing, CommitteeSelector, HashDigestBytes, Hashing,
    MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess, MeshTopologyModel,
};

use crate::topology::{
    adjacent_node_ids, bounded_health_score, optional_health_score_value,
    route_capable_for_engine, DeterministicMeshTopologyModel,
};

/// Default maximum committee size for `DeterministicCommitteeSelector`.
pub const MESH_COMMITTEE_MEMBERSHIP_CAP: u8 = 3;

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
const MESH_COMMITTEE_BEHAVIOR_RELIABILITY_FLOOR: u32 = 400;
const MESH_COMMITTEE_BEHAVIOR_PENALTY_CEILING: u32 = 400;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshBehaviorHistory {
    pub reliability_score: HealthScore,
    pub misbehavior_penalty_points: PenaltyPoints,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CommitteeDiversityKey {
    discovery_scope: Option<jacquard_core::DiscoveryScopeId>,
}

#[derive(Clone, Debug)]
struct CommitteeCandidate {
    score: u32,
    controller_id: ControllerId,
    node_id: NodeId,
    diversity_key: CommitteeDiversityKey,
}

#[derive(Clone, Debug)]
pub struct NoCommitteeSelector;

impl CommitteeSelector for NoCommitteeSelector {
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        _objective: &RoutingObjective,
        _profile: &SelectedRoutingParameters,
        _topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError> {
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicCommitteeSelector<
    Topology = DeterministicMeshTopologyModel,
    Hasher = Blake3Hashing,
> {
    pub local_node_id: NodeId,
    pub engine_id: RoutingEngineId,
    pub membership_cap: u8,
    pub topology_model: Topology,
    pub hashing: Hasher,
    pub behavior_history: BTreeMap<NodeId, MeshBehaviorHistory>,
}

impl DeterministicCommitteeSelector<DeterministicMeshTopologyModel, Blake3Hashing> {
    #[must_use]
    pub fn new(local_node_id: NodeId) -> Self {
        Self::with_topology_model(local_node_id, DeterministicMeshTopologyModel::new())
    }
}

impl<Topology> DeterministicCommitteeSelector<Topology, Blake3Hashing> {
    #[must_use]
    pub fn with_topology_model(
        local_node_id: NodeId,
        topology_model: Topology,
    ) -> Self {
        Self {
            local_node_id,
            engine_id: RoutingEngineId::Mesh,
            membership_cap: MESH_COMMITTEE_MEMBERSHIP_CAP,
            topology_model,
            hashing: Blake3Hashing,
            behavior_history: BTreeMap::new(),
        }
    }
}

impl<Topology, Hasher> DeterministicCommitteeSelector<Topology, Hasher> {
    #[must_use]
    pub fn with_behavior_history(
        mut self,
        behavior_history: BTreeMap<NodeId, MeshBehaviorHistory>,
    ) -> Self {
        self.behavior_history = behavior_history;
        self
    }

    #[must_use]
    pub fn with_hashing<NextHasher>(
        self,
        hashing: NextHasher,
    ) -> DeterministicCommitteeSelector<Topology, NextHasher> {
        DeterministicCommitteeSelector {
            local_node_id: self.local_node_id,
            engine_id: self.engine_id,
            membership_cap: self.membership_cap,
            topology_model: self.topology_model,
            hashing,
            behavior_history: self.behavior_history,
        }
    }
}

impl<Topology, Hasher> DeterministicCommitteeSelector<Topology, Hasher>
where
    Topology: MeshTopologyModel,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
{
    fn discovery_scope_key(
        &self,
        peer_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<CommitteeDiversityKey> {
        let node = configuration.nodes.get(peer_node_id)?;
        let scope = node.profile.services.iter().find_map(|service| {
            if service.routing_engines.contains(&self.engine_id) {
                match service.scope {
                    | ServiceScope::Discovery(scope) => Some(scope),
                    | _ => None,
                }
            } else {
                None
            }
        });
        Some(CommitteeDiversityKey { discovery_scope: scope })
    }

    fn committee_eligible(
        &self,
        peer_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> bool {
        let Some(node) = configuration.nodes.get(peer_node_id) else {
            return false;
        };
        if !route_capable_for_engine(node, &self.engine_id, observed_at_tick) {
            return false;
        }
        let Some(estimate) = self.topology_model.peer_estimate(
            &self.local_node_id,
            peer_node_id,
            observed_at_tick,
            configuration,
        ) else {
            return false;
        };
        if optional_health_score_value(estimate.service_score()) == 0 {
            return false;
        }
        self.behavior_history
            .get(peer_node_id)
            .is_none_or(|history| {
                history.reliability_score.0 >= MESH_COMMITTEE_BEHAVIOR_RELIABILITY_FLOOR
                    && history.misbehavior_penalty_points.0
                        <= MESH_COMMITTEE_BEHAVIOR_PENALTY_CEILING
            })
    }

    fn membership_candidate(
        &self,
        peer_node_id: &NodeId,
        observed_at_tick: Tick,
        configuration: &Configuration,
    ) -> Option<CommitteeCandidate> {
        let node = configuration.nodes.get(peer_node_id)?;
        let estimate = self.topology_model.peer_estimate(
            &self.local_node_id,
            peer_node_id,
            observed_at_tick,
            configuration,
        )?;
        let relay_score = optional_health_score_value(estimate.relay_value_score());
        let retention_score =
            optional_health_score_value(estimate.retention_value_score());
        let stability_score = optional_health_score_value(estimate.stability_score());
        let service_score = optional_health_score_value(estimate.service_score());
        let behavior_entry = self.behavior_history.get(peer_node_id);
        let behavior_bonus = behavior_entry
            .map(|history| history.reliability_score.0 / 2)
            .unwrap_or(0);
        let behavior_penalty = behavior_entry
            .map(|history| history.misbehavior_penalty_points.0)
            .unwrap_or(0);
        // Service score is weighted by `MESH_COMMITTEE_SERVICE_WEIGHT` so a
        // peer without the required routing services can never outrank a
        // peer that has them, regardless of relay or link quality.
        Some(CommitteeCandidate {
            score: relay_score
                .saturating_add(retention_score)
                .saturating_add(stability_score)
                .saturating_add(
                    service_score.saturating_mul(MESH_COMMITTEE_SERVICE_WEIGHT),
                )
                .saturating_add(behavior_bonus)
                .saturating_sub(behavior_penalty),
            controller_id: node.controller_id,
            node_id: *peer_node_id,
            diversity_key: self.discovery_scope_key(peer_node_id, configuration)?,
        })
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
        let density_score = optional_health_score_value(estimate.density_score());
        let service_stability =
            optional_health_score_value(estimate.service_stability_score());
        let partition_risk =
            optional_health_score_value(estimate.partition_risk_score());
        density_score > 0
            && service_stability >= MESH_COMMITTEE_SERVICE_STABILITY_FLOOR
            && service_stability >= partition_risk
    }
}

impl<Topology, Hasher> CommitteeSelector
    for DeterministicCommitteeSelector<Topology, Hasher>
where
    Topology: MeshTopologyModel,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Hasher: Hashing,
    Hasher::Digest: HashDigestBytes,
{
    type TopologyView = Configuration;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, RouteError> {
        let configuration = &topology.value;
        let current_tick = topology.observed_at_tick;
        if !self.should_coordinate(profile, current_tick, configuration) {
            return Ok(None);
        }

        let ranked = self.ranked_candidates(current_tick, configuration);
        if ranked.len() < 2 {
            return Ok(None);
        }
        let members = self.select_diverse_members(ranked);
        if members.len() < 2 {
            return Ok(None);
        }
        Ok(Some(self.committee_selection(
            objective,
            configuration,
            current_tick,
            members,
        )))
    }
}

impl<Topology, Hasher> DeterministicCommitteeSelector<Topology, Hasher>
where
    Topology: MeshTopologyModel,
    Topology::PeerEstimate: MeshPeerEstimateAccess,
    Topology::NeighborhoodEstimate: MeshNeighborhoodEstimateAccess,
    Hasher: Hashing,
    Hasher::Digest: HashDigestBytes,
{
    fn should_coordinate(
        &self,
        profile: &SelectedRoutingParameters,
        current_tick: Tick,
        configuration: &Configuration,
    ) -> bool {
        profile.selected_connectivity.repair == RouteRepairClass::Repairable
            && matches!(
                profile.selected_connectivity.partition,
                RoutePartitionClass::PartitionTolerant
            )
            && configuration.environment.reachable_neighbor_count
                >= MESH_COMMITTEE_MIN_NEIGHBOR_COUNT
            && self.neighborhood_allows_coordination(current_tick, configuration)
    }

    fn ranked_candidates(
        &self,
        current_tick: Tick,
        configuration: &Configuration,
    ) -> Vec<(Reverse<u32>, ControllerId, CommitteeDiversityKey, NodeId)> {
        let mut ranked: Vec<_> = adjacent_node_ids(&self.local_node_id, configuration)
            .into_iter()
            .filter(|peer_node_id| {
                self.committee_eligible(peer_node_id, current_tick, configuration)
            })
            .filter_map(|peer_node_id| {
                self.membership_candidate(&peer_node_id, current_tick, configuration)
                    .map(|candidate| {
                        (
                            Reverse(candidate.score),
                            candidate.controller_id,
                            candidate.diversity_key.clone(),
                            candidate.node_id,
                        )
                    })
            })
            .collect();
        ranked.sort();
        ranked
    }

    // First pass: prefer cross-scope and cross-controller candidates.
    // Second pass: if a quorum-capable committee (>=2) was not formed,
    // relax the diversity constraint so coordination is possible at all.
    fn select_diverse_members(
        &self,
        ranked: Vec<(Reverse<u32>, ControllerId, CommitteeDiversityKey, NodeId)>,
    ) -> Vec<CommitteeMember> {
        let mut seen_controllers = BTreeSet::new();
        let mut seen_diversity_keys = BTreeSet::new();
        let mut members = Vec::new();
        let mut deferred_candidates = Vec::new();

        for (_score, controller_id, diversity_key, node_id) in ranked {
            if seen_controllers.contains(&controller_id) {
                continue;
            }
            if seen_diversity_keys.contains(&diversity_key) {
                deferred_candidates.push((controller_id, diversity_key, node_id));
                continue;
            }
            Self::push_committee_member(
                &mut members,
                &mut seen_controllers,
                &mut seen_diversity_keys,
                controller_id,
                diversity_key,
                node_id,
            );
            if members.len() >= usize::from(self.membership_cap) {
                return members;
            }
        }

        if members.len() < 2 {
            for (controller_id, diversity_key, node_id) in deferred_candidates {
                if seen_controllers.contains(&controller_id) {
                    continue;
                }
                Self::push_committee_member(
                    &mut members,
                    &mut seen_controllers,
                    &mut seen_diversity_keys,
                    controller_id,
                    diversity_key,
                    node_id,
                );
                if members.len() >= 2 {
                    break;
                }
            }
        }

        members
    }

    fn push_committee_member(
        members: &mut Vec<CommitteeMember>,
        seen_controllers: &mut BTreeSet<ControllerId>,
        seen_diversity_keys: &mut BTreeSet<CommitteeDiversityKey>,
        controller_id: ControllerId,
        diversity_key: CommitteeDiversityKey,
        node_id: NodeId,
    ) {
        seen_controllers.insert(controller_id);
        seen_diversity_keys.insert(diversity_key);
        members.push(CommitteeMember {
            node_id,
            controller_id,
            role: if members.is_empty() {
                CommitteeRole::Participant
            } else {
                CommitteeRole::Witness
            },
        });
    }

    fn committee_selection(
        &self,
        objective: &RoutingObjective,
        configuration: &Configuration,
        current_tick: Tick,
        members: Vec<CommitteeMember>,
    ) -> CommitteeSelection {
        // Simple majority quorum: floor(n/2) + 1. The u8 cast is safe
        // because MESH_COMMITTEE_MEMBERSHIP_CAP (3) keeps the max at 2.
        let quorum_threshold = u8::try_from((members.len() / 2) + 1)
            .expect("committee size is bounded by MESH_COMMITTEE_MEMBERSHIP_CAP");
        let validity_end =
            Tick(current_tick.0.saturating_add(MESH_COMMITTEE_VALIDITY_TICKS));
        CommitteeSelection {
            committee_id: self.committee_id_for(objective, configuration.epoch),
            topology_epoch: configuration.epoch,
            selected_at_tick: current_tick,
            valid_for: TimeWindow::new(current_tick, validity_end)
                .expect("committee selection uses a positive validity window"),
            evidence_basis: FactBasis::Estimated,
            claim_strength: ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold,
            members,
        }
    }
}

// Maps observed environment conditions to the admission-assumption envelope
// the engine reports for every candidate. Churn thresholds drive the
// connectivity regime; repair policy drives claim strength.
#[must_use]
pub fn mesh_admission_assumptions(
    profile: &SelectedRoutingParameters,
    configuration: &Configuration,
) -> AdmissionAssumptions {
    let churn = configuration.environment.churn_permille.get();
    AdmissionAssumptions {
        message_flow_assumption:
            jacquard_core::MessageFlowAssumptionClass::PerRouteSequenced,
        failure_model: jacquard_core::FailureModelClass::Benign,
        runtime_envelope: jacquard_core::RuntimeEnvelopeClass::Canonical,
        node_density_class: density_class(
            configuration.environment.reachable_neighbor_count,
        ),
        connectivity_regime: connectivity_regime(churn),
        adversary_regime: jacquard_core::AdversaryRegime::BenignUntrusted,
        claim_strength: if profile.selected_connectivity.repair
            == RouteRepairClass::Repairable
        {
            ClaimStrength::ExactUnderAssumptions
        } else {
            ClaimStrength::ConservativeUnderProfile
        },
    }
}

#[must_use]
pub fn mesh_route_connectivity(
    profile: &SelectedRoutingParameters,
) -> ConnectivityPosture {
    ConnectivityPosture {
        repair: profile.selected_connectivity.repair,
        partition: profile.selected_connectivity.partition,
    }
}

#[must_use]
pub fn mesh_health_score(configuration: &Configuration) -> HealthScore {
    // Mesh health stays on the shared 0..=HEALTH_SCORE_MAX scale. The score
    // rewards both usable neighbor density and environmental stability instead
    // of letting either dominate alone:
    // - density_score measures how close the local neighborhood is to a dense mesh
    //   regime, saturating once dense membership is reached.
    // - stability_score measures how calm the environment is after averaging churn
    //   and contention penalties.
    // The final score is the midpoint of the two so sparse-but-calm and
    // dense-but-chaotic regimes both land below a balanced middle regime.
    let density_score = if DENSITY_DENSE_NEIGHBOR_MIN == 0 {
        bounded_health_score(0)
    } else {
        let capped_neighbors = configuration
            .environment
            .reachable_neighbor_count
            .min(DENSITY_DENSE_NEIGHBOR_MIN);
        bounded_health_score(
            capped_neighbors.saturating_mul(1000) / DENSITY_DENSE_NEIGHBOR_MIN,
        )
    };
    let churn_penalty = u32::from(configuration.environment.churn_permille.get());
    let contention_penalty =
        u32::from(configuration.environment.contention_permille.get());
    let stability_score = bounded_health_score(
        1000_u32.saturating_sub((churn_penalty + contention_penalty) / 2),
    );
    bounded_health_score((density_score.0 + stability_score.0) / 2)
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

fn connectivity_regime(churn_permille: u16) -> jacquard_core::ConnectivityRegime {
    if churn_permille > CHURN_PARTITION_PRONE_PERMILLE {
        jacquard_core::ConnectivityRegime::PartitionProne
    } else if churn_permille > CHURN_HIGH_CHURN_PERMILLE {
        jacquard_core::ConnectivityRegime::HighChurn
    } else {
        jacquard_core::ConnectivityRegime::Stable
    }
}

impl<Topology, Hasher> DeterministicCommitteeSelector<Topology, Hasher>
where
    Hasher: Hashing,
    Hasher::Digest: HashDigestBytes,
{
    fn committee_id_for(
        &self,
        objective: &RoutingObjective,
        epoch: RouteEpoch,
    ) -> CommitteeId {
        #[derive(serde::Serialize)]
        struct CommitteeSeed<'a> {
            destination: &'a jacquard_core::DestinationId,
            epoch: RouteEpoch,
        }

        let payload = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .serialize(&CommitteeSeed {
                destination: &objective.destination,
                epoch,
            })
            .expect("committee seed is always serializable");
        let digest = self
            .hashing
            .hash_tagged(crate::engine::DOMAIN_TAG_COMMITTEE_ID, &payload);
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest.as_bytes()[..16]);
        CommitteeId(bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        SelectedRoutingParameters, Belief, BleDeviceId, BleProfileId, ByteCount,
        Configuration, ControllerId, OperatingMode, DestinationId, Environment,
        Estimate, FactSourceClass, HoldFallbackPolicy, Limit, Link, LinkEndpoint,
        LinkRuntimeState, LinkState, Node, NodeProfile, NodeRelayBudget, NodeState,
        Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille,
        ConnectivityPosture, RouteEpoch, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
        RouteServiceKind, RoutingEngineFallbackPolicy, RoutingEvidenceClass,
        RoutingObjective, ServiceDescriptor, ServiceId, ServiceScope, Tick, TimeWindow,
        TransportProtocol,
    };

    use super::*;
    use crate::HEALTH_SCORE_MAX;

    // These fixture builders intentionally stay local to the committee unit
    // tests. They pin exact controller/service layouts for committee scoring
    // and diversity behavior, while the integration fixtures are broader
    // end-to-end network shapes owned by `tests/common`.
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
        [RouteServiceKind::Discover, RouteServiceKind::Move, RouteServiceKind::Hold]
            .into_iter()
            .map(|kind| ServiceDescriptor {
                provider_node_id: node_id,
                controller_id,
                service_kind: kind,
                endpoints: vec![ble_endpoint(node_id.0[0])],
                routing_engines: vec![RoutingEngineId::Mesh],
                scope: ServiceScope::Discovery(jacquard_core::DiscoveryScopeId(
                    [7; 16],
                )),
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
            target_connectivity: ConnectivityPosture {
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
    ) -> SelectedRoutingParameters {
        SelectedRoutingParameters {
            selected_protection: RouteProtectionClass::LinkProtected,
            selected_connectivity: ConnectivityPosture { repair, partition },
            deployment_profile: OperatingMode::FieldPartitionTolerant,
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
    fn select_committee_returns_none_when_profile_does_not_require_partition_tolerance()
    {
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

    #[test]
    fn mesh_health_score_is_clamped_to_health_score_max() {
        let topology = Observation {
            value: Configuration {
                epoch: RouteEpoch(0),
                nodes: BTreeMap::from([(NodeId([1; 32]), node(1))]),
                links: BTreeMap::new(),
                environment: Environment {
                    reachable_neighbor_count: 99,
                    churn_permille: RatioPermille(0),
                    contention_permille: RatioPermille(0),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(0),
        };

        assert_eq!(
            mesh_health_score(&topology.value),
            HealthScore(HEALTH_SCORE_MAX)
        );
    }

    #[test]
    fn mesh_health_score_prefers_balanced_middle_regime_over_extremes() {
        let sparse_calm = topology_with_neighbor_count(1);
        let mut dense_chaotic = topology_with_neighbor_count(8);
        dense_chaotic.value.environment.churn_permille = RatioPermille(900);
        dense_chaotic.value.environment.contention_permille = RatioPermille(900);
        let mut balanced = topology_with_neighbor_count(4);
        balanced.value.environment.churn_permille = RatioPermille(350);
        balanced.value.environment.contention_permille = RatioPermille(350);

        let sparse_score = mesh_health_score(&sparse_calm.value);
        let dense_chaotic_score = mesh_health_score(&dense_chaotic.value);
        let balanced_score = mesh_health_score(&balanced.value);

        assert!(balanced_score.0 > sparse_score.0);
        assert!(balanced_score.0 > dense_chaotic_score.0);
    }

    #[test]
    fn connectivity_regime_thresholds_are_explicit() {
        assert_eq!(
            connectivity_regime(CHURN_HIGH_CHURN_PERMILLE),
            jacquard_core::ConnectivityRegime::Stable
        );
        assert_eq!(
            connectivity_regime(CHURN_HIGH_CHURN_PERMILLE + 1),
            jacquard_core::ConnectivityRegime::HighChurn
        );
        assert_eq!(
            connectivity_regime(CHURN_PARTITION_PRONE_PERMILLE),
            jacquard_core::ConnectivityRegime::HighChurn
        );
        assert_eq!(
            connectivity_regime(CHURN_PARTITION_PRONE_PERMILLE + 1),
            jacquard_core::ConnectivityRegime::PartitionProne
        );
    }

    #[test]
    fn density_class_thresholds_are_explicit() {
        assert_eq!(
            density_class(DENSITY_MODERATE_NEIGHBOR_MIN.saturating_sub(1)),
            NodeDensityClass::Sparse
        );
        assert_eq!(
            density_class(DENSITY_MODERATE_NEIGHBOR_MIN),
            NodeDensityClass::Moderate
        );
        assert_eq!(
            density_class(DENSITY_DENSE_NEIGHBOR_MIN.saturating_sub(1)),
            NodeDensityClass::Moderate
        );
        assert_eq!(
            density_class(DENSITY_DENSE_NEIGHBOR_MIN),
            NodeDensityClass::Dense
        );
    }
}
