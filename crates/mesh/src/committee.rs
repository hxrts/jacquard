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
use jacquard_traits::CommitteeSelector;

use crate::topology::{
    adjacent_link_between, adjacent_node_ids, route_capable_for_engine, service_surface_score,
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
pub struct DeterministicCommitteeSelector {
    pub local_node_id: NodeId,
    pub engine_id: RoutingEngineId,
    pub membership_cap: usize,
}

impl DeterministicCommitteeSelector {
    #[must_use]
    pub fn new(local_node_id: NodeId) -> Self {
        Self {
            local_node_id,
            engine_id: RoutingEngineId::Mesh,
            membership_cap: MESH_COMMITTEE_MEMBERSHIP_CAP,
        }
    }

    fn membership_score(
        &self,
        peer_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Option<(u32, ControllerId)> {
        let node = configuration.nodes.get(peer_node_id)?;
        let link = adjacent_link_between(&self.local_node_id, peer_node_id, configuration)?;
        let relay_score = match &node.state.relay_budget {
            jacquard_core::Belief::Absent => 0,
            jacquard_core::Belief::Estimated(estimate) => {
                1000_u32.saturating_sub(u32::from(estimate.value.utilization_permille.get()))
            }
        };
        let stability_score = u32::from(
            link.state
                .delivery_confidence_permille
                .into_estimate()
                .map_or(jacquard_core::RatioPermille(0), |estimate| estimate.value)
                .get(),
        ) + u32::from(
            link.state
                .symmetry_permille
                .into_estimate()
                .map_or(jacquard_core::RatioPermille(0), |estimate| estimate.value)
                .get(),
        );
        let service_score =
            service_surface_score(&node.profile.services, &self.engine_id, configuration.epoch);
        // Service score is weighted by `MESH_COMMITTEE_SERVICE_WEIGHT` so a
        // peer without the required routing services can never outrank a
        // peer that has them, regardless of relay or link quality.
        Some((
            relay_score
                + stability_score
                + service_score.saturating_mul(MESH_COMMITTEE_SERVICE_WEIGHT),
            node.controller_id,
        ))
    }
}

impl CommitteeSelector for DeterministicCommitteeSelector {
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
                >= MESH_COMMITTEE_MIN_NEIGHBOR_COUNT;
        if !should_coordinate {
            return Ok(None);
        }

        let mut ranked: Vec<_> = adjacent_node_ids(&self.local_node_id, configuration)
            .into_iter()
            .filter(|peer_node_id| {
                configuration.nodes.get(peer_node_id).is_some_and(|node| {
                    route_capable_for_engine(node, &self.engine_id, configuration.epoch)
                })
            })
            .filter_map(|peer_node_id| {
                self.membership_score(&peer_node_id, configuration)
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

trait BeliefExt<T> {
    fn into_estimate(self) -> Option<jacquard_core::Estimate<T>>;
}

impl<T> BeliefExt<T> for jacquard_core::Belief<T> {
    fn into_estimate(self) -> Option<jacquard_core::Estimate<T>> {
        match self {
            jacquard_core::Belief::Absent => None,
            jacquard_core::Belief::Estimated(estimate) => Some(estimate),
        }
    }
}
