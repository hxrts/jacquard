use super::{
    classify_field_transfer, compute_field_posture_signals, count_field_posture_round,
    covered_target_clusters, desired_field_posture, diffusion_bridge_candidate,
    dominant_field_posture_name, field_budget_kind, field_forwarding_suppressed, forwarding_score,
    holder_count_in_cluster, initial_field_budget, initial_field_posture, mean_option_u32,
    mean_u32, mode_option_string, mode_string, sender_energy_ratio_permille, BTreeMap, BTreeSet,
    DiffusionAggregateSummary, DiffusionBoundarySummary, DiffusionContactEvent,
    DiffusionFieldPosture, DiffusionMessageMode, DiffusionMobilityProfile, DiffusionNodeSpec,
    DiffusionRunSpec, DiffusionRunSummary, DiffusionScenarioSpec, DiffusionTransportKind,
    FieldBudgetKind, FieldBudgetState, FieldExecutionMetrics, FieldPostureMetrics,
    FieldSuppressionState, FieldTransferFeatures, HolderState, PendingTransfer,
};

// long-block-exception: the deterministic round loop keeps contact generation,
// posture transitions, forwarding, and metric accounting in one ordered state machine.
pub(super) fn simulate_diffusion_run(spec: &DiffusionRunSpec) -> DiffusionRunSummary {
    let scenario = &spec.scenario;
    let policy = &spec.policy;
    let target_count = scenario_target_count(scenario);
    let field_posture_enabled =
        policy.config_id.starts_with("field") && policy.config_id != "field-static";
    let mut holders = BTreeMap::new();
    let mut remaining_energy = scenario
        .nodes
        .iter()
        .map(|node| (node.node_id, node.energy_budget))
        .collect::<BTreeMap<_, _>>();
    let mut pending = Vec::<PendingTransfer>::new();
    holders.insert(
        scenario.source_node_id,
        HolderState {
            first_round: scenario.creation_round,
        },
    );
    let mut delivered_targets = BTreeSet::new();
    let mut delivery_rounds = Vec::<u32>::new();
    let mut copy_budget_remaining = policy.replication_budget;
    let mut total_transmissions = 0_u32;
    let mut total_energy = 0_u32;
    let mut peak_holders = 1_u32;
    let mut round_new_copies = Vec::<u32>::new();
    let mut edge_flows = BTreeMap::<(u32, u32), u32>::new();
    let mut dominant_edge_by_round = Vec::<Option<(u32, u32)>>::new();
    let mut observer_touches = 0_u32;
    let mut field_posture = if field_posture_enabled {
        Some(initial_field_posture(scenario, policy))
    } else {
        None
    };
    let mut field_posture_metrics = FieldPostureMetrics::default();
    let mut field_pending_posture: Option<DiffusionFieldPosture> = None;
    let mut field_pending_rounds = 0_u32;
    let mut field_budget_state =
        field_posture_enabled.then(|| initial_field_budget(policy, scenario));
    let mut field_execution_metrics = FieldExecutionMetrics::default();
    let mut field_suppression_state = FieldSuppressionState::default();

    for round in 0..scenario.round_count {
        let mut arrivals = Vec::new();
        pending.retain(|transfer| {
            if transfer.arrival_round <= round {
                arrivals.push(transfer.target_node_id);
                false
            } else {
                true
            }
        });
        let mut new_copies_this_round = 0_u32;
        for node_id in arrivals {
            if holders.contains_key(&node_id) {
                continue;
            }
            holders.insert(node_id, HolderState { first_round: round });
            new_copies_this_round = new_copies_this_round.saturating_add(1);
            if is_target_node(scenario, node_id) {
                delivered_targets.insert(node_id);
                delivery_rounds.push(round.saturating_sub(scenario.creation_round));
            }
        }

        if round < scenario.creation_round
            || round
                > scenario
                    .creation_round
                    .saturating_add(policy.message_horizon)
        {
            round_new_copies.push(new_copies_this_round);
            dominant_edge_by_round.push(None);
            continue;
        }

        let contacts = generate_contacts(spec.seed, scenario, round);
        let covered_clusters = covered_target_clusters(scenario, &delivered_targets, &pending);
        if let Some(current_posture) = field_posture {
            let posture_signals = compute_field_posture_signals(
                scenario,
                &holders,
                &remaining_energy,
                &contacts,
                target_count,
                delivered_targets.len(),
                scenario_target_cluster_count(scenario),
                covered_clusters.len(),
                *round_new_copies.last().unwrap_or(&0),
                total_transmissions,
                observer_touches,
            );
            let desired_posture = desired_field_posture(scenario, &posture_signals);
            if desired_posture == current_posture {
                field_pending_posture = None;
                field_pending_rounds = 0;
            } else if field_pending_posture == Some(desired_posture) {
                field_pending_rounds = field_pending_rounds.saturating_add(1);
                if field_pending_rounds >= 2 {
                    field_posture = Some(desired_posture);
                    field_pending_posture = None;
                    field_pending_rounds = 0;
                    field_posture_metrics.transitions =
                        field_posture_metrics.transitions.saturating_add(1);
                    match desired_posture {
                        DiffusionFieldPosture::ScarcityConservative => {
                            field_posture_metrics
                                .first_scarcity_transition_round
                                .get_or_insert(round);
                        }
                        DiffusionFieldPosture::ClusterSeeding
                        | DiffusionFieldPosture::DuplicateSuppressed => {
                            field_posture_metrics
                                .first_congestion_transition_round
                                .get_or_insert(round);
                        }
                        _ => {}
                    }
                }
            } else {
                field_pending_posture = Some(desired_posture);
                field_pending_rounds = 1;
            }
            count_field_posture_round(
                &mut field_posture_metrics,
                field_posture.unwrap_or(current_posture),
            );
        }
        let mut round_edge_counts = BTreeMap::<(u32, u32), u32>::new();
        let mut planned_covered_clusters = covered_clusters.clone();
        for contact in contacts {
            for (from, to) in [
                (contact.node_a, contact.node_b),
                (contact.node_b, contact.node_a),
            ] {
                if !holders.contains_key(&from) || holders.contains_key(&to) {
                    continue;
                }
                let Some(receiver_node) = node_by_id(scenario, to) else {
                    continue;
                };
                if scenario.payload_bytes > receiver_node.storage_capacity {
                    continue;
                }
                let transfer_energy = scenario
                    .payload_bytes
                    .saturating_mul(contact.energy_cost_per_byte);
                let sender_energy = remaining_energy.get(&from).copied().unwrap_or(0);
                if sender_energy < transfer_energy {
                    continue;
                }
                let field_features = if field_posture_enabled {
                    classify_field_transfer(scenario, from, to, &contact, &planned_covered_clusters)
                } else {
                    None
                };
                let field_budget_kind = if let Some(features) = field_features.as_ref() {
                    if features.new_cluster_coverage {
                        field_execution_metrics.cluster_seed_opportunity_count =
                            field_execution_metrics
                                .cluster_seed_opportunity_count
                                .saturating_add(1);
                    } else if features.protected_opportunity && !features.receiver_is_target {
                        field_execution_metrics.bridge_opportunity_count = field_execution_metrics
                            .bridge_opportunity_count
                            .saturating_add(1);
                    }
                    let Some(budget_state) = field_budget_state.as_ref() else {
                        continue;
                    };
                    let Some(budget_kind) = field_budget_kind(
                        scenario,
                        field_posture,
                        features,
                        budget_state,
                        &planned_covered_clusters,
                    ) else {
                        if features.new_cluster_coverage {
                            field_execution_metrics.cluster_coverage_starvation_count =
                                field_execution_metrics
                                    .cluster_coverage_starvation_count
                                    .saturating_add(1);
                        }
                        if let Some(posture) = field_posture {
                            if matches!(
                                posture,
                                DiffusionFieldPosture::ScarcityConservative
                                    | DiffusionFieldPosture::ClusterSeeding
                                    | DiffusionFieldPosture::DuplicateSuppressed
                            ) && !features.receiver_is_target
                            {
                                field_execution_metrics.redundant_forward_suppression_count =
                                    field_execution_metrics
                                        .redundant_forward_suppression_count
                                        .saturating_add(1);
                            }
                        }
                        continue;
                    };
                    let Some(from_node) = node_by_id(scenario, from) else {
                        continue;
                    };
                    let sender_energy_ratio =
                        sender_energy_ratio_permille(from_node, sender_energy);
                    let receiver_cluster_holders = holder_count_in_cluster(
                        scenario,
                        &holders,
                        &pending,
                        features.to_cluster_id,
                    );
                    if let Some(posture) = field_posture {
                        if field_forwarding_suppressed(
                            posture,
                            round,
                            holders.len(),
                            receiver_cluster_holders,
                            sender_energy_ratio,
                            features,
                            &field_suppression_state,
                            &mut field_execution_metrics,
                        ) {
                            if features.new_cluster_coverage {
                                field_execution_metrics.cluster_coverage_starvation_count =
                                    field_execution_metrics
                                        .cluster_coverage_starvation_count
                                        .saturating_add(1);
                            }
                            continue;
                        }
                    }
                    Some(budget_kind)
                } else {
                    let allow_budget =
                        copy_budget_remaining > 0 || is_terminal_target(scenario, to);
                    if !allow_budget {
                        continue;
                    }
                    None
                };
                let score = forwarding_score(
                    scenario,
                    policy,
                    from,
                    to,
                    &contact,
                    holders.len(),
                    sender_energy,
                    field_posture,
                    field_features.as_ref(),
                );
                if score
                    <= permille_hash(spec.seed, scenario.family_id.as_str(), round, from, to, 0)
                {
                    continue;
                }
                if contact.bandwidth_bytes < scenario.payload_bytes {
                    continue;
                }
                if let Some(entry) = remaining_energy.get_mut(&from) {
                    *entry = entry.saturating_sub(transfer_energy);
                }
                total_transmissions = total_transmissions.saturating_add(1);
                total_energy = total_energy.saturating_add(transfer_energy);
                if is_observer_node(scenario, from)
                    || (is_observer_node(scenario, to) && !is_terminal_target(scenario, to))
                {
                    observer_touches = observer_touches.saturating_add(1);
                }
                let edge = normalized_edge(from, to);
                *edge_flows.entry(edge).or_insert(0) += 1;
                *round_edge_counts.entry(edge).or_insert(0) += 1;
                let arrival_round = round.saturating_add(contact.connection_delay);
                if arrival_round
                    <= scenario
                        .creation_round
                        .saturating_add(policy.message_horizon)
                {
                    pending.push(PendingTransfer {
                        arrival_round,
                        target_node_id: to,
                    });
                    if is_target_node(scenario, to) {
                        if let Some(node) = node_by_id(scenario, to) {
                            planned_covered_clusters.insert(node.cluster_id);
                        }
                    }
                }
                if let (Some(features), Some(budget_kind), Some(budget_state)) = (
                    field_features.as_ref(),
                    field_budget_kind,
                    field_budget_state.as_mut(),
                ) {
                    record_field_forward(
                        round,
                        budget_kind,
                        features,
                        budget_state,
                        &mut field_suppression_state,
                        &mut field_execution_metrics,
                    );
                } else if !is_terminal_target(scenario, to) && copy_budget_remaining > 0 {
                    copy_budget_remaining -= 1;
                }
            }
        }
        dominant_edge_by_round.push(
            round_edge_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(edge, _)| edge),
        );
        peak_holders = peak_holders.max(u32::try_from(holders.len()).unwrap_or(u32::MAX));
        round_new_copies.push(new_copies_this_round);
    }

    let delivery_probability_permille = match scenario.message_mode {
        DiffusionMessageMode::Unicast => {
            if delivered_targets.contains(&scenario.destination_node_id.unwrap_or_default()) {
                1000
            } else {
                0
            }
        }
        DiffusionMessageMode::Broadcast => {
            coverage_permille_for(target_count, delivered_targets.len())
        }
    };
    let coverage_permille = coverage_permille_for(target_count, delivered_targets.len());
    let cluster_coverage_permille = coverage_permille_for(
        scenario_target_cluster_count(scenario),
        covered_target_clusters(scenario, &delivered_targets, &[]).len(),
    );
    let energy_per_delivered_message = if delivered_targets.is_empty() {
        None
    } else {
        Some(total_energy / u32::try_from(delivered_targets.len()).unwrap_or(1))
    };
    let delivery_delay = if delivery_rounds.is_empty() {
        None
    } else {
        Some(
            delivery_rounds.iter().copied().sum::<u32>()
                / u32::try_from(delivery_rounds.len()).unwrap_or(1),
        )
    };
    let total_storage_capacity = scenario
        .nodes
        .iter()
        .map(|node| node.storage_capacity)
        .sum::<u32>();
    let storage_utilization_permille = if total_storage_capacity == 0 {
        0
    } else {
        peak_holders
            .saturating_mul(scenario.payload_bytes)
            .saturating_mul(1000)
            / total_storage_capacity
    };
    let holders_before_contact = round_new_copies
        .iter()
        .fold((0_u32, 0_u32), |(holders_so_far, sum), new_copies| {
            let updated_sum = sum.saturating_add(holders_so_far);
            (holders_so_far.saturating_add(*new_copies), updated_sum)
        })
        .1
        .saturating_add(1);
    let estimated_reproduction_permille = if holders_before_contact == 0 {
        0
    } else {
        u32::try_from((u64::from(total_transmissions) * 1000) / u64::from(holders_before_contact))
            .unwrap_or(u32::MAX)
    };
    let corridor_persistence_permille = if total_transmissions == 0 {
        0
    } else {
        edge_flows
            .values()
            .copied()
            .max()
            .unwrap_or(0)
            .saturating_mul(1000)
            / total_transmissions
    };
    let decision_churn_count = u32::try_from(
        dominant_edge_by_round
            .windows(2)
            .filter(|window| window[0].is_some() && window[1].is_some() && window[0] != window[1])
            .count(),
    )
    .unwrap_or(u32::MAX);
    let observer_leakage_permille = if total_transmissions == 0 {
        0
    } else {
        observer_touches.saturating_mul(1000) / total_transmissions
    };
    let bounded_state = bounded_state(
        delivery_probability_permille,
        coverage_permille,
        estimated_reproduction_permille,
        total_transmissions,
        storage_utilization_permille,
        energy_per_delivered_message,
    )
    .to_string();
    let message_persistence_rounds = if holders.is_empty() {
        0
    } else {
        scenario.round_count.saturating_sub(
            holders
                .values()
                .map(|holder| holder.first_round)
                .min()
                .unwrap_or(scenario.round_count),
        )
    };

    DiffusionRunSummary {
        suite_id: spec.suite_id.clone(),
        family_id: spec.family_id.clone(),
        config_id: policy.config_id.clone(),
        seed: spec.seed,
        density: scenario.regime.density.clone(),
        mobility_model: scenario.regime.mobility_model.clone(),
        transport_mix: scenario.regime.transport_mix.clone(),
        pressure: scenario.regime.pressure.clone(),
        objective_regime: scenario.regime.objective_regime.clone(),
        stress_score: scenario.regime.stress_score,
        replication_budget: policy.replication_budget,
        message_horizon: policy.message_horizon,
        forward_probability_permille: policy.forward_probability_permille,
        bridge_bias_permille: policy.bridge_bias_permille,
        delivery_probability_permille,
        delivery_delay,
        coverage_permille,
        cluster_coverage_permille,
        total_transmissions,
        energy_spent_units: total_energy,
        energy_per_delivered_message,
        storage_utilization_permille,
        estimated_reproduction_permille,
        corridor_persistence_permille,
        decision_churn_count,
        observer_leakage_permille,
        bounded_state,
        message_persistence_rounds,
        field_posture_mode: dominant_field_posture_name(&field_posture_metrics),
        field_posture_transition_count: field_posture_metrics.transitions,
        field_continuity_biased_rounds: field_posture_metrics.continuity_biased_rounds,
        field_balanced_rounds: field_posture_metrics.balanced_rounds,
        field_scarcity_conservative_rounds: field_posture_metrics.scarcity_conservative_rounds,
        field_congestion_suppressed_rounds: field_posture_metrics
            .cluster_seeding_rounds
            .saturating_add(field_posture_metrics.duplicate_suppressed_rounds),
        field_cluster_seeding_rounds: field_posture_metrics.cluster_seeding_rounds,
        field_duplicate_suppressed_rounds: field_posture_metrics.duplicate_suppressed_rounds,
        field_privacy_conservative_rounds: field_posture_metrics.privacy_conservative_rounds,
        field_first_scarcity_transition_round: field_posture_metrics
            .first_scarcity_transition_round,
        field_first_congestion_transition_round: field_posture_metrics
            .first_congestion_transition_round,
        field_protected_budget_used: field_budget_state
            .as_ref()
            .map(|budget| budget.protected_used)
            .unwrap_or(0),
        field_generic_budget_used: field_budget_state
            .as_ref()
            .map(|budget| budget.generic_used)
            .unwrap_or(0),
        field_bridge_opportunity_count: field_execution_metrics.bridge_opportunity_count,
        field_protected_bridge_usage_count: field_execution_metrics.protected_bridge_usage_count,
        field_cluster_seed_opportunity_count: field_execution_metrics
            .cluster_seed_opportunity_count,
        field_cluster_seed_usage_count: field_execution_metrics.cluster_seed_usage_count,
        field_cluster_coverage_starvation_count: field_execution_metrics
            .cluster_coverage_starvation_count,
        field_redundant_forward_suppression_count: field_execution_metrics
            .redundant_forward_suppression_count,
        field_same_cluster_suppression_count: field_execution_metrics
            .same_cluster_suppression_count,
        field_expensive_transport_suppression_count: field_execution_metrics
            .expensive_transport_suppression_count,
    }
}

// long-block-exception: the diffusion aggregate reducer keeps the full summary-to-report
// mapping in one pass so the emitted analysis schema remains reviewable as one unit.
pub(super) fn aggregate_diffusion_runs(
    runs: &[DiffusionRunSummary],
) -> Vec<DiffusionAggregateSummary> {
    let mut grouped = BTreeMap::<(String, String), Vec<&DiffusionRunSummary>>::new();
    for run in runs {
        grouped
            .entry((run.family_id.clone(), run.config_id.clone()))
            .or_default()
            .push(run);
    }
    let mut aggregates = Vec::new();
    for ((_family_id, _config_id), group) in grouped {
        let first = group[0];
        let run_count = u32::try_from(group.len()).unwrap_or(u32::MAX);
        let mode = mode_string(group.iter().map(|row| row.bounded_state.clone()));
        aggregates.push(DiffusionAggregateSummary {
            suite_id: first.suite_id.clone(),
            family_id: first.family_id.clone(),
            config_id: first.config_id.clone(),
            density: first.density.clone(),
            mobility_model: first.mobility_model.clone(),
            transport_mix: first.transport_mix.clone(),
            pressure: first.pressure.clone(),
            objective_regime: first.objective_regime.clone(),
            stress_score: first.stress_score,
            replication_budget: first.replication_budget,
            message_horizon: first.message_horizon,
            forward_probability_permille: first.forward_probability_permille,
            bridge_bias_permille: first.bridge_bias_permille,
            run_count,
            delivery_probability_permille_mean: mean_u32(
                group.iter().map(|row| row.delivery_probability_permille),
            ),
            delivery_delay_mean: mean_option_u32(group.iter().map(|row| row.delivery_delay)),
            coverage_permille_mean: mean_u32(group.iter().map(|row| row.coverage_permille)),
            cluster_coverage_permille_mean: mean_u32(
                group.iter().map(|row| row.cluster_coverage_permille),
            ),
            total_transmissions_mean: mean_u32(group.iter().map(|row| row.total_transmissions)),
            energy_spent_units_mean: mean_u32(group.iter().map(|row| row.energy_spent_units)),
            energy_per_delivered_message_mean: mean_option_u32(
                group.iter().map(|row| row.energy_per_delivered_message),
            ),
            storage_utilization_permille_mean: mean_u32(
                group.iter().map(|row| row.storage_utilization_permille),
            ),
            estimated_reproduction_permille_mean: mean_u32(
                group.iter().map(|row| row.estimated_reproduction_permille),
            ),
            corridor_persistence_permille_mean: mean_u32(
                group.iter().map(|row| row.corridor_persistence_permille),
            ),
            decision_churn_count_mean: mean_u32(group.iter().map(|row| row.decision_churn_count)),
            observer_leakage_permille_mean: mean_u32(
                group.iter().map(|row| row.observer_leakage_permille),
            ),
            message_persistence_rounds_mean: mean_u32(
                group.iter().map(|row| row.message_persistence_rounds),
            ),
            bounded_state_mode: mode,
            field_posture_mode: mode_option_string(
                group.iter().map(|row| row.field_posture_mode.clone()),
            ),
            field_posture_transition_count_mean: mean_u32(
                group.iter().map(|row| row.field_posture_transition_count),
            ),
            field_continuity_biased_rounds_mean: mean_u32(
                group.iter().map(|row| row.field_continuity_biased_rounds),
            ),
            field_balanced_rounds_mean: mean_u32(group.iter().map(|row| row.field_balanced_rounds)),
            field_scarcity_conservative_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_scarcity_conservative_rounds),
            ),
            field_congestion_suppressed_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_congestion_suppressed_rounds),
            ),
            field_cluster_seeding_rounds_mean: mean_u32(
                group.iter().map(|row| row.field_cluster_seeding_rounds),
            ),
            field_duplicate_suppressed_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_duplicate_suppressed_rounds),
            ),
            field_privacy_conservative_rounds_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_privacy_conservative_rounds),
            ),
            field_first_scarcity_transition_round_mean: mean_option_u32(
                group
                    .iter()
                    .map(|row| row.field_first_scarcity_transition_round),
            ),
            field_first_congestion_transition_round_mean: mean_option_u32(
                group
                    .iter()
                    .map(|row| row.field_first_congestion_transition_round),
            ),
            field_protected_budget_used_mean: mean_u32(
                group.iter().map(|row| row.field_protected_budget_used),
            ),
            field_generic_budget_used_mean: mean_u32(
                group.iter().map(|row| row.field_generic_budget_used),
            ),
            field_bridge_opportunity_count_mean: mean_u32(
                group.iter().map(|row| row.field_bridge_opportunity_count),
            ),
            field_protected_bridge_usage_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_protected_bridge_usage_count),
            ),
            field_cluster_seed_opportunity_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_cluster_seed_opportunity_count),
            ),
            field_cluster_seed_usage_count_mean: mean_u32(
                group.iter().map(|row| row.field_cluster_seed_usage_count),
            ),
            field_cluster_coverage_starvation_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_cluster_coverage_starvation_count),
            ),
            field_redundant_forward_suppression_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_redundant_forward_suppression_count),
            ),
            field_same_cluster_suppression_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_same_cluster_suppression_count),
            ),
            field_expensive_transport_suppression_count_mean: mean_u32(
                group
                    .iter()
                    .map(|row| row.field_expensive_transport_suppression_count),
            ),
        });
    }
    aggregates.sort_by(|left, right| {
        left.family_id.cmp(&right.family_id).then(
            left.delivery_probability_permille_mean
                .cmp(&right.delivery_probability_permille_mean)
                .reverse(),
        )
    });
    aggregates
}

pub(super) fn summarize_diffusion_boundaries(
    aggregates: &[DiffusionAggregateSummary],
) -> Vec<DiffusionBoundarySummary> {
    let mut grouped = BTreeMap::<String, Vec<&DiffusionAggregateSummary>>::new();
    for aggregate in aggregates {
        grouped
            .entry(aggregate.config_id.clone())
            .or_default()
            .push(aggregate);
    }
    let mut rows = Vec::new();
    for (config_id, mut group) in grouped {
        group.sort_by_key(|row| row.stress_score);
        let viable_family_count = u32::try_from(
            group
                .iter()
                .filter(|row| row.bounded_state_mode == "viable")
                .count(),
        )
        .unwrap_or(u32::MAX);
        let collapse = group
            .iter()
            .find(|row| row.bounded_state_mode == "collapse");
        let explosive = group
            .iter()
            .find(|row| row.bounded_state_mode == "explosive");
        rows.push(DiffusionBoundarySummary {
            suite_id: group[0].suite_id.clone(),
            config_id,
            viable_family_count,
            first_collapse_family_id: collapse.map(|row| row.family_id.clone()),
            first_collapse_stress_score: collapse.map(|row| row.stress_score),
            first_explosive_family_id: explosive.map(|row| row.family_id.clone()),
            first_explosive_stress_score: explosive.map(|row| row.stress_score),
        });
    }
    rows.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    rows
}

pub(super) fn record_field_forward(
    round: u32,
    budget_kind: FieldBudgetKind,
    features: &FieldTransferFeatures,
    budget_state: &mut FieldBudgetState,
    suppression_state: &mut FieldSuppressionState,
    metrics: &mut FieldExecutionMetrics,
) {
    match budget_kind {
        FieldBudgetKind::Target => {}
        FieldBudgetKind::Protected => {
            budget_state.protected_remaining = budget_state.protected_remaining.saturating_sub(1);
            budget_state.protected_used = budget_state.protected_used.saturating_add(1);
            if features.protected_opportunity && !features.new_cluster_coverage {
                metrics.protected_bridge_usage_count =
                    metrics.protected_bridge_usage_count.saturating_add(1);
            }
            if features.new_cluster_coverage {
                metrics.cluster_seed_usage_count =
                    metrics.cluster_seed_usage_count.saturating_add(1);
            }
        }
        FieldBudgetKind::Generic => {
            budget_state.generic_remaining = budget_state.generic_remaining.saturating_sub(1);
            budget_state.generic_used = budget_state.generic_used.saturating_add(1);
        }
    }
    if !features.receiver_is_target {
        suppression_state
            .recent_cluster_forward_round
            .insert(features.to_cluster_id, round);
        suppression_state
            .recent_corridor_forward_round
            .insert((features.from_cluster_id, features.to_cluster_id), round);
        if features.same_cluster {
            suppression_state
                .recent_same_cluster_forward_round
                .insert(features.from_cluster_id, round);
        }
    }
}

pub(super) fn generate_contacts(
    seed: u64,
    scenario: &DiffusionScenarioSpec,
    round: u32,
) -> Vec<DiffusionContactEvent> {
    let mut contacts = Vec::new();
    for index in 0..scenario.nodes.len() {
        for peer_index in index + 1..scenario.nodes.len() {
            let left = &scenario.nodes[index];
            let right = &scenario.nodes[peer_index];
            let probability = contact_probability_permille(scenario, left, right, round);
            if probability
                <= permille_hash(
                    seed,
                    scenario.family_id.as_str(),
                    round,
                    left.node_id,
                    right.node_id,
                    1,
                )
            {
                continue;
            }
            let transport_kind = choose_transport(left, right, round);
            let (bandwidth_bytes, energy_cost_per_byte, connection_delay) =
                transport_properties(transport_kind);
            contacts.push(DiffusionContactEvent {
                round_index: round,
                node_a: left.node_id,
                node_b: right.node_id,
                contact_window: 1,
                bandwidth_bytes,
                transport_kind,
                connection_delay,
                energy_cost_per_byte,
            });
        }
    }
    contacts
}

fn same_cluster_bridged_probability(
    family_id: &str,
    same_cluster: bool,
    bridged: bool,
) -> Option<u32> {
    let (same_cluster_probability, bridged_probability, fallback_probability) =
        family_cluster_probabilities(family_id)?;
    Some(if same_cluster {
        same_cluster_probability
    } else if bridged {
        bridged_probability
    } else {
        fallback_probability
    })
}

pub(super) fn family_cluster_probabilities(family_id: &str) -> Option<(u32, u32, u32)> {
    match family_id {
        "diffusion-partitioned-clusters" => Some((720, 260, 28)),
        "diffusion-random-waypoint-sanity" => Some((440, 180, 120)),
        "diffusion-disaster-broadcast" => Some((660, 210, 55)),
        "diffusion-sparse-long-delay" => Some((140, 120, 18)),
        "diffusion-high-density-overload" => Some((900, 620, 420)),
        "diffusion-adversarial-observation" => Some((540, 180, 42)),
        "diffusion-bridge-drought" => Some((190, 72, 6)),
        "diffusion-energy-starved-relay" => Some((260, 110, 20)),
        "diffusion-congestion-cascade" => Some((960, 700, 480)),
        _ => None,
    }
}

pub(super) fn contact_probability_permille(
    scenario: &DiffusionScenarioSpec,
    left: &DiffusionNodeSpec,
    right: &DiffusionNodeSpec,
    round: u32,
) -> u32 {
    let same_cluster = left.cluster_id == right.cluster_id;
    let bridged = diffusion_bridge_candidate(left) || diffusion_bridge_candidate(right);
    match scenario.family_id.as_str() {
        "diffusion-mobility-shift" => {
            if round < scenario.round_count / 2 {
                if same_cluster {
                    650
                } else if bridged {
                    140
                } else {
                    30
                }
            } else if same_cluster {
                380
            } else if bridged {
                460
            } else {
                140
            }
        }
        family_id => {
            same_cluster_bridged_probability(family_id, same_cluster, bridged).unwrap_or(0)
        }
    }
}

pub(super) fn choose_transport(
    left: &DiffusionNodeSpec,
    right: &DiffusionNodeSpec,
    round: u32,
) -> DiffusionTransportKind {
    let same_cluster = left.cluster_id == right.cluster_id;
    if same_cluster {
        if matches!(left.mobility_profile, DiffusionMobilityProfile::Observer)
            || matches!(right.mobility_profile, DiffusionMobilityProfile::Observer)
        {
            DiffusionTransportKind::Ble
        } else {
            DiffusionTransportKind::WifiAware
        }
    } else if matches!(
        left.mobility_profile,
        DiffusionMobilityProfile::LongRangeMover
    ) || matches!(
        right.mobility_profile,
        DiffusionMobilityProfile::LongRangeMover
    ) || round.is_multiple_of(5)
    {
        DiffusionTransportKind::LoRa
    } else {
        DiffusionTransportKind::WifiAware
    }
}

pub(super) fn transport_properties(kind: DiffusionTransportKind) -> (u32, u32, u32) {
    match kind {
        DiffusionTransportKind::Ble => (192, 4, 0),
        DiffusionTransportKind::WifiAware => (640, 2, 0),
        DiffusionTransportKind::LoRa => (96, 8, 1),
    }
}

pub(super) fn bounded_state(
    delivery_probability_permille: u32,
    coverage_permille: u32,
    reproduction_permille: u32,
    transmissions: u32,
    storage_utilization_permille: u32,
    energy_per_delivered_message: Option<u32>,
) -> &'static str {
    if delivery_probability_permille < 300 || coverage_permille < 350 {
        "collapse"
    } else if reproduction_permille > 1600
        || transmissions > 48
        || storage_utilization_permille > 700
        || energy_per_delivered_message.unwrap_or(0) > 1400
    {
        "explosive"
    } else {
        "viable"
    }
}

pub(super) fn coverage_permille_for(target_count: usize, delivered_count: usize) -> u32 {
    if target_count == 0 {
        return 0;
    }
    u32::try_from(
        (u64::try_from(delivered_count).unwrap_or(0) * 1000)
            / u64::try_from(target_count).unwrap_or(1),
    )
    .unwrap_or(u32::MAX)
}

pub(super) fn scenario_target_count(scenario: &DiffusionScenarioSpec) -> usize {
    match scenario.message_mode {
        DiffusionMessageMode::Unicast => 1,
        DiffusionMessageMode::Broadcast => scenario.nodes.len().saturating_sub(1),
    }
}

pub(super) fn scenario_target_cluster_count(scenario: &DiffusionScenarioSpec) -> usize {
    let mut clusters = BTreeSet::new();
    for node in &scenario.nodes {
        if is_target_node(scenario, node.node_id) {
            clusters.insert(node.cluster_id);
        }
    }
    clusters.len()
}

pub(super) fn is_target_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    match scenario.message_mode {
        DiffusionMessageMode::Unicast => scenario.destination_node_id == Some(node_id),
        DiffusionMessageMode::Broadcast => node_id != scenario.source_node_id,
    }
}

pub(super) fn is_terminal_target(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    matches!(scenario.message_mode, DiffusionMessageMode::Unicast)
        && scenario.destination_node_id == Some(node_id)
}

pub(super) fn is_observer_node(scenario: &DiffusionScenarioSpec, node_id: u32) -> bool {
    scenario
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .map(|node| matches!(node.mobility_profile, DiffusionMobilityProfile::Observer))
        .unwrap_or(false)
}

pub(super) fn node_by_id(
    scenario: &DiffusionScenarioSpec,
    node_id: u32,
) -> Option<&DiffusionNodeSpec> {
    scenario.nodes.iter().find(|node| node.node_id == node_id)
}

pub(super) fn normalized_edge(left: u32, right: u32) -> (u32, u32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

pub(super) fn permille_hash(
    seed: u64,
    family_id: &str,
    round: u32,
    left: u32,
    right: u32,
    lane: u64,
) -> u32 {
    let mut value = seed
        ^ u64::from(round).wrapping_mul(0x9E37_79B9)
        ^ u64::from(left).wrapping_mul(0x85EB_CA6B)
        ^ u64::from(right).wrapping_mul(0xC2B2_AE35)
        ^ lane;
    for byte in family_id.as_bytes() {
        value ^= u64::from(*byte);
        value = value.rotate_left(13).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    u32::try_from(value % 1000).unwrap_or(0)
}
