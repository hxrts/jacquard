//! Topology, node, link, and service fixtures shared across router tests.
//!
//! Provides a stable, deterministic sample world used by all router integration
//! test binaries. Every helper here produces the same output for the same
//! inputs so that tests can rely on fixed canonical state without coupling to
//! host-dependent or time-dependent values.
//!
//! Exported constants:
//! - `LOCAL_NODE_ID`, `PEER_NODE_ID`, `FAR_NODE_ID`, `BRIDGE_NODE_ID`: fixed
//!   `NodeId` values that stand in for a local node, a direct peer, a remote
//!   multi-hop destination, and a bridge relay respectively.
//!
//! Exported helpers:
//! - `sample_configuration`: a minimal `Configuration` observation with one
//!   active link per peer, seeded so activation tests have at least one
//!   admissible candidate.
//! - `sample_policy_inputs`: derives `RoutingPolicyInputs` from a topology
//!   observation, matching the local-node projection expected by the router.
//! - `objective`: constructs a `RoutingObjective` for a given `DestinationId`.
//! - `profile`: returns the `SelectedRoutingParameters` fixture used by the
//!   `FixedPolicyEngine` in pre-wired router builders.

use std::collections::BTreeMap;

use jacquard_core::{
    ConnectivityPosture, DestinationId, DiversityFloor, DurationMs, Environment, FactSourceClass,
    HealthScore, IdentityAssuranceClass, NodeId, Observation, OriginAuthenticationClass,
    PriorityPoints, RatioPermille, RouteProtectionClass, RouteRepairClass, RouteReplacementPolicy,
    RouteServiceKind, RoutingEngineFallbackPolicy, RoutingEvidenceClass, RoutingObjective,
    RoutingPolicyInputs, SelectedRoutingParameters, Tick,
};
use jacquard_reference_client::topology;

pub(crate) const LOCAL_NODE_ID: NodeId = NodeId([1; 32]);
pub(crate) const PEER_NODE_ID: NodeId = NodeId([2; 32]);
pub(crate) const FAR_NODE_ID: NodeId = NodeId([3; 32]);
pub(crate) const BRIDGE_NODE_ID: NodeId = NodeId([4; 32]);

pub(crate) fn sample_policy_inputs(
    topology: &Observation<jacquard_core::Configuration>,
) -> RoutingPolicyInputs {
    RoutingPolicyInputs {
        local_node: Observation {
            value: topology.value.nodes[&LOCAL_NODE_ID].clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        local_environment: Observation {
            value: topology.value.environment.clone(),
            source_class: topology.source_class,
            evidence_class: topology.evidence_class,
            origin_authentication: topology.origin_authentication,
            observed_at_tick: topology.observed_at_tick,
        },
        routing_engine_count: 1,
        median_rtt_ms: DurationMs(40),
        loss_permille: RatioPermille(50),
        partition_risk_permille: RatioPermille(150),
        adversary_pressure_permille: RatioPermille(25),
        identity_assurance: IdentityAssuranceClass::ControllerBound,
        direct_reachability_score: HealthScore(900),
    }
}

pub(crate) fn profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: jacquard_core::RoutePartitionClass::PartitionTolerant,
        },
        deployment_profile: jacquard_core::OperatingMode::FieldPartitionTolerant,
        diversity_floor: DiversityFloor(1),
        routing_engine_fallback_policy: RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: RouteReplacementPolicy::Allowed,
    }
}

pub(crate) fn objective(destination: DestinationId) -> RoutingObjective {
    RoutingObjective {
        destination,
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: jacquard_core::RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(crate) fn sample_configuration() -> Observation<jacquard_core::Configuration> {
    Observation {
        value: jacquard_core::Configuration {
            epoch: jacquard_core::RouteEpoch(2),
            nodes: BTreeMap::from([
                (LOCAL_NODE_ID, topology::node(1).pathway().build()),
                (PEER_NODE_ID, topology::node(2).pathway().build()),
                (FAR_NODE_ID, topology::node(3).pathway().build()),
                (BRIDGE_NODE_ID, topology::node(4).pathway().build()),
            ]),
            links: BTreeMap::from([
                (
                    (LOCAL_NODE_ID, PEER_NODE_ID),
                    topology::link(2)
                        .with_confidence(RatioPermille(950))
                        .build(),
                ),
                (
                    (PEER_NODE_ID, FAR_NODE_ID),
                    topology::link(3)
                        .with_confidence(RatioPermille(875))
                        .build(),
                ),
                (
                    (LOCAL_NODE_ID, BRIDGE_NODE_ID),
                    topology::link(4)
                        .with_confidence(RatioPermille(925))
                        .build(),
                ),
            ]),
            environment: Environment {
                reachable_neighbor_count: 3,
                churn_permille: RatioPermille(150),
                contention_permille: RatioPermille(120),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}
