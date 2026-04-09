//! Integration tests for mesh engine-wide progress.
//!
//! These tests exercise the bounded engine-private state retained by
//! `engine_tick`: pre-tick bootstrap health, transport-observation
//! summaries, and deterministic replay for the same inputs.

mod common;

use std::collections::BTreeMap;

use common::{
    engine::{
        activate_route_with_profile, build_engine, lease, materialization_input,
        objective, profile_with_connectivity, LOCAL_NODE_ID,
    },
    fixtures::sample_configuration,
};
use jacquard_pathway::{PathwayTransportFreshness, PathwayTransportObservationSummary};
use jacquard_traits::{
    jacquard_core::{
        Belief, DestinationId, DurationMs, EndpointAddress, Estimate, FactSourceClass,
        HealthScore, Link, LinkEndpoint, LinkRuntimeState, LinkState, NodeId,
        Observation, OriginAuthenticationClass, PenaltyPoints, RatioPermille,
        RouteError, RouteMaintenanceOutcome, RouteMaintenanceTrigger,
        RoutePartitionClass, RouteRepairClass, RouteRuntimeError, RoutingEvidenceClass,
        RoutingTickChange, RoutingTickContext, Tick, TransportObservation,
        TransportProtocol,
    },
    RoutingEngine, RoutingEnginePlanner,
};

fn connected_only_policy() -> jacquard_traits::jacquard_core::SelectedRoutingParameters
{
    profile_with_connectivity(
        RouteRepairClass::BestEffort,
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
                        profile_id: jacquard_traits::jacquard_core::BleProfileId(
                            [4; 16],
                        ),
                    },
                    mtu_bytes: jacquard_traits::jacquard_core::ByteCount(256),
                },
                profile: jacquard_traits::jacquard_core::LinkProfile {
                    latency_floor_ms: DurationMs(8),
                    repair_capability:
                        jacquard_traits::jacquard_core::RepairCapability::TransportRetransmit,
                    partition_recovery:
                        jacquard_traits::jacquard_core::PartitionRecoveryClass::LocalReconnect,
                },
                state: LinkState {
                    state: LinkRuntimeState::Active,
                    median_rtt_ms: Belief::Absent,
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
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("route admission");
    let error = engine
        .materialize_route(materialization_input(
            route_id,
            admission,
            lease(Tick(2), Tick(12)),
        ))
        .expect_err("materialization should fail before any topology tick");

    assert!(matches!(
        error,
        RouteError::Runtime(RouteRuntimeError::Invalidated)
    ));
}

#[test]
// long-block-exception: transport-to-health update path in one audit block.
fn engine_tick_transport_observations_change_health_inputs() {
    let topology = sample_configuration();
    let mut plain_engine = build_engine();
    let (_, plain_runtime) = activate_route_with_profile(
        &mut plain_engine,
        &topology,
        &direct_goal(),
        &connected_only_policy(),
        lease(Tick(2), Tick(12)),
    );

    let mut observed_engine = build_engine();
    observed_engine
        .transport
        .push_observation(low_quality_link_observation());
    let (_, observed_runtime) = activate_route_with_profile(
        &mut observed_engine,
        &topology,
        &direct_goal(),
        &connected_only_policy(),
        lease(Tick(2), Tick(12)),
    );

    assert_eq!(
        plain_runtime.health.congestion_penalty_points,
        PenaltyPoints(0)
    );
    assert_eq!(
        observed_runtime.health.congestion_penalty_points,
        PenaltyPoints(4),
    );
    assert_eq!(
        observed_engine
            .transport_observation_summary()
            .expect("transport summary")
            .observed_link_count,
        1,
    );
    assert_eq!(observed_runtime.health.last_validated_at_tick, Tick(2),);
}

#[test]
fn engine_tick_replay_is_deterministic_for_the_same_observations() {
    let topology = sample_configuration();
    let mut left = build_engine();
    let mut right = build_engine();

    left.transport
        .push_observation(low_quality_link_observation());
    right
        .transport
        .push_observation(low_quality_link_observation());

    let left_outcome = left
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("left tick");
    let right_outcome = right
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("right tick");
    assert_eq!(left_outcome, right_outcome);
    assert_eq!(left_outcome.topology_epoch, topology.value.epoch);
    assert_eq!(left_outcome.change, RoutingTickChange::PrivateStateUpdated);

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
        PathwayTransportObservationSummary {
            last_observed_at_tick: Some(Tick(2)),
            payload_event_count: 0,
            observed_link_count: 1,
            reachable_remote_count: 1,
            freshness: PathwayTransportFreshness::Fresh,
            stability_score: HealthScore(750),
            congestion_penalty_points: PenaltyPoints(4),
            remote_links: BTreeMap::from([(
                NodeId([4; 32]),
                jacquard_pathway::PathwayObservedRemoteLink {
                    last_observed_at_tick: Tick(2),
                    stability_score: HealthScore(750),
                    congestion_penalty_points: PenaltyPoints(4),
                },
            )]),
        }
    );
    assert_eq!(left.control_state(), right.control_state());
}

#[test]
fn quiet_tick_preserves_still_fresh_transport_summary() {
    let topology = sample_configuration();
    let mut engine = build_engine();
    engine
        .transport
        .push_observation(low_quality_link_observation());

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("observed tick");
    let initial = engine
        .transport_observation_summary()
        .expect("fresh summary")
        .clone();

    let mut quiet_topology = topology.clone();
    quiet_topology.observed_at_tick = Tick(3);
    engine
        .engine_tick(&RoutingTickContext::new(quiet_topology.clone()))
        .expect("quiet tick");
    let quiet = engine
        .transport_observation_summary()
        .expect("quiet summary")
        .clone();

    assert_eq!(quiet.freshness, PathwayTransportFreshness::Quiet);
    assert_eq!(quiet.last_observed_at_tick, initial.last_observed_at_tick);
    assert_eq!(quiet.payload_event_count, 0);
    assert_eq!(quiet.observed_link_count, 0);
    assert!(quiet.stability_score.0 < initial.stability_score.0);
}

#[test]
fn repeated_quiet_ticks_decay_transport_summary_to_stale_until_refreshed() {
    let topology = sample_configuration();
    let mut engine = build_engine();
    engine
        .transport
        .push_observation(low_quality_link_observation());
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("observed tick");

    for tick in [Tick(3), Tick(4), Tick(5)] {
        let mut quiet_topology = topology.clone();
        quiet_topology.observed_at_tick = tick;
        engine
            .engine_tick(&RoutingTickContext::new(quiet_topology.clone()))
            .expect("quiet tick");
    }

    let stale = engine
        .transport_observation_summary()
        .expect("stale summary")
        .clone();
    assert_eq!(stale.freshness, PathwayTransportFreshness::Stale);

    let mut refreshed_topology = topology.clone();
    refreshed_topology.observed_at_tick = Tick(6);
    let mut refreshed_observation = low_quality_link_observation();
    if let TransportObservation::LinkObserved { observation, .. } =
        &mut refreshed_observation
    {
        observation.observed_at_tick = Tick(6);
    }
    engine.transport.push_observation(refreshed_observation);
    engine
        .engine_tick(&RoutingTickContext::new(refreshed_topology.clone()))
        .expect("refresh tick");
    let refreshed = engine
        .transport_observation_summary()
        .expect("refreshed summary")
        .clone();
    assert_eq!(refreshed.freshness, PathwayTransportFreshness::Fresh);
    assert!(refreshed.stability_score.0 >= stale.stability_score.0);
}

#[test]
fn repeated_ticks_on_the_same_epoch_do_not_rewrite_epoch_checkpoint() {
    let topology = sample_configuration();
    let mut engine = build_engine();
    let topology_epoch_key = {
        let mut key = b"mesh/".to_vec();
        key.extend_from_slice(&LOCAL_NODE_ID.0);
        key.extend_from_slice(b"/topology-epoch");
        key
    };

    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("first tick");
    let stored_after_first_tick = engine
        .effects
        .storage_value(&topology_epoch_key)
        .expect("topology epoch checkpoint");

    let mut same_epoch_topology = topology.clone();
    same_epoch_topology.observed_at_tick = Tick(3);
    engine
        .engine_tick(&RoutingTickContext::new(same_epoch_topology.clone()))
        .expect("second same-epoch tick");

    assert_eq!(
        engine.effects.storage_value(&topology_epoch_key),
        Some(stored_after_first_tick),
        "same-epoch ticks should preserve the topology epoch checkpoint bytes",
    );
}

#[test]
// long-block-exception: route setup, degraded suffix, and assertions together.
fn route_health_is_scoped_to_the_active_route_suffix() {
    let topology = sample_configuration();
    let mut engine = build_engine();

    let (route_three_identity, mut route_three_runtime) = activate_route_with_profile(
        &mut engine,
        &topology,
        &objective(DestinationId::Node(NodeId([3; 32]))),
        &profile_with_connectivity(
            RouteRepairClass::Repairable,
            RoutePartitionClass::PartitionTolerant,
        ),
        lease(Tick(2), Tick(20)),
    );

    let (route_four_identity, mut route_four_runtime) = activate_route_with_profile(
        &mut engine,
        &topology,
        &direct_goal(),
        &connected_only_policy(),
        lease(Tick(2), Tick(20)),
    );

    let mut broken_topology = topology.clone();
    broken_topology
        .value
        .links
        .remove(&(NodeId([2; 32]), NodeId([3; 32])));
    engine
        .engine_tick(&RoutingTickContext::new(broken_topology.clone()))
        .expect("broken-topology tick");

    engine
        .maintain_route(
            &route_three_identity,
            &mut route_three_runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("route-three maintenance");
    engine
        .maintain_route(
            &route_four_identity,
            &mut route_four_runtime,
            jacquard_traits::jacquard_core::RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("route-four maintenance");

    assert_eq!(
        route_three_runtime.health.reachability_state,
        jacquard_traits::jacquard_core::ReachabilityState::Unreachable
    );
    assert_eq!(
        route_four_runtime.health.reachability_state,
        jacquard_traits::jacquard_core::ReachabilityState::Reachable
    );
}

#[test]
// long-block-exception: calm and pressured engines compared side by side.
fn high_transport_pressure_changes_repair_posture() {
    let topology = sample_configuration();
    let mut calm_engine = build_engine();
    let mut pressured_engine = build_engine();

    let (calm_identity, mut calm_runtime) = common::engine::activate_route(
        &mut calm_engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );
    let (pressured_identity, mut pressured_runtime) = common::engine::activate_route(
        &mut pressured_engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );
    pressured_engine
        .transport
        .push_observation(low_quality_link_observation());
    pressured_engine
        .transport
        .push_observation(low_quality_link_observation());

    calm_engine
        .maintain_route(
            &calm_identity,
            &mut calm_runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("calm repair budget reduction");
    pressured_engine
        .maintain_route(
            &pressured_identity,
            &mut pressured_runtime,
            RouteMaintenanceTrigger::EpochAdvanced,
        )
        .expect("pressured repair budget reduction");

    calm_engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("calm tick");
    pressured_engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("pressured tick");

    let calm = calm_engine
        .maintain_route(
            &calm_identity,
            &mut calm_runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("calm degraded maintenance");
    let pressured = pressured_engine
        .maintain_route(
            &pressured_identity,
            &mut pressured_runtime,
            RouteMaintenanceTrigger::LinkDegraded,
        )
        .expect("pressured degraded maintenance");

    assert_eq!(calm.outcome, RouteMaintenanceOutcome::Repaired);
    assert!(matches!(
        pressured.outcome,
        RouteMaintenanceOutcome::ReplacementRequired {
            trigger: RouteMaintenanceTrigger::LinkDegraded,
        }
    ));
}

#[test]
fn anti_entropy_required_consumes_bounded_control_pressure() {
    let topology = sample_configuration();
    let mut engine = build_engine();

    let (identity, mut runtime) = common::engine::activate_route(
        &mut engine,
        &topology,
        NodeId([3; 32]),
        lease(Tick(2), Tick(20)),
    );
    engine
        .transport
        .push_observation(low_quality_link_observation());
    engine
        .transport
        .push_observation(low_quality_link_observation());
    engine
        .engine_tick(&RoutingTickContext::new(topology.clone()))
        .expect("refresh tick");
    let before = engine
        .control_state()
        .expect("control state after tick")
        .anti_entropy
        .pressure_score;

    engine
        .maintain_route(
            &identity,
            &mut runtime,
            RouteMaintenanceTrigger::AntiEntropyRequired,
        )
        .expect("anti-entropy maintenance");
    let after = engine
        .control_state()
        .expect("control state after anti-entropy")
        .anti_entropy
        .pressure_score;

    assert!(after.0 < before.0);
}
