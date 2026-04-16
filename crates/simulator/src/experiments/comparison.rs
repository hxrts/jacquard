//! Cross-engine comparison scenario builders: connected, partitioned, and asymmetric families.
// long-file-exception: the maintained comparison family catalog is still kept in
// one file so scenario variants and their tests remain auditable together.

#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) fn build_comparison_connected_low_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let mut topology = ring_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 2, RatioPermille(30), RatioPermille(20));
    let scenario = route_visible_template(
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
    )
    .into_scenario(parameters);
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_comparison_connected_high_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
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
    let scenario = route_visible_template(
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
    )
    .into_scenario(parameters);
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
    let comparison_engine_set = parameters.comparison_engine_set;
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
    let scenario = route_visible_template(
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
    )
    .into_scenario(parameters);
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
    let comparison_engine_set = parameters.comparison_engine_set;
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
    let scenario = route_visible_template(
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
    )
    .into_scenario(parameters);
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

fn comparison_concurrent_mixed_hosts(
    comparison_engine_set: Option<ComparisonEngineSet>,
    service_destination: &DestinationId,
    service_bootstrap: &[FieldBootstrapSeed],
) -> Vec<HostSpec> {
    vec![
        comparison_host_spec(NODE_A, comparison_engine_set)
            .with_profile(best_effort_connected_profile()),
        seed_standalone_field_bootstrap(
            comparison_host_spec(NODE_B, comparison_engine_set)
                .with_profile(best_effort_connected_profile()),
            comparison_engine_set,
            service_destination,
            service_bootstrap,
        ),
        comparison_host_spec(NODE_C, comparison_engine_set),
        comparison_host_spec(NODE_D, comparison_engine_set),
    ]
}

fn comparison_concurrent_mixed_environment() -> ScriptedEnvironmentModel {
    ScriptedEnvironmentModel::new(vec![
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
    ])
}

pub(super) fn build_comparison_concurrent_mixed(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![13; 16]));
    let service_bootstrap = [
        (NODE_C, 860, 1, 1, Some(810)),
        (NODE_D, 800, 1, 1, Some(760)),
    ];
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
            comparison_concurrent_mixed_hosts(
                comparison_engine_set,
                &service_destination,
                &service_bootstrap,
            ),
            vec![
                BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2),
                BoundObjective::new(NODE_B, service_objective(vec![13; 16]))
                    .with_activation_round(4),
            ],
            20,
        ),
        parameters,
    );
    let environment = comparison_concurrent_mixed_environment();
    (scenario, environment)
}

pub(super) fn build_comparison_corridor_continuity_uncertainty(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let destination = DestinationId::Node(NODE_D);
    let bootstrap = [
        (NODE_B, 940, 2, 3, Some(900)),
        (NODE_C, 900, 2, 4, Some(840)),
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
                    .with_profile(repairable_partition_tolerant_profile()),
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

fn medium_bridge_repair_alternate(topology: &Observation<Configuration>) -> Configuration {
    let mut alternate = topology.value.clone();
    alternate.links.remove(&(NODE_B, NODE_C));
    alternate.links.remove(&(NODE_C, NODE_B));
    alternate
        .links
        .insert((NODE_B, NODE_E), crate::topology::link(5).build());
    alternate
        .links
        .insert((NODE_E, NODE_B), crate::topology::link(2).build());
    alternate
}

fn medium_bridge_repair_hosts(
    comparison_engine_set: Option<ComparisonEngineSet>,
    destination: &DestinationId,
    bootstrap: &[FieldBootstrapSeed],
) -> Vec<HostSpec> {
    host_specs_with_primary(
        seed_standalone_field_bootstrap(
            comparison_host_spec(NODE_A, comparison_engine_set)
                .with_profile(repairable_connected_profile()),
            comparison_engine_set,
            destination,
            bootstrap,
        ),
        &[NODE_B, NODE_C, NODE_D, NODE_E, NODE_F],
        |node_id| comparison_host_spec(node_id, comparison_engine_set),
    )
}

pub(super) fn build_comparison_medium_bridge_repair(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let destination = DestinationId::Node(NODE_F);
    let bootstrap = [
        (NODE_B, 920, 4, 4, Some(860)),
        (NODE_C, 840, 3, 3, Some(780)),
        (NODE_D, 760, 2, 2, Some(720)),
    ];
    let mut topology = medium_bridge_repair_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
        comparison_topology_node(5, comparison_engine_set),
        comparison_topology_node(6, comparison_engine_set),
    );
    set_environment(&mut topology, 2, RatioPermille(170), RatioPermille(120));
    let alternate = medium_bridge_repair_alternate(&topology);
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!("comparison-medium-bridge-repair-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            medium_bridge_repair_hosts(comparison_engine_set, &destination, &bootstrap),
            vec![BoundObjective::new(NODE_A, connected_objective(NODE_F)).with_activation_round(2)],
            30,
        ),
        parameters,
    );
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            8,
            NODE_C,
            NODE_D,
            RatioPermille(520),
            RatioPermille(320),
            RatioPermille(700),
            RatioPermille(170),
        ),
        replace_topology_hook(14, &alternate),
    ]);
    (scenario, environment)
}

fn size_band_label(size_band: LargePopulationSizeBand) -> &'static str {
    match size_band {
        LargePopulationSizeBand::Moderate => "moderate",
        LargePopulationSizeBand::High => "high",
    }
}

fn large_population_destination_byte(size_band: LargePopulationSizeBand) -> u8 {
    match size_band {
        LargePopulationSizeBand::Moderate => 10,
        LargePopulationSizeBand::High => 14,
    }
}

fn large_population_activation_round(size_band: LargePopulationSizeBand) -> u32 {
    match size_band {
        LargePopulationSizeBand::Moderate => 3,
        LargePopulationSizeBand::High => 4,
    }
}

fn large_population_round_limit(family: &str, size_band: LargePopulationSizeBand) -> u32 {
    match (family, size_band) {
        ("core-periphery", LargePopulationSizeBand::Moderate) => 36,
        ("core-periphery", LargePopulationSizeBand::High) => 44,
        ("multi-bottleneck", LargePopulationSizeBand::Moderate) => 42,
        ("multi-bottleneck", LargePopulationSizeBand::High) => 50,
        _ => 36,
    }
}

fn large_population_bootstrap(
    family: &str,
    size_band: LargePopulationSizeBand,
) -> Vec<FieldBootstrapSeed> {
    match (family, size_band) {
        ("core-periphery", LargePopulationSizeBand::Moderate) => vec![
            (node_id(4), 920, 4, 5, Some(860)),
            (node_id(6), 820, 3, 4, Some(780)),
            (node_id(8), 720, 2, 3, Some(680)),
        ],
        ("core-periphery", LargePopulationSizeBand::High) => vec![
            (node_id(5), 940, 5, 6, Some(900)),
            (node_id(7), 860, 4, 5, Some(820)),
            (node_id(10), 760, 3, 4, Some(720)),
            (node_id(12), 700, 2, 3, Some(660)),
        ],
        ("multi-bottleneck", LargePopulationSizeBand::Moderate) => vec![
            (node_id(4), 900, 4, 4, Some(840)),
            (node_id(7), 780, 3, 3, Some(740)),
            (node_id(9), 700, 2, 2, Some(660)),
        ],
        ("multi-bottleneck", LargePopulationSizeBand::High) => vec![
            (node_id(4), 920, 5, 5, Some(860)),
            (node_id(8), 820, 4, 4, Some(780)),
            (node_id(12), 720, 3, 3, Some(680)),
            (node_id(13), 660, 2, 2, Some(620)),
        ],
        _ => Vec::new(),
    }
}

fn large_population_hosts(
    size_band: LargePopulationSizeBand,
    comparison_engine_set: Option<ComparisonEngineSet>,
    destination: &DestinationId,
    bootstrap: &[FieldBootstrapSeed],
) -> Vec<HostSpec> {
    let bytes = size_band.node_bytes();
    let node_ids = node_ids(bytes);
    let primary = seed_standalone_field_bootstrap(
        comparison_host_spec(node_ids[0], comparison_engine_set)
            .with_profile(repairable_connected_profile()),
        comparison_engine_set,
        destination,
        bootstrap,
    );
    host_specs_with_primary(primary, &node_ids[1..], |node_id| {
        comparison_host_spec(node_id, comparison_engine_set)
    })
}

fn large_core_periphery_alternate(
    topology: &Observation<Configuration>,
    size_band: LargePopulationSizeBand,
) -> Configuration {
    let mut alternate = topology.value.clone();
    match size_band {
        LargePopulationSizeBand::Moderate => {
            alternate.links.remove(&(node_id(5), node_id(6)));
            alternate.links.remove(&(node_id(6), node_id(5)));
            alternate
                .links
                .insert((node_id(3), node_id(6)), crate::topology::link(6).build());
            alternate
                .links
                .insert((node_id(6), node_id(3)), crate::topology::link(3).build());
        }
        LargePopulationSizeBand::High => {
            alternate.links.remove(&(node_id(6), node_id(7)));
            alternate.links.remove(&(node_id(7), node_id(6)));
            alternate
                .links
                .insert((node_id(4), node_id(7)), crate::topology::link(7).build());
            alternate
                .links
                .insert((node_id(7), node_id(4)), crate::topology::link(4).build());
        }
    }
    alternate
}

fn large_core_periphery_environment(
    size_band: LargePopulationSizeBand,
    alternate: &Configuration,
) -> ScriptedEnvironmentModel {
    match size_band {
        LargePopulationSizeBand::Moderate => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                9,
                node_id(5),
                node_id(6),
                RatioPermille(560),
                RatioPermille(260),
                RatioPermille(720),
                RatioPermille(150),
            ),
            replace_topology_hook(16, alternate),
            medium_degradation_hook(
                22,
                node_id(8),
                node_id(9),
                RatioPermille(620),
                RatioPermille(180),
            ),
        ]),
        LargePopulationSizeBand::High => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                10,
                node_id(6),
                node_id(7),
                RatioPermille(540),
                RatioPermille(280),
                RatioPermille(720),
                RatioPermille(150),
            ),
            replace_topology_hook(18, alternate),
            medium_degradation_hook(
                26,
                node_id(10),
                node_id(11),
                RatioPermille(600),
                RatioPermille(210),
            ),
        ]),
    }
}

fn large_bottleneck_alternate(
    topology: &Observation<Configuration>,
    size_band: LargePopulationSizeBand,
) -> Configuration {
    let mut alternate = topology.value.clone();
    match size_band {
        LargePopulationSizeBand::Moderate => {
            alternate
                .links
                .insert((node_id(3), node_id(5)), crate::topology::link(5).build());
            alternate
                .links
                .insert((node_id(5), node_id(3)), crate::topology::link(3).build());
            alternate
                .links
                .insert((node_id(6), node_id(8)), crate::topology::link(8).build());
            alternate
                .links
                .insert((node_id(8), node_id(6)), crate::topology::link(6).build());
        }
        LargePopulationSizeBand::High => {
            alternate
                .links
                .insert((node_id(3), node_id(6)), crate::topology::link(6).build());
            alternate
                .links
                .insert((node_id(6), node_id(3)), crate::topology::link(3).build());
            alternate
                .links
                .insert((node_id(7), node_id(10)), crate::topology::link(10).build());
            alternate
                .links
                .insert((node_id(10), node_id(7)), crate::topology::link(7).build());
            alternate.links.insert(
                (node_id(11), node_id(13)),
                crate::topology::link(13).build(),
            );
            alternate.links.insert(
                (node_id(13), node_id(11)),
                crate::topology::link(11).build(),
            );
        }
    }
    alternate
}

// long-block-exception: the scripted hook schedule is easiest to audit as one
// explicit per-band mapping rather than several tiny indirections.
fn large_bottleneck_environment(
    size_band: LargePopulationSizeBand,
    alternate: &Configuration,
) -> ScriptedEnvironmentModel {
    match size_band {
        LargePopulationSizeBand::Moderate => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                8,
                node_id(4),
                node_id(5),
                RatioPermille(520),
                RatioPermille(310),
                RatioPermille(700),
                RatioPermille(180),
            ),
            intrinsic_limit_hook(10, node_id(4), 2, jacquard_core::ByteCount(320)),
            asymmetric_degradation_hook(
                13,
                node_id(7),
                node_id(8),
                RatioPermille(500),
                RatioPermille(340),
                RatioPermille(680),
                RatioPermille(190),
            ),
            intrinsic_limit_hook(15, node_id(7), 2, jacquard_core::ByteCount(320)),
            replace_topology_hook(18, alternate),
        ]),
        LargePopulationSizeBand::High => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                8,
                node_id(4),
                node_id(5),
                RatioPermille(520),
                RatioPermille(320),
                RatioPermille(700),
                RatioPermille(180),
            ),
            intrinsic_limit_hook(9, node_id(4), 2, jacquard_core::ByteCount(320)),
            asymmetric_degradation_hook(
                12,
                node_id(8),
                node_id(9),
                RatioPermille(500),
                RatioPermille(340),
                RatioPermille(670),
                RatioPermille(190),
            ),
            intrinsic_limit_hook(13, node_id(8), 2, jacquard_core::ByteCount(256)),
            asymmetric_degradation_hook(
                16,
                node_id(12),
                node_id(13),
                RatioPermille(480),
                RatioPermille(360),
                RatioPermille(650),
                RatioPermille(220),
            ),
            replace_topology_hook(21, alternate),
            intrinsic_limit_hook(22, node_id(12), 1, jacquard_core::ByteCount(256)),
        ]),
    }
}

fn build_large_core_periphery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
    size_band: LargePopulationSizeBand,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let destination = DestinationId::Node(node_id(large_population_destination_byte(size_band)));
    let bootstrap = large_population_bootstrap("core-periphery", size_band);
    let mut topology = large_population_core_periphery_topology(comparison_engine_set, size_band);
    match size_band {
        LargePopulationSizeBand::Moderate => {
            set_environment(&mut topology, 4, RatioPermille(180), RatioPermille(120));
        }
        LargePopulationSizeBand::High => {
            set_environment(&mut topology, 5, RatioPermille(220), RatioPermille(140));
        }
    }
    let alternate = large_core_periphery_alternate(&topology, size_band);
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "comparison-large-core-periphery-{}-{}",
                size_band_label(size_band),
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            large_population_hosts(size_band, comparison_engine_set, &destination, &bootstrap),
            vec![BoundObjective::new(
                NODE_A,
                connected_objective(node_id(large_population_destination_byte(size_band))),
            )
            .with_activation_round(large_population_activation_round(size_band))],
            large_population_round_limit("core-periphery", size_band),
        ),
        parameters,
    );
    let environment = large_core_periphery_environment(size_band, &alternate);
    (scenario, environment)
}

fn build_large_bottleneck(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
    size_band: LargePopulationSizeBand,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let destination = DestinationId::Node(node_id(large_population_destination_byte(size_band)));
    let bootstrap = large_population_bootstrap("multi-bottleneck", size_band);
    let mut topology = large_population_bottleneck_topology(comparison_engine_set, size_band);
    match size_band {
        LargePopulationSizeBand::Moderate => {
            set_environment(&mut topology, 3, RatioPermille(200), RatioPermille(150));
        }
        LargePopulationSizeBand::High => {
            set_environment(&mut topology, 4, RatioPermille(240), RatioPermille(180));
        }
    }
    let alternate = large_bottleneck_alternate(&topology, size_band);
    let scenario = apply_overrides(
        &JacquardScenario::new(
            format!(
                "comparison-large-multi-bottleneck-{}-{}",
                size_band_label(size_band),
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            large_population_hosts(size_band, comparison_engine_set, &destination, &bootstrap),
            vec![BoundObjective::new(
                NODE_A,
                connected_objective(node_id(large_population_destination_byte(size_band))),
            )
            .with_activation_round(large_population_activation_round(size_band))],
            large_population_round_limit("multi-bottleneck", size_band),
        ),
        parameters,
    );
    let environment = large_bottleneck_environment(size_band, &alternate);
    (scenario, environment)
}

// Analytical question: how do the connected-route engines behave when a dense
// local core feeds a long sparse tail and the core-to-corridor egress changes
// mid-run, forcing larger-diameter stale-state cleanup?
pub(super) fn build_comparison_large_core_periphery_moderate(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    build_large_core_periphery(parameters, seed, LargePopulationSizeBand::Moderate)
}

// Analytical question: does the same mixed-density route-visible family stay
// legible when both diameter and local fanout grow further into the maintained
// high large-population band?
pub(super) fn build_comparison_large_core_periphery_high(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    build_large_core_periphery(parameters, seed, LargePopulationSizeBand::High)
}

// Analytical question: which engines remain viable when several articulation
// points and corridor links degrade in staggered windows and reroute pressure
// accumulates across more than one bottleneck at a time?
pub(super) fn build_comparison_large_multi_bottleneck_moderate(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    build_large_bottleneck(parameters, seed, LargePopulationSizeBand::Moderate)
}

// Analytical question: which single-engine and mixed-engine combinations fail
// first when the maintained bottleneck family scales into a higher node-count
// corridor with three serial broker points under overlapping stress?
pub(super) fn build_comparison_large_multi_bottleneck_high(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    build_large_bottleneck(parameters, seed, LargePopulationSizeBand::High)
}

#[cfg(test)]
// long-block-exception: the test matrix is a single maintained roster of
// comparison cases and activation windows.
fn comparison_activation_window_cases(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> Vec<(JacquardScenario, Vec<u32>, u32)> {
    vec![
        (
            build_comparison_connected_low_loss(parameters, seed).0,
            vec![2u32],
            18u32,
        ),
        (
            build_comparison_connected_high_loss(parameters, seed).0,
            vec![2u32],
            24u32,
        ),
        (
            build_comparison_bridge_transition(parameters, seed).0,
            vec![2u32],
            24u32,
        ),
        (
            build_comparison_partial_observability_bridge(parameters, seed).0,
            vec![3u32],
            24u32,
        ),
        (
            build_comparison_concurrent_mixed(parameters, seed).0,
            vec![2u32, 4u32],
            20u32,
        ),
        (
            build_comparison_corridor_continuity_uncertainty(parameters, seed).0,
            vec![3u32],
            28u32,
        ),
        (
            build_comparison_medium_bridge_repair(parameters, seed).0,
            vec![2u32],
            30u32,
        ),
        (
            build_comparison_large_core_periphery_moderate(parameters, seed).0,
            vec![3u32],
            36u32,
        ),
        (
            build_comparison_large_core_periphery_high(parameters, seed).0,
            vec![4u32],
            44u32,
        ),
        (
            build_comparison_large_multi_bottleneck_moderate(parameters, seed).0,
            vec![3u32],
            42u32,
        ),
        (
            build_comparison_large_multi_bottleneck_high(parameters, seed).0,
            vec![4u32],
            50u32,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
    use jacquard_core::Tick;
    use jacquard_olsrv2::OLSRV2_ENGINE_ID;
    use jacquard_traits::{RoutingEnvironmentModel, RoutingScenario, RoutingSimulator};

    use super::*;
    use crate::{JacquardSimulator, ReducedReplayView, ReferenceClientAdapter};

    fn sample_parameters() -> ExperimentParameterSet {
        ExperimentParameterSet::head_to_head(ComparisonEngineSet::Babel, Some((4, 2)), None, None)
    }

    fn applied_hook_labels(
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
    ) -> Vec<(u64, &'static str)> {
        let mut configuration = scenario.initial_configuration().value.clone();
        let mut labels = Vec::new();
        for round in 0..scenario.round_limit() {
            let tick = Tick(u64::from(round));
            let (next, applied) = environment.advance_environment(&configuration, tick);
            labels.extend(applied.into_iter().map(|artifact| {
                let label = match artifact.hook {
                    EnvironmentHook::ReplaceTopology { .. } => "replace-topology",
                    EnvironmentHook::MediumDegradation { .. } => "medium-degradation",
                    EnvironmentHook::AsymmetricDegradation { .. } => "asymmetric-degradation",
                    EnvironmentHook::Partition { .. } => "partition",
                    EnvironmentHook::CascadePartition { .. } => "cascade-partition",
                    EnvironmentHook::MobilityRelink { .. } => "mobility-relink",
                    EnvironmentHook::IntrinsicLimit { .. } => "intrinsic-limit",
                };
                (artifact.at_tick.0, label)
            }));
            configuration = next.value;
        }
        labels
    }

    fn run_reduced_replay(
        scenario: &JacquardScenario,
        environment: &ScriptedEnvironmentModel,
    ) -> ReducedReplayView {
        let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
        let (replay, _) = simulator
            .run_scenario(scenario, environment)
            .expect("run comparison scenario");
        ReducedReplayView::from_replay(&replay)
    }

    #[test]
    fn comparison_families_document_activation_rounds_and_active_windows() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let cases = comparison_activation_window_cases(&parameters, seed);

        for (scenario, expected_activations, expected_round_limit) in cases {
            let activations = scenario
                .bound_objectives()
                .iter()
                .map(|binding| binding.activate_at_round)
                .collect::<Vec<_>>();
            assert_eq!(activations, expected_activations, "{}", scenario.name());
            assert_eq!(
                scenario.round_limit(),
                expected_round_limit,
                "{}",
                scenario.name()
            );
            let active_windows = activations
                .iter()
                .map(|activation| expected_round_limit.saturating_sub(*activation))
                .collect::<Vec<_>>();
            assert!(
                active_windows
                    .iter()
                    .all(|active_rounds| *active_rounds > 0),
                "{} active windows: {active_windows:?}",
                scenario.name()
            );
        }
    }

    #[test]
    // long-block-exception: this is a single exhaustive hook-round contract for
    // the maintained comparison families and is clearer as one assertion block.
    fn comparison_family_environment_hooks_fire_on_documented_rounds() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let connected_high_loss = build_comparison_connected_high_loss(&parameters, seed);
        let bridge_transition = build_comparison_bridge_transition(&parameters, seed);
        let partial_observability =
            build_comparison_partial_observability_bridge(&parameters, seed);
        let concurrent_mixed = build_comparison_concurrent_mixed(&parameters, seed);
        let corridor_uncertainty =
            build_comparison_corridor_continuity_uncertainty(&parameters, seed);
        let medium_bridge_repair = build_comparison_medium_bridge_repair(&parameters, seed);
        let large_core_periphery_moderate =
            build_comparison_large_core_periphery_moderate(&parameters, seed);
        let large_core_periphery_high =
            build_comparison_large_core_periphery_high(&parameters, seed);
        let large_multi_bridge_ten_nodes_scenario =
            build_comparison_large_multi_bottleneck_moderate(&parameters, seed);
        let large_multi_bridge_fourteen_nodes_scenario =
            build_comparison_large_multi_bottleneck_high(&parameters, seed);

        assert_eq!(
            applied_hook_labels(&connected_high_loss.0, &connected_high_loss.1),
            vec![(7, "asymmetric-degradation"), (12, "mobility-relink")]
        );
        assert_eq!(
            applied_hook_labels(&bridge_transition.0, &bridge_transition.1),
            vec![
                (7, "asymmetric-degradation"),
                (11, "cascade-partition"),
                (16, "replace-topology"),
            ]
        );
        assert_eq!(
            applied_hook_labels(&partial_observability.0, &partial_observability.1),
            vec![(8, "asymmetric-degradation"), (16, "replace-topology")]
        );
        assert_eq!(
            applied_hook_labels(&concurrent_mixed.0, &concurrent_mixed.1),
            vec![(9, "intrinsic-limit"), (12, "cascade-partition")]
        );
        assert_eq!(
            applied_hook_labels(&corridor_uncertainty.0, &corridor_uncertainty.1),
            vec![
                (7, "asymmetric-degradation"),
                (11, "medium-degradation"),
                (16, "replace-topology"),
                (19, "asymmetric-degradation"),
                (23, "replace-topology"),
            ]
        );
        assert_eq!(
            applied_hook_labels(&medium_bridge_repair.0, &medium_bridge_repair.1),
            vec![(8, "asymmetric-degradation"), (14, "replace-topology")]
        );
        assert_eq!(
            applied_hook_labels(
                &large_core_periphery_moderate.0,
                &large_core_periphery_moderate.1,
            ),
            vec![
                (9, "asymmetric-degradation"),
                (16, "replace-topology"),
                (22, "medium-degradation"),
            ]
        );
        assert_eq!(
            applied_hook_labels(&large_core_periphery_high.0, &large_core_periphery_high.1),
            vec![
                (10, "asymmetric-degradation"),
                (18, "replace-topology"),
                (26, "medium-degradation"),
            ]
        );
        assert_eq!(
            applied_hook_labels(
                &large_multi_bridge_ten_nodes_scenario.0,
                &large_multi_bridge_ten_nodes_scenario.1,
            ),
            vec![
                (8, "asymmetric-degradation"),
                (10, "intrinsic-limit"),
                (13, "asymmetric-degradation"),
                (15, "intrinsic-limit"),
                (18, "replace-topology"),
            ]
        );
        assert_eq!(
            applied_hook_labels(
                &large_multi_bridge_fourteen_nodes_scenario.0,
                &large_multi_bridge_fourteen_nodes_scenario.1,
            ),
            vec![
                (8, "asymmetric-degradation"),
                (9, "intrinsic-limit"),
                (12, "asymmetric-degradation"),
                (13, "intrinsic-limit"),
                (16, "asymmetric-degradation"),
                (21, "replace-topology"),
                (22, "intrinsic-limit"),
            ]
        );
    }

    #[test]
    fn comparison_environment_hooks_produce_expected_connectivity_changes() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let (scenario, environment) = build_comparison_bridge_transition(&parameters, seed);
        let mut configuration = scenario.initial_configuration().value.clone();

        let (_, initial) = environment.advance_environment(&configuration, Tick(6));
        assert!(initial.is_empty());

        let (after_degradation, degradation) =
            environment.advance_environment(&configuration, Tick(7));
        assert_eq!(degradation.len(), 1);
        assert!(
            after_degradation
                .value
                .links
                .contains_key(&(NODE_B, NODE_C))
                && after_degradation
                    .value
                    .links
                    .contains_key(&(NODE_C, NODE_B))
        );
        configuration = after_degradation.value;

        let (after_partition, partition) =
            environment.advance_environment(&configuration, Tick(11));
        assert_eq!(partition.len(), 1);
        assert!(!after_partition.value.links.contains_key(&(NODE_B, NODE_C)));
        assert!(!after_partition.value.links.contains_key(&(NODE_C, NODE_B)));
        configuration = after_partition.value;

        let (after_restore, restore) = environment.advance_environment(&configuration, Tick(16));
        assert_eq!(restore.len(), 1);
        assert!(after_restore.value.links.contains_key(&(NODE_B, NODE_C)));
        assert!(after_restore.value.links.contains_key(&(NODE_C, NODE_B)));
    }

    #[test]
    fn large_core_periphery_reassigns_the_dense_core_egress() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let (scenario, environment) =
            build_comparison_large_core_periphery_moderate(&parameters, seed);
        let mut configuration = scenario.initial_configuration().value.clone();

        let (after_shift, _) = environment.advance_environment(&configuration, Tick(16));
        assert!(!after_shift
            .value
            .links
            .contains_key(&(node_id(5), node_id(6))));
        assert!(!after_shift
            .value
            .links
            .contains_key(&(node_id(6), node_id(5))));
        assert!(after_shift
            .value
            .links
            .contains_key(&(node_id(3), node_id(6))));
        assert!(after_shift
            .value
            .links
            .contains_key(&(node_id(6), node_id(3))));
        configuration = after_shift.value;

        let (after_tail_pressure, applied) =
            environment.advance_environment(&configuration, Tick(22));
        assert_eq!(applied.len(), 1);
        assert!(after_tail_pressure
            .value
            .links
            .contains_key(&(node_id(8), node_id(9))));
    }

    #[test]
    fn large_multi_bottleneck_adds_bypass_links_during_repair() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let (scenario, environment) =
            build_comparison_large_multi_bottleneck_moderate(&parameters, seed);
        let mut configuration = scenario.initial_configuration().value.clone();

        let (after_repair, applied) = environment.advance_environment(&configuration, Tick(18));
        assert_eq!(applied.len(), 1);
        assert!(after_repair
            .value
            .links
            .contains_key(&(node_id(3), node_id(5))));
        assert!(after_repair
            .value
            .links
            .contains_key(&(node_id(5), node_id(3))));
        assert!(after_repair
            .value
            .links
            .contains_key(&(node_id(6), node_id(8))));
        assert!(after_repair
            .value
            .links
            .contains_key(&(node_id(8), node_id(6))));
        configuration = after_repair.value;

        let (after_follow_on, _) = environment.advance_environment(&configuration, Tick(20));
        assert!(after_follow_on
            .value
            .links
            .contains_key(&(node_id(6), node_id(8))));
    }

    #[test]
    fn medium_bridge_repair_replaces_the_bridge_with_an_alternate_corridor() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let (scenario, environment) = build_comparison_medium_bridge_repair(&parameters, seed);
        let mut configuration = scenario.initial_configuration().value.clone();

        let (after_degradation, _) = environment.advance_environment(&configuration, Tick(8));
        assert!(after_degradation
            .value
            .links
            .contains_key(&(NODE_C, NODE_D)));
        configuration = after_degradation.value;

        let (after_repair, applied) = environment.advance_environment(&configuration, Tick(14));
        assert_eq!(applied.len(), 1);
        assert!(!after_repair.value.links.contains_key(&(NODE_B, NODE_C)));
        assert!(!after_repair.value.links.contains_key(&(NODE_C, NODE_B)));
        assert!(after_repair.value.links.contains_key(&(NODE_B, NODE_E)));
        assert!(after_repair.value.links.contains_key(&(NODE_E, NODE_B)));
    }

    #[test]
    fn mixed_comparison_high_loss_prefers_the_next_hop_engine_that_keeps_the_route_up() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let (scenario, environment) =
            build_comparison_connected_high_loss(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let destination = DestinationId::Node(NODE_D);

        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(reduced.route_seen_with_engine(NODE_A, &destination, &BATMAN_BELLMAN_ENGINE_ID));
        assert_eq!(
            reduced.first_round_with_engine(NODE_A, &destination, &BATMAN_BELLMAN_ENGINE_ID),
            Some(2)
        );
    }

    #[test]
    fn mixed_comparison_partial_observability_is_not_masked_by_batman_bellman() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let (scenario, environment) =
            build_comparison_partial_observability_bridge(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let destination = DestinationId::Node(NODE_D);

        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(reduced.route_seen_with_engine(NODE_A, &destination, &OLSRV2_ENGINE_ID));
        assert!(!reduced.route_seen_with_engine(NODE_A, &destination, &BATMAN_BELLMAN_ENGINE_ID));
    }

    #[test]
    fn mixed_comparison_concurrent_family_records_real_engine_selections() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let (scenario, environment) =
            build_comparison_concurrent_mixed(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let node_destination = DestinationId::Node(NODE_D);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![13; 16]));

        assert!(reduced.route_seen(NODE_A, &node_destination));
        assert!(reduced.route_seen(NODE_B, &service_destination));
        assert!(
            !reduced.distinct_engine_ids.is_empty(),
            "mixed comparison should record at least one real engine id"
        );
        assert!(
            reduced.route_observations().iter().all(|observation| {
                observation.engine_id == BATMAN_BELLMAN_ENGINE_ID
                    || observation.engine_id == BATMAN_CLASSIC_ENGINE_ID
                    || observation.engine_id == BABEL_ENGINE_ID
                    || observation.engine_id == OLSRV2_ENGINE_ID
                    || observation.engine_id == PATHWAY_ENGINE_ID
                    || observation.engine_id == FIELD_ENGINE_ID
                    || observation.engine_id == SCATTER_ENGINE_ID
            }),
            "mixed comparison emitted an unexpected engine id",
        );
    }

    #[test]
    fn comparison_connected_high_loss_is_seed_stable_under_scripted_hooks() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let first = build_comparison_connected_high_loss(&parameters, SimulationSeed(41));
        let second = build_comparison_connected_high_loss(&parameters, SimulationSeed(43));
        let first_reduced = run_reduced_replay(&first.0, &first.1);
        let second_reduced = run_reduced_replay(&second.0, &second.1);
        let destination = DestinationId::Node(NODE_D);

        assert_eq!(
            first_reduced.route_present_rounds(NODE_A, &destination),
            second_reduced.route_present_rounds(NODE_A, &destination),
        );
        assert_eq!(
            first_reduced.first_round_with_engine(NODE_A, &destination, &BATMAN_BELLMAN_ENGINE_ID),
            second_reduced.first_round_with_engine(NODE_A, &destination, &BATMAN_BELLMAN_ENGINE_ID),
        );
    }

    #[test]
    fn standalone_scatter_medium_bridge_repair_activates_with_scatter_engine() {
        let parameters = ExperimentParameterSet::scatter("balanced");
        let (scenario, environment) =
            build_comparison_medium_bridge_repair(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_F);
        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(reduced.route_seen_with_engine(NODE_A, &destination, &SCATTER_ENGINE_ID));
    }

    #[test]
    fn head_to_head_scatter_connected_low_loss_activates_route() {
        let parameters =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Scatter, None, None, None);
        let (scenario, environment) =
            build_comparison_connected_low_loss(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_C);
        assert!(
            reduced.route_seen(NODE_A, &destination),
            "scatter connected-low-loss failed with summaries: {:?}",
            reduced.failure_summaries,
        );
        assert!(reduced.route_seen_with_engine(NODE_A, &destination, &SCATTER_ENGINE_ID));
    }

    #[test]
    fn head_to_head_field_concurrent_mixed_activates_both_objectives() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        let (scenario, environment) =
            build_comparison_concurrent_mixed(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        assert!(reduced.route_seen(NODE_A, &DestinationId::Node(NODE_D)));
        assert!(reduced.route_seen(
            NODE_B,
            &DestinationId::Service(jacquard_core::ServiceId(vec![13; 16])),
        ));
    }

    #[test]
    fn head_to_head_field_medium_bridge_repair_activates_route() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        let (scenario, environment) =
            build_comparison_medium_bridge_repair(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_F);
        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(reduced.route_present_rounds(NODE_A, &destination).len() >= 10);
    }

    #[test]
    fn comparison_large_core_periphery_high_is_more_diameter_sensitive_than_connected_low_loss() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let baseline = build_comparison_connected_low_loss(&parameters, SimulationSeed(41));
        let large = build_comparison_large_core_periphery_high(&parameters, SimulationSeed(41));
        let baseline_reduced = run_reduced_replay(&baseline.0, &baseline.1);
        let large_reduced = run_reduced_replay(&large.0, &large.1);
        let baseline_destination = DestinationId::Node(NODE_C);
        let large_destination = DestinationId::Node(node_id(14));

        assert!(large_reduced.route_seen(NODE_A, &large_destination));
        assert!(
            large_reduced.first_round_with_route(NODE_A, &large_destination)
                > baseline_reduced.first_round_with_route(NODE_A, &baseline_destination)
        );
    }

    #[test]
    fn comparison_large_multi_bottleneck_high_records_route_fragility() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let (scenario, environment) =
            build_comparison_large_multi_bottleneck_high(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let destination = DestinationId::Node(node_id(14));
        let _present_rounds = reduced.route_present_rounds(NODE_A, &destination);

        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(
            reduced
                .first_round_without_route_after_presence(NODE_A, &destination)
                .is_some()
                || !reduced.failure_summaries.is_empty()
        );
    }

    #[test]
    fn head_to_head_field_large_core_periphery_high_materializes_route() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        let (scenario, environment) =
            build_comparison_large_core_periphery_high(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let destination = DestinationId::Node(node_id(14));

        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(!reduced
            .route_present_rounds(NODE_A, &destination)
            .is_empty());
    }

    #[test]
    fn head_to_head_field_large_multi_bottleneck_moderate_materializes_route() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        let (scenario, environment) =
            build_comparison_large_multi_bottleneck_moderate(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let destination = DestinationId::Node(node_id(10));

        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(reduced.route_present_rounds(NODE_A, &destination).len() >= 8);
    }

    #[test]
    fn head_to_head_field_corridor_uncertainty_survives_initial_uncertainty_window() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        let (scenario, environment) =
            build_comparison_corridor_continuity_uncertainty(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_D);
        let present_rounds = reduced.route_present_rounds(NODE_A, &destination);
        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(
            present_rounds.len() >= 8,
            "field retained route for {} rounds: {:?}",
            present_rounds.len(),
            present_rounds
        );
    }
}
