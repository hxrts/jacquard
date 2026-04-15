//! Pathway and Field scenario builders: service fanout, budget stress, and bootstrap families.

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

pub(super) fn build_field_partial_observability_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 900, 2, 3, Some(860)),
        (NODE_C, 780, 2, 4, Some(720)),
    ];
    let mut topology = bridge_cluster_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(140));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "field-partial-observability-bridge-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D]),
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(620),
                forward_loss: RatioPermille(220),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(150),
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

pub(super) fn build_field_reconfiguration_recovery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Node(NODE_C);
    let bootstrap = [
        (NODE_B, 920, 1, 2, Some(880)),
        (NODE_D, 840, 1, 2, Some(810)),
    ];
    let mut topology = ring_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(100), RatioPermille(120));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-reconfiguration-recovery-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D]),
            vec![BoundObjective::new(NODE_A, default_objective(NODE_C)).with_activation_round(3)],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(8),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_D,
                link: Box::new(topology::link(3).build()),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::ReplaceTopology {
                configuration: ring_topology(
                    topology::node(1).field().build(),
                    topology::node(2).field().build(),
                    topology::node(3).field().build(),
                    topology::node(4).field().build(),
                )
                .value,
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_field_asymmetric_envelope_shift(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let mut topology = bridge_cluster_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(120), RatioPermille(120));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-asymmetric-envelope-shift-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            vec![
                HostSpec::field(NODE_A).with_field_bootstrap_summary(field_bootstrap_summary(
                    DestinationId::Node(NODE_D),
                    NODE_B,
                    910,
                    2,
                    3,
                    Some(870),
                )),
                HostSpec::field(NODE_B),
                HostSpec::field(NODE_C),
                HostSpec::field(NODE_D),
            ],
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(7),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_B,
                right: NODE_C,
                forward_confidence: RatioPermille(540),
                forward_loss: RatioPermille(320),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(120),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(13),
            EnvironmentHook::MobilityRelink {
                left: NODE_A,
                from_right: NODE_B,
                to_right: NODE_C,
                link: Box::new(topology::link(2).build()),
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_field_uncertain_service_fanout(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Service(jacquard_core::ServiceId(vec![13; 16]));
    let bootstrap = [
        (NODE_B, 910, 1, 1, Some(860)),
        (NODE_C, 840, 1, 1, Some(790)),
        (NODE_D, 760, 1, 1, Some(730)),
    ];
    let mut topology = full_mesh_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 3, RatioPermille(140), RatioPermille(110));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-uncertain-service-fanout-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D]),
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![13; 16]))
                    .with_activation_round(3),
            ],
            20,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![ScheduledEnvironmentHook::new(
        Tick(10),
        EnvironmentHook::IntrinsicLimit {
            node_id: NODE_C,
            connection_count_max: 1,
            hold_capacity_bytes_max: jacquard_core::ByteCount(384),
        },
    )]);
    (scenario, environment)
}

pub(super) fn build_field_service_overlap_reselection(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Service(jacquard_core::ServiceId(vec![14; 16]));
    let bootstrap = [
        (NODE_B, 920, 1, 1, Some(880)),
        (NODE_C, 860, 1, 1, Some(820)),
        (NODE_D, 760, 1, 1, Some(730)),
        (NODE_E, 720, 1, 1, Some(690)),
    ];
    let mut topology = fanout_service_topology5(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
        topology::node(5).field().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(120), RatioPermille(90));
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-service-overlap-reselection-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D, NODE_E]),
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![14; 16]))
                    .with_activation_round(3),
            ],
            22,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        ScheduledEnvironmentHook::new(
            Tick(9),
            EnvironmentHook::IntrinsicLimit {
                node_id: NODE_B,
                connection_count_max: 1,
                hold_capacity_bytes_max: jacquard_core::ByteCount(320),
            },
        ),
        ScheduledEnvironmentHook::new(
            Tick(12),
            EnvironmentHook::AsymmetricDegradation {
                left: NODE_A,
                right: NODE_C,
                forward_confidence: RatioPermille(520),
                forward_loss: RatioPermille(320),
                reverse_confidence: RatioPermille(760),
                reverse_loss: RatioPermille(120),
            },
        ),
    ]);
    (scenario, environment)
}

pub(super) fn build_field_service_freshness_inversion(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Service(jacquard_core::ServiceId(vec![15; 16]));
    let bootstrap = [
        (NODE_B, 930, 1, 1, Some(900)),
        (NODE_C, 860, 1, 1, Some(820)),
        (NODE_D, 780, 1, 1, Some(740)),
        (NODE_E, 720, 1, 1, Some(690)),
    ];
    let topology = field_fanout_service_topology5(RatioPermille(130), RatioPermille(100));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-service-freshness-inversion-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D, NODE_E]),
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![15; 16]))
                    .with_activation_round(3),
            ],
            24,
        ),
        parameters,
    );
    let environment = field_service_freshness_inversion_environment(&restore);
    (scenario, environment)
}

pub(super) fn build_field_service_publication_pressure(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Service(jacquard_core::ServiceId(vec![16; 16]));
    let bootstrap = [
        (NODE_B, 910, 1, 1, Some(870)),
        (NODE_C, 860, 1, 1, Some(820)),
        (NODE_D, 790, 1, 1, Some(760)),
        (NODE_E, 750, 1, 1, Some(700)),
    ];
    let mut topology = fanout_service_topology5(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
        topology::node(5).field().build(),
    );
    set_environment(&mut topology, 4, RatioPermille(180), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "field-service-publication-pressure-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D, NODE_E]),
            vec![
                BoundObjective::new(NODE_A, field_service_objective(vec![16; 16]))
                    .with_activation_round(3),
            ],
            24,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        medium_degradation_hook(8, NODE_A, NODE_D, RatioPermille(600), RatioPermille(220)),
        asymmetric_degradation_hook(
            10,
            NODE_A,
            NODE_E,
            RatioPermille(520),
            RatioPermille(320),
            RatioPermille(760),
            RatioPermille(150),
        ),
        intrinsic_limit_hook(12, NODE_C, 1, jacquard_core::ByteCount(288)),
        replace_topology_hook(17, &restore),
    ]);
    (scenario, environment)
}

pub(super) fn build_field_bridge_anti_entropy_continuity(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 900, 2, 3, Some(850)),
        (NODE_C, 820, 2, 4, Some(760)),
    ];
    let mut topology = bridge_cluster_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 1, RatioPermille(130), RatioPermille(130));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "field-bridge-anti-entropy-continuity-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D]),
            vec![BoundObjective::new(NODE_A, default_objective(NODE_D)).with_activation_round(3)],
            28,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            7,
            NODE_B,
            NODE_C,
            RatioPermille(560),
            RatioPermille(260),
            RatioPermille(760),
            RatioPermille(140),
        ),
        medium_degradation_hook(11, NODE_A, NODE_B, RatioPermille(640), RatioPermille(180)),
        replace_topology_hook(16, &restore),
        asymmetric_degradation_hook(
            19,
            NODE_B,
            NODE_C,
            RatioPermille(610),
            RatioPermille(220),
            RatioPermille(760),
            RatioPermille(150),
        ),
        replace_topology_hook(23, &restore),
    ]);
    (scenario, environment)
}

pub(super) fn build_field_bootstrap_upgrade_window(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let destination = DestinationId::Node(NODE_C);
    let bootstrap = [
        (NODE_B, 830, 1, 2, Some(770)),
        (NODE_D, 780, 1, 2, Some(730)),
    ];
    let mut topology = ring_topology(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
    );
    set_environment(&mut topology, 2, RatioPermille(100), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("field-bootstrap-upgrade-window-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            field_hosts_with_bootstrap(&destination, &bootstrap, &[NODE_B, NODE_C, NODE_D]),
            vec![BoundObjective::new(NODE_A, default_objective(NODE_C)).with_activation_round(3)],
            26,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            8,
            NODE_B,
            NODE_C,
            RatioPermille(580),
            RatioPermille(240),
            RatioPermille(720),
            RatioPermille(150),
        ),
        replace_topology_hook(12, &restore),
        asymmetric_degradation_hook(
            15,
            NODE_D,
            NODE_C,
            RatioPermille(560),
            RatioPermille(250),
            RatioPermille(730),
            RatioPermille(160),
        ),
        replace_topology_hook(20, &restore),
    ]);
    (scenario, environment)
}
