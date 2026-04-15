//! Shared topology builders, environment helpers, and parameter override application.

use super::*;

pub(super) fn apply_overrides(
    scenario: &JacquardScenario,
    parameters: &ExperimentParameterSet,
) -> JacquardScenario {
    let hosts = scenario
        .hosts()
        .iter()
        .cloned()
        .map(|mut host| {
            if let Some(decay_window) = parameters.batman_bellman_decay_window() {
                host = host.with_batman_bellman_decay_window(decay_window);
            }
            if let Some(decay_window) = parameters.batman_classic_decay_window() {
                host = host.with_batman_classic_decay_window(decay_window);
            }
            if let Some(decay_window) = parameters.babel_decay_window() {
                host = host.with_babel_decay_window(decay_window);
            }
            if let Some(decay_window) = parameters.olsrv2_decay_window() {
                host = host.with_olsrv2_decay_window(decay_window);
            }
            if let Some(search_config) = parameters.pathway_search_config() {
                host = host.with_pathway_search_config(search_config);
            }
            if let Some(search_config) = parameters.field_search_config() {
                host = host.with_field_search_config(search_config);
            }
            host
        })
        .collect::<Vec<_>>();
    JacquardScenario::new(
        scenario.name().to_string(),
        scenario.seed(),
        scenario.deployment_profile().clone(),
        scenario.initial_configuration().clone(),
        hosts,
        scenario.bound_objectives().to_vec(),
        scenario.round_limit(),
    )
    .with_seed(scenario.seed())
}

pub(super) fn comparison_topology_node(node_byte: u8, comparison_engine_set: Option<&str>) -> Node {
    match comparison_engine_set.unwrap_or("all-engines") {
        "batman-bellman" => topology::node(node_byte).batman_bellman().build(),
        "batman-classic" => topology::node(node_byte).batman_classic().build(),
        "babel" => topology::node(node_byte).babel().build(),
        "olsrv2" => topology::node(node_byte).olsrv2().build(),
        "pathway" => topology::node(node_byte).pathway().build(),
        "field" => topology::node(node_byte).field().build(),
        "pathway-batman-bellman" => topology::node(node_byte)
            .pathway_and_batman_bellman()
            .build(),
        _ => topology::node(node_byte).all_engines().build(),
    }
}

pub(super) fn comparison_host_spec(
    local_node_id: NodeId,
    comparison_engine_set: Option<&str>,
) -> HostSpec {
    match comparison_engine_set.unwrap_or("all-engines") {
        "batman-bellman" => HostSpec::batman_bellman(local_node_id),
        "batman-classic" => HostSpec::batman_classic(local_node_id),
        "babel" => HostSpec::babel(local_node_id),
        "olsrv2" => HostSpec::olsrv2(local_node_id),
        "pathway" => HostSpec::pathway(local_node_id),
        "field" => HostSpec::field(local_node_id),
        "pathway-batman-bellman" => HostSpec::pathway_and_batman_bellman(local_node_id),
        _ => HostSpec::all_engines(local_node_id),
    }
}

pub(super) fn with_field_bootstrap_summaries(
    host: HostSpec,
    destination: &DestinationId,
    summaries: &[FieldBootstrapSeed],
) -> HostSpec {
    summaries.iter().fold(host, |host, summary| {
        host.with_field_bootstrap_summary(field_bootstrap_summary(
            destination.clone(),
            summary.0,
            summary.1,
            summary.2,
            summary.3,
            summary.4,
        ))
    })
}

pub(super) fn field_hosts_with_bootstrap(
    destination: &DestinationId,
    summaries: &[FieldBootstrapSeed],
    peer_node_ids: &[NodeId],
) -> Vec<HostSpec> {
    std::iter::once(with_field_bootstrap_summaries(
        HostSpec::field(NODE_A),
        destination,
        summaries,
    ))
    .chain(peer_node_ids.iter().copied().map(HostSpec::field))
    .collect()
}

pub(super) fn comparison_hosts_with_bootstrap(
    comparison_engine_set: Option<&str>,
    destination: &DestinationId,
    summaries: &[FieldBootstrapSeed],
    primary_host: HostSpec,
    peer_node_ids: &[NodeId],
) -> Vec<HostSpec> {
    std::iter::once(with_field_bootstrap_summaries(
        primary_host,
        destination,
        summaries,
    ))
    .chain(
        peer_node_ids
            .iter()
            .copied()
            .map(|node_id| comparison_host_spec(node_id, comparison_engine_set)),
    )
    .collect()
}

pub(super) fn host_specs<F>(node_ids: &[NodeId], host_builder: F) -> Vec<HostSpec>
where
    F: Fn(NodeId) -> HostSpec,
{
    node_ids.iter().copied().map(host_builder).collect()
}

pub(super) fn host_specs_with_primary<F>(
    primary_host: HostSpec,
    peer_node_ids: &[NodeId],
    host_builder: F,
) -> Vec<HostSpec>
where
    F: Fn(NodeId) -> HostSpec,
{
    std::iter::once(primary_host)
        .chain(peer_node_ids.iter().copied().map(host_builder))
        .collect()
}

pub(super) fn seed_standalone_field_bootstrap(
    host: HostSpec,
    comparison_engine_set: Option<&str>,
    destination: &DestinationId,
    summaries: &[FieldBootstrapSeed],
) -> HostSpec {
    if comparison_engine_set != Some("field") {
        return host;
    }
    with_field_bootstrap_summaries(host, destination, summaries)
}

pub(super) fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

pub(super) fn average_u32<I>(iter: I) -> u32
where
    I: Iterator<Item = u32>,
{
    let values = iter.collect::<Vec<_>>();
    if values.is_empty() {
        return 0;
    }
    let sum = values
        .iter()
        .fold(0u64, |acc, value| acc.saturating_add(u64::from(*value)));
    u32::try_from(sum / u64::try_from(values.len()).unwrap_or(1)).unwrap_or(u32::MAX)
}

pub(super) fn average_option_u32(values: &[Option<u32>]) -> Option<u32> {
    average_option_u32_from_iter(values.iter().copied())
}

pub(super) fn average_option_u32_from_iter<I>(iter: I) -> Option<u32>
where
    I: Iterator<Item = Option<u32>>,
{
    let values = iter.flatten().collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(average_u32(values.into_iter()))
}

pub(super) fn median_u32(values: &[u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    Some(sorted[sorted.len() / 2])
}

pub(super) fn mode<I>(iter: I) -> Option<String>
where
    I: Iterator<Item = String>,
{
    let mut counts = BTreeMap::new();
    for value in iter {
        *counts.entry(value).or_insert(0u32) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(value, count)| (*count, value.clone()))
        .map(|(value, _)| value)
}

pub(super) fn engine_round_counts(reduced: &ReducedReplayView) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for round in &reduced.rounds {
        let engines = round
            .active_routes
            .iter()
            .map(|route| normalized_engine_id(&route.engine_id))
            .collect::<BTreeSet<_>>();
        for engine in engines {
            *counts.entry(engine.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

pub(super) fn dominant_engine(engine_counts: &BTreeMap<String, u32>) -> Option<String> {
    engine_counts
        .iter()
        .max_by_key(|(engine, count)| (**count, engine.as_str()))
        .map(|(engine, _)| engine.clone())
}

pub(super) fn heuristic_mode_label(mode: PathwaySearchHeuristicMode) -> &'static str {
    match mode {
        PathwaySearchHeuristicMode::Zero => "zero",
        PathwaySearchHeuristicMode::HopLowerBound => "hop-lower-bound",
    }
}

pub(super) fn heuristic_mode_from_str(label: &str) -> PathwaySearchHeuristicMode {
    match label {
        "hop-lower-bound" => PathwaySearchHeuristicMode::HopLowerBound,
        _ => PathwaySearchHeuristicMode::Zero,
    }
}

pub(super) fn field_heuristic_mode_label(mode: FieldSearchHeuristicMode) -> &'static str {
    match mode {
        FieldSearchHeuristicMode::Zero => "zero",
        FieldSearchHeuristicMode::HopLowerBound => "hop-lower-bound",
    }
}

pub(super) fn field_heuristic_mode_from_str(label: &str) -> FieldSearchHeuristicMode {
    match label {
        "hop-lower-bound" => FieldSearchHeuristicMode::HopLowerBound,
        _ => FieldSearchHeuristicMode::Zero,
    }
}

pub(super) fn normalized_engine_id(engine_id: &jacquard_core::RoutingEngineId) -> &'static str {
    if engine_id == &BATMAN_BELLMAN_ENGINE_ID {
        "batman-bellman"
    } else if engine_id == &BATMAN_CLASSIC_ENGINE_ID {
        "batman-classic"
    } else if engine_id == &BABEL_ENGINE_ID {
        "babel"
    } else if engine_id == &OLSRV2_ENGINE_ID {
        "olsrv2"
    } else if engine_id == &PATHWAY_ENGINE_ID {
        "pathway"
    } else if engine_id == &FIELD_ENGINE_ID {
        "field"
    } else {
        "other"
    }
}

pub(super) fn set_environment(
    topology: &mut Observation<Configuration>,
    reachable_neighbor_count: u32,
    contention_permille: RatioPermille,
    loss_permille: RatioPermille,
) {
    topology.value.environment = Environment {
        reachable_neighbor_count,
        churn_permille: RatioPermille(0),
        contention_permille,
    };
    for link in topology.value.links.values_mut() {
        link.state.loss_permille = loss_permille;
        link.state.delivery_confidence_permille = Belief::certain(
            RatioPermille(950u16.saturating_sub(loss_permille.0 / 2)),
            topology.observed_at_tick,
        );
    }
}

pub(super) fn replace_topology_hook(
    round_index: u64,
    configuration: &Configuration,
) -> ScheduledEnvironmentHook {
    ScheduledEnvironmentHook::new(
        Tick(round_index),
        EnvironmentHook::ReplaceTopology {
            configuration: configuration.clone(),
        },
    )
}

pub(super) fn mobility_relink_hook(
    round_index: u64,
    left: NodeId,
    from_right: NodeId,
    to_right: NodeId,
    link_byte: u8,
) -> ScheduledEnvironmentHook {
    ScheduledEnvironmentHook::new(
        Tick(round_index),
        EnvironmentHook::MobilityRelink {
            left,
            from_right,
            to_right,
            link: Box::new(topology::link(link_byte).build()),
        },
    )
}

pub(super) fn intrinsic_limit_hook(
    round_index: u64,
    node_id: NodeId,
    connection_count_max: u32,
    hold_capacity_bytes_max: jacquard_core::ByteCount,
) -> ScheduledEnvironmentHook {
    ScheduledEnvironmentHook::new(
        Tick(round_index),
        EnvironmentHook::IntrinsicLimit {
            node_id,
            connection_count_max,
            hold_capacity_bytes_max,
        },
    )
}

pub(super) fn medium_degradation_hook(
    round_index: u64,
    left: NodeId,
    right: NodeId,
    confidence: RatioPermille,
    loss: RatioPermille,
) -> ScheduledEnvironmentHook {
    ScheduledEnvironmentHook::new(
        Tick(round_index),
        EnvironmentHook::MediumDegradation {
            left,
            right,
            confidence,
            loss,
        },
    )
}

pub(super) fn asymmetric_degradation_hook(
    round_index: u64,
    left: NodeId,
    right: NodeId,
    forward_confidence: RatioPermille,
    forward_loss: RatioPermille,
    reverse_confidence: RatioPermille,
    reverse_loss: RatioPermille,
) -> ScheduledEnvironmentHook {
    ScheduledEnvironmentHook::new(
        Tick(round_index),
        EnvironmentHook::AsymmetricDegradation {
            left,
            right,
            forward_confidence,
            forward_loss,
            reverse_confidence,
            reverse_loss,
        },
    )
}

pub(super) fn cascade_partition_hook(
    round_index: u64,
    cuts: &[(NodeId, NodeId)],
) -> ScheduledEnvironmentHook {
    ScheduledEnvironmentHook::new(
        Tick(round_index),
        EnvironmentHook::CascadePartition {
            cuts: cuts.to_vec(),
        },
    )
}

pub(super) fn field_service_freshness_inversion_environment(
    restore: &Configuration,
) -> ScriptedEnvironmentModel {
    ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            8,
            NODE_A,
            NODE_B,
            RatioPermille(520),
            RatioPermille(340),
            RatioPermille(760),
            RatioPermille(120),
        ),
        replace_topology_hook(11, restore),
        asymmetric_degradation_hook(
            13,
            NODE_A,
            NODE_C,
            RatioPermille(560),
            RatioPermille(300),
            RatioPermille(760),
            RatioPermille(130),
        ),
        replace_topology_hook(16, restore),
        asymmetric_degradation_hook(
            18,
            NODE_A,
            NODE_D,
            RatioPermille(600),
            RatioPermille(260),
            RatioPermille(760),
            RatioPermille(140),
        ),
    ])
}

pub(super) fn comparison_corridor_continuity_uncertainty_environment(
    restore: &Configuration,
) -> ScriptedEnvironmentModel {
    ScriptedEnvironmentModel::new(vec![
        asymmetric_degradation_hook(
            7,
            NODE_B,
            NODE_C,
            RatioPermille(560),
            RatioPermille(250),
            RatioPermille(760),
            RatioPermille(140),
        ),
        medium_degradation_hook(11, NODE_A, NODE_B, RatioPermille(650), RatioPermille(170)),
        replace_topology_hook(16, restore),
        asymmetric_degradation_hook(
            19,
            NODE_B,
            NODE_C,
            RatioPermille(610),
            RatioPermille(220),
            RatioPermille(760),
            RatioPermille(150),
        ),
        replace_topology_hook(23, restore),
    ])
}

pub(super) fn routing_observation(configuration: Configuration) -> Observation<Configuration> {
    Observation {
        value: configuration,
        source_class: FactSourceClass::Local,
        evidence_class: RoutingEvidenceClass::DirectObservation,
        origin_authentication: OriginAuthenticationClass::Controlled,
        observed_at_tick: Tick(2),
    }
}

pub(super) fn bidirectional_line_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
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
    })
}

pub(super) fn ring_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
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
    })
}

pub(super) fn full_mesh_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
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
            ((NODE_A, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
            ((NODE_B, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_B), topology::link(2).build()),
            ((NODE_C, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_C), topology::link(3).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 3,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

pub(super) fn bridge_cluster_topology(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
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
        ]),
        environment: Environment {
            reachable_neighbor_count: 1,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

pub(super) fn fanout_service_topology4(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
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
            ((NODE_A, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 3,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

pub(super) fn fanout_service_topology5(
    node_a: Node,
    node_b: Node,
    node_c: Node,
    node_d: Node,
    node_e: Node,
) -> Observation<Configuration> {
    routing_observation(Configuration {
        epoch: RouteEpoch(1),
        nodes: BTreeMap::from([
            (NODE_A, node_a),
            (NODE_B, node_b),
            (NODE_C, node_c),
            (NODE_D, node_d),
            (NODE_E, node_e),
        ]),
        links: BTreeMap::from([
            ((NODE_A, NODE_B), topology::link(2).build()),
            ((NODE_B, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_C), topology::link(3).build()),
            ((NODE_C, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_D), topology::link(4).build()),
            ((NODE_D, NODE_A), topology::link(1).build()),
            ((NODE_A, NODE_E), topology::link(5).build()),
            ((NODE_E, NODE_A), topology::link(1).build()),
        ]),
        environment: Environment {
            reachable_neighbor_count: 4,
            churn_permille: RatioPermille(0),
            contention_permille: RatioPermille(0),
        },
    })
}

pub(super) fn field_fanout_service_topology5(
    contention_permille: RatioPermille,
    loss_permille: RatioPermille,
) -> Observation<Configuration> {
    let mut topology = fanout_service_topology5(
        topology::node(1).field().build(),
        topology::node(2).field().build(),
        topology::node(3).field().build(),
        topology::node(4).field().build(),
        topology::node(5).field().build(),
    );
    set_environment(&mut topology, 4, contention_permille, loss_permille);
    topology
}

pub(super) fn comparison_bridge_topology(
    comparison_engine_set: Option<&str>,
    contention_permille: RatioPermille,
    loss_permille: RatioPermille,
) -> Observation<Configuration> {
    let mut topology = bridge_cluster_topology(
        comparison_topology_node(1, comparison_engine_set),
        comparison_topology_node(2, comparison_engine_set),
        comparison_topology_node(3, comparison_engine_set),
        comparison_topology_node(4, comparison_engine_set),
    );
    set_environment(&mut topology, 1, contention_permille, loss_permille);
    topology
}

pub(super) fn connected_objective(destination: NodeId) -> RoutingObjective {
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

pub(super) fn field_bootstrap_summary(
    destination: DestinationId,
    from_neighbor: NodeId,
    delivery_support: u16,
    min_hops: u8,
    max_hops: u8,
    reverse_feedback: Option<u16>,
) -> FieldBootstrapSummary {
    let observation = FieldForwardSummaryObservation::new(
        RouteEpoch(1),
        Tick(1),
        delivery_support,
        min_hops,
        max_hops,
    );
    let summary = FieldBootstrapSummary::new(destination, from_neighbor, observation);
    if let Some(reverse_feedback) = reverse_feedback {
        summary.with_reverse_feedback(reverse_feedback, Tick(1))
    } else {
        summary
    }
}

pub(super) fn default_objective(destination: NodeId) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Node(destination),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(super) fn service_objective(service_id: Vec<u8>) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(jacquard_core::ServiceId(service_id)),
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

pub(super) fn field_service_objective(service_id: Vec<u8>) -> RoutingObjective {
    RoutingObjective {
        destination: DestinationId::Service(jacquard_core::ServiceId(service_id)),
        service_kind: RouteServiceKind::Move,
        target_protection: RouteProtectionClass::LinkProtected,
        protection_floor: RouteProtectionClass::LinkProtected,
        target_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::PartitionTolerant,
        },
        hold_fallback_policy: jacquard_core::HoldFallbackPolicy::Allowed,
        latency_budget_ms: jacquard_core::Limit::Bounded(DurationMs(250)),
        protection_priority: PriorityPoints(10),
        connectivity_priority: PriorityPoints(20),
    }
}

pub(super) fn best_effort_connected_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::BestEffort,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}

pub(super) fn repairable_connected_profile() -> SelectedRoutingParameters {
    SelectedRoutingParameters {
        selected_protection: RouteProtectionClass::LinkProtected,
        selected_connectivity: ConnectivityPosture {
            repair: RouteRepairClass::Repairable,
            partition: RoutePartitionClass::ConnectedOnly,
        },
        deployment_profile: jacquard_core::OperatingMode::DenseInteractive,
        diversity_floor: jacquard_core::DiversityFloor(1),
        routing_engine_fallback_policy: jacquard_core::RoutingEngineFallbackPolicy::Allowed,
        route_replacement_policy: jacquard_core::RouteReplacementPolicy::Allowed,
    }
}
