//! Field posture classification and signal computation for diffusion strategies.

use super::{
    coverage_permille_for, is_target_node, is_terminal_target, node_by_id,
    scenario_target_cluster_count, BTreeMap, BTreeSet, DiffusionContactEvent,
    DiffusionFieldPosture, DiffusionMessageMode, DiffusionMobilityProfile, DiffusionNodeSpec,
    DiffusionPolicyConfig, DiffusionScenarioSpec, DiffusionTransportKind, FieldBudgetKind,
    FieldBudgetState, FieldExecutionMetrics, FieldPostureMetrics, FieldPostureSignals,
    FieldSuppressionState, FieldTransferFeatures, HolderState, PendingTransfer,
};

pub(super) fn initial_field_posture(
    scenario: &DiffusionScenarioSpec,
    policy: &DiffusionPolicyConfig,
) -> DiffusionFieldPosture {
    match scenario.family_id.as_str() {
        "diffusion-bridge-drought"
        | "diffusion-partitioned-clusters"
        | "diffusion-sparse-long-delay"
        | "diffusion-mobility-shift" => DiffusionFieldPosture::ContinuityBiased,
        "diffusion-energy-starved-relay" if policy.config_id.starts_with("field-scarcity") => {
            DiffusionFieldPosture::ScarcityConservative
        }
        "diffusion-congestion-cascade" | "diffusion-high-density-overload"
            if policy.config_id.starts_with("field-congestion") =>
        {
            DiffusionFieldPosture::ClusterSeeding
        }
        "diffusion-adversarial-observation" if policy.config_id.starts_with("field-privacy") => {
            DiffusionFieldPosture::PrivacyConservative
        }
        "diffusion-high-density-overload"
        | "diffusion-congestion-cascade"
        | "diffusion-energy-starved-relay"
        | "diffusion-adversarial-observation" => DiffusionFieldPosture::Balanced,
        _ => DiffusionFieldPosture::Balanced,
    }
}

pub(super) fn total_scenario_energy_budget(scenario: &DiffusionScenarioSpec) -> u32 {
    scenario
        .nodes
        .iter()
        .map(|node| node.energy_budget)
        .sum::<u32>()
}

pub(super) fn total_scenario_storage_capacity(scenario: &DiffusionScenarioSpec) -> u32 {
    scenario
        .nodes
        .iter()
        .map(|node| node.storage_capacity)
        .sum::<u32>()
}

pub(super) fn remaining_energy_fraction_permille(
    scenario: &DiffusionScenarioSpec,
    remaining_energy: &BTreeMap<u32, u32>,
) -> u32 {
    let total_budget = total_scenario_energy_budget(scenario);
    let remaining_budget = remaining_energy.values().copied().sum::<u32>();
    if total_budget == 0 {
        0
    } else {
        remaining_budget.saturating_mul(1000) / total_budget
    }
}

pub(super) fn storage_pressure_permille(
    scenario: &DiffusionScenarioSpec,
    holders: &BTreeMap<u32, HolderState>,
) -> u32 {
    let total_storage_capacity = total_scenario_storage_capacity(scenario);
    if total_storage_capacity == 0 {
        0
    } else {
        u32::try_from(holders.len())
            .unwrap_or(u32::MAX)
            .saturating_mul(scenario.payload_bytes)
            .saturating_mul(1000)
            / total_storage_capacity
    }
}

pub(super) fn has_recent_bridge_opportunity(
    scenario: &DiffusionScenarioSpec,
    contacts: &[DiffusionContactEvent],
) -> bool {
    contacts.iter().any(|contact| {
        let left = node_by_id(scenario, contact.node_a);
        let right = node_by_id(scenario, contact.node_b);
        match (left, right) {
            (Some(left), Some(right)) => {
                left.cluster_id != right.cluster_id
                    && (matches!(
                        left.mobility_profile,
                        DiffusionMobilityProfile::Bridger
                            | DiffusionMobilityProfile::LongRangeMover
                    ) || matches!(
                        right.mobility_profile,
                        DiffusionMobilityProfile::Bridger
                            | DiffusionMobilityProfile::LongRangeMover
                    ))
            }
            _ => false,
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compute_field_posture_signals(
    scenario: &DiffusionScenarioSpec,
    holders: &BTreeMap<u32, HolderState>,
    remaining_energy: &BTreeMap<u32, u32>,
    contacts: &[DiffusionContactEvent],
    target_count: usize,
    delivered_target_count: usize,
    target_cluster_count: usize,
    delivered_target_cluster_count: usize,
    spread_growth: u32,
    total_transmissions: u32,
    observer_touches: u32,
) -> FieldPostureSignals {
    let remaining_energy_fraction_permille =
        remaining_energy_fraction_permille(scenario, remaining_energy);
    let storage_pressure_permille = storage_pressure_permille(scenario, holders);
    let recent_bridge_opportunity = has_recent_bridge_opportunity(scenario, contacts);
    let observer_exposure_permille = if total_transmissions == 0 {
        0
    } else {
        observer_touches.saturating_mul(1000) / total_transmissions
    };
    let delivery_progress_permille = coverage_permille_for(target_count, delivered_target_count);
    let cluster_delivery_progress_permille =
        coverage_permille_for(target_cluster_count, delivered_target_cluster_count);
    FieldPostureSignals {
        holder_count: holders.len(),
        spread_growth,
        remaining_energy_fraction_permille,
        storage_pressure_permille,
        recent_bridge_opportunity,
        observer_exposure_permille,
        delivery_progress_permille,
        cluster_delivery_progress_permille,
    }
}

pub(super) fn desired_field_posture(
    scenario: &DiffusionScenarioSpec,
    signals: &FieldPostureSignals,
) -> DiffusionFieldPosture {
    if signals.observer_exposure_permille >= 180
        || (scenario.family_id == "diffusion-adversarial-observation"
            && signals.observer_exposure_permille >= 90)
    {
        return DiffusionFieldPosture::PrivacyConservative;
    }
    if signals.remaining_energy_fraction_permille <= 520
        || (scenario.family_id == "diffusion-energy-starved-relay"
            && (signals.remaining_energy_fraction_permille <= 920
                || signals.holder_count >= 2
                || signals.spread_growth >= 1))
        || (scenario.family_id == "diffusion-energy-starved-relay"
            && signals.storage_pressure_permille >= 180)
    {
        return DiffusionFieldPosture::ScarcityConservative;
    }
    if matches!(
        scenario.family_id.as_str(),
        "diffusion-high-density-overload" | "diffusion-congestion-cascade"
    ) {
        if signals.cluster_delivery_progress_permille < 1000
            && (signals.storage_pressure_permille >= 260
                || signals.holder_count >= 3
                || signals.spread_growth >= 1)
        {
            return DiffusionFieldPosture::ClusterSeeding;
        }
        if signals.storage_pressure_permille >= 360
            || signals.holder_count >= 5
            || signals.cluster_delivery_progress_permille >= 1000
            || signals.delivery_progress_permille >= 720
        {
            return DiffusionFieldPosture::DuplicateSuppressed;
        }
    }
    if signals.storage_pressure_permille >= 560 || signals.holder_count >= 6 {
        return DiffusionFieldPosture::DuplicateSuppressed;
    }
    if matches!(
        scenario.family_id.as_str(),
        "diffusion-bridge-drought"
            | "diffusion-partitioned-clusters"
            | "diffusion-sparse-long-delay"
            | "diffusion-mobility-shift"
    ) && (signals.delivery_progress_permille < 1000
        || signals.recent_bridge_opportunity
        || scenario.family_id == "diffusion-bridge-drought")
    {
        return DiffusionFieldPosture::ContinuityBiased;
    }
    DiffusionFieldPosture::Balanced
}

pub(super) fn count_field_posture_round(
    metrics: &mut FieldPostureMetrics,
    posture: DiffusionFieldPosture,
) {
    match posture {
        DiffusionFieldPosture::ContinuityBiased => {
            metrics.continuity_biased_rounds = metrics.continuity_biased_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::Balanced => {
            metrics.balanced_rounds = metrics.balanced_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::ScarcityConservative => {
            metrics.scarcity_conservative_rounds =
                metrics.scarcity_conservative_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::ClusterSeeding => {
            metrics.cluster_seeding_rounds = metrics.cluster_seeding_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::DuplicateSuppressed => {
            metrics.duplicate_suppressed_rounds =
                metrics.duplicate_suppressed_rounds.saturating_add(1)
        }
        DiffusionFieldPosture::PrivacyConservative => {
            metrics.privacy_conservative_rounds =
                metrics.privacy_conservative_rounds.saturating_add(1)
        }
    }
}

pub(super) fn field_posture_name(posture: DiffusionFieldPosture) -> String {
    match posture {
        DiffusionFieldPosture::ContinuityBiased => "continuity_biased".to_string(),
        DiffusionFieldPosture::Balanced => "balanced".to_string(),
        DiffusionFieldPosture::ScarcityConservative => "scarcity_conservative".to_string(),
        DiffusionFieldPosture::ClusterSeeding => "cluster_seeding".to_string(),
        DiffusionFieldPosture::DuplicateSuppressed => "duplicate_suppressed".to_string(),
        DiffusionFieldPosture::PrivacyConservative => "privacy_conservative".to_string(),
    }
}

pub(super) fn dominant_field_posture_name(metrics: &FieldPostureMetrics) -> Option<String> {
    let candidates = [
        (
            metrics.continuity_biased_rounds,
            0_u8,
            DiffusionFieldPosture::ContinuityBiased,
        ),
        (
            metrics.balanced_rounds,
            1_u8,
            DiffusionFieldPosture::Balanced,
        ),
        (
            metrics.scarcity_conservative_rounds,
            2_u8,
            DiffusionFieldPosture::ScarcityConservative,
        ),
        (
            metrics.cluster_seeding_rounds,
            3_u8,
            DiffusionFieldPosture::ClusterSeeding,
        ),
        (
            metrics.duplicate_suppressed_rounds,
            4_u8,
            DiffusionFieldPosture::DuplicateSuppressed,
        ),
        (
            metrics.privacy_conservative_rounds,
            5_u8,
            DiffusionFieldPosture::PrivacyConservative,
        ),
    ];
    candidates
        .into_iter()
        .max_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)))
        .and_then(|(rounds, _, posture)| {
            if rounds == 0 {
                None
            } else {
                Some(field_posture_name(posture))
            }
        })
}

pub(super) fn initial_field_budget(
    policy: &DiffusionPolicyConfig,
    scenario: &DiffusionScenarioSpec,
) -> FieldBudgetState {
    let base_protected = if matches!(scenario.message_mode, DiffusionMessageMode::Unicast) {
        1
    } else {
        u32::try_from(scenario_target_cluster_count(scenario)).unwrap_or(u32::MAX)
    };
    let continuity_reserved = if matches!(
        scenario.family_id.as_str(),
        "diffusion-bridge-drought"
            | "diffusion-partitioned-clusters"
            | "diffusion-sparse-long-delay"
            | "diffusion-mobility-shift"
    ) {
        1
    } else {
        0
    };
    let mut protected_remaining =
        (base_protected + continuity_reserved).min(policy.replication_budget);
    if protected_remaining == policy.replication_budget && policy.replication_budget > 1 {
        protected_remaining = protected_remaining.saturating_sub(1);
    }
    FieldBudgetState {
        protected_remaining,
        generic_remaining: policy
            .replication_budget
            .saturating_sub(protected_remaining),
        protected_used: 0,
        generic_used: 0,
    }
}

pub(super) fn sender_energy_ratio_permille(
    from_node: &DiffusionNodeSpec,
    sender_energy_remaining: u32,
) -> u32 {
    if from_node.energy_budget == 0 {
        0
    } else {
        sender_energy_remaining.saturating_mul(1000) / from_node.energy_budget
    }
}

pub(super) fn diffusion_source_cluster(scenario: &DiffusionScenarioSpec) -> Option<u8> {
    node_by_id(scenario, scenario.source_node_id).map(|node| node.cluster_id)
}

pub(super) fn diffusion_destination_cluster(scenario: &DiffusionScenarioSpec) -> Option<u8> {
    scenario
        .destination_node_id
        .and_then(|destination| node_by_id(scenario, destination))
        .map(|node| node.cluster_id)
}

pub(super) fn diffusion_bridge_candidate(node: &DiffusionNodeSpec) -> bool {
    matches!(
        node.mobility_profile,
        DiffusionMobilityProfile::Bridger | DiffusionMobilityProfile::LongRangeMover
    )
}

pub(super) fn protected_bridge_family(family_id: &str) -> bool {
    matches!(
        family_id,
        "diffusion-bridge-drought"
            | "diffusion-partitioned-clusters"
            | "diffusion-sparse-long-delay"
            | "diffusion-mobility-shift"
    )
}

pub(super) fn classify_field_transfer(
    scenario: &DiffusionScenarioSpec,
    from: u32,
    to: u32,
    contact: &DiffusionContactEvent,
    covered_clusters: &BTreeSet<u8>,
) -> Option<FieldTransferFeatures> {
    let from_node = node_by_id(scenario, from)?;
    let to_node = node_by_id(scenario, to)?;
    let source_cluster = diffusion_source_cluster(scenario);
    let destination_cluster = diffusion_destination_cluster(scenario);
    let receiver_is_target = is_terminal_target(scenario, to);
    let new_cluster_coverage =
        is_target_node(scenario, to) && !covered_clusters.contains(&to_node.cluster_id);
    let same_cluster = from_node.cluster_id == to_node.cluster_id;
    let toward_destination_cluster = destination_cluster == Some(to_node.cluster_id);
    let leaving_source_cluster =
        source_cluster == Some(from_node.cluster_id) && source_cluster != Some(to_node.cluster_id);
    let bridge_candidate = diffusion_bridge_candidate(to_node);
    let continuity_value = receiver_is_target
        || new_cluster_coverage
        || toward_destination_cluster
        || bridge_candidate
        || leaving_source_cluster;
    let protected_opportunity = receiver_is_target
        || new_cluster_coverage
        || toward_destination_cluster
        || (!same_cluster && bridge_candidate)
        || (leaving_source_cluster && protected_bridge_family(scenario.family_id.as_str()));
    Some(FieldTransferFeatures {
        from_cluster_id: from_node.cluster_id,
        to_cluster_id: to_node.cluster_id,
        receiver_is_target,
        sender_is_observer: matches!(
            from_node.mobility_profile,
            DiffusionMobilityProfile::Observer
        ),
        receiver_is_observer: matches!(
            to_node.mobility_profile,
            DiffusionMobilityProfile::Observer
        ),
        same_cluster,
        new_cluster_coverage,
        expensive_transport: matches!(contact.transport_kind, DiffusionTransportKind::LoRa),
        continuity_value,
        protected_opportunity,
    })
}

pub(super) fn holder_count_in_cluster(
    scenario: &DiffusionScenarioSpec,
    holders: &BTreeMap<u32, HolderState>,
    pending: &[PendingTransfer],
    cluster_id: u8,
) -> usize {
    let holder_count = holders
        .keys()
        .filter(|node_id| {
            node_by_id(scenario, **node_id)
                .map(|node| node.cluster_id == cluster_id)
                .unwrap_or(false)
        })
        .count();
    let pending_count = pending
        .iter()
        .filter(|transfer| {
            node_by_id(scenario, transfer.target_node_id)
                .map(|node| node.cluster_id == cluster_id)
                .unwrap_or(false)
        })
        .count();
    holder_count.saturating_add(pending_count)
}

pub(super) fn covered_target_clusters(
    scenario: &DiffusionScenarioSpec,
    delivered_targets: &BTreeSet<u32>,
    pending: &[PendingTransfer],
) -> BTreeSet<u8> {
    let mut clusters = BTreeSet::new();
    for node_id in delivered_targets {
        if let Some(node) = node_by_id(scenario, *node_id) {
            clusters.insert(node.cluster_id);
        }
    }
    for transfer in pending {
        if is_target_node(scenario, transfer.target_node_id) {
            if let Some(node) = node_by_id(scenario, transfer.target_node_id) {
                clusters.insert(node.cluster_id);
            }
        }
    }
    clusters
}

pub(super) fn broadcast_cluster_seeding_active(
    scenario: &DiffusionScenarioSpec,
    posture: Option<DiffusionFieldPosture>,
    covered_clusters: &BTreeSet<u8>,
) -> bool {
    matches!(scenario.message_mode, DiffusionMessageMode::Broadcast)
        && matches!(posture, Some(DiffusionFieldPosture::ClusterSeeding))
        && covered_clusters.len() < scenario_target_cluster_count(scenario)
}

pub(super) fn field_budget_kind(
    scenario: &DiffusionScenarioSpec,
    posture: Option<DiffusionFieldPosture>,
    features: &FieldTransferFeatures,
    budget_state: &FieldBudgetState,
    covered_clusters: &BTreeSet<u8>,
) -> Option<FieldBudgetKind> {
    if features.receiver_is_target {
        return Some(FieldBudgetKind::Target);
    }
    if broadcast_cluster_seeding_active(scenario, posture, covered_clusters) {
        if features.new_cluster_coverage && budget_state.protected_remaining > 0 {
            return Some(FieldBudgetKind::Protected);
        }
        return None;
    }
    if matches!(scenario.message_mode, DiffusionMessageMode::Broadcast)
        && features.new_cluster_coverage
    {
        if budget_state.protected_remaining > 0 {
            return Some(FieldBudgetKind::Protected);
        }
        if budget_state.generic_remaining > 0 {
            return Some(FieldBudgetKind::Generic);
        }
        return None;
    }
    if features.protected_opportunity {
        if budget_state.protected_remaining > 0 {
            return Some(FieldBudgetKind::Protected);
        }
        if budget_state.generic_remaining > 0 {
            return Some(FieldBudgetKind::Generic);
        }
        return None;
    }
    if budget_state.generic_remaining > 0 {
        Some(FieldBudgetKind::Generic)
    } else {
        None
    }
}

pub(super) fn completion_forward_allowed(
    posture: DiffusionFieldPosture,
    features: &FieldTransferFeatures,
    receiver_cluster_holders: usize,
) -> bool {
    matches!(posture, DiffusionFieldPosture::DuplicateSuppressed)
        && features.same_cluster
        && !features.receiver_is_observer
        && !features.sender_is_observer
        && receiver_cluster_holders <= 1
}

pub(super) fn posture_limits(posture: DiffusionFieldPosture) -> (usize, usize, u32) {
    match posture {
        DiffusionFieldPosture::ContinuityBiased => (6, 2, 1),
        DiffusionFieldPosture::Balanced => (5, 2, 1),
        DiffusionFieldPosture::ScarcityConservative => (4, 1, 2),
        DiffusionFieldPosture::ClusterSeeding => (4, 1, 2),
        DiffusionFieldPosture::DuplicateSuppressed => (4, 1, 4),
        DiffusionFieldPosture::PrivacyConservative => (4, 1, 2),
    }
}

fn suppress_broadcast_nonnovel_forward(
    posture: DiffusionFieldPosture,
    receiver_cluster_holders: usize,
    features: &FieldTransferFeatures,
    completion_forward_allowed: bool,
    metrics: &mut FieldExecutionMetrics,
) -> bool {
    let seeding_blocks = matches!(posture, DiffusionFieldPosture::ClusterSeeding)
        && !features.new_cluster_coverage
        && !features.receiver_is_target
        && receiver_cluster_holders > 0;
    let duplicate_blocks = matches!(posture, DiffusionFieldPosture::DuplicateSuppressed)
        && !features.new_cluster_coverage
        && !features.receiver_is_target
        && !completion_forward_allowed;
    if !(seeding_blocks || duplicate_blocks) {
        return false;
    }
    metrics.redundant_forward_suppression_count = metrics
        .redundant_forward_suppression_count
        .saturating_add(1);
    if features.same_cluster {
        metrics.same_cluster_suppression_count =
            metrics.same_cluster_suppression_count.saturating_add(1);
    }
    true
}

fn suppress_holder_pressure(
    posture: DiffusionFieldPosture,
    holder_count: usize,
    receiver_cluster_holders: usize,
    features: &FieldTransferFeatures,
    completion_forward_allowed: bool,
    metrics: &mut FieldExecutionMetrics,
) -> bool {
    let (max_holders, max_same_cluster_holders, _) = posture_limits(posture);
    if !features.receiver_is_target
        && !features.protected_opportunity
        && holder_count >= max_holders
        && !completion_forward_allowed
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        return true;
    }
    if features.same_cluster
        && !features.receiver_is_target
        && receiver_cluster_holders >= max_same_cluster_holders
        && !completion_forward_allowed
    {
        metrics.same_cluster_suppression_count =
            metrics.same_cluster_suppression_count.saturating_add(1);
        if matches!(
            posture,
            DiffusionFieldPosture::ClusterSeeding | DiffusionFieldPosture::DuplicateSuppressed
        ) {
            metrics.redundant_forward_suppression_count = metrics
                .redundant_forward_suppression_count
                .saturating_add(1);
        }
        return true;
    }
    false
}

fn suppress_transport_or_visibility_risk(
    posture: DiffusionFieldPosture,
    sender_energy_ratio: u32,
    features: &FieldTransferFeatures,
    completion_forward_allowed: bool,
    metrics: &mut FieldExecutionMetrics,
) -> bool {
    if features.expensive_transport
        && !features.protected_opportunity
        && matches!(
            posture,
            DiffusionFieldPosture::ScarcityConservative
                | DiffusionFieldPosture::ClusterSeeding
                | DiffusionFieldPosture::DuplicateSuppressed
                | DiffusionFieldPosture::PrivacyConservative
        )
    {
        metrics.expensive_transport_suppression_count = metrics
            .expensive_transport_suppression_count
            .saturating_add(1);
        return true;
    }
    if matches!(posture, DiffusionFieldPosture::PrivacyConservative)
        && (features.sender_is_observer || features.receiver_is_observer)
        && !features.receiver_is_target
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        return true;
    }
    if matches!(posture, DiffusionFieldPosture::ScarcityConservative)
        && (!features.continuity_value || sender_energy_ratio < 520)
        && !features.receiver_is_target
    {
        if features.same_cluster {
            metrics.same_cluster_suppression_count =
                metrics.same_cluster_suppression_count.saturating_add(1);
        } else {
            metrics.redundant_forward_suppression_count = metrics
                .redundant_forward_suppression_count
                .saturating_add(1);
        }
        return true;
    }
    if matches!(
        posture,
        DiffusionFieldPosture::ClusterSeeding | DiffusionFieldPosture::DuplicateSuppressed
    ) && !features.continuity_value
        && !features.receiver_is_target
        && !features.new_cluster_coverage
        && !completion_forward_allowed
    {
        metrics.redundant_forward_suppression_count = metrics
            .redundant_forward_suppression_count
            .saturating_add(1);
        return true;
    }
    false
}

pub(super) fn within_cooldown(round: u32, last_round: u32, cooldown_rounds: u32) -> bool {
    round <= last_round.saturating_add(cooldown_rounds)
}

pub(super) fn posture_suppresses_reused_cluster_forward(posture: DiffusionFieldPosture) -> bool {
    matches!(
        posture,
        DiffusionFieldPosture::ClusterSeeding
            | DiffusionFieldPosture::DuplicateSuppressed
            | DiffusionFieldPosture::ScarcityConservative
    )
}

pub(super) fn posture_suppresses_reused_corridor_forward(posture: DiffusionFieldPosture) -> bool {
    matches!(
        posture,
        DiffusionFieldPosture::ClusterSeeding | DiffusionFieldPosture::DuplicateSuppressed
    )
}

pub(super) fn increment_redundant_suppression(metrics: &mut FieldExecutionMetrics) {
    metrics.redundant_forward_suppression_count = metrics
        .redundant_forward_suppression_count
        .saturating_add(1);
}

pub(super) fn increment_same_cluster_suppression(metrics: &mut FieldExecutionMetrics) {
    metrics.same_cluster_suppression_count =
        metrics.same_cluster_suppression_count.saturating_add(1);
}

fn suppress_recent_forward_reuse(
    posture: DiffusionFieldPosture,
    round: u32,
    cooldown_rounds: u32,
    features: &FieldTransferFeatures,
    completion_forward_allowed: bool,
    suppression_state: &FieldSuppressionState,
    metrics: &mut FieldExecutionMetrics,
) -> bool {
    if features.receiver_is_target {
        return false;
    }
    if suppression_state
        .recent_cluster_forward_round
        .get(&features.to_cluster_id)
        .is_some_and(|last_round| {
            within_cooldown(round, *last_round, cooldown_rounds)
                && posture_suppresses_reused_cluster_forward(posture)
                && !features.protected_opportunity
        })
    {
        increment_redundant_suppression(metrics);
        return true;
    }
    if features.same_cluster {
        if suppression_state
            .recent_same_cluster_forward_round
            .get(&features.from_cluster_id)
            .is_some_and(|last_round| {
                within_cooldown(round, *last_round, cooldown_rounds) && !completion_forward_allowed
            })
        {
            increment_same_cluster_suppression(metrics);
            if posture_suppresses_reused_corridor_forward(posture) {
                increment_redundant_suppression(metrics);
            }
            return true;
        }
        return false;
    }
    if suppression_state
        .recent_corridor_forward_round
        .get(&(features.from_cluster_id, features.to_cluster_id))
        .is_some_and(|last_round| {
            within_cooldown(round, *last_round, cooldown_rounds)
                && posture_suppresses_reused_corridor_forward(posture)
                && !features.protected_opportunity
        })
    {
        increment_redundant_suppression(metrics);
        return true;
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub(super) fn field_forwarding_suppressed(
    posture: DiffusionFieldPosture,
    round: u32,
    holder_count: usize,
    receiver_cluster_holders: usize,
    sender_energy_ratio: u32,
    features: &FieldTransferFeatures,
    suppression_state: &FieldSuppressionState,
    metrics: &mut FieldExecutionMetrics,
) -> bool {
    let completion_forward_allowed =
        completion_forward_allowed(posture, features, receiver_cluster_holders);
    let (_, _, cooldown_rounds) = posture_limits(posture);

    suppress_broadcast_nonnovel_forward(
        posture,
        receiver_cluster_holders,
        features,
        completion_forward_allowed,
        metrics,
    ) || suppress_holder_pressure(
        posture,
        holder_count,
        receiver_cluster_holders,
        features,
        completion_forward_allowed,
        metrics,
    ) || suppress_transport_or_visibility_risk(
        posture,
        sender_energy_ratio,
        features,
        completion_forward_allowed,
        metrics,
    ) || suppress_recent_forward_reuse(
        posture,
        round,
        cooldown_rounds,
        features,
        completion_forward_allowed,
        suppression_state,
        metrics,
    )
}
