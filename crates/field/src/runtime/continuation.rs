#![allow(clippy::wildcard_imports)]

use super::*;

pub(super) fn pending_forward_continuations_for_maintenance(
    destination_state: &crate::state::DestinationFieldState,
) -> Vec<NeighborContinuation> {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let service_bias = matches!(
        destination_state.destination,
        crate::state::DestinationKey::Service(_)
    );
    destination_state
        .pending_forward_evidence
        .iter()
        .filter(|evidence| {
            evidence.summary.retention_support.value()
                >= if service_bias {
                    policy.service_pending_retention_floor_permille
                } else {
                    policy.node_pending_retention_floor_permille
                }
                && evidence.summary.delivery_support.value()
                    >= if service_bias {
                        policy.service_pending_support_floor_permille
                    } else {
                        policy.node_pending_support_floor_permille
                    }
                && evidence.summary.uncertainty_penalty.value()
                    <= if service_bias {
                        policy.service_pending_uncertainty_ceiling_permille
                    } else {
                        policy.node_pending_uncertainty_ceiling_permille
                    }
        })
        .map(|evidence| NeighborContinuation {
            neighbor_id: evidence.from_neighbor,
            net_value: SupportBucket::new(
                evidence
                    .summary
                    .delivery_support
                    .value()
                    .saturating_add(evidence.summary.retention_support.value() / 3)
                    .min(1000),
            ),
            downstream_support: evidence.summary.delivery_support,
            expected_hop_band: HopBand::new(
                evidence.summary.hop_band.min_hops.saturating_add(1),
                evidence.summary.hop_band.max_hops.saturating_add(1),
            ),
            freshness: evidence.observed_at_tick,
        })
        .collect()
}

pub(super) fn preferred_service_shift_neighbor(
    active: &ActiveFieldRoute,
    ranked: &[(NeighborContinuation, SupportBucket)],
    ranked_best: &NeighborContinuation,
    search_config: &crate::FieldSearchConfig,
) -> Option<NodeId> {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let quality_margin = policy
        .service_shift_quality_margin_permille
        .saturating_sub(search_config.service_narrowing_bias() / 2);
    let downstream_margin = policy
        .service_shift_downstream_margin_permille
        .saturating_sub(search_config.service_narrowing_bias() / 3);
    ranked
        .iter()
        .find(|(entry, _)| {
            (entry.neighbor_id != active.selected_neighbor
                && active.continuation_neighbors.contains(&entry.neighbor_id)
                && service_neighbor_quality(entry, search_config)
                    >= service_neighbor_quality(ranked_best, search_config))
                || (entry.neighbor_id != active.selected_neighbor
                    && active.continuation_neighbors.contains(&entry.neighbor_id)
                    && entry.net_value.value().saturating_add(quality_margin)
                        >= ranked_best.net_value.value()
                    && entry
                        .downstream_support
                        .value()
                        .saturating_add(downstream_margin)
                        >= ranked_best.downstream_support.value()
                    && entry.freshness.0.saturating_add(2) >= ranked_best.freshness.0)
        })
        .map(|(entry, _)| entry.neighbor_id)
}

pub(super) fn service_runtime_continuation_neighbors(
    ranked: &[(NeighborContinuation, SupportBucket)],
    destination_state: &crate::state::DestinationFieldState,
    selected_neighbor: NodeId,
    search_config: &crate::FieldSearchConfig,
) -> Vec<NodeId> {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let support_floor = policy
        .service_runtime_support_floor_permille
        .saturating_add(search_config.service_narrowing_bias() / 5);
    let max_neighbors = search_config
        .service_publication_neighbor_limit()
        .clamp(1, crate::state::MAX_CONTINUATION_NEIGHBOR_COUNT)
        .min(if search_config.service_narrowing_bias() >= 160 {
            2
        } else {
            crate::state::MAX_CONTINUATION_NEIGHBOR_COUNT
        });
    let mut service_ranked: Vec<_> = ranked
        .iter()
        .map(|(entry, _)| entry.clone())
        .filter(|entry| {
            entry.neighbor_id == selected_neighbor
                || entry.downstream_support.value() >= support_floor
                || corroborated_service_forward_support(destination_state, entry.neighbor_id)
                    >= support_floor
        })
        .collect();
    service_ranked.sort_by(|left, right| {
        service_neighbor_quality(right, search_config)
            .cmp(&service_neighbor_quality(left, search_config))
            .then_with(|| left.neighbor_id.cmp(&right.neighbor_id))
    });
    let mut continuation_neighbors = Vec::with_capacity(max_neighbors);
    for entry in service_ranked {
        if continuation_neighbors.contains(&entry.neighbor_id) {
            continue;
        }
        continuation_neighbors.push(entry.neighbor_id);
        if continuation_neighbors.len() >= max_neighbors {
            break;
        }
    }
    if !continuation_neighbors.contains(&selected_neighbor) {
        continuation_neighbors.insert(0, selected_neighbor);
    }
    continuation_neighbors.truncate(max_neighbors);
    continuation_neighbors
}

pub(super) fn preferred_node_shift_neighbor(
    active: &ActiveFieldRoute,
    ranked: &[(NeighborContinuation, SupportBucket)],
    destination_state: &crate::state::DestinationFieldState,
    neighbor_endpoints: &std::collections::BTreeMap<NodeId, jacquard_core::LinkEndpoint>,
    search_config: &crate::FieldSearchConfig,
) -> Option<NodeId> {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let support_floor = search_config
        .node_bootstrap_support_floor()
        .saturating_sub(20)
        .max(policy.node_selection_support_floor_permille);
    ranked
        .iter()
        .find(|(entry, _)| {
            entry.neighbor_id != active.selected_neighbor
                && active.continuation_neighbors.contains(&entry.neighbor_id)
                && neighbor_endpoints.contains_key(&entry.neighbor_id)
                && (entry.downstream_support.value() >= support_floor
                    || crate::planner::publication::corroborated_node_forward_support(
                        destination_state,
                        entry.neighbor_id,
                    ) >= support_floor)
        })
        .map(|(entry, _)| entry.neighbor_id)
}

pub(super) fn node_runtime_continuation_neighbors(
    ranked: &[(NeighborContinuation, SupportBucket)],
    destination_state: &crate::state::DestinationFieldState,
    selected_neighbor: NodeId,
    search_config: &crate::FieldSearchConfig,
) -> Vec<NodeId> {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let support_floor = search_config
        .node_bootstrap_support_floor()
        .saturating_sub(20)
        .max(policy.node_runtime_support_floor_permille);
    let max_neighbors = 2usize.min(crate::state::MAX_CONTINUATION_NEIGHBOR_COUNT);
    let mut node_ranked: Vec<_> = ranked
        .iter()
        .map(|(entry, _)| entry.clone())
        .filter(|entry| {
            entry.neighbor_id == selected_neighbor
                || entry.downstream_support.value() >= support_floor
                || crate::planner::publication::corroborated_node_forward_support(
                    destination_state,
                    entry.neighbor_id,
                ) >= support_floor
        })
        .collect();
    node_ranked.sort_by(|left, right| {
        right
            .downstream_support
            .value()
            .cmp(&left.downstream_support.value())
            .then_with(|| right.net_value.value().cmp(&left.net_value.value()))
            .then_with(|| left.neighbor_id.cmp(&right.neighbor_id))
    });
    let mut continuation_neighbors = Vec::with_capacity(max_neighbors);
    for entry in node_ranked {
        if continuation_neighbors.contains(&entry.neighbor_id) {
            continue;
        }
        continuation_neighbors.push(entry.neighbor_id);
        if continuation_neighbors.len() >= max_neighbors {
            break;
        }
    }
    if !continuation_neighbors.contains(&selected_neighbor) {
        continuation_neighbors.insert(0, selected_neighbor);
    }
    continuation_neighbors.truncate(max_neighbors);
    continuation_neighbors
}

// long-block-exception: synthesized carry-forward ranking keeps the degraded
// node-route fallback ordering in one deterministic selection pass.
pub(super) fn synthesized_node_carry_forward_ranked(
    active: &ActiveFieldRoute,
    destination_state: &crate::state::DestinationFieldState,
    neighbor_endpoints: &std::collections::BTreeMap<NodeId, jacquard_core::LinkEndpoint>,
    now_tick: Tick,
    search_config: &crate::FieldSearchConfig,
) -> Vec<(NeighborContinuation, SupportBucket)> {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let observer_policy = &crate::policy::DEFAULT_FIELD_POLICY.evidence.observer;
    let Some(last_summary) = destination_state.publication.last_summary.as_ref() else {
        return Vec::new();
    };
    let Some(last_sent_at) = destination_state.publication.last_sent_at else {
        return Vec::new();
    };
    if now_tick.0.saturating_sub(last_sent_at.0)
        > FIELD_DEGRADED_STEADY_STALE_TICKS_MAX
            .saturating_add(observer_policy.synthesized_node_publication_staleness_slack_ticks)
    {
        return Vec::new();
    }
    let support_floor = search_config
        .node_bootstrap_support_floor()
        .saturating_sub(40)
        .max(observer_policy.synthesized_node_support_floor_min_permille);
    let base_delivery = destination_state
        .corridor_belief
        .delivery_support
        .value()
        .max(last_summary.delivery_support.value());
    let base_retention = destination_state
        .corridor_belief
        .retention_affinity
        .value()
        .max(last_summary.retention_support.value());
    let hop_band = active
        .corridor_envelope
        .expected_hop_band
        .max(last_summary.hop_band);
    let mut ranked = active
        .continuation_neighbors
        .iter()
        .enumerate()
        .filter_map(|(index, neighbor_id)| {
            let corroborated = crate::planner::publication::corroborated_node_forward_support(
                destination_state,
                *neighbor_id,
            );
            let reachable = neighbor_endpoints.contains_key(neighbor_id);
            if !reachable && corroborated < support_floor {
                return None;
            }
            let rank_penalty = u16::try_from(index)
                .unwrap_or(u16::MAX)
                .saturating_mul(policy.synthesized_rank_penalty_permille);
            let reachability_bonus = if reachable {
                policy.synthesized_reachability_bonus_permille
            } else {
                0
            };
            let selection_bonus = if *neighbor_id == active.selected_neighbor {
                policy.synthesized_selected_neighbor_bonus_permille
            } else {
                0
            };
            let delivery = base_delivery
                .max(corroborated)
                .saturating_add(reachability_bonus)
                .saturating_add(selection_bonus)
                .saturating_sub(rank_penalty)
                .min(1000);
            let retention = base_retention
                .saturating_add(if reachable {
                    policy.synthesized_retention_bonus_permille
                } else {
                    0
                })
                .saturating_sub(rank_penalty / 2)
                .min(1000);
            Some((
                NeighborContinuation {
                    neighbor_id: *neighbor_id,
                    net_value: SupportBucket::new(retention),
                    downstream_support: SupportBucket::new(delivery),
                    expected_hop_band: hop_band,
                    freshness: now_tick,
                },
                SupportBucket::new(delivery.max(retention / 2)),
            ))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_entry, left_score), (right_entry, right_score)| {
        right_score
            .value()
            .cmp(&left_score.value())
            .then_with(|| {
                right_entry
                    .downstream_support
                    .value()
                    .cmp(&left_entry.downstream_support.value())
            })
            .then_with(|| {
                right_entry
                    .net_value
                    .value()
                    .cmp(&left_entry.net_value.value())
            })
            .then_with(|| left_entry.neighbor_id.cmp(&right_entry.neighbor_id))
    });
    ranked
}

pub(super) fn corroborated_service_forward_support(
    destination_state: &crate::state::DestinationFieldState,
    neighbor_id: NodeId,
) -> u16 {
    destination_state
        .pending_forward_evidence
        .iter()
        .filter(|evidence| evidence.from_neighbor == neighbor_id)
        .map(|evidence| {
            evidence
                .summary
                .delivery_support
                .value()
                .saturating_add(evidence.summary.retention_support.value() / 2)
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn service_neighbor_quality(
    entry: &NeighborContinuation,
    search_config: &crate::FieldSearchConfig,
) -> u32 {
    let freshness_weight = u32::from(search_config.service_freshness_weight().clamp(25, 200));
    u32::from(entry.downstream_support.value())
        .saturating_add(u32::from(entry.net_value.value()))
        .saturating_add(
            u32::try_from(entry.freshness.0.min(32)).expect("freshness fits u32")
                * (freshness_weight / 10).max(1),
        )
}

pub(super) fn continuation_shift_grace_active(
    active: &ActiveFieldRoute,
    promotion_assessment: &crate::planner::promotion::FieldPromotionAssessment,
) -> bool {
    active.recovery.state.last_outcome == Some(FieldRouteRecoveryOutcome::ContinuationRetained)
        && matches!(
            active.continuity_band,
            FieldContinuityBand::DegradedSteady | FieldContinuityBand::Bootstrap
        )
        && promotion_assessment.anti_entropy_confirmed
        && promotion_assessment.continuation_coherent
}

pub(super) fn service_corridor_viable(
    active: &ActiveFieldRoute,
    destination_state: &crate::state::DestinationFieldState,
) -> bool {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let viable_frontier_branches = destination_state
        .frontier
        .as_slice()
        .iter()
        .filter(|entry| {
            active.continuation_neighbors.contains(&entry.neighbor_id)
                && entry.downstream_support.value()
                    >= policy.service_frontier_viability_support_floor_permille
                && entry.net_value.value()
                    >= policy.service_frontier_viability_retention_floor_permille
        })
        .count();
    let viable_forward_branches = destination_state
        .pending_forward_evidence
        .iter()
        .filter(|evidence| {
            active
                .continuation_neighbors
                .contains(&evidence.from_neighbor)
                && evidence.summary.delivery_support.value()
                    >= policy.service_forward_viability_support_floor_permille
                && evidence.summary.retention_support.value()
                    >= policy.service_forward_viability_retention_floor_permille
                && evidence.summary.uncertainty_penalty.value()
                    <= policy.service_forward_viability_uncertainty_ceiling_permille
        })
        .count();
    viable_frontier_branches + viable_forward_branches >= policy.service_viable_branch_count_min
}

pub(super) fn node_corridor_viable(
    active: &ActiveFieldRoute,
    destination_state: &crate::state::DestinationFieldState,
) -> bool {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let viable_frontier_branches = destination_state
        .frontier
        .as_slice()
        .iter()
        .filter(|entry| {
            active.continuation_neighbors.contains(&entry.neighbor_id)
                && entry.downstream_support.value()
                    >= policy.node_frontier_viability_support_floor_permille
                && entry.net_value.value()
                    >= policy.node_frontier_viability_retention_floor_permille
        })
        .count();
    let viable_forward_branches = destination_state
        .pending_forward_evidence
        .iter()
        .filter(|evidence| {
            active
                .continuation_neighbors
                .contains(&evidence.from_neighbor)
                && evidence.summary.delivery_support.value()
                    >= policy.node_forward_viability_support_floor_permille
                && evidence.summary.retention_support.value()
                    >= policy.node_forward_viability_retention_floor_permille
                && evidence.summary.uncertainty_penalty.value()
                    <= policy.node_forward_viability_uncertainty_ceiling_permille
        })
        .count();
    viable_frontier_branches + viable_forward_branches >= policy.node_viable_branch_count_min
}

pub(super) fn observer_input_signature(
    topology_epoch: jacquard_core::RouteEpoch,
    regime: crate::state::OperatingRegime,
    control_state: &crate::state::ControlState,
    direct_evidence: &[DirectEvidence],
    forward_evidence: &[crate::summary::ForwardPropagatedEvidence],
    reverse_feedback: &[crate::summary::ReverseFeedbackEvidence],
) -> ObserverInputSignature {
    ObserverInputSignature {
        topology_epoch,
        regime,
        direct_digest: direct_evidence_digest(direct_evidence),
        forward_digest: forward_evidence_digest(forward_evidence),
        reverse_digest: reverse_feedback_digest(reverse_feedback),
        control_signature: [
            control_state.congestion_price.value(),
            control_state.relay_price.value(),
            control_state.retention_price.value(),
            control_state.risk_price.value(),
            control_state.congestion_error_integral.value(),
            control_state.retention_error_integral.value(),
            control_state.relay_error_integral.value(),
            control_state.churn_error_integral.value(),
        ],
    }
}

pub(super) fn should_transmit_summary(
    destination_state: &crate::state::DestinationFieldState,
    summary: &FieldSummary,
    now_tick: Tick,
) -> bool {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.continuity.continuation;
    let Some(previous_summary) = destination_state.publication.last_summary.as_ref() else {
        return true;
    };
    let Some(last_sent_at) = destination_state.publication.last_sent_at else {
        return true;
    };
    if now_tick.0.saturating_sub(last_sent_at.0) >= SUMMARY_HEARTBEAT_TICKS {
        return true;
    }
    if summary_divergence(previous_summary, summary).value()
        >= policy.transmission_divergence_trigger_permille
    {
        return true;
    }
    destination_state.corridor_belief.delivery_support.value()
        < policy.transmission_weak_support_floor_permille
        && destination_state.corridor_belief.retention_affinity.value()
            >= policy.transmission_retention_affinity_floor_permille
        && now_tick.0.saturating_sub(last_sent_at.0) >= SUMMARY_HEARTBEAT_TICKS.saturating_sub(1)
}

pub(super) fn direct_evidence_digest(direct_evidence: &[DirectEvidence]) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for evidence in direct_evidence {
        digest = mix_digest(digest, &evidence.neighbor_id.0);
        digest = mix_digest(
            digest,
            &evidence.link.profile.latency_floor_ms.0.to_le_bytes(),
        );
        digest = mix_digest(digest, &evidence.link.state.loss_permille.0.to_le_bytes());
    }
    digest
}

pub(super) fn forward_evidence_digest(
    forward_evidence: &[crate::summary::ForwardPropagatedEvidence],
) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for evidence in forward_evidence {
        digest = mix_digest(digest, &evidence.from_neighbor.0);
        digest = mix_digest(digest, &evidence.summary.encode());
    }
    digest
}

pub(super) fn reverse_feedback_digest(
    reverse_feedback: &[crate::summary::ReverseFeedbackEvidence],
) -> u64 {
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for feedback in reverse_feedback {
        digest = mix_digest(digest, &feedback.from_neighbor.0);
        digest = mix_digest(digest, &feedback.delivery_feedback.value().to_le_bytes());
    }
    digest
}

pub(super) fn mix_digest(mut digest: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
    }
    digest
}

pub(super) fn route_health_for(
    corridor_envelope: &crate::state::CorridorBeliefEnvelope,
    now_tick: Tick,
) -> RouteHealth {
    RouteHealth {
        reachability_state: ReachabilityState::Reachable,
        stability_score: HealthScore(u32::from(corridor_envelope.delivery_support.value())),
        congestion_penalty_points: jacquard_core::PenaltyPoints(u32::from(
            corridor_envelope.congestion_penalty.value(),
        )),
        last_validated_at_tick: now_tick,
    }
}

pub(super) fn field_commitment_id_for_route(route_id: &RouteId) -> RouteCommitmentId {
    let digest = Blake3Hashing.hash_tagged(FIELD_COMMITMENT_ID_DOMAIN, &route_id.0);
    RouteCommitmentId::from(&digest)
}
