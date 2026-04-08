//! Committee-selection control flow.
//!
//! This module owns the optional coordination protocol that turns a topology
//! observation into an advisory committee selection. The flow is:
//! 1. gate on the requested connectivity profile and neighborhood stability
//! 2. rank eligible peers from mesh-private estimates and behavior history
//! 3. prefer controller- and scope-diverse members
//! 4. emit an advisory `CommitteeSelection` with a bounded validity window

use core::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use bincode::Options;
use jacquard_core::{
    CommitteeId, CommitteeMember, CommitteeRole, CommitteeSelection, Configuration,
    ControllerId, FactBasis, IdentityAssuranceClass, NodeId, Observation,
    QuorumThreshold, RouteEpoch, RouteError, RoutePartitionClass, RouteRepairClass,
    RoutingEngineId, RoutingObjective, SelectedRoutingParameters, ServiceScope, Tick,
    TimeWindow,
};
use jacquard_traits::{Blake3Hashing, CommitteeSelector, HashDigestBytes, Hashing};

use crate::{
    committee::{
        MeshBehaviorHistory, MESH_COMMITTEE_BEHAVIOR_PENALTY_CEILING,
        MESH_COMMITTEE_BEHAVIOR_RELIABILITY_FLOOR, MESH_COMMITTEE_MEMBERSHIP_CAP,
        MESH_COMMITTEE_MIN_NEIGHBOR_COUNT, MESH_COMMITTEE_SERVICE_STABILITY_FLOOR,
        MESH_COMMITTEE_SERVICE_WEIGHT, MESH_COMMITTEE_VALIDITY_TICKS,
    },
    topology::{
        adjacent_node_ids, optional_health_score_value, route_capable_for_engine,
        DeterministicMeshTopologyModel,
    },
    MeshNeighborhoodEstimateAccess, MeshPeerEstimateAccess, MeshTopologyModel,
};

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
    behavior_history: BTreeMap<NodeId, MeshBehaviorHistory>,
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
            engine_id: crate::MESH_ENGINE_ID,
            membership_cap: MESH_COMMITTEE_MEMBERSHIP_CAP,
            topology_model,
            hashing: Blake3Hashing,
            behavior_history: BTreeMap::new(),
        }
    }
}

impl<Topology, Hasher> DeterministicCommitteeSelector<Topology, Hasher> {
    #[must_use]
    #[cfg(test)]
    pub(crate) fn with_behavior_history(
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
        let quorum_threshold = QuorumThreshold(
            u8::try_from((members.len() / 2) + 1)
                .expect("committee size is bounded by MESH_COMMITTEE_MEMBERSHIP_CAP"),
        );
        let validity_end =
            Tick(current_tick.0.saturating_add(MESH_COMMITTEE_VALIDITY_TICKS));
        CommitteeSelection {
            committee_id: self.committee_id_for(objective, configuration.epoch),
            topology_epoch: configuration.epoch,
            selected_at_tick: current_tick,
            valid_for: TimeWindow::new(current_tick, validity_end)
                .expect("committee selection uses a positive validity window"),
            evidence_basis: FactBasis::Estimated,
            claim_strength: jacquard_core::ClaimStrength::ConservativeUnderProfile,
            identity_assurance: IdentityAssuranceClass::ControllerBound,
            quorum_threshold,
            members,
        }
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
        CommitteeId(crate::engine::digest_prefix::<16>(digest.as_bytes()))
    }
}
