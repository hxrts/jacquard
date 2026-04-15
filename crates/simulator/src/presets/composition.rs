use super::*;

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
