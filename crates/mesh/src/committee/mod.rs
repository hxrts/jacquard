//! Optional committee coordination for mesh.
//!
//! `NoCommitteeSelector` is the always-`None` default.
//! `DeterministicCommitteeSelector` is the in-tree implementation used when an
//! engine opts in. Committee output is advisory evidence only: canonical route
//! admission, witnesses, and lease ownership still flow through the shared
//! router boundary. This module also exposes the helpers that derive mesh
//! admission assumptions and health scores from the shared `Configuration`.

mod selection;

#[allow(unused_imports)]
use jacquard_core::HoldItemCount;
#[allow(unused_imports)]
use jacquard_core::{
    AdmissionAssumptions, ClaimStrength, Configuration, ConnectivityPosture,
    DiversityFloor, HealthScore, MaintenanceWorkBudget, NodeDensityClass, NodeId,
    Observation, PenaltyPoints, RelayWorkBudget, RoutePartitionClass, RouteRepairClass,
    RoutingObjective, SelectedRoutingParameters, Tick,
};
pub use selection::{DeterministicCommitteeSelector, NoCommitteeSelector};

use crate::topology::bounded_health_score;

/// Default maximum committee size for `DeterministicCommitteeSelector`.
pub(crate) const MESH_COMMITTEE_MEMBERSHIP_CAP: u8 = 3;

/// Committee validity window, measured in ticks.
pub(crate) const MESH_COMMITTEE_VALIDITY_TICKS: u64 = 8;

/// Minimum reachable neighbor count required before forming a committee.
/// Below this, mesh does not attempt local coordination.
pub(crate) const MESH_COMMITTEE_MIN_NEIGHBOR_COUNT: u32 = 2;

/// Churn threshold (permille) at which the admission assumptions flip to
/// `PartitionProne`.
pub(crate) const CHURN_PARTITION_PRONE_PERMILLE: u16 = 600;

/// Churn threshold (permille) at which the admission assumptions flip to
/// `HighChurn`.
pub(crate) const CHURN_HIGH_CHURN_PERMILLE: u16 = 250;

/// Neighbor count at or above which a node density is classified `Dense`.
pub(crate) const DENSITY_DENSE_NEIGHBOR_MIN: u32 = 8;

/// Neighbor count at or above which a node density is classified `Moderate`.
pub(crate) const DENSITY_MODERATE_NEIGHBOR_MIN: u32 = 3;

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
pub(crate) struct MeshBehaviorHistory {
    pub reliability_score: HealthScore,
    pub misbehavior_penalty_points: PenaltyPoints,
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
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_core::{
        Belief, BleDeviceId, BleProfileId, ByteCount, Configuration,
        ConnectivityPosture, ControllerId, DestinationId, Environment, Estimate,
        FactSourceClass, HoldFallbackPolicy, Limit, Link, LinkEndpoint,
        LinkRuntimeState, LinkState, Node, NodeProfile, NodeRelayBudget, NodeState,
        Observation, OperatingMode, OriginAuthenticationClass, PriorityPoints,
        QuorumThreshold, RatioPermille, RouteEpoch, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
        RouteServiceKind, RoutingEngineFallbackPolicy, RoutingEvidenceClass,
        RoutingObjective, SelectedRoutingParameters, ServiceDescriptor, ServiceId,
        ServiceScope, Tick, TimeWindow, TransportProtocol,
    };
    use jacquard_traits::CommitteeSelector;

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
                routing_engines: vec![crate::MESH_ENGINE_ID],
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
                relay_work_budget_max: RelayWorkBudget(8),
                maintenance_work_budget_max: MaintenanceWorkBudget(8),
                hold_item_count_max: HoldItemCount(4),
                hold_capacity_bytes_max: ByteCount(2048),
            },
            state: NodeState {
                relay_budget: Belief::Estimated(Estimate {
                    value: NodeRelayBudget {
                        relay_work_budget: Belief::Estimated(Estimate {
                            value: RelayWorkBudget(4),
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

    fn node_with_identity_and_scope(
        node_byte: u8,
        controller_id: ControllerId,
        discovery_scope: jacquard_core::DiscoveryScopeId,
    ) -> Node {
        let mut node = node(node_byte);
        node.controller_id = controller_id;
        for service in &mut node.profile.services {
            service.controller_id = controller_id;
            service.scope = ServiceScope::Discovery(discovery_scope);
        }
        node
    }

    // long-block-exception: dense committee-diversity fixture kept inline for
    // readability.
    fn diversity_topology() -> Observation<Configuration> {
        let local = NodeId([1; 32]);
        let node_two = NodeId([2; 32]);
        let node_four = NodeId([4; 32]);
        let node_five = NodeId([5; 32]);
        let node_six = NodeId([6; 32]);

        Observation {
            value: Configuration {
                epoch: RouteEpoch(2),
                nodes: BTreeMap::from([
                    (local, node(1)),
                    (
                        node_two,
                        node_with_identity_and_scope(
                            2,
                            ControllerId([2; 32]),
                            jacquard_core::DiscoveryScopeId([2; 16]),
                        ),
                    ),
                    (
                        node_four,
                        node_with_identity_and_scope(
                            4,
                            ControllerId([2; 32]),
                            jacquard_core::DiscoveryScopeId([4; 16]),
                        ),
                    ),
                    (
                        node_five,
                        node_with_identity_and_scope(
                            5,
                            ControllerId([5; 32]),
                            jacquard_core::DiscoveryScopeId([2; 16]),
                        ),
                    ),
                    (
                        node_six,
                        node_with_identity_and_scope(
                            6,
                            ControllerId([6; 32]),
                            jacquard_core::DiscoveryScopeId([6; 16]),
                        ),
                    ),
                ]),
                links: BTreeMap::from([
                    ((local, node_two), link(2)),
                    ((local, node_four), link(4)),
                    ((local, node_five), link(5)),
                    ((local, node_six), link(6)),
                ]),
                environment: Environment {
                    reachable_neighbor_count: 4,
                    churn_permille: RatioPermille(120),
                    contention_permille: RatioPermille(110),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(2),
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
            diversity_floor: DiversityFloor(1),
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
        assert!(committee.quorum_threshold >= QuorumThreshold(1));
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

    #[test]
    fn behavior_history_can_disqualify_otherwise_high_scoring_members() {
        let topology = diversity_topology();
        let goal = objective_for_service(vec![1, 2]);
        let profile = profile_with(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        );
        let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]))
            .with_behavior_history(BTreeMap::from([(
                NodeId([2; 32]),
                MeshBehaviorHistory {
                    reliability_score: HealthScore(100),
                    misbehavior_penalty_points: PenaltyPoints(800),
                },
            )]));

        let committee = selector
            .select_committee(&goal, &profile, &topology)
            .expect("selector result")
            .expect("committee");
        let member_ids = committee
            .members
            .iter()
            .map(|member| member.node_id)
            .collect::<Vec<_>>();

        assert!(!member_ids.contains(&NodeId([2; 32])));
    }
}
