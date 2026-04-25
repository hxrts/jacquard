//! Pathway scenario builders: service fanout, budget stress, and recovery families.

#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) fn build_pathway_sparse_service_fanout(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology4(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(0), RatioPermille(20));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-sparse-service-fanout-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![9; 16]))
                .with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_pathway_search_budget_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology4(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(20), RatioPermille(40));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-search-budget-pressure-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![14; 16]))
                .with_activation_round(2)],
            10,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(4),
        EnvironmentHook::CascadePartition {
            cuts: vec![(NODE_A, NODE_B), (NODE_B, NODE_A)],
        },
    )]);
    (scenario, environment)
}

pub(super) fn build_pathway_medium_service_mesh(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = full_mesh_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(140), RatioPermille(40));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-medium-service-mesh-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![10; 16]))
                .with_activation_round(2)],
            18,
        ),
        parameters,
    );
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_pathway_dense_contention_service(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = full_mesh_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(320), RatioPermille(140));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-dense-contention-service-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![11; 16]))
                .with_activation_round(2)],
            20,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_B,
                confidence: RatioPermille(700),
                loss: RatioPermille(250),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::MediumDegradation {
                left: NODE_A,
                right: NODE_C,
                confidence: RatioPermille(660),
                loss: RatioPermille(300),
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_pathway_high_fanout_budget_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = fanout_service_topology5(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
        topology::node(5).pathway().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(120), RatioPermille(80));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "pathway-high-fanout-budget-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            host_specs_with_primary(
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                &[NODE_B, NODE_C, NODE_D, NODE_E],
                HostSpec::pathway,
            ),
            vec![BoundObjective::new(NODE_A, service_objective(vec![15; 16]))
                .with_activation_round(2)],
            12,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        cascade_partition_hook(4, &[(NODE_A, NODE_B), (NODE_B, NODE_A)]),
        medium_degradation_hook(5, NODE_A, NODE_C, RatioPermille(680), RatioPermille(240)),
        medium_degradation_hook(6, NODE_A, NODE_D, RatioPermille(700), RatioPermille(180)),
    ]);
    (scenario, environment)
}

pub(super) fn build_pathway_churn_replacement(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = ring_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(200), RatioPermille(80));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-churn-replacement-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_C)).with_activation_round(2)],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::CascadePartition {
                cuts: vec![(NODE_B, NODE_C), (NODE_C, NODE_B)],
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(14),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_D,
                link: Box::new(topology::link(4).build()),
            },
        ),
    ]);
    (scenario, environment)
}

// long-block-exception: this regime fixture keeps the topology, objective, and
// staged failure/recovery hooks together so the tuned boundary is clear.
pub(super) fn build_pathway_bridge_failure_service(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).pathway().build(),
        topology::node(2).pathway().build(),
        topology::node(3).pathway().build(),
        topology::node(4).pathway().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(180), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("pathway-bridge-failure-service-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            vec![
                HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
                HostSpec::pathway(NODE_B),
                HostSpec::pathway(NODE_C),
                HostSpec::pathway(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, service_objective(vec![12; 16]))
                .with_activation_round(2)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::Partition {
                left: NODE_B,
                right: NODE_C,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::Partition {
                left: NODE_C,
                right: NODE_B,
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(10),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(384),
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
