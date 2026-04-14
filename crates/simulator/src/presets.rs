//! Scenario presets for simulator smoke tests and examples.
// long-file-exception: this file is the single maintained catalog of simulator
// scenario fixtures. Keeping the preset definitions together preserves shared
// topology/objective helpers, makes scenario diffs reviewable, and avoids
// scattering the canonical simulator corpus across many tiny modules.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, ConnectivityPosture, DestinationId, DurationMs, Environment, FactSourceClass,
    Observation, OriginAuthenticationClass, PriorityPoints, RatioPermille, RouteEpoch,
    RoutePartitionClass, RouteProtectionClass, RouteRepairClass, RouteServiceKind,
    RoutingEvidenceClass, RoutingObjective, ServiceId, Tick,
};
use jacquard_pathway::PathwaySearchConfig;
use jacquard_reference_client::topology;

use crate::{
    environment::{EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel},
    harness::default_objective,
    scenario::{BoundObjective, FieldBootstrapSummary, HostSpec, JacquardScenario},
};

const NODE_A: jacquard_core::NodeId = jacquard_core::NodeId([1; 32]);
const NODE_B: jacquard_core::NodeId = jacquard_core::NodeId([2; 32]);
const NODE_C: jacquard_core::NodeId = jacquard_core::NodeId([3; 32]);
const NODE_D: jacquard_core::NodeId = jacquard_core::NodeId([4; 32]);

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
pub fn field_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "field-line",
        jacquard_core::SimulationSeed(17),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
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
pub fn field_bootstrap_multihop() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let bootstrap = FieldBootstrapSummary::new(
        DestinationId::Node(NODE_C),
        NODE_B,
        jacquard_field::FieldForwardSummaryObservation::new(RouteEpoch(1), Tick(1), 900, 1, 2),
    )
    .with_reverse_feedback(860, Tick(1));
    let scenario = JacquardScenario::new(
        "field-bootstrap-multihop",
        jacquard_core::SimulationSeed(71),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::field(NODE_A).with_field_bootstrap_summary(bootstrap),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_C)).with_activation_round(3)],
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
        topology::node(2).field_and_batman_bellman().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "mixed-line",
        jacquard_core::SimulationSeed(13),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_bellman(NODE_A),
            HostSpec::field_and_batman_bellman(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![
            BoundObjective::new(NODE_A, connected_objective(NODE_B)),
            BoundObjective::new(NODE_B, default_objective(NODE_C)),
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

#[must_use]
// long-block-exception: this preset is a single scenario fixture definition
// pairing topology and environment hooks for regression readability.
pub fn churn_regression() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "churn-regression",
        jacquard_core::SimulationSeed(21),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
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
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "partition-regression",
        jacquard_core::SimulationSeed(22),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
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
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "deferred-delivery-regression",
        jacquard_core::SimulationSeed(23),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
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
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "adversarial-relay-regression",
        jacquard_core::SimulationSeed(24),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
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
    let topology = Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (NODE_A, topology::node(1).field().build()),
                (NODE_B, topology::node(2).field().build()),
                (NODE_C, topology::node(3).field().build()),
                (NODE_D, topology::node(4).field().build()),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_A, NODE_C), topology::link(3).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
                ((NODE_B, NODE_D), topology::link(4).build()),
                ((NODE_C, NODE_D), topology::link(4).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 4,
                churn_permille: RatioPermille(120),
                contention_permille: RatioPermille(280),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    };
    let scenario = JacquardScenario::new(
        "dense-saturation-regression",
        jacquard_core::SimulationSeed(25),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::field(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
            HostSpec::field(NODE_D),
        ],
        vec![
            BoundObjective::new(NODE_A, default_objective(NODE_B)),
            BoundObjective::new(NODE_C, default_objective(NODE_D)),
        ],
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

#[must_use]
pub fn composition_explicit_path_preferred() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let scenario = JacquardScenario::new(
        "composition-explicit-path-preferred",
        jacquard_core::SimulationSeed(31),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(1)],
        4,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn composition_next_hop_only_viable() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).all_engines().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    let scenario = JacquardScenario::new(
        "composition-next-hop-only-viable",
        jacquard_core::SimulationSeed(32),
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        vec![
            HostSpec::all_engines(NODE_A).with_profile(best_effort_connected_profile()),
            HostSpec::batman_bellman(NODE_B),
            HostSpec::batman_bellman(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
        5,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn composition_corridor_preferred() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).all_engines().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
    );
    let scenario = JacquardScenario::new(
        "composition-corridor-preferred",
        jacquard_core::SimulationSeed(33),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::all_engines(NODE_A),
            HostSpec::field(NODE_B),
            HostSpec::field(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
        5,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn composition_concurrent_objectives() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = dual_pair_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    let scenario = JacquardScenario::new(
        "composition-concurrent-objectives",
        jacquard_core::SimulationSeed(34),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::batman_bellman(NODE_A),
            HostSpec::batman_bellman(NODE_B),
            HostSpec::field(NODE_C),
            HostSpec::field(NODE_D),
        ],
        vec![
            BoundObjective::new(NODE_A, connected_objective(NODE_B)),
            BoundObjective::new(NODE_C, default_objective(NODE_D)),
        ],
        5,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

#[must_use]
pub fn composition_cascade_partition_eliminates_route(
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = dual_pair_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    let scenario = JacquardScenario::new(
        "composition-cascade-partition-eliminates-route",
        jacquard_core::SimulationSeed(35),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::batman_bellman(NODE_A),
            HostSpec::batman_bellman(NODE_B),
            HostSpec::field(NODE_C),
            HostSpec::field(NODE_D),
        ],
        vec![
            BoundObjective::new(NODE_A, connected_objective(NODE_B)),
            BoundObjective::new(NODE_C, default_objective(NODE_D)),
        ],
        6,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(4),
        EnvironmentHook::CascadePartition {
            cuts: vec![(NODE_A, NODE_B), (NODE_C, NODE_D)],
        },
    )]);
    (scenario, environment)
}

#[must_use]
pub fn batman_decay_tuning() -> Vec<(JacquardScenario, ScriptedEnvironmentModel)> {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    let slow = JacquardScenario::new(
        "batman-decay-slow",
        jacquard_core::SimulationSeed(41),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_bellman(NODE_A)
                .with_batman_bellman_decay_window(jacquard_batman_bellman::DecayWindow::new(8, 4)),
            HostSpec::batman_bellman(NODE_B)
                .with_batman_bellman_decay_window(jacquard_batman_bellman::DecayWindow::new(8, 4)),
            HostSpec::batman_bellman(NODE_C)
                .with_batman_bellman_decay_window(jacquard_batman_bellman::DecayWindow::new(8, 4)),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(6)],
        36,
    );
    let fast = JacquardScenario::new(
        "batman-decay-fast",
        jacquard_core::SimulationSeed(42),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman_bellman(NODE_A)
                .with_batman_bellman_decay_window(jacquard_batman_bellman::DecayWindow::new(1, 1)),
            HostSpec::batman_bellman(NODE_B)
                .with_batman_bellman_decay_window(jacquard_batman_bellman::DecayWindow::new(1, 1)),
            HostSpec::batman_bellman(NODE_C)
                .with_batman_bellman_decay_window(jacquard_batman_bellman::DecayWindow::new(1, 1)),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(6)],
        36,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(26),
            EnvironmentHook::ReplaceTopology {
                configuration: topology.value.clone(),
            },
        ),
    ]);
    vec![(slow, environment.clone()), (fast, environment)]
}

#[must_use]
pub fn olsrv2_decay_tuning() -> Vec<(JacquardScenario, ScriptedEnvironmentModel)> {
    let topology = ring_topology(
        topology::node(1).olsrv2().build(),
        topology::node(2).olsrv2().build(),
        topology::node(3).olsrv2().build(),
        topology::node(4).olsrv2().build(),
    );
    let slow = JacquardScenario::new(
        "olsrv2-decay-slow",
        jacquard_core::SimulationSeed(51),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::olsrv2(NODE_A)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(8, 4)),
            HostSpec::olsrv2(NODE_B)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(8, 4)),
            HostSpec::olsrv2(NODE_C)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(8, 4)),
            HostSpec::olsrv2(NODE_D)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(8, 4)),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
        26,
    );
    let fast = JacquardScenario::new(
        "olsrv2-decay-fast",
        jacquard_core::SimulationSeed(52),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::olsrv2(NODE_A)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(1, 1)),
            HostSpec::olsrv2(NODE_B)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(1, 1)),
            HostSpec::olsrv2(NODE_C)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(1, 1)),
            HostSpec::olsrv2(NODE_D)
                .with_olsrv2_decay_window(jacquard_olsrv2::DecayWindow::new(1, 1)),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
        26,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(540),
                forward_loss: RatioPermille(320),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(160),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(16),
            EnvironmentHook::ReplaceTopology {
                configuration: topology.value.clone(),
            },
        ),
    ]);
    vec![(slow, environment.clone()), (fast, environment)]
}

#[must_use]
pub fn profile_driven_engine_selection() -> Vec<(JacquardScenario, ScriptedEnvironmentModel)> {
    let topology = bidirectional_line_topology(
        topology::node(1).field_and_batman_bellman().build(),
        topology::node(2).field_and_batman_bellman().build(),
        topology::node(3).field_and_batman_bellman().build(),
    );
    let connected = JacquardScenario::new(
        "profile-driven-engine-selection-connected",
        jacquard_core::SimulationSeed(43),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::field_and_batman_bellman(NODE_A)
                .with_profile(best_effort_connected_profile()),
            HostSpec::field_and_batman_bellman(NODE_B),
            HostSpec::field_and_batman_bellman(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_B))],
        5,
    );
    let partition_tolerant = JacquardScenario::new(
        "profile-driven-engine-selection-partition",
        jacquard_core::SimulationSeed(44),
        jacquard_core::OperatingMode::FieldPartitionTolerant,
        topology,
        vec![
            HostSpec::field_and_batman_bellman(NODE_A),
            HostSpec::field_and_batman_bellman(NODE_B),
            HostSpec::field_and_batman_bellman(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
        5,
    );
    vec![
        (connected, ScriptedEnvironmentModel::default()),
        (partition_tolerant, ScriptedEnvironmentModel::default()),
    ]
}

#[must_use]
pub fn pathway_search_budget_tuning() -> Vec<(JacquardScenario, ScriptedEnvironmentModel)> {
    let topology = fanout_service_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
    );
    let low_budget = JacquardScenario::new(
        "pathway-search-budget-low",
        jacquard_core::SimulationSeed(45),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::pathway(NODE_A)
                .with_pathway_search_config(
                    PathwaySearchConfig::default().with_per_objective_query_budget(1),
                )
                .with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(
            NODE_A,
            service_objective(ServiceId(vec![9; 16])),
        )],
        6,
    );
    let high_budget = JacquardScenario::new(
        "pathway-search-budget-high",
        jacquard_core::SimulationSeed(46),
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        vec![
            HostSpec::pathway(NODE_A)
                .with_pathway_search_config(
                    PathwaySearchConfig::default().with_per_objective_query_budget(2),
                )
                .with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B),
            HostSpec::pathway(NODE_C),
        ],
        vec![BoundObjective::new(
            NODE_A,
            service_objective(ServiceId(vec![9; 16])),
        )],
        6,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(4),
        EnvironmentHook::CascadePartition {
            cuts: vec![(NODE_A, NODE_B), (NODE_B, NODE_A)],
        },
    )]);
    vec![
        (low_budget, environment.clone()),
        (high_budget, environment),
    ]
}

fn line_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn fanout_service_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_A, NODE_C), topology::link(3).build()),
                ((NODE_C, NODE_A), topology::link(1).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn bidirectional_line_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([(NODE_A, node_a), (NODE_B, node_b), (NODE_C, node_c)]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
                ((NODE_C, NODE_B), topology::link(2).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn dual_pair_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
    node_d: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (NODE_A, node_a),
                (NODE_B, node_b),
                (NODE_C, node_c),
                (NODE_D, node_d),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_C, NODE_D), topology::link(4).build()),
                ((NODE_D, NODE_C), topology::link(3).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn ring_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
    node_d: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (NODE_A, node_a),
                (NODE_B, node_b),
                (NODE_C, node_c),
                (NODE_D, node_d),
            ]),
            links: BTreeMap::from([
                ((NODE_A, NODE_B), topology::link(2).build()),
                ((NODE_B, NODE_A), topology::link(1).build()),
                ((NODE_B, NODE_C), topology::link(3).build()),
                ((NODE_C, NODE_B), topology::link(2).build()),
                ((NODE_C, NODE_D), topology::link(4).build()),
                ((NODE_D, NODE_C), topology::link(3).build()),
                ((NODE_D, NODE_A), topology::link(1).build()),
                ((NODE_A, NODE_D), topology::link(4).build()),
            ]),
            environment: Environment {
                reachable_neighbor_count: 2,
                churn_permille: RatioPermille(0),
                contention_permille: RatioPermille(0),
            },
        },
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

fn connected_objective(destination: jacquard_core::NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn service_objective(service_id: ServiceId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(service_id),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

fn best_effort_connected_profile() -> jacquard_core::SelectedRoutingParameters {
    jacquard_core::SelectedRoutingParameters {
        selected_protection: jacquard_core::RouteProtectionClass::LinkProtected,
        selected_connectivity: jacquard_core::ConnectivityPosture {
            repair: jacquard_core::RouteRepairClass::BestEffort,
            partition: jacquard_core::RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}
