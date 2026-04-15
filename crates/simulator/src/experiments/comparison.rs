//! Cross-engine comparison scenario builders: connected, partitioned, and asymmetric families.

use super::*;

pub(super) fn build_comparison_connected_low_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = ring_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 2, RatioPermille(30), RatioPermille(20));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-connected-low-loss-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(best_effort_connected_profile()),
                comparison_host_spec(NODE_B, comparison_engine_set),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_comparison_connected_high_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 760, 2, 4, Some(680)),
        (NODE_C, 820, 2, 4, Some(760)),
    ];
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(220), RatioPermille(220));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-connected-high-loss-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs_with_primary(
                seed_standalone_field_bootstrap(
                    comparison_host_spec(NODE_A, comparison_engine_set)
                        .with_profile(repairable_connected_profile()),
                    comparison_engine_set,
                    &destination,
                    &bootstrap,
                ),
                &[NODE_B, NODE_C, NODE_D],
                |node_id| comparison_host_spec(node_id, comparison_engine_set),
            ),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            7,
            NODE_B,
            NODE_C,
            RatioPermille(600),
            RatioPermille(280),
            RatioPermille(680),
            RatioPermille(220),
        ),
        mobility_relink_hook(12, NODE_A, NODE_B, NODE_C, 3),
    ]);
    (scenario, environment)
}

pub(super) fn build_comparison_bridge_transition(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 820, 2, 4, Some(760)),
        (NODE_C, 720, 2, 4, Some(680)),
    ];
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(140), RatioPermille(140));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-bridge-transition-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs_with_primary(
                seed_standalone_field_bootstrap(
                    comparison_host_spec(NODE_A, comparison_engine_set)
                        .with_profile(repairable_connected_profile()),
                    comparison_engine_set,
                    &destination,
                    &bootstrap,
                ),
                &[NODE_B, NODE_C, NODE_D],
                |node_id| comparison_host_spec(node_id, comparison_engine_set),
            ),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            7,
            NODE_B,
            NODE_C,
            RatioPermille(620),
            RatioPermille(220),
            RatioPermille(720),
            RatioPermille(160),
        ),
        cascade_partition_hook(11, &[(NODE_B, NODE_C), (NODE_C, NODE_B)]),
        replace_topology_hook(16, &restore),
    ]);
    (scenario, environment)
}

pub(super) fn build_comparison_partial_observability_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 900, 2, 3, Some(860)),
        (NODE_C, 780, 2, 4, Some(720)),
    ];
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(150));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "comparison-partial-observability-bridge-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            comparison_hosts_with_bootstrap(
                comparison_engine_set,
                &destination,
                &bootstrap,
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(repairable_connected_profile()),
                &[NODE_B, NODE_C, NODE_D],
            ),
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            8,
            NODE_B,
            NODE_C,
            RatioPermille(640),
            RatioPermille(210),
            RatioPermille(780),
            RatioPermille(130),
        ),
        replace_topology_hook(16, &restore),
    ]);
    (scenario, environment)
}

pub(super) fn build_comparison_concurrent_mixed(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let mut topology = full_mesh_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 3, RatioPermille(160), RatioPermille(90));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-concurrent-mixed-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(best_effort_connected_profile()),
                comparison_host_spec(NODE_B, comparison_engine_set)
                    .with_profile(best_effort_connected_profile()),
                comparison_host_spec(NODE_C, comparison_engine_set),
                comparison_host_spec(NODE_D, comparison_engine_set),
            ],
            vec![
                BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2),
                BoundObjective::new(NODE_B, service_objective(vec![13; 16]))
                    .with_activation_round(4),
            ],
            20,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_C,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(384),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_A, NODE_D), (NODE_D, NODE_A)],
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_comparison_corridor_continuity_uncertainty(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set.as_deref();
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 900, 2, 3, Some(850)),
        (NODE_C, 820, 2, 4, Some(760)),
    ];
    let topology = comparison_bridge_topology(
        comparison_engine_set,
        RatioPermille(130),
        RatioPermille(130),
    );
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "comparison-corridor-continuity-uncertainty-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            comparison_hosts_with_bootstrap(
                comparison_engine_set,
                &destination,
                &bootstrap,
                comparison_host_spec(NODE_A, comparison_engine_set)
                    .with_profile(repairable_connected_profile()),
                &[NODE_B, NODE_C, NODE_D],
            ),
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            28,
        ),
        parameters,
    );
    let environment = comparison_corridor_continuity_uncertainty_environment(&restore);
    (scenario, environment)
}
