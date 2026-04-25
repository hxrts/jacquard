//! Regression test scenarios for churn, relay stability, and edge cases.

#![allow(clippy::wildcard_imports)]

use super::*;

#[must_use]
// long-block-exception: this preset is a single scenario fixture definition
// pairing topology and environment hooks for regression readability.
pub fn churn_regression() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "churn-regression",
        jacquard_core::SimulationSeed(21),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        8,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: topology::link(3)
                    .with_confidence(RatioPermille(720))
                    .build()
                    .into(),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::ReplaceTopology {
                configuration: Configuration {
                    epoch: RouteEpoch(10),
                    nodes: topology.value.nodes.clone(),
                    links: topology.value.links.clone(),
                    environment: Environment {
                        reachable_neighbor_count: 2,
                        churn_permille: RatioPermille(550),
                        contention_permille: RatioPermille(220),
                    },
                },
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_C,
                to_right: NODE_B,
                link: topology::link(2)
                    .with_confidence(RatioPermille(860))
                    .build()
                    .into(),
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
pub fn partition_regression() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "partition-regression",
        jacquard_core::SimulationSeed(22),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
        8,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::Partition {
                left: NODE_A,
                right: NODE_B,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(3).build()),
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
pub fn deferred_delivery_regression() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "deferred-delivery-regression",
        jacquard_core::SimulationSeed(23),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(1)],
        8,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(760),
                loss: RatioPermille(140),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(3).build()),
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
pub fn adversarial_relay_regression() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "adversarial-relay-regression",
        jacquard_core::SimulationSeed(24),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        7,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(540),
                loss: RatioPermille(320),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::ReplaceTopology {
                configuration: Configuration {
                    epoch: RouteEpoch(14),
                    nodes: topology.value.nodes.clone(),
                    links: topology.value.links.clone(),
                    environment: Environment {
                        reachable_neighbor_count: 2,
                        churn_permille: RatioPermille(400),
                        contention_permille: RatioPermille(450),
                    },
                },
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
// long-block-exception: this preset is a single scenario fixture definition
// pairing topology and environment hooks for regression readability.
pub fn dense_saturation_regression() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    topology.value.environment = Environment {
        reachable_neighbor_count: 3,
        churn_permille: RatioPermille(120),
        contention_permille: RatioPermille(280),
    };
    let scenario = JacquardScenario::new(
        "dense-saturation-regression",
        jacquard_core::SimulationSeed(25),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        8,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(512),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_C,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(512),
            },
        ),
    ]);
    (scenario, environment)
}
