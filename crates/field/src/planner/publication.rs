//! Neighbor publication selection and corroboration helpers.

use crate::state::{DestinationFieldState, DestinationKey, MAX_CONTINUATION_NEIGHBOR_COUNT};

// long-block-exception: service publication narrowing keeps corroboration and
// freshness ordering in one deterministic neighbor-selection pass.
pub(super) fn service_publication_neighbors(
    destination_state: &DestinationFieldState,
    selected_neighbor: jacquard_core::NodeId,
    search_config: &crate::FieldSearchConfig,
) -> Vec<jacquard_core::NodeId> {
    let mut scores: std::collections::BTreeMap<jacquard_core::NodeId, u32> =
        std::collections::BTreeMap::new();
    let freshest_forward_tick = destination_state
        .pending_forward_evidence
        .iter()
        .map(|evidence| evidence.observed_at_tick.0)
        .max()
        .unwrap_or(0);
    let freshest_frontier_tick = destination_state
        .frontier
        .as_slice()
        .iter()
        .map(|entry| entry.freshness.0)
        .max()
        .unwrap_or(0);
    for evidence in &destination_state.pending_forward_evidence {
        if evidence.summary.retention_support.value() >= 120
            && evidence.summary.delivery_support.value() >= 80
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            let freshness_gap = freshest_forward_tick.saturating_sub(evidence.observed_at_tick.0);
            let freshness_penalty =
                u32::try_from(freshness_gap.min(4)).expect("bounded freshness gap fits u32");
            let freshness_weight =
                u32::from(search_config.service_freshness_weight().clamp(25, 200));
            let score = u32::from(evidence.summary.retention_support.value())
                .saturating_add(u32::from(evidence.summary.delivery_support.value()))
                .saturating_sub(u32::from(evidence.summary.uncertainty_penalty.value()) / 2);
            let score = score
                .saturating_add(160)
                .saturating_sub((freshness_penalty * freshness_weight) / 4);
            scores
                .entry(evidence.from_neighbor)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    for entry in destination_state.frontier.as_slice() {
        let freshness_gap = freshest_frontier_tick.saturating_sub(entry.freshness.0);
        let freshness_penalty =
            u32::try_from(freshness_gap.min(4)).expect("bounded freshness gap fits u32");
        let freshness_weight = u32::from(search_config.service_freshness_weight().clamp(25, 200));
        let score = u32::from(entry.downstream_support.value())
            .saturating_add(u32::from(entry.net_value.value()));
        let score = score
            .saturating_add(120)
            .saturating_sub((freshness_penalty * freshness_weight) / 5);
        scores
            .entry(entry.neighbor_id)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    let mut ranked: Vec<(jacquard_core::NodeId, u32)> = scores.into_iter().collect();
    ranked.sort_by(
        |(left_neighbor, left_score), (right_neighbor, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_neighbor.cmp(right_neighbor))
        },
    );
    ranked
        .into_iter()
        .filter_map(|(neighbor, _)| (neighbor != selected_neighbor).then_some(neighbor))
        .take(
            search_config
                .service_publication_neighbor_limit()
                .min(MAX_CONTINUATION_NEIGHBOR_COUNT),
        )
        .collect()
}

pub(super) fn node_publication_neighbors(
    destination_state: &DestinationFieldState,
    selected_neighbor: jacquard_core::NodeId,
    search_config: &crate::FieldSearchConfig,
) -> Vec<jacquard_core::NodeId> {
    let support_floor = search_config
        .node_bootstrap_support_floor()
        .saturating_sub(20)
        .max(140);
    let mut scores: std::collections::BTreeMap<jacquard_core::NodeId, u32> =
        std::collections::BTreeMap::new();
    for evidence in &destination_state.pending_forward_evidence {
        if evidence.summary.delivery_support.value() >= support_floor.saturating_sub(20)
            && evidence.summary.uncertainty_penalty.value()
                <= search_config.node_bootstrap_entropy_ceiling()
        {
            let score = u32::from(evidence.summary.delivery_support.value())
                .saturating_add(u32::from(evidence.summary.retention_support.value()))
                .saturating_add(120);
            scores
                .entry(evidence.from_neighbor)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    for entry in destination_state.frontier.as_slice() {
        if entry.downstream_support.value() >= support_floor
            || corroborated_node_forward_support(destination_state, entry.neighbor_id)
                >= support_floor
        {
            let score = u32::from(entry.downstream_support.value())
                .saturating_add(u32::from(entry.net_value.value()))
                .saturating_add(80);
            scores
                .entry(entry.neighbor_id)
                .and_modify(|current| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    let mut ranked: Vec<(jacquard_core::NodeId, u32)> = scores.into_iter().collect();
    ranked.sort_by(
        |(left_neighbor, left_score), (right_neighbor, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_neighbor.cmp(right_neighbor))
        },
    );
    ranked
        .into_iter()
        .filter_map(|(neighbor, _)| (neighbor != selected_neighbor).then_some(neighbor))
        .take(2.min(MAX_CONTINUATION_NEIGHBOR_COUNT))
        .collect()
}

pub(crate) fn corroborated_node_forward_support(
    destination_state: &DestinationFieldState,
    neighbor_id: jacquard_core::NodeId,
) -> u16 {
    destination_state
        .pending_forward_evidence
        .iter()
        .filter(|evidence| evidence.from_neighbor == neighbor_id)
        .map(|evidence| evidence.summary.delivery_support.value())
        .max()
        .unwrap_or(0)
}

pub(super) fn service_corroborating_branch_count(
    destination_state: &DestinationFieldState,
) -> usize {
    if !matches!(destination_state.destination, DestinationKey::Service(_)) {
        return 0;
    }
    let mut neighbors = std::collections::BTreeSet::new();
    for entry in destination_state.frontier.as_slice() {
        if entry.downstream_support.value() >= 140 && entry.net_value.value() >= 180 {
            neighbors.insert(entry.neighbor_id);
        }
    }
    for evidence in &destination_state.pending_forward_evidence {
        if evidence.summary.retention_support.value() >= 120
            && evidence.summary.delivery_support.value() >= 80
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            neighbors.insert(evidence.from_neighbor);
        }
    }
    neighbors.len()
}

// long-block-exception: service corroboration scoring keeps the fused
// per-neighbor evidence buckets in one audited support calculation.
pub(super) fn service_corroborated_support_score(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> u16 {
    let mut per_neighbor: std::collections::BTreeMap<jacquard_core::NodeId, u32> =
        std::collections::BTreeMap::new();
    let freshest_forward_tick = destination_state
        .pending_forward_evidence
        .iter()
        .map(|evidence| evidence.observed_at_tick.0)
        .max()
        .unwrap_or(0);
    let freshest_frontier_tick = destination_state
        .frontier
        .as_slice()
        .iter()
        .map(|entry| entry.freshness.0)
        .max()
        .unwrap_or(0);
    for entry in destination_state.frontier.as_slice() {
        let freshness_gap = freshest_frontier_tick.saturating_sub(entry.freshness.0);
        let freshness_penalty = u32::try_from(freshness_gap.min(5))
            .expect("bounded freshness gap fits u32")
            * (u32::from(search_config.service_freshness_weight().clamp(25, 200)) / 10).max(1);
        let score = u32::from(entry.downstream_support.value())
            .saturating_add(u32::from(entry.net_value.value()))
            .saturating_sub(freshness_penalty);
        per_neighbor
            .entry(entry.neighbor_id)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    for evidence in &destination_state.pending_forward_evidence {
        let freshness_gap = freshest_forward_tick.saturating_sub(evidence.observed_at_tick.0);
        let freshness_penalty = u32::try_from(freshness_gap.min(5))
            .expect("bounded freshness gap fits u32")
            * (u32::from(search_config.service_freshness_weight().clamp(25, 200)) / 8).max(1);
        let score = u32::from(evidence.summary.delivery_support.value())
            .saturating_add(u32::from(evidence.summary.retention_support.value()))
            .saturating_sub(u32::from(evidence.summary.uncertainty_penalty.value()) / 3)
            .saturating_sub(freshness_penalty);
        per_neighbor
            .entry(evidence.from_neighbor)
            .and_modify(|current| *current = (*current).max(score))
            .or_insert(score);
    }
    let corroborating_count = per_neighbor.len();
    let mut branch_scores: Vec<u32> = per_neighbor.into_values().collect();
    branch_scores.sort_unstable_by(|left, right| right.cmp(left));
    let branch_mass = branch_scores.iter().take(3).copied().sum::<u32>()
        / u32::try_from(corroborating_count.clamp(1, 3)).expect("bounded branch count fits");
    let diversity_floor = branch_scores
        .get(1)
        .copied()
        .unwrap_or(branch_scores.first().copied().unwrap_or(0));
    let score = u32::from(destination_state.posterior.top_corridor_mass.value())
        .max(u32::from(
            destination_state.corridor_belief.delivery_support.value(),
        ))
        .saturating_add(branch_mass / 2)
        .saturating_add(diversity_floor / 4)
        .saturating_add(
            u32::try_from(corroborating_count.saturating_sub(1))
                .expect("branch count fits")
                .saturating_mul(70),
        );
    u16::try_from(score.min(1000)).expect("service support score capped to bucket max")
}

pub(super) fn publication_confidence_for(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> u16 {
    if matches!(destination_state.destination, DestinationKey::Service(_)) {
        destination_state.posterior.top_corridor_mass.value().max(
            service_corroborated_support_score(destination_state, search_config),
        )
    } else {
        destination_state.posterior.top_corridor_mass.value()
    }
}
