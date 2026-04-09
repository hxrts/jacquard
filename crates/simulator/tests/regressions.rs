use jacquard_core::{DegradationReason, RouteDegradation, RouteEvent};
use jacquard_simulator::{
    presets, EnvironmentHook, JacquardSimulator, ReferenceClientAdapter,
};
use jacquard_traits::{RoutingScenario, RoutingSimulator};

#[test]
fn churn_regression_emits_environment_artifacts_and_route_progress() {
    let (scenario, environment) = presets::churn_regression();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run churn regression");

    assert_eq!(replay.scenario.name(), "churn-regression");
    assert!(stats.executed_round_count > 0);
    assert!(replay
        .rounds
        .iter()
        .flat_map(|round| round.environment_artifacts.iter())
        .any(|artifact| matches!(
            artifact.hook,
            EnvironmentHook::MobilityRelink { .. }
        )));
    assert!(stats.route_event_count > 0);
}

#[test]
fn partition_regression_records_partition_and_recovery_rounds() {
    let (scenario, environment) = presets::partition_regression();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run partition regression");

    assert!(stats.executed_round_count > 0);
    assert!(replay
        .rounds
        .iter()
        .flat_map(|round| round.environment_artifacts.iter())
        .any(|artifact| matches!(artifact.hook, EnvironmentHook::Partition { .. })));
}

#[test]
fn deferred_delivery_regression_surfaces_degraded_partition_tolerant_route_events() {
    let (scenario, environment) = presets::deferred_delivery_regression();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run deferred-delivery regression");

    assert!(stats.route_event_count > 0);
    assert!(replay.route_events.iter().any(|event| matches!(
        event,
        RouteEvent::RouteMaterialized {
            proof: jacquard_core::RouteMaterializationProof {
                witness: jacquard_core::Fact {
                    value: jacquard_core::RouteWitness {
                        degradation: RouteDegradation::Degraded(
                            DegradationReason::LinkInstability
                        ),
                        ..
                    },
                    ..
                },
                ..
            },
            ..
        }
    )));
}

#[test]
fn adversarial_relay_regression_survives_heavy_contention_inputs() {
    let (scenario, environment) = presets::adversarial_relay_regression();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run adversarial relay regression");

    assert_eq!(replay.scenario.name(), "adversarial-relay-regression");
    assert!(stats.executed_round_count > 0);
    assert!(replay
        .rounds
        .iter()
        .flat_map(|round| round.environment_artifacts.iter())
        .any(|artifact| matches!(
            artifact.hook,
            EnvironmentHook::MediumDegradation { .. }
        )));
}

#[test]
fn dense_saturation_regression_keeps_checkpointing_under_load() {
    let (scenario, environment) = presets::dense_saturation_regression();
    let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run dense saturation regression");

    assert!(stats.executed_round_count > 0);
    assert!(stats.checkpoint_count > 0);
    assert!(replay
        .rounds
        .iter()
        .flat_map(|round| round.host_rounds.iter())
        .any(|artifact| matches!(
            artifact.status,
            jacquard_simulator::HostRoundStatus::Advanced { .. }
        )));
}
