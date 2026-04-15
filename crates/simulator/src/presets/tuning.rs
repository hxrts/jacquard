#![allow(clippy::wildcard_imports)]

use super::*;

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
    let slow_decay = jacquard_olsrv2::DecayWindow::new(8, 4);
    let fast_decay = jacquard_olsrv2::DecayWindow::new(1, 1);
    let slow = JacquardScenario::new(
        "olsrv2-decay-slow",
        jacquard_core::SimulationSeed(51),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        olsrv2_decay_hosts(slow_decay),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
        26,
    );
    let fast = JacquardScenario::new(
        "olsrv2-decay-fast",
        jacquard_core::SimulationSeed(52),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        olsrv2_decay_hosts(fast_decay),
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
pub fn batman_classic_decay_tuning() -> Vec<(JacquardScenario, ScriptedEnvironmentModel)> {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
    );
    let slow_decay = jacquard_batman_classic::DecayWindow::new(8, 4);
    let fast_decay = jacquard_batman_classic::DecayWindow::new(1, 1);
    let slow = JacquardScenario::new(
        "batman-classic-decay-slow",
        jacquard_core::SimulationSeed(53),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        batman_classic_decay_hosts(slow_decay),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(6)],
        36,
    );
    let fast = JacquardScenario::new(
        "batman-classic-decay-fast",
        jacquard_core::SimulationSeed(54),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        batman_classic_decay_hosts(fast_decay),
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
pub fn babel_decay_tuning() -> Vec<(JacquardScenario, ScriptedEnvironmentModel)> {
    let mut topology = bidirectional_line_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
    );
    topology.value.environment.reachable_neighbor_count = 2;
    topology.value.environment.contention_permille = RatioPermille(40);
    for link in topology.value.links.values_mut() {
        link.state.loss_permille = RatioPermille(100);
        link.state.delivery_confidence_permille =
            jacquard_core::Belief::certain(RatioPermille(900), topology.observed_at_tick);
    }
    let slow_decay = jacquard_babel::DecayWindow::new(8, 4);
    let fast_decay = jacquard_babel::DecayWindow::new(1, 1);
    let restore = topology.value.clone();
    let slow = JacquardScenario::new(
        "babel-decay-slow",
        jacquard_core::SimulationSeed(55),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        babel_decay_hosts(slow_decay),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
        26,
    );
    let fast = JacquardScenario::new(
        "babel-decay-fast",
        jacquard_core::SimulationSeed(56),
        jacquard_core::OperatingMode::DenseInteractive,
        topology.clone(),
        babel_decay_hosts(fast_decay),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
        26,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MediumDegradation {
                left: NODE_B,
                right: NODE_C,
                confidence: RatioPermille(600),
                loss: RatioPermille(250),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(20),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    vec![(slow, environment.clone()), (fast, environment)]
}

fn olsrv2_decay_hosts(decay_window: jacquard_olsrv2::DecayWindow) -> Vec<HostSpec> {
    [NODE_A, NODE_B, NODE_C, NODE_D]
        .into_iter()
        .map(|node_id| HostSpec::olsrv2(node_id).with_olsrv2_decay_window(decay_window))
        .collect()
}

fn batman_classic_decay_hosts(decay_window: jacquard_batman_classic::DecayWindow) -> Vec<HostSpec> {
    [NODE_A, NODE_B, NODE_C]
        .into_iter()
        .map(|node_id| {
            HostSpec::batman_classic(node_id).with_batman_classic_decay_window(decay_window)
        })
        .collect()
}

fn babel_decay_hosts(decay_window: jacquard_babel::DecayWindow) -> Vec<HostSpec> {
    [NODE_A, NODE_B, NODE_C]
        .into_iter()
        .map(|node_id| HostSpec::babel(node_id).with_babel_decay_window(decay_window))
        .collect()
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
