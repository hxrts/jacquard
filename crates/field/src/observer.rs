//! Bayesian belief updates for destination reachability and delivery support.
//!
//! For each destination, runs a predict-fuse-correct cycle. `predict_summary`
//! extrapolates an expected summary from the prior corridor belief.
//! `fuse_evidence` combines three evidence classes: direct link observations,
//! forward-propagated neighbor summaries, and reverse delivery feedback,
//! applying age decay and reflection discounting to each source.
//! `correct_posterior` updates the entropy and observation class of the fused
//! result.
//!
//! `project_posterior_to_claim` converts the corrected posterior into a
//! corridor envelope claim bounded by the posterior delivery support.
//! `progress_belief_from_envelope` derives the progress contract used in route
//! admission. Routes with entropy above 850 permille or delivery support below
//! 300 permille fail admission.
//!
//! `update_destination_observer` is the entry point called from the runtime's
//! `refresh_destination_observers` phase on every `engine_tick`.
//!
//! Retained coded-diffusion role: the predict-fuse-correct scaffold maps to
//! `FragmentSpreadBelief` and `ReconstructionQuorum` once evidence is expressed
//! as fragment custody, innovative arrivals, duplicate arrivals, and receiver
//! rank instead of baseline corridor claims.

use jacquard_core::{DestinationId, RouteEpoch, Tick};

use crate::{
    state::{
        ControlState, CorridorBeliefEnvelope, DestinationFieldState, DestinationPosterior,
        DivergenceBucket, EntropyBucket, ObservationClass, OperatingRegime, ProgressBelief,
        SupportBucket,
    },
    summary::{
        clamp_corridor_envelope, compose_summary_with_link, decay_summary,
        discount_reflected_evidence, evidence_classification, merge_neighbor_summaries,
        project_posterior_to_claim, summary_divergence, DirectEvidence, FieldEvidence,
        FieldSummary, ForwardPropagatedEvidence, LocalOriginTrace, ReverseFeedbackEvidence,
        SummaryDestinationKey, SummaryUncertaintyClass,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct ObserverInputs {
    pub(crate) destination: DestinationId,
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) now_tick: Tick,
    pub(crate) direct_evidence: Vec<DirectEvidence>,
    pub(crate) forward_evidence: Vec<ForwardPropagatedEvidence>,
    pub(crate) reverse_feedback: Vec<ReverseFeedbackEvidence>,
    pub(crate) local_origin_trace: LocalOriginTrace,
    pub(crate) regime: OperatingRegime,
    pub(crate) control_state: ControlState,
    pub(crate) service_freshness_weight: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObserverOutcome {
    pub(crate) predicted_summary: FieldSummary,
    pub(crate) fused_summary: FieldSummary,
    pub(crate) divergence: DivergenceBucket,
    pub(crate) corridor_envelope: CorridorBeliefEnvelope,
    pub(crate) posterior: DestinationPosterior,
    pub(crate) progress_belief: ProgressBelief,
}

pub(crate) fn update_destination_observer(
    state: &mut DestinationFieldState,
    inputs: &ObserverInputs,
) -> ObserverOutcome {
    let predicted_summary = predict_summary(
        &state.posterior,
        &state.corridor_belief,
        &inputs.destination,
        inputs.topology_epoch,
        inputs.now_tick,
    );
    let fused_summary = fuse_evidence(&predicted_summary, inputs);
    let divergence = summary_divergence(&predicted_summary, &fused_summary);
    let posterior = correct_posterior(
        &state.posterior,
        &fused_summary,
        divergence,
        has_evidence(inputs),
        !inputs.reverse_feedback.is_empty(),
        coherent_service_branch_metrics(inputs),
    );
    let clamped = clamp_corridor_envelope(&fused_summary, inputs.regime, &inputs.control_state);
    let corridor_envelope = project_posterior_to_claim(&posterior, &clamped);
    let progress_belief = progress_belief_from_envelope(&corridor_envelope, &posterior);

    state.posterior = posterior.clone();
    state.progress_belief = progress_belief.clone();
    state.corridor_belief = corridor_envelope.clone();

    ObserverOutcome {
        predicted_summary,
        fused_summary,
        divergence,
        corridor_envelope,
        posterior,
        progress_belief,
    }
}

fn predict_summary(
    posterior: &DestinationPosterior,
    corridor_belief: &CorridorBeliefEnvelope,
    destination: &DestinationId,
    topology_epoch: RouteEpoch,
    now_tick: Tick,
) -> FieldSummary {
    FieldSummary {
        destination: SummaryDestinationKey::from(destination),
        topology_epoch,
        freshness_tick: now_tick,
        hop_band: corridor_belief.expected_hop_band,
        delivery_support: SupportBucket::new(
            corridor_belief
                .delivery_support
                .value()
                .min(posterior.top_corridor_mass.value()),
        ),
        congestion_penalty: corridor_belief.congestion_penalty,
        retention_support: corridor_belief.retention_affinity,
        uncertainty_penalty: posterior.usability_entropy,
        evidence_class: evidence_classification(&FieldEvidence::Forward(
            ForwardPropagatedEvidence {
                from_neighbor: inputs_local_node(destination),
                summary: FieldSummary {
                    destination: SummaryDestinationKey::from(destination),
                    topology_epoch,
                    freshness_tick: now_tick,
                    hop_band: corridor_belief.expected_hop_band,
                    delivery_support: corridor_belief.delivery_support,
                    congestion_penalty: corridor_belief.congestion_penalty,
                    retention_support: corridor_belief.retention_affinity,
                    uncertainty_penalty: posterior.usability_entropy,
                    evidence_class: crate::summary::EvidenceContributionClass::ForwardPropagated,
                    uncertainty_class: uncertainty_class_for(posterior.usability_entropy.value()),
                },
                observed_at_tick: now_tick,
            },
        )),
        uncertainty_class: uncertainty_class_for(posterior.usability_entropy.value()),
    }
}

fn fuse_evidence(predicted_summary: &FieldSummary, inputs: &ObserverInputs) -> FieldSummary {
    let mut fused: Option<FieldSummary> = None;

    for evidence in &inputs.forward_evidence {
        let decayed = decay_summary(&evidence.summary, inputs.now_tick);
        let discounted = discount_reflected_evidence(&decayed, inputs.local_origin_trace);
        fused = Some(match fused {
            Some(current) => merge_neighbor_summaries(&current, &discounted),
            None => discounted,
        });
    }

    for evidence in &inputs.direct_evidence {
        let direct = compose_summary_with_link(predicted_summary, &evidence.link);
        fused = Some(match fused {
            Some(current) => merge_neighbor_summaries(&direct, &current),
            None => direct,
        });
    }

    let mut fused = fused.unwrap_or_else(|| decay_summary(predicted_summary, inputs.now_tick));

    if let Some(best_reverse) = inputs
        .reverse_feedback
        .iter()
        .max_by_key(|feedback| feedback.delivery_feedback.value())
    {
        fused.delivery_support = SupportBucket::new(
            fused
                .delivery_support
                .value()
                .max(best_reverse.delivery_feedback.value()),
        );
        fused.uncertainty_penalty =
            EntropyBucket::new(fused.uncertainty_penalty.value().saturating_sub(100));
        fused.uncertainty_class = SummaryUncertaintyClass::Low;
    } else {
        fused.uncertainty_penalty =
            EntropyBucket::new(fused.uncertainty_penalty.value().saturating_add(50));
        fused.uncertainty_class = uncertainty_class_for(fused.uncertainty_penalty.value());
    }

    fused
}

// long-block-exception: posterior correction keeps the fused summary,
// divergence, and service-branch adjustments in one Bayesian-style update.
fn correct_posterior(
    previous: &DestinationPosterior,
    fused_summary: &FieldSummary,
    divergence: DivergenceBucket,
    has_any_evidence: bool,
    has_reverse_feedback: bool,
    service_branch_metrics: Option<ServiceBranchMetrics>,
) -> DestinationPosterior {
    let mut entropy = previous
        .usability_entropy
        .value()
        .saturating_add(divergence.value() / 2);
    if has_any_evidence {
        entropy = entropy.saturating_sub(75);
    } else {
        entropy = entropy.saturating_add(75);
    }
    if has_reverse_feedback {
        entropy = entropy.saturating_sub(125);
    }
    if let Some(metrics) = service_branch_metrics {
        let corroboration_discount = u16::try_from(
            metrics
                .branch_count
                .saturating_sub(1)
                .saturating_mul(40)
                .min(120),
        )
        .expect("bounded service corroboration discount fits u16");
        let freshness_bonus = metrics.freshness_bonus.min(60);
        entropy = entropy
            .saturating_sub(corroboration_discount)
            .saturating_sub(freshness_bonus);
    }
    let observation_class = if has_reverse_feedback {
        ObservationClass::ReverseValidated
    } else if !has_any_evidence {
        ObservationClass::DirectOnly
    } else {
        match fused_summary.evidence_class {
            crate::summary::EvidenceContributionClass::Direct => ObservationClass::DirectOnly,
            crate::summary::EvidenceContributionClass::ForwardPropagated => {
                ObservationClass::ForwardPropagated
            }
            crate::summary::EvidenceContributionClass::ReverseFeedback => {
                ObservationClass::ReverseValidated
            }
        }
    };
    let mut top_corridor_mass = fused_summary
        .delivery_support
        .value()
        .saturating_sub(EntropyBucket::new(entropy).value() / 2);
    if let Some(metrics) = service_branch_metrics {
        let corroboration_bonus = u16::try_from(
            metrics
                .branch_count
                .saturating_sub(1)
                .saturating_mul(70)
                .min(220),
        )
        .expect("bounded service corroboration bonus fits u16");
        let diversity_bonus = u16::try_from((metrics.diversity_score / 4).min(120))
            .expect("bounded diversity bonus fits u16");
        top_corridor_mass = top_corridor_mass
            .saturating_add(corroboration_bonus)
            .saturating_add(diversity_bonus)
            .saturating_sub(metrics.freshness_penalty.min(80))
            .min(1000);
    }
    DestinationPosterior {
        usability_entropy: EntropyBucket::new(entropy),
        top_corridor_mass: SupportBucket::new(top_corridor_mass),
        regime_belief: previous.regime_belief.clone(),
        predicted_observation_class: observation_class,
    }
}

fn progress_belief_from_envelope(
    corridor_envelope: &CorridorBeliefEnvelope,
    posterior: &DestinationPosterior,
) -> ProgressBelief {
    ProgressBelief {
        progress_score: jacquard_core::Belief::certain(
            jacquard_core::HealthScore(u32::from(corridor_envelope.delivery_support.value())),
            corridor_envelope.validity_window.start_tick(),
        ),
        uncertainty_penalty: jacquard_core::Belief::certain(
            jacquard_core::PenaltyPoints(u32::from(posterior.usability_entropy.value())),
            corridor_envelope.validity_window.start_tick(),
        ),
        posterior_support: SupportBucket::new(
            corridor_envelope
                .delivery_support
                .value()
                .min(posterior.top_corridor_mass.value()),
        ),
    }
}

fn has_evidence(inputs: &ObserverInputs) -> bool {
    !(inputs.direct_evidence.is_empty() && inputs.forward_evidence.is_empty())
}

#[derive(Clone, Copy, Debug)]
struct ServiceBranchMetrics {
    branch_count: usize,
    freshness_bonus: u16,
    freshness_penalty: u16,
    diversity_score: u32,
}

fn coherent_service_branch_metrics(inputs: &ObserverInputs) -> Option<ServiceBranchMetrics> {
    if !matches!(inputs.destination, DestinationId::Service(_)) {
        return None;
    }
    let mut per_neighbor = std::collections::BTreeMap::new();
    let mut freshness_gaps = Vec::new();
    let freshness_weight = u32::from(inputs.service_freshness_weight.clamp(25, 200));
    for evidence in &inputs.forward_evidence {
        if evidence.summary.retention_support.value() >= 140
            && evidence.summary.delivery_support.value() >= 120
            && evidence.summary.uncertainty_penalty.value() <= 900
        {
            let freshness_gap = inputs
                .now_tick
                .0
                .saturating_sub(evidence.observed_at_tick.0);
            freshness_gaps.push(freshness_gap);
            let freshness_penalty = u32::try_from(freshness_gap.min(5))
                .expect("bounded freshness gap fits")
                * (freshness_weight / 4).max(1);
            let score = u32::from(evidence.summary.delivery_support.value())
                .saturating_add(u32::from(evidence.summary.retention_support.value()))
                .saturating_sub(u32::from(evidence.summary.uncertainty_penalty.value()) / 3)
                .saturating_sub(freshness_penalty);
            per_neighbor
                .entry(evidence.from_neighbor)
                .and_modify(|current: &mut u32| *current = (*current).max(score))
                .or_insert(score);
        }
    }
    if per_neighbor.len() < 2 {
        return None;
    }
    let mut scores: Vec<u32> = per_neighbor.into_values().collect();
    scores.sort_unstable_by(|left: &u32, right: &u32| right.cmp(left));
    let diversity_score = scores.get(1).copied().unwrap_or(0);
    let average_gap = freshness_gaps.iter().copied().sum::<u64>()
        / u64::try_from(freshness_gaps.len()).expect("freshness gap length fits");
    let freshness_bonus = u16::try_from(
        80_u64
            .saturating_sub(average_gap.saturating_mul(u64::from((freshness_weight / 10).max(6))))
            .min(80),
    )
    .expect("bounded freshness bonus fits u16");
    let freshness_penalty = u16::try_from(
        (average_gap.saturating_mul(u64::from((freshness_weight / 8).max(8)))).min(80),
    )
    .expect("bounded penalty fits");
    Some(ServiceBranchMetrics {
        branch_count: scores.len(),
        freshness_bonus,
        freshness_penalty,
        diversity_score,
    })
}

fn uncertainty_class_for(value: u16) -> SummaryUncertaintyClass {
    match value {
        0..=249 => SummaryUncertaintyClass::Low,
        250..=599 => SummaryUncertaintyClass::Medium,
        _ => SummaryUncertaintyClass::High,
    }
}

fn inputs_local_node(destination: &DestinationId) -> jacquard_core::NodeId {
    match destination {
        DestinationId::Node(node_id) => *node_id,
        _ => jacquard_core::NodeId([0; 32]),
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        Belief, ByteCount, DurationMs, EndpointLocator, Link, LinkEndpoint, LinkProfile,
        LinkRuntimeState, LinkState, PartitionRecoveryClass, RatioPermille, RepairCapability,
    };

    use super::*;
    use crate::{
        state::{DestinationFieldState, DestinationKey, HopBand},
        summary::{
            DirectEvidence, EvidenceContributionClass, FieldSummary, ForwardPropagatedEvidence,
            ReverseFeedbackEvidence, SummaryDestinationKey,
        },
    };

    fn node(byte: u8) -> jacquard_core::NodeId {
        jacquard_core::NodeId([byte; 32])
    }

    fn link(confidence: u16) -> Link {
        Link {
            endpoint: LinkEndpoint {
                transport_kind: jacquard_core::TransportKind::WifiAware,
                locator: EndpointLocator::Opaque(vec![1]),
                mtu_bytes: ByteCount(128),
            },
            profile: LinkProfile {
                latency_floor_ms: DurationMs(5),
                repair_capability: RepairCapability::TransportRetransmit,
                partition_recovery: PartitionRecoveryClass::LocalReconnect,
            },
            state: LinkState {
                state: LinkRuntimeState::Active,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(10),
                delivery_confidence_permille: Belief::certain(RatioPermille(confidence), Tick(4)),
                symmetry_permille: Belief::Absent,
            },
        }
    }

    fn state(now: Tick) -> DestinationFieldState {
        DestinationFieldState::new(DestinationKey::Node(node(9)), now)
    }

    fn forward_summary(support: u16, now: Tick) -> FieldSummary {
        FieldSummary {
            destination: SummaryDestinationKey::Node(node(9)),
            topology_epoch: RouteEpoch(1),
            freshness_tick: now,
            hop_band: HopBand::new(1, 3),
            delivery_support: SupportBucket::new(support),
            congestion_penalty: EntropyBucket::new(100),
            retention_support: SupportBucket::new(100),
            uncertainty_penalty: EntropyBucket::new(300),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Medium,
        }
    }

    fn base_inputs(now: Tick) -> ObserverInputs {
        ObserverInputs {
            destination: DestinationId::Node(node(9)),
            topology_epoch: RouteEpoch(1),
            now_tick: now,
            direct_evidence: Vec::new(),
            forward_evidence: Vec::new(),
            reverse_feedback: Vec::new(),
            local_origin_trace: LocalOriginTrace {
                local_node_id: node(1),
                topology_epoch: RouteEpoch(1),
            },
            regime: OperatingRegime::Sparse,
            control_state: ControlState::default(),
            service_freshness_weight: 100,
        }
    }

    fn service_state(now: Tick) -> DestinationFieldState {
        DestinationFieldState::new(DestinationKey::Service(vec![7; 16]), now)
    }

    fn service_forward_summary(support: u16, retention: u16, now: Tick) -> FieldSummary {
        let destination = DestinationId::Service(jacquard_core::ServiceId(vec![7; 16]));
        FieldSummary {
            destination: SummaryDestinationKey::from(&destination),
            topology_epoch: RouteEpoch(1),
            freshness_tick: now,
            hop_band: HopBand::new(1, 2),
            delivery_support: SupportBucket::new(support),
            congestion_penalty: EntropyBucket::new(80),
            retention_support: SupportBucket::new(retention),
            uncertainty_penalty: EntropyBucket::new(240),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Low,
        }
    }

    fn service_inputs(now: Tick) -> ObserverInputs {
        ObserverInputs {
            destination: DestinationId::Service(jacquard_core::ServiceId(vec![7; 16])),
            topology_epoch: RouteEpoch(1),
            now_tick: now,
            direct_evidence: Vec::new(),
            forward_evidence: Vec::new(),
            reverse_feedback: Vec::new(),
            local_origin_trace: LocalOriginTrace {
                local_node_id: node(1),
                topology_epoch: RouteEpoch(1),
            },
            regime: OperatingRegime::Sparse,
            control_state: ControlState::default(),
            service_freshness_weight: 100,
        }
    }

    #[test]
    fn low_information_operation_degrades_but_remains_conservative() {
        let mut destination_state = state(Tick(4));
        let outcome = update_destination_observer(&mut destination_state, &base_inputs(Tick(5)));
        assert!(
            outcome.corridor_envelope.delivery_support.value()
                <= outcome.posterior.top_corridor_mass.value()
        );
        assert!(outcome.posterior.usability_entropy.value() >= 50);
    }

    #[test]
    fn absent_reverse_feedback_widens_uncertainty() {
        let mut destination_state = state(Tick(4));
        let mut inputs = base_inputs(Tick(5));
        inputs.forward_evidence.push(ForwardPropagatedEvidence {
            from_neighbor: node(2),
            summary: forward_summary(700, Tick(4)),
            observed_at_tick: Tick(5),
        });
        let outcome = update_destination_observer(&mut destination_state, &inputs);
        assert!(outcome.fused_summary.uncertainty_penalty.value() >= 300);
    }

    #[test]
    fn direct_evidence_tightens_support_without_changing_corridor_contract() {
        let mut destination_state = state(Tick(4));
        let mut sparse_inputs = base_inputs(Tick(5));
        sparse_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: forward_summary(500, Tick(4)),
                observed_at_tick: Tick(5),
            });
        let sparse = update_destination_observer(&mut destination_state, &sparse_inputs);

        let mut richer_state = state(Tick(4));
        let mut richer_inputs = base_inputs(Tick(5));
        richer_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: forward_summary(500, Tick(4)),
                observed_at_tick: Tick(5),
            });
        richer_inputs.direct_evidence.push(DirectEvidence {
            neighbor_id: node(2),
            link: link(900),
            observed_at_tick: Tick(5),
        });
        richer_inputs
            .reverse_feedback
            .push(ReverseFeedbackEvidence {
                from_neighbor: node(2),
                delivery_feedback: SupportBucket::new(850),
                observed_at_tick: Tick(5),
            });
        let richer = update_destination_observer(&mut richer_state, &richer_inputs);

        assert!(
            richer.corridor_envelope.delivery_support.value()
                >= sparse.corridor_envelope.delivery_support.value()
        );
        assert!(
            richer.posterior.usability_entropy.value()
                <= sparse.posterior.usability_entropy.value()
        );
        assert!(
            richer.corridor_envelope.delivery_support.value()
                <= richer.posterior.top_corridor_mass.value()
        );
    }

    #[test]
    fn observer_consumes_cooperative_observations_explicitly() {
        let mut destination_state = state(Tick(4));
        let mut inputs = base_inputs(Tick(5));
        inputs.forward_evidence.push(ForwardPropagatedEvidence {
            from_neighbor: node(4),
            summary: forward_summary(650, Tick(4)),
            observed_at_tick: Tick(5),
        });
        let outcome = update_destination_observer(&mut destination_state, &inputs);
        assert_eq!(
            outcome.posterior.predicted_observation_class,
            ObservationClass::ForwardPropagated
        );
    }

    #[test]
    fn service_fanout_corroboration_boosts_posterior_mass() {
        let mut destination_state = service_state(Tick(4));
        let mut inputs = service_inputs(Tick(5));
        inputs.forward_evidence.push(ForwardPropagatedEvidence {
            from_neighbor: node(2),
            summary: service_forward_summary(640, 360, Tick(4)),
            observed_at_tick: Tick(5),
        });
        inputs.forward_evidence.push(ForwardPropagatedEvidence {
            from_neighbor: node(3),
            summary: service_forward_summary(610, 340, Tick(4)),
            observed_at_tick: Tick(5),
        });
        let corroborated = update_destination_observer(&mut destination_state, &inputs);

        let mut single_state = service_state(Tick(4));
        let mut single_inputs = service_inputs(Tick(5));
        single_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: service_forward_summary(640, 360, Tick(4)),
                observed_at_tick: Tick(5),
            });
        let single = update_destination_observer(&mut single_state, &single_inputs);

        assert!(
            corroborated.posterior.top_corridor_mass.value()
                > single.posterior.top_corridor_mass.value()
        );
        assert!(
            corroborated.posterior.usability_entropy.value()
                <= single.posterior.usability_entropy.value()
        );
    }

    #[test]
    fn fresh_service_fanout_beats_stale_service_fanout() {
        let mut fresh_state = service_state(Tick(8));
        let mut fresh_inputs = service_inputs(Tick(8));
        fresh_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: service_forward_summary(620, 360, Tick(8)),
                observed_at_tick: Tick(8),
            });
        fresh_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(3),
                summary: service_forward_summary(600, 350, Tick(8)),
                observed_at_tick: Tick(8),
            });
        let fresh = update_destination_observer(&mut fresh_state, &fresh_inputs);

        let mut stale_state = service_state(Tick(8));
        let mut stale_inputs = service_inputs(Tick(8));
        stale_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: service_forward_summary(620, 360, Tick(2)),
                observed_at_tick: Tick(2),
            });
        stale_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(3),
                summary: service_forward_summary(600, 350, Tick(2)),
                observed_at_tick: Tick(2),
            });
        let stale = update_destination_observer(&mut stale_state, &stale_inputs);

        assert!(
            fresh.posterior.top_corridor_mass.value() > stale.posterior.top_corridor_mass.value()
        );
        assert!(
            fresh.posterior.usability_entropy.value() <= stale.posterior.usability_entropy.value()
        );
    }

    #[test]
    fn higher_service_freshness_weight_penalizes_stale_corroboration_more() {
        let mut strict_state = service_state(Tick(8));
        let mut strict_inputs = service_inputs(Tick(8));
        strict_inputs.service_freshness_weight = 180;
        strict_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: service_forward_summary(620, 360, Tick(3)),
                observed_at_tick: Tick(3),
            });
        strict_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(3),
                summary: service_forward_summary(600, 350, Tick(3)),
                observed_at_tick: Tick(3),
            });
        let strict = update_destination_observer(&mut strict_state, &strict_inputs);

        let mut relaxed_state = service_state(Tick(8));
        let mut relaxed_inputs = service_inputs(Tick(8));
        relaxed_inputs.service_freshness_weight = 60;
        relaxed_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(2),
                summary: service_forward_summary(620, 360, Tick(3)),
                observed_at_tick: Tick(3),
            });
        relaxed_inputs
            .forward_evidence
            .push(ForwardPropagatedEvidence {
                from_neighbor: node(3),
                summary: service_forward_summary(600, 350, Tick(3)),
                observed_at_tick: Tick(3),
            });
        let relaxed = update_destination_observer(&mut relaxed_state, &relaxed_inputs);

        assert!(
            strict.posterior.top_corridor_mass.value()
                < relaxed.posterior.top_corridor_mass.value()
        );
        assert!(
            strict.posterior.usability_entropy.value()
                >= relaxed.posterior.usability_entropy.value()
        );
    }
}
