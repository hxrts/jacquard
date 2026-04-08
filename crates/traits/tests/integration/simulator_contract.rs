//! Drive stub simulator interfaces through the pure scenario/effectful harness
//! split.

use std::collections::BTreeMap;

use jacquard_traits::{
    jacquard_core::{
        Configuration, ControllerId, Environment, FactSourceClass, HoldItemCount, Link,
        LinkRuntimeState, LinkState, MaintenanceWorkBudget, Node, NodeId, NodeProfile,
        NodeState, Observation, OperatingMode, OriginAuthenticationClass,
        RatioPermille, RelayWorkBudget, RouteEpoch, RouteEvent, RouteEventStamped,
        RoutingObjective, SimulationSeed, Tick,
    },
    RoutingEnvironmentModel, RoutingReplayView, RoutingScenario, RoutingSimulator,
};

#[derive(Clone)]
struct StubScenario {
    name: String,
    seed: SimulationSeed,
    deployment_profile: OperatingMode,
    initial_configuration: Observation<Configuration>,
    objectives: Vec<RoutingObjective>,
}

impl RoutingScenario for StubScenario {
    fn name(&self) -> &str {
        &self.name
    }

    fn seed(&self) -> SimulationSeed {
        self.seed
    }

    fn deployment_profile(&self) -> &OperatingMode {
        &self.deployment_profile
    }

    fn initial_configuration(&self) -> &Observation<Configuration> {
        &self.initial_configuration
    }

    fn objectives(&self) -> &[RoutingObjective] {
        &self.objectives
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StubEnvironmentArtifact {
    AdvancedTo(Tick),
}

struct StubEnvironmentModel;

impl RoutingEnvironmentModel for StubEnvironmentModel {
    type EnvironmentArtifact = StubEnvironmentArtifact;

    fn advance_environment(
        &self,
        configuration: &Configuration,
        at_tick: Tick,
    ) -> (Observation<Configuration>, Vec<Self::EnvironmentArtifact>) {
        (
            Observation {
                value: configuration.clone(),
                source_class: FactSourceClass::Local,
                evidence_class:
                    jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
                origin_authentication: OriginAuthenticationClass::Controlled,
                observed_at_tick: at_tick,
            },
            vec![StubEnvironmentArtifact::AdvancedTo(at_tick)],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StubReplayArtifact {
    route_events: Vec<RouteEvent>,
    stamped_route_events: Vec<RouteEventStamped>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StubSimulationStats {
    productive_step_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StubSimulatorError;

struct StubSimulator;

impl RoutingSimulator for StubSimulator {
    type EnvironmentModel = StubEnvironmentModel;
    type Error = StubSimulatorError;
    type ReplayArtifact = StubReplayArtifact;
    type Scenario = StubScenario;
    type SimulationStats = StubSimulationStats;

    fn run_scenario(
        &mut self,
        scenario: &Self::Scenario,
        environment: &Self::EnvironmentModel,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error> {
        let (_, artifacts) = environment.advance_environment(
            &scenario.initial_configuration.value,
            scenario.initial_configuration.observed_at_tick,
        );
        let stamped_route_events = artifacts
            .into_iter()
            .map(|StubEnvironmentArtifact::AdvancedTo(tick)| RouteEventStamped {
                order_stamp: jacquard_traits::jacquard_core::OrderStamp(1),
                emitted_at_tick: tick,
                event: RouteEvent::RouteHealthObserved {
                    route_id: jacquard_traits::jacquard_core::RouteId([1; 16]),
                    health: Observation {
                        value: jacquard_traits::jacquard_core::RouteHealth {
                            reachability_state:
                                jacquard_traits::jacquard_core::ReachabilityState::Reachable,
                            stability_score: jacquard_traits::jacquard_core::HealthScore(1000),
                            congestion_penalty_points:
                                jacquard_traits::jacquard_core::PenaltyPoints(0),
                            last_validated_at_tick: tick,
                        },
                        source_class: FactSourceClass::Local,
                        evidence_class:
                            jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
                        origin_authentication: OriginAuthenticationClass::Controlled,
                        observed_at_tick: tick,
                    },
                },
            })
            .collect();

        Ok((
            StubReplayArtifact {
                route_events: Vec::new(),
                stamped_route_events,
            },
            StubSimulationStats { productive_step_count: 1 },
        ))
    }

    fn resume_replay(
        &mut self,
        replay: &Self::ReplayArtifact,
    ) -> Result<(Self::ReplayArtifact, Self::SimulationStats), Self::Error> {
        Ok((
            replay.clone(),
            StubSimulationStats { productive_step_count: 0 },
        ))
    }
}

impl RoutingReplayView for StubSimulator {
    type ReplayArtifact = StubReplayArtifact;

    fn route_events<'a>(&self, replay: &'a Self::ReplayArtifact) -> &'a [RouteEvent] {
        &replay.route_events
    }

    fn stamped_route_events<'a>(
        &self,
        replay: &'a Self::ReplayArtifact,
    ) -> &'a [RouteEventStamped] {
        &replay.stamped_route_events
    }
}

// long-block-exception: full simulator world-shape fixture.
fn sample_configuration() -> Configuration {
    let local = NodeId([1; 32]);
    let remote = NodeId([2; 32]);

    let mut nodes = BTreeMap::new();
    nodes.insert(
        local,
        Node {
            controller_id: ControllerId([1; 32]),
            profile: NodeProfile {
                services: Vec::new(),
                endpoints: Vec::new(),
                connection_count_max: 4,
                neighbor_state_count_max: 8,
                simultaneous_transfer_count_max: 2,
                active_route_count_max: 4,
                relay_work_budget_max: RelayWorkBudget(16),
                maintenance_work_budget_max: MaintenanceWorkBudget(8),
                hold_item_count_max: HoldItemCount(8),
                hold_capacity_bytes_max: jacquard_traits::jacquard_core::ByteCount(
                    1024,
                ),
            },
            state: NodeState {
                relay_budget: jacquard_traits::jacquard_core::Belief::Absent,
                available_connection_count:
                    jacquard_traits::jacquard_core::Belief::Absent,
                hold_capacity_available_bytes:
                    jacquard_traits::jacquard_core::Belief::Absent,
                information_summary: jacquard_traits::jacquard_core::Belief::Absent,
            },
        },
    );
    nodes.insert(
        remote,
        Node {
            controller_id: ControllerId([2; 32]),
            profile: NodeProfile {
                services: Vec::new(),
                endpoints: Vec::new(),
                connection_count_max: 4,
                neighbor_state_count_max: 8,
                simultaneous_transfer_count_max: 2,
                active_route_count_max: 4,
                relay_work_budget_max: RelayWorkBudget(16),
                maintenance_work_budget_max: MaintenanceWorkBudget(8),
                hold_item_count_max: HoldItemCount(8),
                hold_capacity_bytes_max: jacquard_traits::jacquard_core::ByteCount(
                    1024,
                ),
            },
            state: NodeState {
                relay_budget: jacquard_traits::jacquard_core::Belief::Absent,
                available_connection_count:
                    jacquard_traits::jacquard_core::Belief::Absent,
                hold_capacity_available_bytes:
                    jacquard_traits::jacquard_core::Belief::Absent,
                information_summary: jacquard_traits::jacquard_core::Belief::Absent,
            },
        },
    );

    let mut links = BTreeMap::new();
    links.insert(
        (local, remote),
        Link {
            endpoint: jacquard_traits::jacquard_core::LinkEndpoint {
                protocol: jacquard_traits::jacquard_core::TransportProtocol::BleGatt,
                address: jacquard_traits::jacquard_core::EndpointAddress::Ble {
                    device_id: jacquard_traits::jacquard_core::BleDeviceId(vec![1]),
                    profile_id: jacquard_traits::jacquard_core::BleProfileId([2; 16]),
                },
                mtu_bytes: jacquard_traits::jacquard_core::ByteCount(512),
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: jacquard_traits::jacquard_core::DurationMs(5),
                transfer_rate_bytes_per_sec:
                    jacquard_traits::jacquard_core::Belief::Absent,
                stability_horizon_ms: jacquard_traits::jacquard_core::Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille:
                    jacquard_traits::jacquard_core::Belief::Absent,
                symmetry_permille: jacquard_traits::jacquard_core::Belief::Absent,
            },
        },
    );

    Configuration {
        epoch: RouteEpoch(1),
        nodes,
        links,
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    }
}

fn sample_scenario() -> StubScenario {
    StubScenario {
        name: "smoke".to_owned(),
        seed: SimulationSeed(7),
        deployment_profile: OperatingMode::SparseLowPower,
        initial_configuration: Observation {
            value: sample_configuration(),
            source_class: FactSourceClass::Local,
            evidence_class:
                jacquard_traits::jacquard_core::RoutingEvidenceClass::DirectObservation,
            origin_authentication: OriginAuthenticationClass::Controlled,
            observed_at_tick: Tick(1),
        },
        objectives: Vec::new(),
    }
}

#[test]
fn routing_scenario_is_a_pure_description_surface() {
    let scenario = sample_scenario();

    assert_eq!(scenario.name(), "smoke");
    assert_eq!(scenario.seed(), SimulationSeed(7));
    assert_eq!(
        scenario.deployment_profile(),
        &OperatingMode::SparseLowPower
    );
    assert!(scenario.objectives().is_empty());
}

#[test]
fn routing_simulator_executes_and_replays_through_explicit_artifacts() {
    let scenario = sample_scenario();
    let environment = StubEnvironmentModel;
    let mut simulator = StubSimulator;

    let (replay, stats) = simulator
        .run_scenario(&scenario, &environment)
        .expect("run scenario");
    let (resumed, resumed_stats) =
        simulator.resume_replay(&replay).expect("resume replay");

    assert!(simulator.route_events(&replay).is_empty());
    assert_eq!(simulator.stamped_route_events(&replay).len(), 1);
    assert_eq!(stats.productive_step_count, 1);
    assert_eq!(resumed, replay);
    assert_eq!(resumed_stats.productive_step_count, 0);
}
