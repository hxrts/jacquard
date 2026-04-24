//! Forwarding score computation for diffusion routing decisions.

use super::{
    diffusion_bridge_candidate, diffusion_destination_cluster, diffusion_source_cluster,
    is_target_node, node_by_id, sender_energy_ratio_permille, DiffusionContactEvent,
    DiffusionFieldPosture, DiffusionForwardingStyle, DiffusionMessageMode,
    DiffusionMobilityProfile, DiffusionNodeSpec, DiffusionPolicyConfig, DiffusionScenarioSpec,
    DiffusionTransportKind, FieldTransferFeatures, ForwardingGeometry, ForwardingNodes,
    ForwardingOpportunity, ForwardingScoreContext,
};

fn base_forwarding_score(
    scenario: &DiffusionScenarioSpec,
    policy: &DiffusionPolicyConfig,
    from_node: &DiffusionNodeSpec,
    to_node: &DiffusionNodeSpec,
    contact: &DiffusionContactEvent,
    toward_destination_cluster: bool,
    bridge_candidate: bool,
) -> i32 {
    let mut score = i32::try_from(policy.forward_probability_permille).unwrap_or(i32::MAX);
    if toward_destination_cluster {
        score = score.saturating_add(policy.target_cluster_bias_permille);
    }
    score = score.saturating_add(if from_node.cluster_id == to_node.cluster_id {
        policy.same_cluster_bias_permille
    } else {
        -policy.same_cluster_bias_permille / 2
    });
    if bridge_candidate {
        score =
            score.saturating_add(i32::try_from(policy.bridge_bias_permille).unwrap_or(i32::MAX));
    }
    if matches!(to_node.mobility_profile, DiffusionMobilityProfile::Observer) {
        score = score.saturating_sub(policy.observer_aversion_permille);
    }
    if matches!(contact.transport_kind, DiffusionTransportKind::LoRa) {
        score = score.saturating_add(policy.lora_bias_permille);
    }
    if is_target_node(scenario, to_node.node_id) {
        score = score.saturating_add(240);
    }
    score
}

pub(super) fn apply_field_broadcast_bonus(
    mut score: i32,
    scenario: &DiffusionScenarioSpec,
    field_features: Option<&FieldTransferFeatures>,
) -> i32 {
    if let Some(features) = field_features {
        if matches!(scenario.message_mode, DiffusionMessageMode::Broadcast) {
            if features.new_cluster_coverage {
                score = score.saturating_add(260);
            } else {
                score = score.saturating_sub(220);
            }
            if features.same_cluster {
                score = score.saturating_sub(140);
            }
        }
    }
    score
}

pub(super) fn apply_energy_and_spread_penalties(
    mut score: i32,
    policy: &DiffusionPolicyConfig,
    sender_energy_ratio: u32,
    holder_count: usize,
) -> i32 {
    if sender_energy_ratio < 250 {
        score =
            score.saturating_sub(i32::try_from(policy.energy_guard_permille).unwrap_or(i32::MAX));
    } else if sender_energy_ratio < 500 {
        score = score
            .saturating_sub(i32::try_from(policy.energy_guard_permille / 2).unwrap_or(i32::MAX));
    }
    let spread_penalty = i32::try_from(
        policy
            .spread_restraint_permille
            .saturating_mul(u32::try_from(holder_count).unwrap_or(u32::MAX))
            / 8,
    )
    .unwrap_or(i32::MAX);
    score.saturating_sub(spread_penalty)
}

// long-block-exception: this match is a one-to-one mapping from posture enum to
// forwarding score adjustments, so keeping it together preserves policy readability.
pub(super) fn apply_posture_score(
    mut score: i32,
    posture: DiffusionFieldPosture,
    opp: ForwardingOpportunity<'_>,
    nodes: ForwardingNodes<'_>,
    holder_count: usize,
    geometry: ForwardingGeometry,
    field_features: Option<&FieldTransferFeatures>,
) -> i32 {
    let ForwardingGeometry {
        toward_destination_cluster,
        leaving_source_cluster,
        bridge_candidate,
    } = geometry;
    match posture {
        DiffusionFieldPosture::ContinuityBiased => {
            if bridge_candidate {
                score = score.saturating_add(160);
            }
            if leaving_source_cluster {
                score = score.saturating_add(90);
            }
            if toward_destination_cluster {
                score = score.saturating_add(95);
            }
            if matches!(opp.contact.transport_kind, DiffusionTransportKind::LoRa) {
                score = score.saturating_add(60);
            }
            score = score.saturating_add(40);
        }
        DiffusionFieldPosture::Balanced => {
            if bridge_candidate {
                score = score.saturating_add(100);
            }
            if toward_destination_cluster {
                score = score.saturating_add(70);
            }
            if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast)
                && field_features
                    .map(|features| features.new_cluster_coverage)
                    .unwrap_or(false)
            {
                score = score.saturating_add(110);
            }
        }
        DiffusionFieldPosture::ScarcityConservative => {
            score = score.saturating_sub(220);
            if bridge_candidate && toward_destination_cluster {
                score = score.saturating_add(130);
            } else if toward_destination_cluster
                || is_target_node(opp.scenario, nodes.to_node.node_id)
            {
                score = score.saturating_add(95);
            }
            if matches!(opp.contact.transport_kind, DiffusionTransportKind::LoRa) {
                score = score.saturating_sub(140);
            }
            if nodes.from_node.cluster_id == nodes.to_node.cluster_id {
                score = score.saturating_sub(140);
            }
            if holder_count > 1 {
                score = score.saturating_sub(110);
            }
            if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast)
                && field_features
                    .map(|features| features.new_cluster_coverage)
                    .unwrap_or(false)
            {
                score = score.saturating_add(140);
            }
        }
        DiffusionFieldPosture::ClusterSeeding => {
            score = score.saturating_sub(210);
            if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast) {
                score = score.saturating_sub(120);
            }
            if toward_destination_cluster || is_target_node(opp.scenario, nodes.to_node.node_id) {
                score = score.saturating_add(120);
            }
            if nodes.from_node.cluster_id == nodes.to_node.cluster_id {
                score = score.saturating_sub(170);
            }
            if holder_count > 2 {
                score = score.saturating_sub(120);
            }
            if let Some(features) = field_features {
                if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast) {
                    if features.new_cluster_coverage {
                        score = score.saturating_add(320);
                        if !features.same_cluster {
                            score = score.saturating_add(70);
                        }
                    } else {
                        score = score.saturating_sub(260);
                    }
                }
            }
        }
        DiffusionFieldPosture::DuplicateSuppressed => {
            score = score.saturating_sub(320);
            if toward_destination_cluster || is_target_node(opp.scenario, nodes.to_node.node_id) {
                score = score.saturating_add(100);
            }
            if nodes.from_node.cluster_id == nodes.to_node.cluster_id {
                score = score.saturating_sub(220);
            }
            if holder_count > 1 {
                score = score.saturating_sub(180);
            }
            if let Some(features) = field_features {
                if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast) {
                    if features.new_cluster_coverage {
                        score = score.saturating_add(280);
                    } else {
                        score = score.saturating_sub(340);
                    }
                }
            }
        }
        DiffusionFieldPosture::PrivacyConservative => {
            score = score.saturating_sub(120);
            if matches!(
                nodes.to_node.mobility_profile,
                DiffusionMobilityProfile::Observer
            ) {
                score = score.saturating_sub(320);
            }
            if toward_destination_cluster || is_target_node(opp.scenario, nodes.to_node.node_id) {
                score = score.saturating_add(110);
            }
        }
    }
    score
}

// long-block-exception: this match is a one-to-one mapping from forwarding style
// enum to score adjustments, so keeping it together preserves policy readability.
pub(super) fn apply_forwarding_style_score(
    mut score: i32,
    opp: ForwardingOpportunity<'_>,
    policy: &DiffusionPolicyConfig,
    from_node: &DiffusionNodeSpec,
    to_node: &DiffusionNodeSpec,
    holder_count: usize,
    geometry: ForwardingGeometry,
) -> i32 {
    let ForwardingGeometry {
        toward_destination_cluster,
        leaving_source_cluster,
        bridge_candidate,
    } = geometry;
    match policy.forwarding_style {
        DiffusionForwardingStyle::ConservativeLocal => {
            if leaving_source_cluster
                && !toward_destination_cluster
                && !is_target_node(opp.scenario, to_node.node_id)
            {
                score = score.saturating_sub(180);
            }
            if matches!(opp.contact.transport_kind, DiffusionTransportKind::LoRa) {
                score = score.saturating_sub(80);
            }
        }
        DiffusionForwardingStyle::BalancedDistanceVector => {
            if leaving_source_cluster && !toward_destination_cluster {
                score = score.saturating_sub(70);
            }
            if bridge_candidate && toward_destination_cluster {
                score = score.saturating_add(40);
            }
        }
        DiffusionForwardingStyle::FreshnessAware => {
            if bridge_candidate {
                score = score.saturating_add(50);
            }
            if holder_count > 3 {
                score = score.saturating_sub(40);
            }
        }
        DiffusionForwardingStyle::ServiceDirected => {
            if toward_destination_cluster {
                score = score.saturating_add(160);
            }
            if leaving_source_cluster {
                score = score.saturating_add(50);
            }
            if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast) {
                score = score.saturating_add(80);
            }
        }
        DiffusionForwardingStyle::ContinuityBiased => {
            if bridge_candidate {
                score = score.saturating_add(110);
            }
            if leaving_source_cluster {
                score = score.saturating_add(75);
            }
            if toward_destination_cluster {
                score = score.saturating_add(85);
            }
            if matches!(to_node.mobility_profile, DiffusionMobilityProfile::Observer) {
                score = score.saturating_sub(80);
            }
            if matches!(opp.scenario.message_mode, DiffusionMessageMode::Broadcast) {
                score = score.saturating_add(120);
                if from_node.cluster_id == to_node.cluster_id {
                    score = score.saturating_sub(90);
                }
                if bridge_candidate || leaving_source_cluster {
                    score = score.saturating_add(70);
                }
            }
        }
        DiffusionForwardingStyle::Composite => {
            if toward_destination_cluster {
                score = score.saturating_add(110);
            }
            if bridge_candidate {
                score = score.saturating_add(65);
            }
            if holder_count > 5 {
                score = score.saturating_sub(30);
            }
        }
    }
    score
}

pub(super) fn apply_family_forwarding_adjustment(
    mut score: i32,
    family_id: &str,
    message_mode: DiffusionMessageMode,
    policy: &DiffusionPolicyConfig,
    sender_energy_ratio: u32,
    holder_count: usize,
    geometry: ForwardingGeometry,
) -> i32 {
    let ForwardingGeometry {
        toward_destination_cluster,
        leaving_source_cluster,
        bridge_candidate,
    } = geometry;
    if matches!(message_mode, DiffusionMessageMode::Broadcast) {
        score = score.saturating_add(60);
    }
    match family_id {
        "diffusion-high-density-overload"
        | "diffusion-congestion-cascade"
        | "diffusion-large-congestion-threshold-moderate"
        | "diffusion-large-congestion-threshold-high" => {
            let holder_penalty = if policy.config_id == "mercator" {
                8
            } else {
                18
            };
            score = score
                .saturating_sub(i32::try_from(holder_count).unwrap_or(i32::MAX) * holder_penalty);
            if policy.config_id == "mercator" && (bridge_candidate || leaving_source_cluster) {
                score = score.saturating_add(90);
            }
        }
        "diffusion-bridge-drought" => {
            if bridge_candidate || toward_destination_cluster {
                score = score.saturating_add(100);
            } else if leaving_source_cluster {
                score = score.saturating_sub(120);
            }
        }
        "diffusion-energy-starved-relay" => {
            if sender_energy_ratio < 400 {
                score = score.saturating_sub(90);
            }
        }
        _ => {}
    }
    score
}

pub(super) fn forwarding_geometry(
    scenario: &DiffusionScenarioSpec,
    from_node: &DiffusionNodeSpec,
    to_node: &DiffusionNodeSpec,
) -> ForwardingGeometry {
    let source_cluster = diffusion_source_cluster(scenario);
    let destination_cluster = diffusion_destination_cluster(scenario);
    ForwardingGeometry {
        toward_destination_cluster: destination_cluster == Some(to_node.cluster_id),
        leaving_source_cluster: source_cluster == Some(from_node.cluster_id)
            && source_cluster != Some(to_node.cluster_id),
        bridge_candidate: diffusion_bridge_candidate(to_node),
    }
}

pub(super) fn apply_posture_or_style_score(
    score: i32,
    field_posture: Option<DiffusionFieldPosture>,
    context: ForwardingScoreContext<'_>,
) -> i32 {
    if let Some(posture) = field_posture {
        apply_posture_score(
            score,
            posture,
            context.opp,
            context.nodes,
            context.holder_count,
            context.geometry,
            context.field_features,
        )
    } else {
        apply_forwarding_style_score(
            score,
            context.opp,
            context.policy,
            context.nodes.from_node,
            context.nodes.to_node,
            context.holder_count,
            context.geometry,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn forwarding_score(
    scenario: &DiffusionScenarioSpec,
    policy: &DiffusionPolicyConfig,
    from: u32,
    to: u32,
    contact: &DiffusionContactEvent,
    holder_count: usize,
    sender_energy_remaining: u32,
    field_posture: Option<DiffusionFieldPosture>,
    field_features: Option<&FieldTransferFeatures>,
) -> u32 {
    let Some(from_node) = node_by_id(scenario, from) else {
        return 0;
    };
    let Some(to_node) = node_by_id(scenario, to) else {
        return 0;
    };
    let geometry = forwarding_geometry(scenario, from_node, to_node);
    let sender_energy_ratio = sender_energy_ratio_permille(from_node, sender_energy_remaining);
    let mut score = base_forwarding_score(
        scenario,
        policy,
        from_node,
        to_node,
        contact,
        geometry.toward_destination_cluster,
        geometry.bridge_candidate,
    );
    score = apply_field_broadcast_bonus(score, scenario, field_features);
    score = apply_energy_and_spread_penalties(score, policy, sender_energy_ratio, holder_count);
    let opp = ForwardingOpportunity { scenario, contact };
    let nodes = ForwardingNodes { from_node, to_node };
    score = apply_posture_or_style_score(
        score,
        field_posture,
        ForwardingScoreContext {
            opp,
            policy,
            nodes,
            holder_count,
            geometry,
            field_features,
        },
    );
    score = apply_family_forwarding_adjustment(
        score,
        scenario.family_id.as_str(),
        scenario.message_mode,
        policy,
        sender_energy_ratio,
        holder_count,
        geometry,
    );
    score.clamp(0, 1000).cast_unsigned()
}
