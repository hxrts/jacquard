//! Integration tests for mesh engine-wide progress.
//!
//! These tests exercise the bounded engine-private state retained by
//! `engine_tick`: pre-tick bootstrap health, transport-observation
//! summaries, and deterministic replay for the same inputs.

mod common;

use jacquard_mesh::MeshTransportObservationSummary;
use jacquard_traits::{
    jacquard_core::{
        Belief, DestinationId, DurationMs, EndpointAddress, Estimate, FactSourceClass, HealthScore,
        Link, LinkEndpoint, LinkRuntimeState, LinkState, NodeId, Observation,
        OriginAuthenticationClass, PenaltyPoints, RatioPermille, RouteError, RoutePartitionClass,
        RouteRepairClass, RouteRuntimeError, RoutingEvidenceClass, Tick, TransportObservation,
        TransportProtocol,
    },
    MeshRoutingEngine, RoutingEngine, RoutingEnginePlanner,
};

use common::engine::{
    build_engine, lease, materialization_input, objective, profile_with_connectivity,
};
use common::fixtures::sample_configuration;

fn connected_only_policy() -> jacquard_traits::jacquard_core::AdaptiveRoutingProfile {
    profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::ConnectedOnly,
    )
}

fn direct_goal() -> jacquard_traits::jacquard_core::RoutingObjective {
    objective(DestinationId::Node(NodeId([4; 32])))
}

fn low_quality_link_observation() -> TransportObservation {
    TransportObservation::LinkObserved {
        remote_node_id: NodeId([4; 32]),
        observation: Observation {
            value: Link {
                endpoint: LinkEndpoint {
                    protocol: TransportProtocol::BleGatt,
                    address: EndpointAddress::Ble {
                        device_id: jacquard_traits::jacquard_core::BleDeviceId(vec![4]),
                        profile_id: jacquard_traits::jacquard_core::BleProfileId([4; 16]),
                    },
                    mtu_bytes: jacquard_traits::jacquard_core::ByteCount(256),
                },
                state: LinkState {
                    state: LinkRuntimeState::Active,
                    median_rtt_ms: DurationMs(40),
                    transfer_rate_bytes_per_sec: Belief::Absent,
                    stability_horizon_ms: Belief::Absent,
                    loss_permille: RatioPermille(400),
                    delivery_confidence_permille: Belief::Estimated(Estimate {
                        value: RatioPermille(600),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(2),
                    }),
                    symmetry_permille: Belief::Estimated(Estimate {
                        value: RatioPermille(900),
                        confidence_permille: RatioPermille(1000),
                        updated_at_tick: Tick(2),
                    }),
                },
            },
            source_class: FactSourceClass::Local,
            evidence_class: RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(2),
        },
    }
}

#[test]
fn materialization_before_first_tick_fails_closed() {
    let topology = sample_configuration();
    let goal = direct_goal();
    let policy = connected_only_policy();
    let mut engine = build_engine();

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("route admission");
    let error = engine
        .materialize_route(materialization_input(admission, lease(Tick(2), Tick(12))))
        .expect_err("materialization should fail before any topology tick");

    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

#[test]
fn engine_tick_transport_observations_change_health_inputs() {
    let topology = sample_configuration();
    let goal = direct_goal();
    let policy = connected_only_policy();

    let mut plain_engine = build_engine();
    plain_engine.engine_tick(&topology).expect("plain tick");
    let plain_candidate = plain_engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("plain candidate");
    let plain_admission = plain_engine
        .admit_route(&goal, &policy, plain_candidate, &topology)
        .expect("plain admission");
    let plain_installation = plain_engine
        .materialize_route(materialization_input(
            plain_admission,
            lease(Tick(2), Tick(12)),
        ))
        .expect("plain materialization");

    let mut observed_engine = build_engine();
    observed_engine
        .transport_mut()
        .observations
        .push(low_quality_link_observation());
    observed_engine
        .engine_tick(&topology)
        .expect("observed tick");
    let observed_candidate = observed_engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("observed candidate");
    let observed_admission = observed_engine
        .admit_route(&goal, &policy, observed_candidate, &topology)
        .expect("observed admission");
    let observed_installation = observed_engine
        .materialize_route(materialization_input(
            observed_admission,
            lease(Tick(2), Tick(12)),
        ))
        .expect("observed materialization");

    assert_eq!(
        plain_installation.health.congestion_penalty_points,
        PenaltyPoints(0)
    );
    assert_eq!(
        observed_installation.health.congestion_penalty_points,
        PenaltyPoints(4),
    );
    assert_eq!(
        observed_engine
            .transport_observation_summary()
            .expect("transport summary")
            .observed_link_count,
        1,
    );
    assert_eq!(observed_installation.health.last_validated_at_tick, Tick(2),);
}

#[test]
fn engine_tick_replay_is_deterministic_for_the_same_observations() {
    let topology = sample_configuration();
    let mut left = build_engine();
    let mut right = build_engine();

    left.transport_mut()
        .observations
        .push(low_quality_link_observation());
    right
        .transport_mut()
        .observations
        .push(low_quality_link_observation());

    left.engine_tick(&topology).expect("left tick");
    right.engine_tick(&topology).expect("right tick");

    let left_summary = left
        .transport_observation_summary()
        .expect("left transport summary")
        .clone();
    let right_summary = right
        .transport_observation_summary()
        .expect("right transport summary")
        .clone();

    assert_eq!(left_summary, right_summary);
    assert_eq!(
        left_summary,
        MeshTransportObservationSummary {
            last_observed_at_tick: Some(Tick(2)),
            payload_event_count: 0,
            observed_link_count: 1,
            reachable_remote_count: 1,
            stability_score: HealthScore(750),
            congestion_penalty_points: PenaltyPoints(4),
        }
    );
}
