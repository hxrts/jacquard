#![allow(clippy::wildcard_imports)]

// Retained coded-diffusion role: this evidence-ingestion scaffold maps pending
// summaries and feedback into fragment custody and receiver-rank observations.

use super::*;

pub(super) fn direct_evidence_for_destination(
    topology: &Configuration,
    local_node_id: NodeId,
    destination: &DestinationId,
    now_tick: Tick,
) -> Vec<DirectEvidence> {
    let DestinationId::Node(node_id) = destination else {
        return Vec::new();
    };
    topology
        .links
        .get(&(local_node_id, *node_id))
        .cloned()
        .map(|link| {
            vec![DirectEvidence {
                neighbor_id: *node_id,
                link,
                observed_at_tick: now_tick,
            }]
        })
        .unwrap_or_default()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ForwardEvidenceInput {
    pub(super) evidence: Vec<crate::summary::ForwardPropagatedEvidence>,
    pub(super) synthesized: bool,
    pub(super) service_carry_forward: bool,
}

// long-block-exception: observer evidence synthesis keeps pending, published,
// and carry-forward evidence in one deterministic fallback order.
#[allow(dead_code)]
pub(super) fn forward_evidence_for_observer(
    destination_state: &crate::state::DestinationFieldState,
    now_tick: Tick,
) -> ForwardEvidenceInput {
    forward_evidence_for_observer_with_policy(
        destination_state,
        now_tick,
        &crate::policy::DEFAULT_FIELD_POLICY.evidence.observer,
    )
}

// long-block-exception: observer evidence synthesis keeps the bounded
// carry-forward and anti-entropy branch ladder aligned with the policy surface.
pub(super) fn forward_evidence_for_observer_with_policy(
    destination_state: &crate::state::DestinationFieldState,
    now_tick: Tick,
    policy: &crate::policy::FieldObserverEvidencePolicy,
) -> ForwardEvidenceInput {
    if !destination_state.pending_forward_evidence.is_empty() {
        return ForwardEvidenceInput {
            evidence: destination_state.pending_forward_evidence.clone(),
            synthesized: false,
            service_carry_forward: false,
        };
    }
    let Some(last_summary) = destination_state.publication.last_summary.clone() else {
        return ForwardEvidenceInput {
            evidence: Vec::new(),
            synthesized: false,
            service_carry_forward: false,
        };
    };
    let Some(last_sent_at) = destination_state.publication.last_sent_at else {
        return ForwardEvidenceInput {
            evidence: Vec::new(),
            synthesized: false,
            service_carry_forward: false,
        };
    };
    let service_bias = matches!(
        destination_state.destination,
        crate::state::DestinationKey::Service(_)
    );
    if now_tick.0.saturating_sub(last_sent_at.0)
        > if service_bias {
            policy.service_carry_forward_freshness_ticks
        } else {
            policy.node_carry_forward_freshness_ticks
        }
    {
        return ForwardEvidenceInput {
            evidence: Vec::new(),
            synthesized: false,
            service_carry_forward: false,
        };
    }
    let retention_floor = if service_bias {
        policy.service_carry_forward_retention_floor_permille
    } else {
        policy.node_carry_forward_retention_floor_permille
    };
    let support_floor = if service_bias {
        policy.service_carry_forward_support_floor_permille
    } else {
        policy.node_carry_forward_support_floor_permille
    };
    if last_summary.retention_support.value() < retention_floor
        || last_summary.delivery_support.value() < support_floor
    {
        return ForwardEvidenceInput {
            evidence: Vec::new(),
            synthesized: false,
            service_carry_forward: false,
        };
    }
    let frontier_entries = destination_state.frontier.as_slice();
    let frontier_neighbors = frontier_entries
        .iter()
        .take(if service_bias { 3 } else { 1 })
        .collect::<Vec<_>>();
    if frontier_neighbors.is_empty() {
        return ForwardEvidenceInput {
            evidence: Vec::new(),
            synthesized: false,
            service_carry_forward: false,
        };
    }
    ForwardEvidenceInput {
        evidence: frontier_neighbors
            .into_iter()
            .enumerate()
            .map(|(index, continuation)| {
                let decay_rank = u16::try_from(index).unwrap_or(u16::MAX);
                let delivery_bonus = if service_bias {
                    policy.service_delivery_bonus_permille.saturating_sub(
                        decay_rank.saturating_mul(policy.service_delivery_decay_step_permille),
                    )
                } else {
                    policy.node_delivery_bonus_permille
                };
                let retention_bonus = if service_bias {
                    policy.service_retention_bonus_permille.saturating_sub(
                        decay_rank.saturating_mul(policy.service_retention_decay_step_permille),
                    )
                } else {
                    policy.node_retention_bonus_permille
                };
                crate::summary::ForwardPropagatedEvidence {
                    from_neighbor: continuation.neighbor_id,
                    summary: FieldSummary {
                        freshness_tick: now_tick,
                        hop_band: continuation.expected_hop_band,
                        delivery_support: SupportBucket::new(
                            last_summary
                                .delivery_support
                                .value()
                                .max(continuation.downstream_support.value())
                                .saturating_add(delivery_bonus)
                                .min(1000),
                        ),
                        retention_support: SupportBucket::new(
                            last_summary
                                .retention_support
                                .value()
                                .max(continuation.net_value.value())
                                .saturating_add(retention_bonus)
                                .min(1000),
                        ),
                        ..last_summary.clone()
                    },
                    observed_at_tick: now_tick,
                }
            })
            .collect(),
        synthesized: true,
        service_carry_forward: service_bias,
    }
}

// long-block-exception: synthesized node evidence keeps degraded carry-forward
// ranking and publication replay in one audited reconstruction path.
#[allow(dead_code)]
pub(super) fn synthesized_node_forward_evidence_from_active_routes(
    destination_state: &crate::state::DestinationFieldState,
    active_routes: &[&ActiveFieldRoute],
    neighbor_endpoints: &std::collections::BTreeMap<NodeId, jacquard_core::LinkEndpoint>,
    now_tick: Tick,
    search_config: &crate::FieldSearchConfig,
) -> Vec<crate::summary::ForwardPropagatedEvidence> {
    synthesized_node_forward_evidence_from_active_routes_with_policy(
        destination_state,
        active_routes,
        neighbor_endpoints,
        now_tick,
        search_config,
        &crate::policy::DEFAULT_FIELD_POLICY.evidence.observer,
    )
}

// long-block-exception: node forward-evidence synthesis intentionally keeps
// ranking, continuation filtering, and summary shaping in one helper.
pub(super) fn synthesized_node_forward_evidence_from_active_routes_with_policy(
    destination_state: &crate::state::DestinationFieldState,
    active_routes: &[&ActiveFieldRoute],
    neighbor_endpoints: &std::collections::BTreeMap<NodeId, jacquard_core::LinkEndpoint>,
    now_tick: Tick,
    search_config: &crate::FieldSearchConfig,
    policy: &crate::policy::FieldObserverEvidencePolicy,
) -> Vec<crate::summary::ForwardPropagatedEvidence> {
    if !search_config.node_discovery_enabled() {
        return Vec::new();
    }
    let crate::state::DestinationKey::Node(_) = destination_state.destination else {
        return Vec::new();
    };
    let Some(last_summary) = destination_state.publication.last_summary.as_ref() else {
        return Vec::new();
    };
    let Some(last_sent_at) = destination_state.publication.last_sent_at else {
        return Vec::new();
    };
    if now_tick.0.saturating_sub(last_sent_at.0)
        > FIELD_DEGRADED_STEADY_STALE_TICKS_MAX
            .saturating_add(policy.synthesized_node_publication_staleness_slack_ticks)
    {
        return Vec::new();
    }
    if last_summary.delivery_support.value()
        < search_config
            .node_bootstrap_support_floor()
            .saturating_sub(policy.synthesized_node_support_relief_permille)
            .max(policy.synthesized_node_support_floor_min_permille)
        || last_summary.retention_support.value() < policy.synthesized_node_retention_floor_permille
    {
        return Vec::new();
    }
    let mut synthesized =
        active_routes
            .iter()
            .flat_map(|active| {
                active.continuation_neighbors.iter().enumerate().filter_map(
                    |(index, neighbor_id)| {
                        let reachable = neighbor_endpoints.contains_key(neighbor_id);
                        if !reachable && *neighbor_id != active.selected_neighbor {
                            return None;
                        }
                        let rank_penalty = u16::try_from(index)
                            .unwrap_or(u16::MAX)
                            .saturating_mul(policy.synthesized_node_rank_penalty_permille);
                        let selection_bonus = if *neighbor_id == active.selected_neighbor {
                            policy.synthesized_node_selected_neighbor_bonus_permille
                        } else {
                            0
                        };
                        let reachability_bonus = if reachable {
                            policy.synthesized_node_reachability_bonus_permille
                        } else {
                            0
                        };
                        Some(crate::summary::ForwardPropagatedEvidence {
                            from_neighbor: *neighbor_id,
                            summary: FieldSummary {
                                freshness_tick: now_tick,
                                hop_band: active.corridor_envelope.expected_hop_band,
                                delivery_support: SupportBucket::new(
                                    last_summary
                                        .delivery_support
                                        .value()
                                        .max(active.corridor_envelope.delivery_support.value())
                                        .saturating_add(reachability_bonus)
                                        .saturating_add(selection_bonus)
                                        .saturating_sub(rank_penalty)
                                        .min(1000),
                                ),
                                retention_support: SupportBucket::new(
                                    last_summary
                                        .retention_support
                                        .value()
                                        .max(active.corridor_envelope.retention_affinity.value())
                                        .saturating_add(
                                            policy.synthesized_node_retention_bonus_permille,
                                        )
                                        .saturating_sub(rank_penalty / 2)
                                        .min(1000),
                                ),
                                ..last_summary.clone()
                            },
                            observed_at_tick: now_tick,
                        })
                    },
                )
            })
            .collect::<Vec<_>>();
    synthesized.sort_by(|left, right| {
        right
            .summary
            .delivery_support
            .value()
            .cmp(&left.summary.delivery_support.value())
            .then_with(|| {
                right
                    .summary
                    .retention_support
                    .value()
                    .cmp(&left.summary.retention_support.value())
            })
            .then_with(|| left.from_neighbor.cmp(&right.from_neighbor))
    });
    synthesized.dedup_by(|left, right| left.from_neighbor == right.from_neighbor);
    synthesized.truncate(2);
    synthesized
}

pub(super) fn summary_for_destination(
    destination_state: &crate::state::DestinationFieldState,
    topology_epoch: jacquard_core::RouteEpoch,
    now_tick: Tick,
    destination: &DestinationId,
) -> FieldSummary {
    FieldSummary {
        destination: SummaryDestinationKey::from(destination),
        topology_epoch,
        freshness_tick: now_tick,
        hop_band: destination_state.corridor_belief.expected_hop_band,
        delivery_support: destination_state.corridor_belief.delivery_support,
        congestion_penalty: destination_state.corridor_belief.congestion_penalty,
        retention_support: destination_state.corridor_belief.retention_affinity,
        uncertainty_penalty: destination_state.posterior.usability_entropy,
        evidence_class: match destination_state.posterior.predicted_observation_class {
            crate::state::ObservationClass::DirectOnly => EvidenceContributionClass::Direct,
            crate::state::ObservationClass::ForwardPropagated
            | crate::state::ObservationClass::Mixed => EvidenceContributionClass::ForwardPropagated,
            crate::state::ObservationClass::ReverseValidated => {
                EvidenceContributionClass::ReverseFeedback
            }
        },
        uncertainty_class: match destination_state.posterior.usability_entropy.value() {
            0..=249 => SummaryUncertaintyClass::Low,
            250..=599 => SummaryUncertaintyClass::Medium,
            _ => SummaryUncertaintyClass::High,
        },
    }
}

// long-block-exception: anti-entropy replay keeps the publication, posterior,
// and bridge-retention floors in one auditable summary synthesis path.
#[allow(dead_code)]
pub(super) fn anti_entropy_summary_for_destination(
    destination_state: &crate::state::DestinationFieldState,
    summary: &FieldSummary,
    now_tick: Tick,
) -> FieldSummary {
    anti_entropy_summary_for_destination_with_policy(
        destination_state,
        summary,
        now_tick,
        &crate::policy::DEFAULT_FIELD_POLICY.evidence.observer,
    )
}

// long-block-exception: anti-entropy summary shaping keeps the replay/evidence
// heuristics in one deterministic transformation.
pub(super) fn anti_entropy_summary_for_destination_with_policy(
    destination_state: &crate::state::DestinationFieldState,
    summary: &FieldSummary,
    now_tick: Tick,
    policy: &crate::policy::FieldObserverEvidencePolicy,
) -> FieldSummary {
    let publication_support = destination_state
        .publication
        .last_summary
        .as_ref()
        .map(|published| published.delivery_support.value())
        .unwrap_or(0);
    let publication_retention = destination_state
        .publication
        .last_summary
        .as_ref()
        .map(|published| published.retention_support.value())
        .unwrap_or(0);
    let bootstrap_delivery_floor = destination_state
        .progress_belief
        .posterior_support
        .value()
        .min(destination_state.posterior.top_corridor_mass.value())
        .max(
            destination_state
                .progress_belief
                .posterior_support
                .value()
                .saturating_add(destination_state.posterior.top_corridor_mass.value())
                / 2,
        );
    let bridge_bias = summary.hop_band.max_hops >= 2
        && summary.evidence_class != EvidenceContributionClass::Direct
        && destination_state.posterior.top_corridor_mass.value() >= 240
        && destination_state.corridor_belief.retention_affinity.value() >= 280;
    let replay_support = summary
        .delivery_support
        .value()
        .max(bootstrap_delivery_floor)
        .max(publication_support.saturating_sub(20))
        .max(
            publication_support
                .saturating_add(publication_retention / 6)
                .min(1000),
        )
        .saturating_add(if bridge_bias {
            policy.replay_bridge_support_bonus_permille
        } else {
            0
        })
        .min(1000);
    let replay_retention = summary
        .retention_support
        .value()
        .max(publication_retention)
        .max((replay_support.saturating_mul(4)) / 5)
        .max(
            destination_state
                .posterior
                .top_corridor_mass
                .value()
                .saturating_add(replay_support)
                / 2,
        )
        .saturating_add(if bridge_bias {
            policy.replay_bridge_retention_bonus_permille
        } else {
            0
        })
        .min(1000);
    let replay_uncertainty = summary
        .uncertainty_penalty
        .value()
        .saturating_sub(
            if replay_retention >= policy.replay_high_retention_floor_permille {
                policy.replay_high_uncertainty_relief_permille
            } else if replay_retention >= policy.replay_medium_retention_floor_permille {
                policy.replay_medium_uncertainty_relief_permille
            } else {
                policy.replay_low_uncertainty_relief_permille
            },
        )
        .saturating_sub(if publication_retention >= 320 {
            policy.replay_publication_retention_relief_permille
        } else {
            0
        });
    FieldSummary {
        freshness_tick: now_tick,
        delivery_support: SupportBucket::new(replay_support),
        retention_support: SupportBucket::new(replay_retention),
        uncertainty_penalty: crate::state::EntropyBucket::new(replay_uncertainty),
        uncertainty_class: match replay_uncertainty {
            0..=249 => SummaryUncertaintyClass::Low,
            250..=599 => SummaryUncertaintyClass::Medium,
            _ => SummaryUncertaintyClass::High,
        },
        ..summary.clone()
    }
}

pub(super) fn updated_promotion_window_score(
    current_score: u8,
    assessment: &crate::planner::promotion::FieldPromotionAssessment,
    destination_state: &crate::state::DestinationFieldState,
    destination_context: &crate::operational::FieldDestinationDecisionContext,
) -> u8 {
    let mut next_score = current_score.saturating_sub(if assessment.continuation_coherent {
        0
    } else {
        2
    });
    if assessment.anti_entropy_confirmed {
        next_score = next_score.saturating_add(2);
    }
    if assessment.continuation_coherent && assessment.fresh_enough {
        next_score = next_score.saturating_add(1);
    }
    if assessment.support_growth || assessment.uncertainty_reduced {
        next_score = next_score.saturating_add(1);
    }
    if assessment.degraded_but_coherent(destination_state) {
        next_score = next_score.saturating_add(if destination_context.service_bias() {
            2
        } else {
            1
        });
    }
    next_score.min(6)
}

pub(super) fn refresh_frontier_from_evidence(
    mut frontier: crate::state::ContinuationFrontier,
    corridor_hops: HopBand,
    corridor_support: SupportBucket,
    corridor_retention: SupportBucket,
    direct_evidence: &[DirectEvidence],
    forward_evidence: &[crate::summary::ForwardPropagatedEvidence],
    now_tick: Tick,
) -> crate::state::ContinuationFrontier {
    let policy = &crate::policy::DEFAULT_FIELD_POLICY.evidence.observer;
    let prune_horizon = if corridor_retention.value()
        >= policy.prune_extended_retention_floor_permille
        && corridor_support.value() >= policy.prune_extended_support_floor_permille
    {
        8
    } else if corridor_retention.value() >= policy.prune_moderate_retention_floor_permille {
        6
    } else {
        4
    };
    frontier = frontier.prune_stale(now_tick, prune_horizon);
    for evidence in direct_evidence {
        frontier = frontier.insert(NeighborContinuation {
            neighbor_id: evidence.neighbor_id,
            net_value: corridor_support,
            downstream_support: corridor_support,
            expected_hop_band: HopBand::new(1, corridor_hops.max_hops.max(1)),
            freshness: now_tick,
        });
    }
    for evidence in forward_evidence {
        frontier = frontier.insert(NeighborContinuation {
            neighbor_id: evidence.from_neighbor,
            net_value: SupportBucket::new(
                corridor_support
                    .value()
                    .max(evidence.summary.delivery_support.value()),
            ),
            downstream_support: evidence.summary.delivery_support,
            expected_hop_band: HopBand::new(
                evidence.summary.hop_band.min_hops.saturating_add(1),
                evidence.summary.hop_band.max_hops.saturating_add(1),
            ),
            freshness: evidence.observed_at_tick,
        });
    }
    frontier
}

pub(super) fn merge_pending_forward_continuations(
    ranked: &mut Vec<(NeighborContinuation, SupportBucket)>,
    destination_state: &crate::state::DestinationFieldState,
) {
    for continuation in pending_forward_continuations_for_maintenance(destination_state) {
        if ranked
            .iter()
            .any(|(entry, _)| entry.neighbor_id == continuation.neighbor_id)
        {
            continue;
        }
        let score = continuation.net_value;
        ranked.push((continuation, score));
    }
    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.neighbor_id.cmp(&right.0.neighbor_id))
    });
}
