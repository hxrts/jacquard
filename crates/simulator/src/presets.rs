//! Scenario presets for simulator smoke tests and examples.

use std::collections::BTreeMap;

use jacquard_core::{
    Configuration, Environment, FactSourceClass, Observation,
    OriginAuthenticationClass, RatioPermille, RouteEpoch, RoutingEvidenceClass, Tick,
};
use jacquard_reference_client::topology;

use crate::{
    environment::{
        EnvironmentHook, ScheduledEnvironmentHook, ScriptedEnvironmentModel,
    },
    harness::default_objective,
    scenario::{BoundObjective, HostSpec, JacquardScenario},
};

const NODE_A: jacquard_core::NodeId = jacquard_core::NodeId([1; 32]);
const NODE_B: jacquard_core::NodeId = jacquard_core::NodeId([2; 32]);
const NODE_C: jacquard_core::NodeId = jacquard_core::NodeId([3; 32]);

#[must_use]
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
            EnvironmentHook::Partition { left: NODE_B, right: NODE_C },
        ),
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: topology::link(3).build(),
            },
        ),
    ]);
    (scenario, environment)
}

#[must_use]
pub fn batman_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1)
            .for_engine(&jacquard_batman::BATMAN_ENGINE_ID)
            .build(),
        topology::node(2)
            .for_engine(&jacquard_batman::BATMAN_ENGINE_ID)
            .build(),
        topology::node(3)
            .for_engine(&jacquard_batman::BATMAN_ENGINE_ID)
            .build(),
    );
    let scenario = JacquardScenario::new(
        "batman-line",
        jacquard_core::SimulationSeed(11),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::batman(NODE_A),
            HostSpec::batman(NODE_B),
            HostSpec::batman(NODE_C),
        ],
        vec![BoundObjective::new(NODE_A, default_objective(NODE_B))],
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
            EnvironmentHook::Partition { left: NODE_B, right: NODE_C },
        ),
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::MobilityRelink {
                left: NODE_B,
                from_right: NODE_C,
                to_right: NODE_A,
                link: topology::link(1).build(),
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
pub fn mixed_line() -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = line_topology(
        topology::node(1).pathway_and_batman().build(),
        topology::node(2).pathway_and_batman().build(),
        topology::node(3).pathway_and_batman().build(),
    );
    let scenario = JacquardScenario::new(
        "mixed-line",
        jacquard_core::SimulationSeed(13),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        vec![
            HostSpec::pathway_and_batman(NODE_A),
            HostSpec::pathway_and_batman(NODE_B),
            HostSpec::pathway_and_batman(NODE_C),
        ],
        vec![
            BoundObjective::new(NODE_A, default_objective(NODE_B)),
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
                link: topology::link(1).build(),
            },
        ),
    ]);
    (scenario, environment)
}

fn line_topology(
    node_a: jacquard_core::Node,
    node_b: jacquard_core::Node,
    node_c: jacquard_core::Node,
) -> Observation<Configuration> {
    Observation {
        value: Configuration {
            epoch: RouteEpoch(1),
            nodes: BTreeMap::from([
                (NODE_A, node_a),
                (NODE_B, node_b),
                (NODE_C, node_c),
            ]),
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
