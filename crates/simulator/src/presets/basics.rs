//! Basic scenario fixtures: line topologies and single-engine host configurations.

#![allow(clippy::wildcard_imports)]

use super::*;

#[must_use]
// long-block-exception: this preset is a single scenario fixture definition
// pairing topology and environment hooks for regression readability.
pub fn pathway_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "pathway-line",
        jacquard_core::SimulationSeed(7),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        Vec::new(),
        7,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(800),
                loss: RatioPermille(150),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(512),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::ReplaceTopology {
                configuration: Configuration {
                    epoch: RouteEpoch(9),
                    nodes: topology.value.nodes.clone(),
                    links: topology.value.links.clone(),
                    environment: Environment {
                        reachable_neighbor_count: 2,
                        churn_permille: RatioPermille(25),
                        contention_permille: RatioPermille(10),
                    },
                },
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::Partition {
                left: NODE_B,
                right: NODE_C,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(7),
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
// long-block-exception: this preset is a single scenario fixture definition
// pairing topology and environment hooks for regression readability.
pub fn batman_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1)
            .for_engine(&jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID)
            .build(),
        topology::node(2)
            .for_engine(&jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID)
            .build(),
        topology::node(3)
            .for_engine(&jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID)
            .build(),
    );
    let scenario = JacquardScenario::new(
        "batman-line",
        jacquard_core::SimulationSeed(11),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_bellman(NODE_A),
            HostSpec::batman_bellman(NODE_B),
            HostSpec::batman_bellman(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        7,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(825),
                loss: RatioPermille(100),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(256),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::Partition {
                left: NODE_B,
                right: NODE_C,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_B,
                from_right: NODE_C,
                to_right: NODE_A,
                link: Box::new(topology::link(1).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::ReplaceTopology {
                configuration: Configuration {
                    epoch: RouteEpoch(12),
                    nodes: topology.value.nodes.clone(),
                    links: topology.value.links.clone(),
                    environment: Environment {
                        reachable_neighbor_count: 1,
                        churn_permille: RatioPermille(50),
                        contention_permille: RatioPermille(15),
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
pub fn babel_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1)
            .for_engine(&jacquard_babel::BABEL_ENGINE_ID)
            .build(),
        topology::node(2)
            .for_engine(&jacquard_babel::BABEL_ENGINE_ID)
            .build(),
        topology::node(3)
            .for_engine(&jacquard_babel::BABEL_ENGINE_ID)
            .build(),
    );
    let scenario = JacquardScenario::new(
        "babel-line",
        jacquard_core::SimulationSeed(51),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::babel(NODE_A),
            HostSpec::babel(NODE_B),
            HostSpec::babel(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        7,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(825),
                loss: RatioPermille(100),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(256),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::Partition {
                left: NODE_B,
                right: NODE_C,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_B,
                from_right: NODE_C,
                to_right: NODE_A,
                link: Box::new(topology::link(1).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::ReplaceTopology {
                configuration: Configuration {
                    epoch: RouteEpoch(12),
                    nodes: topology.value.nodes.clone(),
                    links: topology.value.links.clone(),
                    environment: Environment {
                        reachable_neighbor_count: 1,
                        churn_permille: RatioPermille(50),
                        contention_permille: RatioPermille(15),
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
pub fn olsrv2_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1)
            .for_engine(&jacquard_olsrv2::OLSRV2_ENGINE_ID)
            .build(),
        topology::node(2)
            .for_engine(&jacquard_olsrv2::OLSRV2_ENGINE_ID)
            .build(),
        topology::node(3)
            .for_engine(&jacquard_olsrv2::OLSRV2_ENGINE_ID)
            .build(),
    );
    let scenario = JacquardScenario::new(
        "olsrv2-line",
        jacquard_core::SimulationSeed(61),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::olsrv2(NODE_A),
            HostSpec::olsrv2(NODE_B),
            HostSpec::olsrv2(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        7,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(840),
                loss: RatioPermille(90),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(256),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::Partition {
                left: NODE_B,
                right: NODE_C,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_B,
                from_right: NODE_C,
                to_right: NODE_A,
                link: Box::new(topology::link(1).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::ReplaceTopology {
                configuration: Configuration {
                    epoch: RouteEpoch(13),
                    nodes: topology.value.nodes.clone(),
                    links: topology.value.links.clone(),
                    environment: Environment {
                        reachable_neighbor_count: 1,
                        churn_permille: RatioPermille(55),
                        contention_permille: RatioPermille(20),
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
pub fn batman_classic_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1)
            .for_engine(&jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID)
            .build(),
        topology::node(2)
            .for_engine(&jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID)
            .build(),
        topology::node(3)
            .for_engine(&jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID)
            .build(),
    );
    let scenario = JacquardScenario::new(
        "batman-classic-line",
        jacquard_core::SimulationSeed(61),
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        vec![
            HostSpec::batman_classic(NODE_A),
            HostSpec::batman_classic(NODE_B),
            HostSpec::batman_classic(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(6)],
        30,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn partition_tolerant_pathway_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "partition-tolerant-pathway-line",
        jacquard_core::SimulationSeed(17),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        6,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(820),
                loss: RatioPermille(110),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(384),
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
pub fn pathway_multihop() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "pathway-multihop",
        jacquard_core::SimulationSeed(71),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(3)],
        10,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn all_engines_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).all_engines().build(),
        topology::node(2).all_engines().build(),
        topology::node(3).all_engines().build(),
    );
    let scenario = JacquardScenario::new(
        "all-engines-line",
        jacquard_core::SimulationSeed(18),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::all_engines(NODE_A),
            HostSpec::all_engines(NODE_B),
            HostSpec::all_engines(NODE_C),
        ],
        vec![
            BoundObjective::new(NODE_A, connected_objective(NODE_B)),
            BoundObjective::new(NODE_B, connected_objective(NODE_C)),
        ],
        6,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(840),
                loss: RatioPermille(90),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(5),
            EnvironmentHook::MobilityRelink {
                left: NODE_B,
                from_right: NODE_C,
                to_right: NODE_A,
                link: Box::new(topology::link(1).build()),
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
pub fn all_engines_ring() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = ring_topology(
        topology::node(1).all_engines().build(),
        topology::node(2).all_engines().build(),
        topology::node(3).all_engines().build(),
        topology::node(4).all_engines().build(),
    );
    let scenario = JacquardScenario::new(
        "all-engines-ring",
        jacquard_core::SimulationSeed(19),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::all_engines(NODE_A),
            HostSpec::all_engines(NODE_B),
            HostSpec::all_engines(NODE_C),
            HostSpec::all_engines(NODE_D),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
        4,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn mixed_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).pathway_and_batman_bellman().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "mixed-line",
        jacquard_core::SimulationSeed(13),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_bellman(NODE_A),
            HostSpec::pathway_and_batman_bellman(NODE_B)
                .with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_C),
        ],
        vec![
            BoundObjective::new(NODE_A, connected_objective(NODE_B)),
            BoundObjective::new(NODE_B, connected_objective(NODE_C)),
        ],
        4,
    )
    .with_checkpoint_interval(2);
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(3),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(850),
                loss: RatioPermille(90),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(4),
            EnvironmentHook::MobilityRelink {
                left: NODE_B,
                from_right: NODE_C,
                to_right: NODE_A,
                link: Box::new(topology::link(1).build()),
            },
        ),
    ]);
    (scenario, environment)
}
