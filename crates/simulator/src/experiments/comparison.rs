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

fn mixed_connected_high_loss_topology() -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, topology::node(1).pathway().build()),
            (NODE_B, topology::node(2).pathway().build()),
            (NODE_C, topology::node(3).pathway().build()),
            (NODE_D, topology::node(4).pathway().build()),
            (NODE_E, topology::node(5).pathway().build()),
            (node_id(10), topology::node(10).all_engines().build()),
            (node_id(11), topology::node(11).all_engines().build()),
            (node_id(12), topology::node(12).all_engines().build()),
            (node_id(13), topology::node(13).all_engines().build()),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_E), topology::link(5).build()),
            ((NODE_E, NODE_A), topology::link(1).build()),
            ((node_id(10), node_id(11)), topology::link(11).build()),
            ((node_id(11), node_id(10)), topology::link(10).build()),
            ((node_id(11), node_id(12)), topology::link(12).build()),
            ((node_id(12), node_id(11)), topology::link(11).build()),
            ((node_id(12), node_id(13)), topology::link(13).build()),
            ((node_id(13), node_id(12)), topology::link(12).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn mixed_connected_high_loss_hosts() -> Vec<HostSpec> {
    vec![
        HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
        HostSpec::pathway(NODE_B),
        HostSpec::pathway(NODE_C),
        HostSpec::pathway(NODE_D),
        HostSpec::pathway(NODE_E),
        comparison_host_spec(node_id(10), None).with_profile(repairable_connected_profile()),
        comparison_host_spec(node_id(11), None),
        comparison_host_spec(node_id(12), None),
        comparison_host_spec(node_id(13), None),
    ]
}

fn pathway_service_budget_branch_hosts() -> Vec<HostSpec> {
    vec![
        HostSpec::pathway(NODE_A).with_profile(best_effort_connected_profile()),
        HostSpec::pathway(NODE_B),
        HostSpec::pathway(NODE_C),
        HostSpec::pathway(NODE_D),
        HostSpec::pathway(NODE_E),
    ]
}

fn restore_pathway_service_budget_branch(topology: &mut Observation<Configuration>) {
    topology
        .value
        .links
        .insert((NODE_A, NODE_E), topology::link(5).build());
    topology
        .value
        .links
        .insert((NODE_E, NODE_A), topology::link(1).build());
}

fn restore_mixed_connected_high_loss_service_links(topology: &mut Observation<Configuration>) {
    restore_pathway_service_budget_branch(topology);
}

fn connected_high_loss_environment(
    left_edge_node: NodeId,
    degradation_left_node: NodeId,
    degradation_right_node: NodeId,
) -> ScriptedEnvironmentModel {
    ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            7,
            degradation_left_node,
            degradation_right_node,
            RatioPermille(600),
            RatioPermille(280),
            RatioPermille(680),
            RatioPermille(220),
        ),
        mobility_relink_hook(
            12,
            left_edge_node,
            degradation_left_node,
            degradation_right_node,
            3,
        ),
    ])
}

// long-block-exception: comparison builder keeps mixed-only and head-to-head topology variants adjacent for auditability.
pub(super) fn build_comparison_connected_high_loss(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_connected_high_loss_topology();
        set_environment(&mut topology, 1, RatioPermille(220), RatioPermille(220));
        restore_mixed_connected_high_loss_service_links(&mut topology);
        let scenario = route_visible_template(
            format!("comparison-connected-high-loss-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_connected_high_loss_hosts(),
            vec![
                BoundObjective::new(node_id(10), connected_objective(node_id(13)))
                    .with_activation_round(2),
                BoundObjective::new(NODE_A, service_objective(vec![23; 16]))
                    .with_activation_round(4),
            ],
            24,
        )
        .into_scenario(parameters)
        .with_broker_nodes(vec![node_id(13), node_id(14), node_id(18), node_id(19)]);
        return (
            scenario,
            connected_high_loss_environment(node_id(10), node_id(11), node_id(12)),
        );
    }
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
    .into_scenario(parameters)
    .with_broker_nodes(vec![node_id(4), node_id(5), node_id(9), node_id(10)]);
    (
        scenario,
        connected_high_loss_environment(NODE_A, NODE_B, NODE_C),
    )
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

fn mixed_partial_observability_topology() -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, topology::node(1).pathway().build()),
            (NODE_B, topology::node(2).pathway().build()),
            (NODE_C, topology::node(3).pathway().build()),
            (NODE_D, topology::node(4).pathway().build()),
            (NODE_E, topology::node(5).pathway().build()),
            (node_id(10), topology::node(10).all_engines().build()),
            (node_id(11), topology::node(11).all_engines().build()),
            (node_id(12), topology::node(12).all_engines().build()),
            (node_id(13), topology::node(13).all_engines().build()),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_E), topology::link(5).build()),
            ((NODE_E, NODE_A), topology::link(1).build()),
            ((node_id(10), node_id(11)), topology::link(11).build()),
            ((node_id(11), node_id(10)), topology::link(10).build()),
            ((node_id(11), node_id(12)), topology::link(12).build()),
            ((node_id(12), node_id(11)), topology::link(11).build()),
            ((node_id(12), node_id(13)), topology::link(13).build()),
            ((node_id(13), node_id(12)), topology::link(12).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

fn mixed_partial_observability_hosts() -> Vec<HostSpec> {
    pathway_service_budget_branch_hosts()
        .into_iter()
        .chain([
            comparison_host_spec(node_id(10), None).with_profile(repairable_connected_profile()),
            comparison_host_spec(node_id(11), None),
            comparison_host_spec(node_id(12), None),
            comparison_host_spec(node_id(13), None),
        ])
        .collect()
}

// long-block-exception: comparison builder keeps mixed-only and head-to-head topology variants adjacent for auditability.
pub(super) fn build_comparison_partial_observability_bridge(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_partial_observability_topology();
        set_environment(&mut topology, 1, RatioPermille(120), RatioPermille(150));
        restore_pathway_service_budget_branch(&mut topology);
        let restore = topology.value.clone();
        let scenario = route_visible_template(
            format!(
                "comparison-partial-observability-bridge-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::FieldPartitionTolerant,
            topology,
            mixed_partial_observability_hosts(),
            vec![
                BoundObjective::new(node_id(10), default_objective(node_id(13)))
                    .with_activation_round(3),
                BoundObjective::new(NODE_A, service_objective(vec![25; 16]))
                    .with_activation_round(4),
            ],
            24,
        )
        .into_scenario(parameters)
        .with_broker_nodes(vec![node_id(12), node_id(13)]);
        let environment = ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                8,
                node_id(11),
                node_id(12),
                RatioPermille(640),
                RatioPermille(210),
                RatioPermille(780),
                RatioPermille(130),
            ),
            replace_topology_hook(16, &restore),
        ]);
        return (scenario, environment);
    }
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
    .into_scenario(parameters)
    .with_broker_nodes(vec![node_id(4), node_id(5)]);
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

fn set_scatter_transport_window(
    topology: &mut Observation<Configuration>,
    directed_edges: &[(NodeId, NodeId)],
    transfer_rate_bytes_per_sec: u32,
    stability_horizon_ms: DurationMs,
) {
    for (left, right) in directed_edges {
        if let Some(link) = topology.value.links.get_mut(&(*left, *right)) {
            link.state.transfer_rate_bytes_per_sec =
                Belief::certain(transfer_rate_bytes_per_sec, topology.observed_at_tick);
            link.state.stability_horizon_ms =
                Belief::certain(stability_horizon_ms, topology.observed_at_tick);
        }
    }
}

fn set_scatter_node_state(
    topology: &mut Observation<Configuration>,
    node_id: NodeId,
    hold_capacity_available_bytes: jacquard_core::ByteCount,
    relay_utilization_permille: RatioPermille,
) {
    let Some(node) = topology.value.nodes.get_mut(&node_id) else {
        return;
    };
    node.state.hold_capacity_available_bytes =
        Belief::certain(hold_capacity_available_bytes, topology.observed_at_tick);
    node.state.relay_budget = Belief::certain(
        jacquard_core::NodeRelayBudget::observed(
            jacquard_core::RelayWorkBudget(8),
            relay_utilization_permille,
            DurationMs(500),
            topology.observed_at_tick,
        ),
        topology.observed_at_tick,
    );
}

fn scatter_threshold_hosts(
    comparison_engine_set: Option<ComparisonEngineSet>,
    destination: NodeId,
) -> Vec<HostSpec> {
    host_specs_with_primary(
        comparison_host_spec(NODE_A, comparison_engine_set)
            .with_profile(repairable_connected_profile()),
        &[NODE_B, NODE_C, NODE_D, NODE_E, destination],
        |node_id| comparison_host_spec(node_id, comparison_engine_set),
    )
}

pub(super) fn build_scatter_low_rate_transfer_threshold(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let mut topology = topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[1, 2, 3, 4], comparison_engine_set),
        &[(1, 2), (2, 3), (3, 4)],
        1,
    );
    set_environment(&mut topology, 1, RatioPermille(80), RatioPermille(40));
    set_scatter_transport_window(
        &mut topology,
        &[
            (NODE_A, NODE_B),
            (NODE_B, NODE_A),
            (NODE_B, NODE_C),
            (NODE_C, NODE_B),
            (NODE_C, NODE_D),
            (NODE_D, NODE_C),
        ],
        80,
        DurationMs(2_500),
    );
    set_scatter_node_state(
        &mut topology,
        NODE_A,
        jacquard_core::ByteCount(1_024),
        RatioPermille(760),
    );
    for node_id in [NODE_B, NODE_C, NODE_D] {
        set_scatter_node_state(
            &mut topology,
            node_id,
            jacquard_core::ByteCount(4_096),
            RatioPermille(0),
        );
    }
    let scenario = route_visible_template(
        format!(
            "scatter-low-rate-transfer-threshold-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        host_specs_with_primary(
            comparison_host_spec(NODE_A, comparison_engine_set)
                .with_profile(repairable_connected_profile()),
            &[NODE_B, NODE_C, NODE_D],
            |node_id| comparison_host_spec(node_id, comparison_engine_set),
        ),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
        18,
    )
    .into_scenario(parameters);
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_scatter_stability_window_threshold(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let mut topology = topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[1, 2, 3, 4], comparison_engine_set),
        &[(1, 2), (2, 3), (3, 4)],
        1,
    );
    set_environment(&mut topology, 1, RatioPermille(60), RatioPermille(35));
    set_scatter_transport_window(
        &mut topology,
        &[
            (NODE_A, NODE_B),
            (NODE_B, NODE_A),
            (NODE_B, NODE_C),
            (NODE_C, NODE_B),
            (NODE_C, NODE_D),
            (NODE_D, NODE_C),
        ],
        2_048,
        DurationMs(300),
    );
    set_scatter_node_state(
        &mut topology,
        NODE_A,
        jacquard_core::ByteCount(1_024),
        RatioPermille(760),
    );
    for node_id in [NODE_B, NODE_C, NODE_D] {
        set_scatter_node_state(
            &mut topology,
            node_id,
            jacquard_core::ByteCount(4_096),
            RatioPermille(0),
        );
    }
    let scenario = route_visible_template(
        format!(
            "scatter-stability-window-threshold-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        host_specs_with_primary(
            comparison_host_spec(NODE_A, comparison_engine_set)
                .with_profile(repairable_connected_profile()),
            &[NODE_B, NODE_C, NODE_D],
            |node_id| comparison_host_spec(node_id, comparison_engine_set),
        ),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_D)).with_activation_round(2)],
        18,
    )
    .into_scenario(parameters);
    (scenario, ScriptedEnvironmentModel::default())
}

pub(super) fn build_scatter_conservative_constrained_threshold(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    let mut topology = topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[1, 2, 3, 4, 5, 6], comparison_engine_set),
        &[(1, 2), (1, 3), (1, 4), (1, 5), (5, 6)],
        4,
    );
    set_environment(&mut topology, 4, RatioPermille(120), RatioPermille(30));
    set_scatter_node_state(
        &mut topology,
        NODE_A,
        jacquard_core::ByteCount(4_096),
        RatioPermille(700),
    );
    for node_id in [NODE_B, NODE_C, NODE_D, NODE_E] {
        set_scatter_node_state(
            &mut topology,
            node_id,
            jacquard_core::ByteCount(1_024),
            RatioPermille(448),
        );
    }
    let scenario = route_visible_template(
        format!(
            "scatter-conservative-constrained-threshold-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        scatter_threshold_hosts(comparison_engine_set, NODE_F),
        vec![BoundObjective::new(NODE_A, connected_objective(NODE_F)).with_activation_round(2)],
        18,
    )
    .into_scenario(parameters);
    (scenario, ScriptedEnvironmentModel::default())
}

fn comparison_concurrent_mixed_hosts(
    comparison_engine_set: Option<ComparisonEngineSet>,
    service_destination: &DestinationId,
    service_bootstrap: &[FieldBootstrapSeed],
) -> Vec<HostSpec> {
    if comparison_engine_set.is_none() {
        return vec![
            comparison_host_spec(NODE_A, comparison_engine_set)
                .with_profile(best_effort_connected_profile()),
            HostSpec::pathway(NODE_B).with_profile(best_effort_connected_profile()),
            comparison_host_spec(NODE_C, comparison_engine_set),
            comparison_host_spec(NODE_D, comparison_engine_set),
            HostSpec::pathway(NODE_E),
        ];
    }
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

fn comparison_concurrent_mixed_topology(
    comparison_engine_set: Option<ComparisonEngineSet>,
) -> Observation<Configuration> {
    if comparison_engine_set.is_some() {
        return full_mesh_topology(
            comparison_topology_node(1, comparison_engine_set),
            comparison_topology_node(2, comparison_engine_set),
            comparison_topology_node(3, comparison_engine_set),
            comparison_topology_node(4, comparison_engine_set),
        );
    }
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, comparison_topology_node(1, comparison_engine_set)),
            (NODE_B, topology::node(2).pathway().build()),
            (NODE_C, comparison_topology_node(3, comparison_engine_set)),
            (NODE_D, comparison_topology_node(4, comparison_engine_set)),
            (NODE_E, topology::node(5).pathway().build()),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
            ((NODE_B, NODE_E), topology::link(5).build()),
            ((NODE_E, NODE_B), topology::link(2).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(90),
        },
    })
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
    let mut topology = comparison_concurrent_mixed_topology(comparison_engine_set);
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

// Analytical question: does the mixed comparison matrix expose a true
// candidate-budget boundary when lower-priority service candidates are the
// only reachable choices?
pub(super) fn build_comparison_pathway_budget_boundary(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let topology = routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, topology::node(1).pathway().build()),
            (NODE_B, topology::node(2).pathway().build()),
            (NODE_C, topology::node(3).pathway().build()),
            (NODE_D, topology::node(4).pathway().build()),
            (NODE_E, topology::node(5).pathway().build()),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_E), topology::link(5).build()),
            ((NODE_E, NODE_A), topology::link(1).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(20),
        },
    });
    let scenario = route_visible_template(
        format!(
            "comparison-pathway-budget-boundary-{}",
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
        vec![BoundObjective::new(NODE_A, service_objective(vec![21; 16])).with_activation_round(2)],
        10,
    )
    .into_scenario(parameters);
    (scenario, ScriptedEnvironmentModel::default())
}

// long-block-exception: comparison builder keeps mixed-only and head-to-head topology variants adjacent for auditability.
pub(super) fn build_comparison_corridor_continuity_uncertainty(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_service_plus_topology(&[1, 2, 3, 4], &[(1, 2), (2, 3), (3, 4)], 1);
        set_environment(&mut topology, 1, RatioPermille(130), RatioPermille(130));
        restore_pathway_service_budget_branch(&mut topology);
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
                mixed_service_plus_hosts(&[1, 2, 3, 4]),
                vec![
                    BoundObjective::new(shifted_node_id(1), default_objective(shifted_node_id(4)))
                        .with_activation_round(3),
                    mixed_service_objective(35),
                ],
                28,
            ),
            parameters,
        );
        let environment = ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                7,
                shifted_node_id(2),
                shifted_node_id(3),
                RatioPermille(560),
                RatioPermille(250),
                RatioPermille(760),
                RatioPermille(140),
            ),
            medium_degradation_hook(
                11,
                shifted_node_id(1),
                shifted_node_id(2),
                RatioPermille(650),
                RatioPermille(170),
            ),
            replace_topology_hook(16, &restore),
            asymmetric_degradation_hook(
                19,
                shifted_node_id(2),
                shifted_node_id(3),
                RatioPermille(610),
                RatioPermille(220),
                RatioPermille(760),
                RatioPermille(150),
            ),
            replace_topology_hook(23, &restore),
        ]);
        return (scenario, environment);
    }
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

// long-block-exception: comparison builder keeps mixed-only and head-to-head topology variants adjacent for auditability.
pub(super) fn build_comparison_medium_bridge_repair(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology =
            mixed_service_plus_topology(&[1, 2, 3, 4, 5, 6], MEDIUM_BRIDGE_REPAIR_EDGES, 2);
        set_environment(&mut topology, 2, RatioPermille(170), RatioPermille(120));
        restore_pathway_service_budget_branch(&mut topology);
        let alternate = shifted_alternate(&topology, &[(2, 3)], &[(2, 5)]);
        let scenario = apply_overrides(
            &JacquardScenario::new(
                format!("comparison-medium-bridge-repair-{}", parameters.config_id),
                seed,
                jacquard_core::OperatingMode::DenseInteractive,
                topology,
                mixed_service_plus_hosts(&[1, 2, 3, 4, 5, 6]),
                vec![
                    BoundObjective::new(
                        shifted_node_id(1),
                        connected_objective(shifted_node_id(6)),
                    )
                    .with_activation_round(2),
                    mixed_service_objective(36),
                ],
                30,
            ),
            parameters,
        );
        let environment = ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                8,
                shifted_node_id(3),
                shifted_node_id(4),
                RatioPermille(520),
                RatioPermille(320),
                RatioPermille(700),
                RatioPermille(170),
            ),
            replace_topology_hook(14, &alternate),
        ]);
        return (scenario, environment);
    }
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
        ("core-periphery", LargePopulationSizeBand::Moderate) => {
            vec![(node_id(2), 920, 4, 5, Some(860))]
        }
        ("core-periphery", LargePopulationSizeBand::High) => {
            vec![(node_id(2), 940, 5, 6, Some(900))]
        }
        ("multi-bottleneck", LargePopulationSizeBand::Moderate) => vec![
            (node_id(2), 900, 4, 4, Some(840)),
            (node_id(3), 860, 4, 4, Some(820)),
        ],
        ("multi-bottleneck", LargePopulationSizeBand::High) => vec![
            (node_id(2), 920, 5, 5, Some(860)),
            (node_id(3), 880, 5, 5, Some(840)),
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

fn large_core_periphery_mixed_edges(size_band: LargePopulationSizeBand) -> &'static [(u8, u8)] {
    match size_band {
        LargePopulationSizeBand::Moderate => LARGE_CORE_PERIPHERY_MODERATE_MIXED_EDGES,
        LargePopulationSizeBand::High => LARGE_CORE_PERIPHERY_HIGH_MIXED_EDGES,
    }
}

fn large_bottleneck_mixed_edges(size_band: LargePopulationSizeBand) -> &'static [(u8, u8)] {
    match size_band {
        LargePopulationSizeBand::Moderate => LARGE_MULTI_BRIDGE_MID_MIXED_EDGES,
        LargePopulationSizeBand::High => LARGE_MULTI_BRIDGE_SEVERE_MIXED_EDGES,
    }
}

fn large_mixed_topology(
    size_band: LargePopulationSizeBand,
    edges: &[(u8, u8)],
    reachable_neighbor_count: u32,
) -> Observation<Configuration> {
    mixed_service_plus_topology(size_band.node_bytes(), edges, reachable_neighbor_count)
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

fn mixed_large_core_periphery_environment(
    size_band: LargePopulationSizeBand,
    alternate: &Configuration,
) -> ScriptedEnvironmentModel {
    match size_band {
        LargePopulationSizeBand::Moderate => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                9,
                shifted_node_id(5),
                shifted_node_id(6),
                RatioPermille(560),
                RatioPermille(260),
                RatioPermille(720),
                RatioPermille(150),
            ),
            replace_topology_hook(16, alternate),
            medium_degradation_hook(
                22,
                shifted_node_id(8),
                shifted_node_id(9),
                RatioPermille(620),
                RatioPermille(180),
            ),
        ]),
        LargePopulationSizeBand::High => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                10,
                shifted_node_id(6),
                shifted_node_id(7),
                RatioPermille(540),
                RatioPermille(280),
                RatioPermille(720),
                RatioPermille(150),
            ),
            replace_topology_hook(18, alternate),
            medium_degradation_hook(
                26,
                shifted_node_id(10),
                shifted_node_id(11),
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

// long-block-exception: mirrors the large-bottleneck hook schedule on shifted
// mixed-comparison node ids so the service-budget branch remains independent.
fn mixed_large_bottleneck_environment(
    size_band: LargePopulationSizeBand,
    alternate: &Configuration,
) -> ScriptedEnvironmentModel {
    match size_band {
        LargePopulationSizeBand::Moderate => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                8,
                shifted_node_id(4),
                shifted_node_id(5),
                RatioPermille(520),
                RatioPermille(310),
                RatioPermille(700),
                RatioPermille(180),
            ),
            intrinsic_limit_hook(10, shifted_node_id(4), 2, jacquard_core::ByteCount(320)),
            asymmetric_degradation_hook(
                13,
                shifted_node_id(7),
                shifted_node_id(8),
                RatioPermille(500),
                RatioPermille(340),
                RatioPermille(680),
                RatioPermille(190),
            ),
            intrinsic_limit_hook(15, shifted_node_id(7), 2, jacquard_core::ByteCount(320)),
            replace_topology_hook(18, alternate),
        ]),
        LargePopulationSizeBand::High => ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                8,
                shifted_node_id(4),
                shifted_node_id(5),
                RatioPermille(520),
                RatioPermille(320),
                RatioPermille(700),
                RatioPermille(180),
            ),
            intrinsic_limit_hook(9, shifted_node_id(4), 2, jacquard_core::ByteCount(320)),
            asymmetric_degradation_hook(
                12,
                shifted_node_id(8),
                shifted_node_id(9),
                RatioPermille(500),
                RatioPermille(340),
                RatioPermille(670),
                RatioPermille(190),
            ),
            intrinsic_limit_hook(13, shifted_node_id(8), 2, jacquard_core::ByteCount(256)),
            asymmetric_degradation_hook(
                16,
                shifted_node_id(12),
                shifted_node_id(13),
                RatioPermille(480),
                RatioPermille(360),
                RatioPermille(650),
                RatioPermille(220),
            ),
            replace_topology_hook(21, alternate),
            intrinsic_limit_hook(22, shifted_node_id(12), 1, jacquard_core::ByteCount(256)),
        ]),
    }
}

// long-block-exception: large-family builder keeps shifted mixed-only and standalone-compatible variants adjacent for auditability.
fn build_large_core_periphery(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
    size_band: LargePopulationSizeBand,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let (reachable_neighbor_count, contention, loss) = match size_band {
            LargePopulationSizeBand::Moderate => (4, RatioPermille(180), RatioPermille(120)),
            LargePopulationSizeBand::High => (5, RatioPermille(220), RatioPermille(140)),
        };
        let mut topology = large_mixed_topology(
            size_band,
            large_core_periphery_mixed_edges(size_band),
            reachable_neighbor_count,
        );
        set_environment(&mut topology, reachable_neighbor_count, contention, loss);
        restore_pathway_service_budget_branch(&mut topology);
        let alternate = match size_band {
            LargePopulationSizeBand::Moderate => shifted_alternate(&topology, &[(5, 6)], &[(3, 6)]),
            LargePopulationSizeBand::High => shifted_alternate(&topology, &[(6, 7)], &[(4, 7)]),
        };
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
                mixed_service_plus_hosts(size_band.node_bytes()),
                vec![
                    BoundObjective::new(
                        shifted_node_id(1),
                        connected_objective(shifted_node_id(large_population_destination_byte(
                            size_band,
                        ))),
                    )
                    .with_activation_round(large_population_activation_round(size_band)),
                    mixed_service_objective(match size_band {
                        LargePopulationSizeBand::Moderate => 40,
                        LargePopulationSizeBand::High => 41,
                    }),
                ],
                large_population_round_limit("core-periphery", size_band),
            ),
            parameters,
        );
        let environment = mixed_large_core_periphery_environment(size_band, &alternate);
        return (scenario, environment);
    }
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

// long-block-exception: large-family builder keeps shifted mixed-only and standalone-compatible variants adjacent for auditability.
fn build_large_bottleneck(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
    size_band: LargePopulationSizeBand,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let (reachable_neighbor_count, contention, loss) = match size_band {
            LargePopulationSizeBand::Moderate => (3, RatioPermille(200), RatioPermille(150)),
            LargePopulationSizeBand::High => (4, RatioPermille(240), RatioPermille(180)),
        };
        let mut topology = large_mixed_topology(
            size_band,
            large_bottleneck_mixed_edges(size_band),
            reachable_neighbor_count,
        );
        set_environment(&mut topology, reachable_neighbor_count, contention, loss);
        restore_pathway_service_budget_branch(&mut topology);
        let alternate = match size_band {
            LargePopulationSizeBand::Moderate => {
                shifted_alternate(&topology, &[], &[(3, 5), (6, 8)])
            }
            LargePopulationSizeBand::High => {
                shifted_alternate(&topology, &[], &[(3, 6), (7, 10), (11, 13)])
            }
        };
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
                mixed_service_plus_hosts(size_band.node_bytes()),
                vec![
                    BoundObjective::new(
                        shifted_node_id(1),
                        connected_objective(shifted_node_id(large_population_destination_byte(
                            size_band,
                        ))),
                    )
                    .with_activation_round(large_population_activation_round(size_band)),
                    mixed_service_objective(match size_band {
                        LargePopulationSizeBand::Moderate => 42,
                        LargePopulationSizeBand::High => 43,
                    }),
                ],
                large_population_round_limit("multi-bottleneck", size_band),
            ),
            parameters,
        );
        let environment = mixed_large_bottleneck_environment(size_band, &alternate);
        return (scenario, environment);
    }
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

fn comparison_hosts_for_bytes(
    bytes: &[u8],
    comparison_engine_set: Option<ComparisonEngineSet>,
    destination: &DestinationId,
    bootstrap: &[FieldBootstrapSeed],
    primary_profile: SelectedRoutingParameters,
) -> Vec<HostSpec> {
    let node_ids = node_ids(bytes);
    let primary = seed_standalone_field_bootstrap(
        comparison_host_spec(node_ids[0], comparison_engine_set).with_profile(primary_profile),
        comparison_engine_set,
        destination,
        bootstrap,
    );
    host_specs_with_primary(primary, &node_ids[1..], |node_id| {
        comparison_host_spec(node_id, comparison_engine_set)
    })
}

fn multi_flow_comparison_hosts_for_bytes(
    bytes: &[u8],
    comparison_engine_set: Option<ComparisonEngineSet>,
    owner_bootstraps: &[(NodeId, DestinationId, Vec<FieldBootstrapSeed>)],
    owner_profile: &SelectedRoutingParameters,
) -> Vec<HostSpec> {
    bytes
        .iter()
        .copied()
        .map(|byte| {
            let local_node_id = node_id(byte);
            let Some((_, destination, bootstrap)) = owner_bootstraps
                .iter()
                .find(|(owner_node_id, _, _)| *owner_node_id == local_node_id)
            else {
                return comparison_host_spec(local_node_id, comparison_engine_set);
            };
            let host = comparison_host_spec(local_node_id, comparison_engine_set)
                .with_profile(owner_profile.clone());
            seed_standalone_field_bootstrap(host, comparison_engine_set, destination, bootstrap)
        })
        .collect()
}

fn mixed_multi_flow_shared_corridor_hosts() -> Vec<HostSpec> {
    vec![
        HostSpec::pathway(node_id(1)).with_profile(best_effort_connected_profile()),
        HostSpec::pathway(node_id(2)),
        HostSpec::pathway(node_id(3)),
        HostSpec::pathway(node_id(4)),
        HostSpec::pathway(node_id(5)),
        comparison_host_spec(node_id(10), None).with_profile(repairable_connected_profile()),
        comparison_host_spec(node_id(11), None),
        comparison_host_spec(node_id(12), None),
        comparison_host_spec(node_id(13), None),
        comparison_host_spec(node_id(14), None),
        comparison_host_spec(node_id(15), None),
        comparison_host_spec(node_id(16), None),
        comparison_host_spec(node_id(17), None),
    ]
}

fn multi_flow_shared_corridor_topology(
    comparison_engine_set: Option<ComparisonEngineSet>,
) -> Observation<Configuration> {
    let edges = &[(1, 4), (2, 4), (3, 4), (4, 5), (5, 6), (5, 7), (5, 8)];
    topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[1, 2, 3, 4, 5, 6, 7, 8], comparison_engine_set),
        edges,
        3,
    )
}

fn mixed_multi_flow_shared_corridor_objectives() -> Vec<BoundObjective> {
    vec![
        BoundObjective::new(node_id(1), service_objective(vec![31; 16])).with_activation_round(2),
        BoundObjective::new(node_id(10), connected_objective(node_id(15))).with_activation_round(2),
        BoundObjective::new(node_id(11), connected_objective(node_id(16))).with_activation_round(2),
        BoundObjective::new(node_id(12), connected_objective(node_id(17))).with_activation_round(3),
    ]
}

fn mixed_multi_flow_shared_corridor_topology() -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (node_id(1), topology::node(1).pathway().build()),
            (node_id(2), topology::node(2).pathway().build()),
            (node_id(3), topology::node(3).pathway().build()),
            (node_id(4), topology::node(4).pathway().build()),
            (node_id(5), topology::node(5).pathway().build()),
            (node_id(10), topology::node(10).all_engines().build()),
            (node_id(11), topology::node(11).all_engines().build()),
            (node_id(12), topology::node(12).all_engines().build()),
            (node_id(13), topology::node(13).all_engines().build()),
            (node_id(14), topology::node(14).all_engines().build()),
            (node_id(15), topology::node(15).all_engines().build()),
            (node_id(16), topology::node(16).all_engines().build()),
            (node_id(17), topology::node(17).all_engines().build()),
        ]),
        links: BTreeMap::from([
            ((node_id(1), node_id(5)), topology::link(5).build()),
            ((node_id(5), node_id(1)), topology::link(1).build()),
            ((node_id(10), node_id(13)), topology::link(13).build()),
            ((node_id(13), node_id(10)), topology::link(10).build()),
            ((node_id(11), node_id(13)), topology::link(13).build()),
            ((node_id(13), node_id(11)), topology::link(11).build()),
            ((node_id(12), node_id(13)), topology::link(13).build()),
            ((node_id(13), node_id(12)), topology::link(12).build()),
            ((node_id(13), node_id(14)), topology::link(14).build()),
            ((node_id(14), node_id(13)), topology::link(13).build()),
            ((node_id(14), node_id(15)), topology::link(15).build()),
            ((node_id(15), node_id(14)), topology::link(14).build()),
            ((node_id(14), node_id(16)), topology::link(16).build()),
            ((node_id(16), node_id(14)), topology::link(14).build()),
            ((node_id(14), node_id(17)), topology::link(17).build()),
            ((node_id(17), node_id(14)), topology::link(14).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 3,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(110),
        },
    })
}

fn multi_flow_asymmetric_demand_topology(
    comparison_engine_set: Option<ComparisonEngineSet>,
) -> Observation<Configuration> {
    let edges = &[
        (1, 4),
        (2, 4),
        (3, 4),
        (4, 5),
        (5, 6),
        (6, 9),
        (5, 7),
        (5, 8),
    ];
    topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[1, 2, 3, 4, 5, 6, 7, 8, 9], comparison_engine_set),
        edges,
        3,
    )
}

fn mixed_multi_flow_asymmetric_demand_topology() -> Observation<Configuration> {
    let shifted_edges = &[
        (10, 13),
        (11, 13),
        (12, 13),
        (13, 14),
        (14, 15),
        (15, 18),
        (14, 16),
        (14, 17),
    ];
    let mut topology = topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[10, 11, 12, 13, 14, 15, 16, 17, 18], None),
        shifted_edges,
        3,
    );
    topology.value.nodes.extend(BTreeMap::from([
        (NODE_A, topology::node(1).pathway().build()),
        (NODE_B, topology::node(2).pathway().build()),
        (NODE_C, topology::node(3).pathway().build()),
        (NODE_D, topology::node(4).pathway().build()),
        (NODE_E, topology::node(5).pathway().build()),
    ]));
    topology
        .value
        .links
        .insert((NODE_A, NODE_E), topology::link(5).build());
    topology
        .value
        .links
        .insert((NODE_E, NODE_A), topology::link(1).build());
    topology
}

fn mixed_multi_flow_asymmetric_demand_hosts() -> Vec<HostSpec> {
    pathway_service_budget_branch_hosts()
        .into_iter()
        .chain(
            [10, 11, 12, 13, 14, 15, 16, 17, 18]
                .into_iter()
                .map(|byte| comparison_host_spec(node_id(byte), None)),
        )
        .collect()
}

fn mixed_multi_flow_asymmetric_demand_objectives() -> Vec<BoundObjective> {
    vec![
        BoundObjective::new(NODE_A, service_objective(vec![33; 16])).with_activation_round(3),
        BoundObjective::new(node_id(10), connected_objective(node_id(18))).with_activation_round(2),
        BoundObjective::new(node_id(11), connected_objective(node_id(17))).with_activation_round(2),
        BoundObjective::new(node_id(12), connected_objective(node_id(16))).with_activation_round(4),
    ]
}

fn multi_flow_detour_topology(
    comparison_engine_set: Option<ComparisonEngineSet>,
) -> Observation<Configuration> {
    let edges = &[
        (1, 4),
        (2, 4),
        (3, 4),
        (4, 5),
        (5, 6),
        (5, 7),
        (5, 8),
        (2, 9),
        (9, 10),
        (10, 7),
        (3, 10),
    ];
    topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            comparison_engine_set,
        ),
        edges,
        3,
    )
}

fn mixed_multi_flow_detour_topology() -> Observation<Configuration> {
    let shifted_edges = &[
        (10, 13),
        (11, 13),
        (12, 13),
        (13, 14),
        (14, 15),
        (14, 16),
        (14, 17),
        (11, 18),
        (18, 19),
        (19, 16),
        (12, 19),
    ];
    let mut topology = topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[10, 11, 12, 13, 14, 15, 16, 17, 18, 19], None),
        shifted_edges,
        3,
    );
    topology.value.nodes.extend(BTreeMap::from([
        (NODE_A, topology::node(1).pathway().build()),
        (NODE_B, topology::node(2).pathway().build()),
        (NODE_C, topology::node(3).pathway().build()),
        (NODE_D, topology::node(4).pathway().build()),
        (NODE_E, topology::node(5).pathway().build()),
    ]));
    topology
        .value
        .links
        .insert((NODE_A, NODE_E), topology::link(5).build());
    topology
        .value
        .links
        .insert((NODE_E, NODE_A), topology::link(1).build());
    topology
}

fn mixed_multi_flow_detour_hosts() -> Vec<HostSpec> {
    pathway_service_budget_branch_hosts()
        .into_iter()
        .chain(
            [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
                .into_iter()
                .map(|byte| comparison_host_spec(node_id(byte), None)),
        )
        .collect()
}

fn mixed_multi_flow_detour_objectives() -> Vec<BoundObjective> {
    vec![
        BoundObjective::new(NODE_A, service_objective(vec![34; 16])).with_activation_round(3),
        BoundObjective::new(node_id(10), connected_objective(node_id(15))).with_activation_round(2),
        BoundObjective::new(node_id(11), connected_objective(node_id(16))).with_activation_round(2),
        BoundObjective::new(node_id(12), connected_objective(node_id(17))).with_activation_round(2),
    ]
}

fn stale_bridge_topology(
    comparison_engine_set: Option<ComparisonEngineSet>,
) -> Observation<Configuration> {
    let edges = &[(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (3, 6)];
    topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&[1, 2, 3, 4, 5, 6], comparison_engine_set),
        edges,
        2,
    )
}

fn stale_bridge_alternate(topology: &Observation<Configuration>) -> Configuration {
    let mut alternate = topology.value.clone();
    alternate.links.remove(&(node_id(3), node_id(6)));
    alternate.links.remove(&(node_id(6), node_id(3)));
    alternate
        .links
        .insert((node_id(2), node_id(5)), crate::topology::link(5).build());
    alternate
        .links
        .insert((node_id(5), node_id(2)), crate::topology::link(2).build());
    alternate
}

const STALE_BRIDGE_EDGES: &[(u8, u8)] = &[(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (3, 6)];
const MEDIUM_BRIDGE_REPAIR_EDGES: &[(u8, u8)] = &[(1, 2), (2, 3), (3, 4), (4, 5), (5, 6)];
const LARGE_CORE_PERIPHERY_MODERATE_MIXED_EDGES: &[(u8, u8)] = &[
    (1, 2),
    (2, 3),
    (2, 4),
    (2, 5),
    (3, 4),
    (3, 5),
    (4, 5),
    (4, 6),
    (5, 6),
    (6, 7),
    (7, 8),
    (8, 9),
    (9, 10),
];
const LARGE_CORE_PERIPHERY_HIGH_MIXED_EDGES: &[(u8, u8)] = &[
    (1, 2),
    (2, 3),
    (2, 4),
    (2, 5),
    (2, 6),
    (3, 4),
    (3, 5),
    (3, 6),
    (4, 5),
    (4, 6),
    (5, 6),
    (5, 7),
    (6, 7),
    (7, 8),
    (8, 9),
    (9, 10),
    (10, 11),
    (11, 12),
    (12, 13),
    (13, 14),
];
const LARGE_MULTI_BRIDGE_MID_MIXED_EDGES: &[(u8, u8)] = &[
    (1, 2),
    (1, 3),
    (2, 3),
    (2, 4),
    (3, 4),
    (4, 5),
    (4, 6),
    (5, 6),
    (5, 7),
    (6, 7),
    (7, 8),
    (7, 9),
    (8, 9),
    (9, 10),
];
const LARGE_MULTI_BRIDGE_SEVERE_MIXED_EDGES: &[(u8, u8)] = &[
    (1, 2),
    (1, 3),
    (2, 3),
    (2, 4),
    (3, 4),
    (4, 5),
    (4, 6),
    (4, 7),
    (5, 6),
    (5, 7),
    (6, 7),
    (6, 8),
    (7, 8),
    (8, 9),
    (8, 10),
    (8, 11),
    (9, 10),
    (9, 11),
    (10, 11),
    (10, 12),
    (11, 12),
    (12, 13),
    (13, 14),
];

fn shifted_byte(byte: u8) -> u8 {
    byte.saturating_add(10)
}

fn shifted_node_id(byte: u8) -> NodeId {
    node_id(shifted_byte(byte))
}

fn shifted_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().map(shifted_byte).collect()
}

fn shifted_edges(edges: &[(u8, u8)]) -> Vec<(u8, u8)> {
    edges
        .iter()
        .map(|(left, right)| (shifted_byte(*left), shifted_byte(*right)))
        .collect()
}

fn mixed_service_plus_topology(
    bytes: &[u8],
    edges: &[(u8, u8)],
    reachable_neighbor_count: u32,
) -> Observation<Configuration> {
    let shifted_bytes = shifted_bytes(bytes);
    let shifted_edges = shifted_edges(edges);
    let mut topology = topology_from_byte_nodes_and_edges(
        comparison_topology_nodes_for_bytes(&shifted_bytes, None),
        &shifted_edges,
        reachable_neighbor_count,
    );
    topology.value.nodes.extend(BTreeMap::from([
        (NODE_A, topology::node(1).pathway().build()),
        (NODE_B, topology::node(2).pathway().build()),
        (NODE_C, topology::node(3).pathway().build()),
        (NODE_D, topology::node(4).pathway().build()),
        (NODE_E, topology::node(5).pathway().build()),
    ]));
    restore_pathway_service_budget_branch(&mut topology);
    topology
}

fn mixed_service_plus_hosts(bytes: &[u8]) -> Vec<HostSpec> {
    let shifted_bytes = shifted_bytes(bytes);
    pathway_service_budget_branch_hosts()
        .into_iter()
        .chain(
            shifted_bytes
                .into_iter()
                .map(|byte| comparison_host_spec(node_id(byte), None)),
        )
        .collect()
}

fn mixed_service_objective(service_byte: u8) -> BoundObjective {
    BoundObjective::new(NODE_A, service_objective(vec![service_byte; 16])).with_activation_round(3)
}

fn shifted_alternate(
    topology: &Observation<Configuration>,
    removals: &[(u8, u8)],
    additions: &[(u8, u8)],
) -> Configuration {
    let mut alternate = topology.value.clone();
    for (left, right) in removals {
        alternate
            .links
            .remove(&(shifted_node_id(*left), shifted_node_id(*right)));
        alternate
            .links
            .remove(&(shifted_node_id(*right), shifted_node_id(*left)));
    }
    for (left, right) in additions {
        alternate.links.insert(
            (shifted_node_id(*left), shifted_node_id(*right)),
            crate::topology::link(shifted_byte(*right)).build(),
        );
        alternate.links.insert(
            (shifted_node_id(*right), shifted_node_id(*left)),
            crate::topology::link(shifted_byte(*left)).build(),
        );
    }
    alternate
}

// Analytical question: when several equal-priority flows share the same narrow
// broker corridor, which engine sets keep the worst-flow route presence high
// instead of optimizing only the mean?

// long-block-exception: multi-flow builder keeps service-budget and standalone-compatible variants adjacent for auditability.
pub(super) fn build_comparison_multi_flow_shared_corridor(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_multi_flow_shared_corridor_topology();
        set_environment(&mut topology, 3, RatioPermille(180), RatioPermille(110));
        let scenario = route_visible_template(
            format!(
                "comparison-multi-flow-shared-corridor-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_multi_flow_shared_corridor_hosts(),
            mixed_multi_flow_shared_corridor_objectives(),
            16,
        )
        .into_scenario(parameters)
        .with_broker_nodes(vec![node_id(13), node_id(14), node_id(15)]);
        let environment = ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                6,
                node_id(13),
                node_id(14),
                RatioPermille(520),
                RatioPermille(320),
                RatioPermille(680),
                RatioPermille(180),
            ),
            intrinsic_limit_hook(10, node_id(14), 2, jacquard_core::ByteCount(320)),
        ]);
        return (scenario, environment);
    }
    let mut topology = multi_flow_shared_corridor_topology(comparison_engine_set);
    set_environment(&mut topology, 3, RatioPermille(180), RatioPermille(110));
    let scenario = route_visible_template(
        format!(
            "comparison-multi-flow-shared-corridor-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        multi_flow_comparison_hosts_for_bytes(
            &[1, 2, 3, 4, 5, 6, 7, 8],
            comparison_engine_set,
            &[
                (
                    node_id(1),
                    DestinationId::Node(node_id(6)),
                    vec![(node_id(4), 900, 2, 3, Some(840))],
                ),
                (
                    node_id(2),
                    DestinationId::Node(node_id(7)),
                    vec![(node_id(4), 880, 2, 3, Some(820))],
                ),
                (
                    node_id(3),
                    DestinationId::Node(node_id(8)),
                    vec![
                        (node_id(4), 900, 2, 3, Some(840)),
                        (node_id(5), 820, 1, 2, Some(760)),
                    ],
                ),
            ],
            &repairable_connected_profile(),
        ),
        vec![
            BoundObjective::new(node_id(1), connected_objective(node_id(6)))
                .with_activation_round(2),
            BoundObjective::new(node_id(2), connected_objective(node_id(7)))
                .with_activation_round(2),
            BoundObjective::new(node_id(3), connected_objective(node_id(8)))
                .with_activation_round(3),
        ],
        16,
    )
    .into_scenario(parameters)
    .with_broker_nodes(vec![node_id(4), node_id(5), node_id(6)]);
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            6,
            node_id(4),
            node_id(5),
            RatioPermille(520),
            RatioPermille(320),
            RatioPermille(680),
            RatioPermille(180),
        ),
        intrinsic_limit_hook(10, node_id(5), 2, jacquard_core::ByteCount(320)),
    ]);
    (scenario, environment)
}

// Analytical question: when one route is longer and more corridor-dependent
// than the others, does the candidate stack preserve acceptable tail behavior
// or let the hardest flow collapse first?

// long-block-exception: multi-flow builder keeps service-budget and standalone-compatible variants adjacent for auditability.
pub(super) fn build_comparison_multi_flow_asymmetric_demand(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_multi_flow_asymmetric_demand_topology();
        set_environment(&mut topology, 3, RatioPermille(200), RatioPermille(140));
        restore_pathway_service_budget_branch(&mut topology);
        let scenario = route_visible_template(
            format!(
                "comparison-multi-flow-asymmetric-demand-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_multi_flow_asymmetric_demand_hosts(),
            mixed_multi_flow_asymmetric_demand_objectives(),
            32,
        )
        .into_scenario(parameters)
        .with_broker_nodes(vec![node_id(13), node_id(14), node_id(18)]);
        let environment = ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                10,
                node_id(14),
                node_id(15),
                RatioPermille(520),
                RatioPermille(320),
                RatioPermille(700),
                RatioPermille(180),
            ),
            intrinsic_limit_hook(16, node_id(14), 2, jacquard_core::ByteCount(320)),
        ]);
        return (scenario, environment);
    }
    let mut topology = multi_flow_asymmetric_demand_topology(comparison_engine_set);
    set_environment(&mut topology, 3, RatioPermille(200), RatioPermille(140));
    let scenario = route_visible_template(
        format!(
            "comparison-multi-flow-asymmetric-demand-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        multi_flow_comparison_hosts_for_bytes(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9],
            comparison_engine_set,
            &[
                (
                    node_id(1),
                    DestinationId::Node(node_id(9)),
                    vec![(node_id(4), 900, 4, 4, Some(840))],
                ),
                (
                    node_id(2),
                    DestinationId::Node(node_id(8)),
                    vec![(node_id(4), 860, 3, 3, Some(800))],
                ),
                (
                    node_id(3),
                    DestinationId::Node(node_id(7)),
                    vec![(node_id(4), 820, 3, 3, Some(760))],
                ),
            ],
            &repairable_connected_profile(),
        ),
        vec![
            BoundObjective::new(node_id(1), connected_objective(node_id(9)))
                .with_activation_round(2),
            BoundObjective::new(node_id(2), connected_objective(node_id(8)))
                .with_activation_round(2),
            BoundObjective::new(node_id(3), connected_objective(node_id(7)))
                .with_activation_round(4),
        ],
        32,
    )
    .into_scenario(parameters)
    .with_broker_nodes(vec![node_id(4), node_id(5), node_id(9), node_id(10)]);
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            10,
            node_id(5),
            node_id(6),
            RatioPermille(520),
            RatioPermille(320),
            RatioPermille(700),
            RatioPermille(180),
        ),
        intrinsic_limit_hook(16, node_id(5), 2, jacquard_core::ByteCount(320)),
    ]);
    (scenario, environment)
}

// Analytical question: under shared-flow pressure with one viable detour path,
// which engine sets keep the minimum per-flow service acceptable rather than
// overcommitting to the stressed primary corridor?

// long-block-exception: multi-flow builder keeps service-budget and standalone-compatible variants adjacent for auditability.
pub(super) fn build_comparison_multi_flow_detour_choice(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_multi_flow_detour_topology();
        set_environment(&mut topology, 3, RatioPermille(190), RatioPermille(120));
        restore_pathway_service_budget_branch(&mut topology);
        let scenario = route_visible_template(
            format!(
                "comparison-multi-flow-detour-choice-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_multi_flow_detour_hosts(),
            mixed_multi_flow_detour_objectives(),
            30,
        )
        .into_scenario(parameters)
        .with_broker_nodes(vec![node_id(13), node_id(14), node_id(18), node_id(19)]);
        let environment = ScriptedEnvironmentModel::new(vec![
            asymmetric_degradation_hook(
                9,
                node_id(13),
                node_id(14),
                RatioPermille(500),
                RatioPermille(340),
                RatioPermille(680),
                RatioPermille(190),
            ),
            intrinsic_limit_hook(12, node_id(14), 1, jacquard_core::ByteCount(256)),
            medium_degradation_hook(
                18,
                node_id(18),
                node_id(19),
                RatioPermille(620),
                RatioPermille(220),
            ),
        ]);
        return (scenario, environment);
    }
    let mut topology = multi_flow_detour_topology(comparison_engine_set);
    set_environment(&mut topology, 3, RatioPermille(190), RatioPermille(120));
    let scenario = route_visible_template(
        format!(
            "comparison-multi-flow-detour-choice-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        multi_flow_comparison_hosts_for_bytes(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            comparison_engine_set,
            &[
                (
                    node_id(1),
                    DestinationId::Node(node_id(6)),
                    vec![(node_id(4), 900, 3, 4, Some(840))],
                ),
                (
                    node_id(2),
                    DestinationId::Node(node_id(7)),
                    vec![
                        (node_id(4), 820, 2, 3, Some(760)),
                        (node_id(9), 780, 2, 3, Some(720)),
                    ],
                ),
                (
                    node_id(3),
                    DestinationId::Node(node_id(8)),
                    vec![
                        (node_id(4), 800, 2, 3, Some(740)),
                        (node_id(10), 760, 2, 3, Some(700)),
                    ],
                ),
            ],
            &repairable_connected_profile(),
        ),
        vec![
            BoundObjective::new(node_id(1), connected_objective(node_id(6)))
                .with_activation_round(2),
            BoundObjective::new(node_id(2), connected_objective(node_id(7)))
                .with_activation_round(2),
            BoundObjective::new(node_id(3), connected_objective(node_id(8)))
                .with_activation_round(2),
        ],
        30,
    )
    .into_scenario(parameters)
    .with_broker_nodes(vec![node_id(4), node_id(5), node_id(9), node_id(10)]);
    let environment = ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            9,
            node_id(4),
            node_id(5),
            RatioPermille(500),
            RatioPermille(340),
            RatioPermille(680),
            RatioPermille(190),
        ),
        intrinsic_limit_hook(12, node_id(5), 1, jacquard_core::ByteCount(256)),
        medium_degradation_hook(
            18,
            node_id(9),
            node_id(10),
            RatioPermille(620),
            RatioPermille(220),
        ),
    ]);
    (scenario, environment)
}

fn stale_hosts(
    comparison_engine_set: Option<ComparisonEngineSet>,
    destination: &DestinationId,
    bootstrap: &[FieldBootstrapSeed],
) -> Vec<HostSpec> {
    comparison_hosts_for_bytes(
        &[1, 2, 3, 4, 5, 6],
        comparison_engine_set,
        destination,
        bootstrap,
        repairable_connected_profile(),
    )
}

// Analytical question: how much repair lag appears when topology changes are
// real but one side of the route sees them only after a deterministic delay?

// long-block-exception: stale-family builder keeps lagged mixed-only and standalone-compatible variants adjacent for auditability.
pub(super) fn build_comparison_stale_observation_delay(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_service_plus_topology(&[1, 2, 3, 4, 5, 6], STALE_BRIDGE_EDGES, 2);
        set_environment(&mut topology, 2, RatioPermille(180), RatioPermille(120));
        restore_pathway_service_budget_branch(&mut topology);
        let alternate = shifted_alternate(&topology, &[(3, 6)], &[(2, 5)]);
        let scenario = route_visible_template(
            format!(
                "comparison-stale-observation-delay-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_service_plus_hosts(&[1, 2, 3, 4, 5, 6]),
            vec![
                BoundObjective::new(shifted_node_id(1), connected_objective(shifted_node_id(6)))
                    .with_activation_round(2),
                mixed_service_objective(37),
            ],
            28,
        )
        .into_scenario(parameters)
        .with_topology_lags(vec![
            HostTopologyLag::new(shifted_node_id(1), 8, 12, 3),
            HostTopologyLag::new(shifted_node_id(2), 8, 12, 3),
            HostTopologyLag::new(shifted_node_id(3), 8, 12, 2),
        ]);
        let environment = ScriptedEnvironmentModel::new(vec![
            replace_topology_hook(8, &alternate),
            medium_degradation_hook(
                16,
                shifted_node_id(2),
                shifted_node_id(5),
                RatioPermille(620),
                RatioPermille(180),
            ),
        ]);
        return (scenario, environment);
    }
    let destination = DestinationId::Node(node_id(6));
    let bootstrap = [
        (node_id(2), 860, 3, 4, Some(800)),
        (node_id(3), 760, 2, 3, Some(700)),
    ];
    let mut topology = stale_bridge_topology(comparison_engine_set);
    set_environment(&mut topology, 2, RatioPermille(180), RatioPermille(120));
    let alternate = stale_bridge_alternate(&topology);
    let scenario = route_visible_template(
        format!(
            "comparison-stale-observation-delay-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        stale_hosts(comparison_engine_set, &destination, &bootstrap),
        vec![
            BoundObjective::new(node_id(1), connected_objective(node_id(6)))
                .with_activation_round(2),
        ],
        28,
    )
    .into_scenario(parameters)
    .with_topology_lags(vec![
        HostTopologyLag::new(node_id(1), 8, 12, 3),
        HostTopologyLag::new(node_id(2), 8, 12, 3),
        HostTopologyLag::new(node_id(3), 8, 12, 2),
    ]);
    let environment = ScriptedEnvironmentModel::new(vec![
        replace_topology_hook(8, &alternate),
        medium_degradation_hook(
            16,
            node_id(2),
            node_id(5),
            RatioPermille(620),
            RatioPermille(180),
        ),
    ]);
    (scenario, environment)
}

// Analytical question: when one region is operating on a stale topology while
// the far side has already converged, which engines overcommit longest before
// they repair or withdraw the route?

// long-block-exception: stale-family builder keeps lagged mixed-only and standalone-compatible variants adjacent for auditability.
pub(super) fn build_comparison_stale_asymmetric_region(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_service_plus_topology(&[1, 2, 3, 4, 5, 6], STALE_BRIDGE_EDGES, 2);
        set_environment(&mut topology, 2, RatioPermille(190), RatioPermille(120));
        restore_pathway_service_budget_branch(&mut topology);
        let alternate = shifted_alternate(&topology, &[(3, 6)], &[(2, 5)]);
        let scenario = route_visible_template(
            format!(
                "comparison-stale-asymmetric-region-{}",
                parameters.config_id
            ),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_service_plus_hosts(&[1, 2, 3, 4, 5, 6]),
            vec![
                BoundObjective::new(shifted_node_id(1), connected_objective(shifted_node_id(6)))
                    .with_activation_round(2),
                mixed_service_objective(38),
            ],
            28,
        )
        .into_scenario(parameters)
        .with_topology_lags(vec![
            HostTopologyLag::new(shifted_node_id(1), 8, 14, 4),
            HostTopologyLag::new(shifted_node_id(2), 8, 14, 4),
        ]);
        let environment = ScriptedEnvironmentModel::new(vec![
            replace_topology_hook(8, &alternate),
            asymmetric_degradation_hook(
                12,
                shifted_node_id(2),
                shifted_node_id(5),
                RatioPermille(540),
                RatioPermille(280),
                RatioPermille(700),
                RatioPermille(160),
            ),
        ]);
        return (scenario, environment);
    }
    let destination = DestinationId::Node(node_id(6));
    let bootstrap = [
        (node_id(2), 900, 3, 4, Some(840)),
        (node_id(3), 780, 2, 3, Some(720)),
    ];
    let mut topology = stale_bridge_topology(comparison_engine_set);
    set_environment(&mut topology, 2, RatioPermille(190), RatioPermille(120));
    let alternate = stale_bridge_alternate(&topology);
    let scenario = route_visible_template(
        format!(
            "comparison-stale-asymmetric-region-{}",
            parameters.config_id
        ),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        stale_hosts(comparison_engine_set, &destination, &bootstrap),
        vec![
            BoundObjective::new(node_id(1), connected_objective(node_id(6)))
                .with_activation_round(2),
        ],
        28,
    )
    .into_scenario(parameters)
    .with_topology_lags(vec![
        HostTopologyLag::new(node_id(1), 8, 14, 4),
        HostTopologyLag::new(node_id(2), 8, 14, 4),
    ]);
    let environment = ScriptedEnvironmentModel::new(vec![
        replace_topology_hook(8, &alternate),
        asymmetric_degradation_hook(
            12,
            node_id(2),
            node_id(5),
            RatioPermille(540),
            RatioPermille(280),
            RatioPermille(700),
            RatioPermille(160),
        ),
    ]);
    (scenario, environment)
}

// Analytical question: once stale-view pressure ends and the new corridor is
// stable again, which engines recover cleanly and which remain trapped in the
// stale decision longer than the topology warrants?

// long-block-exception: stale-family builder keeps lagged mixed-only and standalone-compatible variants adjacent for auditability.
pub(super) fn build_comparison_stale_recovery_window(
    parameters: &ExperimentParameterSet,
    seed: SimulationSeed,
) -> (JacquardScenario, ScriptedEnvironmentModel) {
    let comparison_engine_set = parameters.comparison_engine_set;
    if comparison_engine_set.is_none() {
        let mut topology = mixed_service_plus_topology(&[1, 2, 3, 4, 5, 6], STALE_BRIDGE_EDGES, 2);
        set_environment(&mut topology, 2, RatioPermille(180), RatioPermille(120));
        restore_pathway_service_budget_branch(&mut topology);
        let restore = topology.value.clone();
        let scenario = route_visible_template(
            format!("comparison-stale-recovery-window-{}", parameters.config_id),
            seed,
            jacquard_core::OperatingMode::DenseInteractive,
            topology,
            mixed_service_plus_hosts(&[1, 2, 3, 4, 5, 6]),
            vec![
                BoundObjective::new(shifted_node_id(1), connected_objective(shifted_node_id(6)))
                    .with_activation_round(2),
                mixed_service_objective(39),
            ],
            30,
        )
        .into_scenario(parameters)
        .with_topology_lags(vec![
            HostTopologyLag::new(shifted_node_id(1), 8, 11, 3),
            HostTopologyLag::new(shifted_node_id(2), 8, 11, 3),
            HostTopologyLag::new(shifted_node_id(3), 8, 11, 2),
        ]);
        let environment = ScriptedEnvironmentModel::new(vec![
            cascade_partition_hook(
                8,
                &[
                    (shifted_node_id(2), shifted_node_id(3)),
                    (shifted_node_id(3), shifted_node_id(2)),
                    (shifted_node_id(3), shifted_node_id(6)),
                    (shifted_node_id(6), shifted_node_id(3)),
                ],
            ),
            replace_topology_hook(18, &restore),
        ]);
        return (scenario, environment);
    }
    let destination = DestinationId::Node(node_id(6));
    let bootstrap = [
        (node_id(2), 900, 3, 4, Some(840)),
        (node_id(3), 780, 2, 3, Some(720)),
    ];
    let mut topology = stale_bridge_topology(comparison_engine_set);
    set_environment(&mut topology, 2, RatioPermille(180), RatioPermille(120));
    let restore = topology.value.clone();
    let scenario = route_visible_template(
        format!("comparison-stale-recovery-window-{}", parameters.config_id),
        seed,
        jacquard_core::OperatingMode::DenseInteractive,
        topology,
        stale_hosts(comparison_engine_set, &destination, &bootstrap),
        vec![
            BoundObjective::new(node_id(1), connected_objective(node_id(6)))
                .with_activation_round(2),
        ],
        30,
    )
    .into_scenario(parameters)
    .with_topology_lags(vec![
        HostTopologyLag::new(node_id(1), 8, 11, 3),
        HostTopologyLag::new(node_id(2), 8, 11, 3),
        HostTopologyLag::new(node_id(3), 8, 11, 2),
    ]);
    let environment = ScriptedEnvironmentModel::new(vec![
        cascade_partition_hook(
            8,
            &[
                (node_id(2), node_id(3)),
                (node_id(3), node_id(2)),
                (node_id(3), node_id(6)),
                (node_id(6), node_id(3)),
            ],
        ),
        replace_topology_hook(18, &restore),
    ]);
    (scenario, environment)
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
        (
            build_comparison_multi_flow_shared_corridor(parameters, seed).0,
            vec![2u32, 2u32, 3u32],
            16u32,
        ),
        (
            build_comparison_multi_flow_asymmetric_demand(parameters, seed).0,
            vec![2u32, 2u32, 4u32],
            32u32,
        ),
        (
            build_comparison_multi_flow_detour_choice(parameters, seed).0,
            vec![2u32, 2u32, 2u32],
            30u32,
        ),
        (
            build_comparison_stale_observation_delay(parameters, seed).0,
            vec![2u32],
            28u32,
        ),
        (
            build_comparison_stale_asymmetric_region(parameters, seed).0,
            vec![2u32],
            28u32,
        ),
        (
            build_comparison_stale_recovery_window(parameters, seed).0,
            vec![2u32],
            30u32,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
    use jacquard_core::Tick;
    use jacquard_olsrv2::OLSRV2_ENGINE_ID;
    use jacquard_traits::{RoutingEnvironmentModel, RoutingScenario, RoutingSimulator};

    use super::*;
    use crate::{
        JacquardSimulator, ReducedReplayView, ReferenceClientAdapter, SimulationExecutionLane,
    };

    fn sample_parameters() -> ExperimentParameterSet {
        ExperimentParameterSet::head_to_head(ComparisonEngineSet::Babel, Some((4, 2)), None, None)
    }

    type ComparisonBuilder =
        fn(&ExperimentParameterSet, SimulationSeed) -> (JacquardScenario, ScriptedEnvironmentModel);

    fn assert_mixed_service_budget_boundary(
        family: &'static str,
        builder: ComparisonBuilder,
        service_byte: u8,
    ) {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) = builder(&narrow, seed);
        let (wider_scenario, wider_environment) = builder(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination =
            DestinationId::Service(jacquard_core::ServiceId(vec![service_byte; 16]));
        let narrow_summary = summarize_replay(family, &narrow, &narrow_scenario, &narrow_reduced);
        let wider_summary = summarize_replay(family, &wider, &wider_scenario, &wider_reduced);

        assert!(!narrow_reduced.route_seen(NODE_A, &service_destination));
        assert!(wider_reduced.route_seen(NODE_A, &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 500);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
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

    fn reordered_objectives(
        scenario: &JacquardScenario,
        indices: &[usize],
        renamed_suffix: &str,
    ) -> JacquardScenario {
        let mut reordered = JacquardScenario::new(
            format!("{}-{renamed_suffix}", scenario.name()),
            scenario.seed(),
            scenario.deployment_profile().clone(),
            scenario.initial_configuration().clone(),
            scenario.hosts().to_vec(),
            indices
                .iter()
                .map(|index| scenario.bound_objectives()[*index].clone())
                .collect(),
            scenario.round_limit(),
        )
        .with_topology_lags(scenario.topology_lags().to_vec())
        .with_broker_nodes(scenario.broker_nodes().to_vec());
        if let Some(interval) = scenario.checkpoint_interval() {
            reordered = reordered.with_checkpoint_interval(interval);
        }
        reordered
    }

    fn route_rounds_by_objective(
        scenario: &JacquardScenario,
        reduced: &ReducedReplayView,
    ) -> BTreeMap<String, Vec<u32>> {
        scenario
            .bound_objectives()
            .iter()
            .map(|binding| {
                (
                    format!(
                        "{:?}:{:?}",
                        binding.owner_node_id, binding.objective.destination
                    ),
                    reduced.route_present_rounds(
                        binding.owner_node_id,
                        &binding.objective.destination,
                    ),
                )
            })
            .collect()
    }

    fn summarize_replay(
        family_id: &str,
        parameters: &ExperimentParameterSet,
        scenario: &JacquardScenario,
        reduced: &ReducedReplayView,
    ) -> ExperimentRunSummary {
        summarize_run(
            &ExperimentRunSpec {
                run_id: format!("summary-{family_id}-{}", scenario.seed().0),
                suite_id: "comparison-tests".to_string(),
                family_id: family_id.to_string(),
                engine_family: "head-to-head".to_string(),
                execution_lane: SimulationExecutionLane::FullStack,
                seed: scenario.seed(),
                regime: regime((
                    "test",
                    "low",
                    "low",
                    "none",
                    "static",
                    "none",
                    "repairable-connected",
                    0,
                )),
                parameters: parameters.clone(),
                world: ExperimentRunWorld::Prepared {
                    scenario: Box::new(scenario.clone()),
                    environment: ScriptedEnvironmentModel::default(),
                },
                model_case: None,
            },
            scenario,
            reduced,
        )
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
        let multi_flow_shared_corridor =
            build_comparison_multi_flow_shared_corridor(&parameters, seed);
        let multi_flow_asymmetric_demand =
            build_comparison_multi_flow_asymmetric_demand(&parameters, seed);
        let multi_flow_detour_choice = build_comparison_multi_flow_detour_choice(&parameters, seed);
        let stale_observation_delay = build_comparison_stale_observation_delay(&parameters, seed);
        let stale_asymmetric_region = build_comparison_stale_asymmetric_region(&parameters, seed);
        let stale_recovery_window = build_comparison_stale_recovery_window(&parameters, seed);

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
        assert_eq!(
            applied_hook_labels(&multi_flow_shared_corridor.0, &multi_flow_shared_corridor.1),
            vec![(6, "asymmetric-degradation"), (10, "intrinsic-limit")]
        );
        assert_eq!(
            applied_hook_labels(
                &multi_flow_asymmetric_demand.0,
                &multi_flow_asymmetric_demand.1
            ),
            vec![(10, "asymmetric-degradation"), (16, "intrinsic-limit")]
        );
        assert_eq!(
            applied_hook_labels(&multi_flow_detour_choice.0, &multi_flow_detour_choice.1),
            vec![
                (9, "asymmetric-degradation"),
                (12, "intrinsic-limit"),
                (18, "medium-degradation"),
            ]
        );
        assert_eq!(
            applied_hook_labels(&stale_observation_delay.0, &stale_observation_delay.1),
            vec![(8, "replace-topology"), (16, "medium-degradation")]
        );
        assert_eq!(
            applied_hook_labels(&stale_asymmetric_region.0, &stale_asymmetric_region.1),
            vec![(8, "replace-topology"), (12, "asymmetric-degradation")]
        );
        assert_eq!(
            applied_hook_labels(&stale_recovery_window.0, &stale_recovery_window.1),
            vec![(8, "cascade-partition"), (18, "replace-topology")]
        );
    }

    #[test]
    fn stale_families_document_topology_lag_windows() {
        let parameters = sample_parameters();
        let seed = SimulationSeed(41);
        let delay = build_comparison_stale_observation_delay(&parameters, seed).0;
        let asymmetric = build_comparison_stale_asymmetric_region(&parameters, seed).0;
        let recovery = build_comparison_stale_recovery_window(&parameters, seed).0;

        assert_eq!(
            delay
                .topology_lags()
                .iter()
                .map(|lag| (
                    lag.local_node_id,
                    lag.start_round,
                    lag.end_round_inclusive,
                    lag.lag_rounds
                ))
                .collect::<Vec<_>>(),
            vec![
                (node_id(1), 8, 12, 3),
                (node_id(2), 8, 12, 3),
                (node_id(3), 8, 12, 2),
            ]
        );
        assert_eq!(
            asymmetric
                .topology_lags()
                .iter()
                .map(|lag| (
                    lag.local_node_id,
                    lag.start_round,
                    lag.end_round_inclusive,
                    lag.lag_rounds
                ))
                .collect::<Vec<_>>(),
            vec![(node_id(1), 8, 14, 4), (node_id(2), 8, 14, 4)]
        );
        assert_eq!(
            recovery
                .topology_lags()
                .iter()
                .map(|lag| (
                    lag.local_node_id,
                    lag.start_round,
                    lag.end_round_inclusive,
                    lag.lag_rounds
                ))
                .collect::<Vec<_>>(),
            vec![
                (node_id(1), 8, 11, 3),
                (node_id(2), 8, 11, 3),
                (node_id(3), 8, 11, 2),
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
        let owner = node_id(10);
        let destination = DestinationId::Node(node_id(13));

        assert!(reduced.route_seen(owner, &destination));
        assert!(reduced.route_seen_with_engine(owner, &destination, &BATMAN_BELLMAN_ENGINE_ID));
        assert_eq!(
            reduced.first_round_with_engine(owner, &destination, &BATMAN_BELLMAN_ENGINE_ID),
            Some(2)
        );
    }

    #[test]
    fn mixed_comparison_high_loss_separates_service_budget_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_connected_high_loss(&narrow, seed);
        let (wider_scenario, wider_environment) =
            build_comparison_connected_high_loss(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![23; 16]));
        let narrow_summary = summarize_replay(
            "comparison-connected-high-loss",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-connected-high-loss",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert!(!narrow_reduced.route_seen(NODE_A, &service_destination));
        assert!(wider_reduced.route_seen(NODE_A, &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 500);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
        assert_eq!(narrow_summary.broker_participation_permille, Some(0));
        assert_eq!(wider_summary.broker_participation_permille, Some(0));
        assert_eq!(narrow_summary.broker_route_churn_count, Some(0));
        assert_eq!(wider_summary.broker_route_churn_count, Some(0));
    }

    #[test]
    fn mixed_comparison_partial_observability_is_not_masked_by_batman_bellman() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let (scenario, environment) =
            build_comparison_partial_observability_bridge(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let owner = node_id(10);
        let destination = DestinationId::Node(node_id(13));

        assert!(reduced.route_seen(owner, &destination));
        assert!(reduced.route_seen_with_engine(owner, &destination, &OLSRV2_ENGINE_ID));
        assert!(!reduced.route_seen_with_engine(owner, &destination, &BATMAN_BELLMAN_ENGINE_ID));
    }

    #[test]
    fn mixed_comparison_concurrent_family_records_real_engine_selections() {
        let parameters =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
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
    fn mixed_comparison_concurrent_family_separates_service_budget_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_concurrent_mixed(&narrow, seed);
        let (wider_scenario, wider_environment) = build_comparison_concurrent_mixed(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![13; 16]));
        let narrow_summary = summarize_replay(
            "comparison-concurrent-mixed",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-concurrent-mixed",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert!(!narrow_reduced.route_seen(NODE_B, &service_destination));
        assert!(wider_reduced.route_seen(NODE_B, &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 500);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
    }

    #[test]
    fn mixed_comparison_shared_corridor_separates_service_budget_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_multi_flow_shared_corridor(&narrow, seed);
        let (wider_scenario, wider_environment) =
            build_comparison_multi_flow_shared_corridor(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![31; 16]));
        let narrow_summary = summarize_replay(
            "comparison-multi-flow-shared-corridor",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-multi-flow-shared-corridor",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert!(!narrow_reduced.route_seen(node_id(1), &service_destination));
        assert!(wider_reduced.route_seen(node_id(1), &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 750);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
    }

    #[test]
    fn mixed_comparison_partial_observability_separates_service_budget_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_partial_observability_bridge(&narrow, seed);
        let (wider_scenario, wider_environment) =
            build_comparison_partial_observability_bridge(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![25; 16]));
        let narrow_summary = summarize_replay(
            "comparison-partial-observability-bridge",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-partial-observability-bridge",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert!(!narrow_reduced.route_seen(NODE_A, &service_destination));
        assert!(wider_reduced.route_seen(NODE_A, &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 500);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
    }

    #[test]
    fn mixed_comparison_asymmetric_demand_separates_service_budget_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_multi_flow_asymmetric_demand(&narrow, seed);
        let (wider_scenario, wider_environment) =
            build_comparison_multi_flow_asymmetric_demand(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![33; 16]));
        let narrow_summary = summarize_replay(
            "comparison-multi-flow-asymmetric-demand",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-multi-flow-asymmetric-demand",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert!(!narrow_reduced.route_seen(NODE_A, &service_destination));
        assert!(wider_reduced.route_seen(NODE_A, &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 750);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
    }

    #[test]
    fn mixed_comparison_detour_choice_separates_service_budget_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_multi_flow_detour_choice(&narrow, seed);
        let (wider_scenario, wider_environment) =
            build_comparison_multi_flow_detour_choice(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let service_destination = DestinationId::Service(jacquard_core::ServiceId(vec![34; 16]));
        let narrow_summary = summarize_replay(
            "comparison-multi-flow-detour-choice",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-multi-flow-detour-choice",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert!(!narrow_reduced.route_seen(NODE_A, &service_destination));
        assert!(wider_reduced.route_seen(NODE_A, &service_destination));
        assert_eq!(narrow_summary.activation_success_permille, 750);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(
            wider_summary.route_present_total_window_permille
                > narrow_summary.route_present_total_window_permille
        );
    }

    #[test]
    fn mixed_comparison_pathway_budget_boundary_separates_maintained_configs() {
        let narrow = ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let wider =
            ExperimentParameterSet::comparison(6, 3, 4, PathwaySearchHeuristicMode::HopLowerBound);
        let seed = SimulationSeed(41);
        let (narrow_scenario, narrow_environment) =
            build_comparison_pathway_budget_boundary(&narrow, seed);
        let (wider_scenario, wider_environment) =
            build_comparison_pathway_budget_boundary(&wider, seed);
        let narrow_reduced = run_reduced_replay(&narrow_scenario, &narrow_environment);
        let wider_reduced = run_reduced_replay(&wider_scenario, &wider_environment);
        let narrow_summary = summarize_replay(
            "comparison-pathway-budget-boundary",
            &narrow,
            &narrow_scenario,
            &narrow_reduced,
        );
        let wider_summary = summarize_replay(
            "comparison-pathway-budget-boundary",
            &wider,
            &wider_scenario,
            &wider_reduced,
        );

        assert_eq!(narrow_summary.activation_success_permille, 0);
        assert_eq!(narrow_summary.pathway_selected_rounds, 0);
        assert_eq!(wider_summary.activation_success_permille, 1000);
        assert!(wider_summary.route_present_total_window_permille > 0);
        assert!(wider_summary.pathway_selected_rounds > 0);
    }

    #[test]
    fn mixed_comparison_remaining_flat_families_separate_service_budget_configs() {
        let cases: [(&'static str, ComparisonBuilder, u8); 9] = [
            (
                "comparison-corridor-continuity-uncertainty",
                build_comparison_corridor_continuity_uncertainty,
                35,
            ),
            (
                "comparison-medium-bridge-repair",
                build_comparison_medium_bridge_repair,
                36,
            ),
            (
                "comparison-stale-observation-delay",
                build_comparison_stale_observation_delay,
                37,
            ),
            (
                "comparison-stale-asymmetric-region",
                build_comparison_stale_asymmetric_region,
                38,
            ),
            (
                "comparison-stale-recovery-window",
                build_comparison_stale_recovery_window,
                39,
            ),
            (
                "comparison-large-core-periphery-moderate",
                build_comparison_large_core_periphery_moderate,
                40,
            ),
            (
                "comparison-large-core-periphery-high",
                build_comparison_large_core_periphery_high,
                41,
            ),
            (
                "comparison-large-multi-bottleneck-moderate",
                build_comparison_large_multi_bottleneck_moderate,
                42,
            ),
            (
                "comparison-large-multi-bottleneck-high",
                build_comparison_large_multi_bottleneck_high,
                43,
            ),
        ];
        for (family, builder, service_byte) in cases {
            assert_mixed_service_budget_boundary(family, builder, service_byte);
        }
    }

    #[test]
    fn comparison_connected_high_loss_is_seed_stable_under_scripted_hooks() {
        let parameters =
            ExperimentParameterSet::comparison(4, 2, 3, PathwaySearchHeuristicMode::Zero);
        let first = build_comparison_connected_high_loss(&parameters, SimulationSeed(41));
        let second = build_comparison_connected_high_loss(&parameters, SimulationSeed(43));
        let first_reduced = run_reduced_replay(&first.0, &first.1);
        let second_reduced = run_reduced_replay(&second.0, &second.1);
        let owner = node_id(10);
        let destination = DestinationId::Node(node_id(13));

        assert_eq!(
            first_reduced.route_present_rounds(owner, &destination),
            second_reduced.route_present_rounds(owner, &destination),
        );
        assert_eq!(
            first_reduced.first_round_with_engine(owner, &destination, &BATMAN_BELLMAN_ENGINE_ID),
            second_reduced.first_round_with_engine(owner, &destination, &BATMAN_BELLMAN_ENGINE_ID),
        );
    }

    #[test]
    fn crossover_large_population_families_are_seed_stable_under_scripted_hooks() {
        let parameters = sample_parameters();
        let core_first =
            build_comparison_large_core_periphery_high(&parameters, SimulationSeed(41));
        let core_second =
            build_comparison_large_core_periphery_high(&parameters, SimulationSeed(43));
        let multi_first =
            build_comparison_large_multi_bottleneck_high(&parameters, SimulationSeed(41));
        let multi_second =
            build_comparison_large_multi_bottleneck_high(&parameters, SimulationSeed(43));
        let core_destination = DestinationId::Node(node_id(14));
        let multi_destination = DestinationId::Node(node_id(14));

        let core_first_reduced = run_reduced_replay(&core_first.0, &core_first.1);
        let core_second_reduced = run_reduced_replay(&core_second.0, &core_second.1);
        let multi_first_reduced = run_reduced_replay(&multi_first.0, &multi_first.1);
        let multi_second_reduced = run_reduced_replay(&multi_second.0, &multi_second.1);

        assert_eq!(
            applied_hook_labels(&core_first.0, &core_first.1),
            applied_hook_labels(&core_second.0, &core_second.1),
        );
        assert_eq!(
            core_first_reduced.route_present_rounds(NODE_A, &core_destination),
            core_second_reduced.route_present_rounds(NODE_A, &core_destination),
        );
        assert_eq!(
            applied_hook_labels(&multi_first.0, &multi_first.1),
            applied_hook_labels(&multi_second.0, &multi_second.1),
        );
        assert_eq!(
            multi_first_reduced.route_present_rounds(NODE_A, &multi_destination),
            multi_second_reduced.route_present_rounds(NODE_A, &multi_destination),
        );
    }

    // long-block-exception: this regression keeps the reordered-vs-original metric audit in one assertion flow.
    #[test]
    fn multi_flow_summary_metrics_are_stable_under_objective_reordering() {
        let parameters = sample_parameters();
        let (scenario, environment) =
            build_comparison_multi_flow_shared_corridor(&parameters, SimulationSeed(41));
        let reordered = reordered_objectives(&scenario, &[2, 1, 0], "reordered");
        let reduced = run_reduced_replay(&scenario, &environment);
        let reordered_reduced = run_reduced_replay(&reordered, &environment);
        let summary = summarize_replay(
            "head-to-head-multi-flow-shared-corridor",
            &parameters,
            &scenario,
            &reduced,
        );
        let reordered_summary = summarize_replay(
            "head-to-head-multi-flow-shared-corridor",
            &parameters,
            &reordered,
            &reordered_reduced,
        );

        assert_eq!(
            route_rounds_by_objective(&scenario, &reduced),
            route_rounds_by_objective(&reordered, &reordered_reduced),
        );
        assert_eq!(
            summary.objective_route_presence_min_permille,
            reordered_summary.objective_route_presence_min_permille,
        );
        assert_eq!(
            summary.objective_route_presence_max_permille,
            reordered_summary.objective_route_presence_max_permille,
        );
        assert_eq!(
            summary.objective_route_presence_spread,
            reordered_summary.objective_route_presence_spread,
        );
        assert_eq!(
            summary.objective_starvation_count,
            reordered_summary.objective_starvation_count,
        );
        assert_eq!(
            summary.concurrent_route_round_count,
            reordered_summary.concurrent_route_round_count,
        );
        assert_eq!(
            summary.broker_participation_permille,
            reordered_summary.broker_participation_permille,
        );
        assert_eq!(
            summary.broker_concentration_permille,
            reordered_summary.broker_concentration_permille,
        );
        assert_eq!(
            summary.broker_route_churn_count,
            reordered_summary.broker_route_churn_count,
        );
        assert_eq!(
            summary.route_observation_count,
            reordered_summary.route_observation_count,
        );
    }

    #[test]
    fn stale_recovery_window_summary_matches_hand_checked_replay_metrics() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        let (scenario, environment) =
            build_comparison_stale_recovery_window(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);
        let summary = summarize_replay(
            "head-to-head-stale-recovery-window",
            &parameters,
            &scenario,
            &reduced,
        );
        assert_eq!(summary.first_disruption_round_mean, Some(5));
        assert_eq!(summary.first_loss_round_mean, Some(5));
        assert_eq!(summary.stale_persistence_round_mean, Some(10));
        assert_eq!(summary.recovery_round_mean, Some(10));
        assert_eq!(summary.recovery_success_permille, 1000);
        assert_eq!(summary.unrecovered_after_loss_count, 0);
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
    fn standalone_scatter_medium_bridge_repair_does_not_expire_on_service_window_cliff() {
        let parameters = ExperimentParameterSet::scatter("balanced");
        let (scenario, environment) =
            build_comparison_medium_bridge_repair(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_F);
        assert!(reduced.route_seen(NODE_A, &destination));
        assert_eq!(
            reduced.first_round_without_route_after_presence(NODE_A, &destination),
            None,
            "scatter route rounds: {:?}",
            reduced.route_present_rounds(NODE_A, &destination)
        );
    }

    #[test]
    fn standalone_scatter_low_rate_transfer_threshold_separates_conservative_profile() {
        let balanced = ExperimentParameterSet::scatter("balanced");
        let conservative = ExperimentParameterSet::scatter("conservative");
        let degraded = ExperimentParameterSet::scatter("degraded-network");

        let (balanced_scenario, balanced_environment) =
            build_scatter_low_rate_transfer_threshold(&balanced, SimulationSeed(41));
        let (conservative_scenario, conservative_environment) =
            build_scatter_low_rate_transfer_threshold(&conservative, SimulationSeed(41));
        let (degraded_scenario, degraded_environment) =
            build_scatter_low_rate_transfer_threshold(&degraded, SimulationSeed(41));

        let balanced_summary = summarize_replay(
            "scatter-low-rate-transfer-threshold",
            &balanced,
            &balanced_scenario,
            &run_reduced_replay(&balanced_scenario, &balanced_environment),
        );
        let conservative_summary = summarize_replay(
            "scatter-low-rate-transfer-threshold",
            &conservative,
            &conservative_scenario,
            &run_reduced_replay(&conservative_scenario, &conservative_environment),
        );
        let degraded_summary = summarize_replay(
            "scatter-low-rate-transfer-threshold",
            &degraded,
            &degraded_scenario,
            &run_reduced_replay(&degraded_scenario, &degraded_environment),
        );

        assert!(balanced_summary.scatter_handoff_rounds > 0);
        assert_eq!(conservative_summary.scatter_handoff_rounds, 0);
        assert!(degraded_summary.scatter_handoff_rounds >= balanced_summary.scatter_handoff_rounds);
    }

    #[test]
    fn standalone_scatter_stability_window_threshold_separates_conservative_profile() {
        let balanced = ExperimentParameterSet::scatter("balanced");
        let conservative = ExperimentParameterSet::scatter("conservative");
        let degraded = ExperimentParameterSet::scatter("degraded-network");

        let (balanced_scenario, balanced_environment) =
            build_scatter_stability_window_threshold(&balanced, SimulationSeed(41));
        let (conservative_scenario, conservative_environment) =
            build_scatter_stability_window_threshold(&conservative, SimulationSeed(41));
        let (degraded_scenario, degraded_environment) =
            build_scatter_stability_window_threshold(&degraded, SimulationSeed(41));

        let balanced_summary = summarize_replay(
            "scatter-stability-window-threshold",
            &balanced,
            &balanced_scenario,
            &run_reduced_replay(&balanced_scenario, &balanced_environment),
        );
        let conservative_summary = summarize_replay(
            "scatter-stability-window-threshold",
            &conservative,
            &conservative_scenario,
            &run_reduced_replay(&conservative_scenario, &conservative_environment),
        );
        let degraded_summary = summarize_replay(
            "scatter-stability-window-threshold",
            &degraded,
            &degraded_scenario,
            &run_reduced_replay(&degraded_scenario, &degraded_environment),
        );

        assert!(balanced_summary.scatter_handoff_rounds > 0);
        assert_eq!(conservative_summary.scatter_handoff_rounds, 0);
        assert!(degraded_summary.scatter_handoff_rounds >= balanced_summary.scatter_handoff_rounds);
    }

    #[test]
    fn standalone_scatter_conservative_constrained_threshold_is_not_flat() {
        let balanced = ExperimentParameterSet::scatter("balanced");
        let conservative = ExperimentParameterSet::scatter("conservative");
        let degraded = ExperimentParameterSet::scatter("degraded-network");

        let (balanced_scenario, balanced_environment) =
            build_scatter_conservative_constrained_threshold(&balanced, SimulationSeed(41));
        let (conservative_scenario, conservative_environment) =
            build_scatter_conservative_constrained_threshold(&conservative, SimulationSeed(41));
        let (degraded_scenario, degraded_environment) =
            build_scatter_conservative_constrained_threshold(&degraded, SimulationSeed(41));

        let balanced_summary = summarize_replay(
            "scatter-conservative-constrained-threshold",
            &balanced,
            &balanced_scenario,
            &run_reduced_replay(&balanced_scenario, &balanced_environment),
        );
        let conservative_summary = summarize_replay(
            "scatter-conservative-constrained-threshold",
            &conservative,
            &conservative_scenario,
            &run_reduced_replay(&conservative_scenario, &conservative_environment),
        );
        let degraded_summary = summarize_replay(
            "scatter-conservative-constrained-threshold",
            &degraded,
            &degraded_scenario,
            &run_reduced_replay(&degraded_scenario, &degraded_environment),
        );

        assert_eq!(balanced_summary.scatter_constrained_rounds, 0);
        assert!(conservative_summary.scatter_constrained_rounds > 0);
        assert_eq!(degraded_summary.scatter_constrained_rounds, 0);
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
    fn mercator_connected_smoke_connected_low_loss_activates_route() {
        let parameters =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Mercator, None, None, None);
        let (scenario, environment) =
            build_comparison_connected_low_loss(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_C);
        assert!(
            reduced.route_seen(NODE_A, &destination),
            "mercator connected-low-loss failed with summaries: {:?}",
            reduced.failure_summaries,
        );
        assert!(reduced.route_seen_with_engine(NODE_A, &destination, &MERCATOR_ENGINE_ID));
    }

    #[test]
    fn mercator_connected_smoke_bridge_transition_activates_route() {
        let parameters =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Mercator, None, None, None);
        let (scenario, environment) =
            build_comparison_bridge_transition(&parameters, SimulationSeed(41));
        let reduced = run_reduced_replay(&scenario, &environment);

        let destination = DestinationId::Node(NODE_D);
        assert!(
            reduced.route_seen(NODE_A, &destination),
            "mercator bridge-transition failed with summaries: {:?}",
            reduced.failure_summaries,
        );
        assert!(reduced.route_seen_with_engine(NODE_A, &destination, &MERCATOR_ENGINE_ID));
    }

    #[test]
    fn mercator_connected_smoke_matches_pathway_on_fixed_connected_fixture() {
        let mercator =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Mercator, None, None, None);
        let pathway =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Pathway, None, None, None);
        let mercator_reduced = {
            let (scenario, environment) =
                build_comparison_connected_low_loss(&mercator, SimulationSeed(41));
            run_reduced_replay(&scenario, &environment)
        };
        let pathway_reduced = {
            let (scenario, environment) =
                build_comparison_connected_low_loss(&pathway, SimulationSeed(41));
            run_reduced_replay(&scenario, &environment)
        };

        let destination = DestinationId::Node(NODE_C);
        assert!(pathway_reduced.route_seen(NODE_A, &destination));
        assert!(pathway_reduced.route_seen_with_engine(NODE_A, &destination, &PATHWAY_ENGINE_ID));
        assert!(mercator_reduced.route_seen(NODE_A, &destination));
        assert!(mercator_reduced.route_seen_with_engine(NODE_A, &destination, &MERCATOR_ENGINE_ID));
    }

    #[test]
    fn head_to_head_mercator_stale_families_emit_repair_rows() {
        let parameters =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Mercator, None, None, None);
        let destination = DestinationId::Node(node_id(6));
        let cases: [(&str, ComparisonBuilder); 3] = [
            (
                "head-to-head-stale-observation-delay",
                build_comparison_stale_observation_delay,
            ),
            (
                "head-to-head-stale-asymmetric-region",
                build_comparison_stale_asymmetric_region,
            ),
            (
                "head-to-head-stale-recovery-window",
                build_comparison_stale_recovery_window,
            ),
        ];
        for (family, builder) in cases {
            let (scenario, environment) = builder(&parameters, SimulationSeed(41));
            let reduced = run_reduced_replay(&scenario, &environment);
            let summary = summarize_replay(family, &parameters, &scenario, &reduced);

            assert!(
                reduced.route_seen(NODE_A, &destination),
                "{family} did not materialize a Mercator route: {:?}",
                reduced.failure_summaries,
            );
            assert!(reduced.route_seen_with_engine(NODE_A, &destination, &MERCATOR_ENGINE_ID));
            assert!(
                summary.route_present_total_window_permille > 0,
                "{family} did not emit route-visible stale-family summary rows"
            );
        }
    }

    #[test]
    fn head_to_head_mercator_multi_flow_families_avoid_zero_service_tails() {
        let parameters =
            ExperimentParameterSet::head_to_head(ComparisonEngineSet::Mercator, None, None, None);
        let cases: [(&str, ComparisonBuilder); 3] = [
            (
                "head-to-head-multi-flow-shared-corridor",
                build_comparison_multi_flow_shared_corridor,
            ),
            (
                "head-to-head-multi-flow-asymmetric-demand",
                build_comparison_multi_flow_asymmetric_demand,
            ),
            (
                "head-to-head-multi-flow-detour-choice",
                build_comparison_multi_flow_detour_choice,
            ),
        ];
        for (family, builder) in cases {
            let (scenario, environment) = builder(&parameters, SimulationSeed(41));
            let reduced = run_reduced_replay(&scenario, &environment);
            for binding in scenario.bound_objectives() {
                assert!(
                    reduced.route_seen(binding.owner_node_id, &binding.objective.destination),
                    "{family} has a zero-service Mercator tail for {:?}: {:?}",
                    binding.objective.destination,
                    reduced.failure_summaries,
                );
                assert!(reduced.route_seen_with_engine(
                    binding.owner_node_id,
                    &binding.objective.destination,
                    &MERCATOR_ENGINE_ID
                ));
            }
        }
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
        let large_owner = shifted_node_id(1);
        let large_destination = DestinationId::Node(shifted_node_id(14));

        assert!(large_reduced.route_seen(large_owner, &large_destination));
        assert!(
            large_reduced.first_round_with_route(large_owner, &large_destination)
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
        let owner = shifted_node_id(1);
        let destination = DestinationId::Node(shifted_node_id(14));
        let _present_rounds = reduced.route_present_rounds(owner, &destination);

        assert!(reduced.route_seen(owner, &destination));
        assert!(
            reduced
                .first_round_without_route_after_presence(owner, &destination)
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
        let first_loss_round = reduced
            .first_round_without_route_after_presence(NODE_A, &destination)
            .expect("large Field route should expose the post-loss replay window");

        assert!(reduced.route_seen(NODE_A, &destination));
        assert!(!reduced
            .route_present_rounds(NODE_A, &destination)
            .is_empty());
        assert!(
            reduced
                .rounds
                .iter()
                .filter(|round| round.round_index >= first_loss_round)
                .all(|round| {
                    let route_active = round.active_routes.iter().any(|route| {
                        route.owner_node_id == NODE_A && route.destination == destination
                    });
                    let selected_result = round.field_replays.iter().any(|field| {
                        field.local_node_id == NODE_A && field.summary.selected_result_present
                    });
                    route_active || !selected_result
                }),
            "Field selected-result replay must not leak stale planner records after route loss",
        );
        assert!(
            reduced.failure_class_counts().no_candidate > 0
                || reduced.failure_class_counts().inadmissible_candidate > 0,
            "large Field route loss should surface bounded reactivation failures",
        );
    }

    #[test]
    fn large_population_field_bootstrap_uses_owner_adjacent_neighbors() {
        let parameters = ExperimentParameterSet::head_to_head_field_low_churn();
        for build in [
            build_comparison_large_core_periphery_moderate,
            build_comparison_large_core_periphery_high,
            build_comparison_large_multi_bottleneck_moderate,
            build_comparison_large_multi_bottleneck_high,
        ] {
            let (scenario, _) = build(&parameters, SimulationSeed(41));
            let primary = scenario
                .hosts()
                .iter()
                .find(|host| host.local_node_id == NODE_A)
                .expect("large-population Field scenario should include the owner host");
            assert!(
                !primary.overrides.field_bootstrap_summaries.is_empty(),
                "{} should seed Field bootstrap evidence",
                scenario.name()
            );
            for summary in &primary.overrides.field_bootstrap_summaries {
                assert!(
                    scenario
                        .initial_configuration()
                        .value
                        .links
                        .contains_key(&(NODE_A, summary.from_neighbor)),
                    "{} bootstraps through non-adjacent {:?}",
                    scenario.name(),
                    summary.from_neighbor
                );
            }
        }
    }

    #[test]
    fn head_to_head_multi_flow_owners_are_primary_profiled_and_field_seeded() {
        for build in [
            build_comparison_multi_flow_shared_corridor,
            build_comparison_multi_flow_asymmetric_demand,
            build_comparison_multi_flow_detour_choice,
        ] {
            let pathway_parameters = ExperimentParameterSet::head_to_head(
                ComparisonEngineSet::Pathway,
                None,
                None,
                None,
            );
            let (pathway_scenario, _) = build(&pathway_parameters, SimulationSeed(41));
            for binding in pathway_scenario.bound_objectives() {
                let host = pathway_scenario
                    .hosts()
                    .iter()
                    .find(|host| host.local_node_id == binding.owner_node_id)
                    .expect("multi-flow owner should have a host");
                assert!(
                    host.overrides.routing_profile.is_some(),
                    "{} owner {:?} should use the primary routing profile",
                    pathway_scenario.name(),
                    binding.owner_node_id
                );
            }

            let field_parameters = ExperimentParameterSet::head_to_head_field_low_churn();
            let (field_scenario, _) = build(&field_parameters, SimulationSeed(41));
            for binding in field_scenario.bound_objectives() {
                let host = field_scenario
                    .hosts()
                    .iter()
                    .find(|host| host.local_node_id == binding.owner_node_id)
                    .expect("multi-flow owner should have a host");
                assert!(
                    host.overrides
                        .field_bootstrap_summaries
                        .iter()
                        .any(|summary| { summary.destination == binding.objective.destination }),
                    "{} owner {:?} should be seeded for {:?}",
                    field_scenario.name(),
                    binding.owner_node_id,
                    binding.objective.destination
                );
            }
        }
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
