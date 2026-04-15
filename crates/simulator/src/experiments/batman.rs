//! BATMAN-Bellman scenario builders: sparse line, ring, bridge, and degradation families.

use super::*;

pub(super) fn build_batman_bellman_sparse_line_low_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(0), RatioPermille(20));
    let hosts = vec![
        HostSpec::batman_bellman(NODE_A),
        HostSpec::batman_bellman(NODE_B),
        HostSpec::batman_bellman(NODE_C),
    ];
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-sparse-line-low-loss-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            hosts,
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_batman_bellman_partition_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("batman-bellman-partition-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            26,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_batman_bellman_decay_window_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = bidirectional_line_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
    );
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-decay-window-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(6)],
            36,
        ),
        parameters,
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
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_batman_bellman_medium_ring_contention(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(150), RatioPermille(100));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-medium-ring-contention-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
                HostSpec::batman_bellman(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            20,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_batman_bellman_asymmetric_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(200), RatioPermille(80));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("batman-bellman-asymmetric-bridge-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_bellman(NODE_A),
                HostSpec::batman_bellman(NODE_B),
                HostSpec::batman_bellman(NODE_C),
                HostSpec::batman_bellman(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(6),
        EnvironmentHook::AsymmetricDegradation {
            left: NODE_B,
            right: NODE_C,
            forward_confidence: RatioPermille(520),
            forward_loss: RatioPermille(380),
            reverse_confidence: RatioPermille(760),
            reverse_loss: RatioPermille(180),
        },
    )]);
    (scenario, environment)
}

pub(super) fn build_batman_bellman_asymmetry_relink_transition(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(140));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-asymmetry-relink-transition-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs(&[NODE_A, NODE_B, NODE_C, NODE_D], HostSpec::batman_bellman),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            6,
            NODE_B,
            NODE_C,
            RatioPermille(560),
            RatioPermille(260),
            RatioPermille(740),
            RatioPermille(140),
        ),
        mobility_relink_hook(10, NODE_A, NODE_B, NODE_C, 3),
        mobility_relink_hook(15, NODE_A, NODE_C, NODE_B, 2),
    ]);
    (scenario, environment)
}

pub(super) fn build_batman_bellman_churn_intrinsic_limit(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).batman_bellman().build(),
        topology::node(2).batman_bellman().build(),
        topology::node(3).batman_bellman().build(),
        topology::node(4).batman_bellman().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(50), RatioPermille(50));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-bellman-churn-intrinsic-limit-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs(&[NODE_A, NODE_B, NODE_C, NODE_D], HostSpec::batman_bellman),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        intrinsic_limit_hook(8, NODE_B, 1, jacquard_core::ByteCount(256)),
        mobility_relink_hook(10, NODE_A, NODE_B, NODE_C, 3),
        mobility_relink_hook(14, NODE_A, NODE_C, NODE_B, 2),
    ]);
    (scenario, environment)
}

pub(super) fn build_batman_classic_decay_window_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(100));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-classic-decay-window-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_classic(NODE_A),
                HostSpec::batman_classic(NODE_B),
                HostSpec::batman_classic(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(12)],
            50,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_batman_classic_partition_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("batman-classic-partition-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::batman_classic(NODE_A),
                HostSpec::batman_classic(NODE_B),
                HostSpec::batman_classic(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_B)).with_activation_round(12)],
            60,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(30),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_A, NODE_B), (NODE_B, NODE_A)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(45),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_batman_classic_asymmetry_relink_transition(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).batman_classic().build(),
        topology::node(2).batman_classic().build(),
        topology::node(3).batman_classic().build(),
        topology::node(4).batman_classic().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(140));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "batman-classic-asymmetry-relink-transition-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs(&[NODE_A, NODE_B, NODE_C, NODE_D], HostSpec::batman_classic),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            6,
            NODE_B,
            NODE_C,
            RatioPermille(560),
            RatioPermille(260),
            RatioPermille(740),
            RatioPermille(140),
        ),
        mobility_relink_hook(10, NODE_A, NODE_B, NODE_C, 3),
        mobility_relink_hook(15, NODE_A, NODE_C, NODE_B, 2),
    ]);
    (scenario, environment)
}

pub(super) fn build_babel_decay_window_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(100));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("babel-decay-window-pressure-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::babel(NODE_A),
                HostSpec::babel(NODE_B),
                HostSpec::babel(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            26,
        ),
        parameters,
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
    (scenario, environment)
}

pub(super) fn build_babel_asymmetry_cost_penalty(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
        topology::node(4).babel().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(200), RatioPermille(80));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("babel-asymmetry-cost-penalty-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::babel(NODE_A),
                HostSpec::babel(NODE_B),
                HostSpec::babel(NODE_C),
                HostSpec::babel(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            30,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(6),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(520),
                forward_loss: RatioPermille(380),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(180),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(22),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_babel_partition_feasibility_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).babel().build(),
        topology::node(2).babel().build(),
        topology::node(3).babel().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "babel-partition-feasibility-recovery-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::babel(NODE_A),
                HostSpec::babel(NODE_B),
                HostSpec::babel(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            36,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_olsrv2_topology_propagation_latency(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).olsrv2().build(),
        topology::node(2).olsrv2().build(),
        topology::node(3).olsrv2().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(90));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "olsrv2-topology-propagation-latency-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs(&[NODE_A, NODE_B, NODE_C], HostSpec::olsrv2),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        medium_degradation_hook(6, NODE_A, NODE_B, RatioPermille(640), RatioPermille(200)),
        replace_topology_hook(10, &restore),
        medium_degradation_hook(14, NODE_B, NODE_C, RatioPermille(600), RatioPermille(240)),
        replace_topology_hook(18, &restore),
    ]);
    (scenario, environment)
}

pub(super) fn build_olsrv2_partition_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bidirectional_line_topology(
        topology::node(1).olsrv2().build(),
        topology::node(2).olsrv2().build(),
        topology::node(3).olsrv2().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(40), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("olsrv2-partition-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::olsrv2(NODE_A),
                HostSpec::olsrv2(NODE_B),
                HostSpec::olsrv2(NODE_C),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            34,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_olsrv2_mpr_flooding_stability(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).olsrv2().build(),
        topology::node(2).olsrv2().build(),
        topology::node(3).olsrv2().build(),
        topology::node(4).olsrv2().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(150), RatioPermille(110));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("olsrv2-mpr-flooding-stability-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::olsrv2(NODE_A),
                HostSpec::olsrv2(NODE_B),
                HostSpec::olsrv2(NODE_C),
                HostSpec::olsrv2(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            26,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::MediumDegradation {
                left: NODE_B,
                right: NODE_C,
                confidence: RatioPermille(630),
                loss: RatioPermille(190),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_D,
                link: Box::new(topology::link(4).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(18),
            EnvironmentHook::ReplaceTopology {
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_olsrv2_asymmetric_relink_transition(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).olsrv2().build(),
        topology::node(2).olsrv2().build(),
        topology::node(3).olsrv2().build(),
        topology::node(4).olsrv2().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(130));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "olsrv2-asymmetric-relink-transition-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::olsrv2(NODE_A),
                HostSpec::olsrv2(NODE_B),
                HostSpec::olsrv2(NODE_C),
                HostSpec::olsrv2(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
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
                configuration: restore,
            },
        ),
    ]);
    (scenario, environment)
}
