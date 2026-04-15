//! Promotion assessment for active field routes.

use jacquard_core::{DestinationId, Tick};

use crate::{
    policy::{FieldPromotionPolicy, DEFAULT_FIELD_POLICY},
    recovery::FieldPromotionBlocker,
    route::ActiveFieldRoute,
    runtime::FIELD_ROUTE_WEAK_SUPPORT_FLOOR,
    state::DestinationFieldState,
    summary::{summary_divergence, EvidenceContributionClass, FieldSummary, SummaryDestinationKey},
};

use super::admission::{
    evidence_class_from_state, promoted_corridor_admissible_with_config, uncertainty_class_for,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FieldBootstrapDecision {
    Hold,
    Narrow,
    Promote,
    Withdraw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldPromotionAssessment {
    pub(crate) support_growth: bool,
    pub(crate) uncertainty_reduced: bool,
    pub(crate) anti_entropy_confirmed: bool,
    pub(crate) continuation_coherent: bool,
    pub(crate) fresh_enough: bool,
}

impl FieldPromotionAssessment {
    #[must_use]
    fn confirmed_stability(
        self,
        destination_state: &DestinationFieldState,
        confirmation_streak: u8,
        promotion_window_score: u8,
    ) -> bool {
        (confirmation_streak >= 1 || promotion_window_score >= 3)
            && self.anti_entropy_confirmed
            && self.continuation_coherent
            && self.fresh_enough
            && destination_state.corridor_belief.delivery_support.value() >= 180
            && destination_state.corridor_belief.retention_affinity.value() >= 240
            && destination_state.posterior.top_corridor_mass.value() >= 220
            && destination_state.posterior.usability_entropy.value() <= 925
    }

    #[must_use]
    pub(crate) fn can_promote(self, promotion_window_score: u8) -> bool {
        self.anti_entropy_confirmed
            && self.continuation_coherent
            && self.fresh_enough
            && ((self.support_growth && self.uncertainty_reduced) || promotion_window_score >= 4)
    }

    #[must_use]
    pub(crate) fn degraded_but_coherent(self, destination_state: &DestinationFieldState) -> bool {
        self.continuation_coherent
            && (self.fresh_enough || self.anti_entropy_confirmed)
            && destination_state.corridor_belief.retention_affinity.value() >= 260
            && destination_state.corridor_belief.delivery_support.value()
                >= FIELD_ROUTE_WEAK_SUPPORT_FLOOR.saturating_sub(40)
    }

    #[must_use]
    pub(crate) fn decision_for_bootstrap(
        self,
        destination_state: &DestinationFieldState,
        confirmation_streak: u8,
        promotion_window_score: u8,
        search_config: &crate::FieldSearchConfig,
    ) -> FieldBootstrapDecision {
        if (self.can_promote(promotion_window_score)
            || self.confirmed_stability(
                destination_state,
                confirmation_streak,
                promotion_window_score,
            ))
            && promoted_corridor_admissible_with_config(
                destination_state,
                confirmation_streak,
                promotion_window_score,
                search_config,
            )
        {
            FieldBootstrapDecision::Promote
        } else if self.degraded_but_coherent(destination_state)
            && destination_state.frontier.len() > 1
        {
            FieldBootstrapDecision::Narrow
        } else if self.degraded_but_coherent(destination_state) {
            FieldBootstrapDecision::Hold
        } else {
            FieldBootstrapDecision::Withdraw
        }
    }

    #[must_use]
    pub(crate) fn primary_blocker(self) -> FieldPromotionBlocker {
        if !self.support_growth {
            FieldPromotionBlocker::SupportTrend
        } else if !self.uncertainty_reduced {
            FieldPromotionBlocker::Uncertainty
        } else if !self.anti_entropy_confirmed {
            FieldPromotionBlocker::AntiEntropyConfirmation
        } else if !self.continuation_coherent {
            FieldPromotionBlocker::ContinuationCoherence
        } else {
            FieldPromotionBlocker::Freshness
        }
    }
}

#[must_use]
#[allow(dead_code)]
// long-block-exception: promotion assessment keeps the bootstrap, degraded,
// and anti-entropy upgrade rules in one coherent route-state evaluation.
pub(crate) fn promotion_assessment_for_route(
    active_route: &ActiveFieldRoute,
    destination_state: &DestinationFieldState,
    best_neighbor: &crate::state::NeighborContinuation,
    now_tick: Tick,
) -> FieldPromotionAssessment {
    promotion_assessment_for_route_with_policy(
        active_route,
        destination_state,
        best_neighbor,
        now_tick,
        &DEFAULT_FIELD_POLICY.promotion,
    )
}

#[must_use]
pub(crate) fn promotion_assessment_for_route_with_policy(
    active_route: &ActiveFieldRoute,
    destination_state: &DestinationFieldState,
    best_neighbor: &crate::state::NeighborContinuation,
    now_tick: Tick,
    policy: &FieldPromotionPolicy,
) -> FieldPromotionAssessment {
    let destination_view = crate::operational::destination_operational_view(destination_state);
    let route_view = crate::operational::route_operational_view(now_tick, best_neighbor.freshness);
    let confirmation_streak = active_route.bootstrap_confirmation_streak;
    let corridor_support = destination_state.corridor_belief.delivery_support.value();
    let corridor_entropy = destination_state.posterior.usability_entropy.value();
    let corridor_retention = destination_state.corridor_belief.retention_affinity.value();
    let corridor_mass = destination_state.posterior.top_corridor_mass.value();
    let promotion_window_score = active_route.promotion_window_score;
    let support_growth = destination_state.corridor_belief.delivery_support.value()
        >= active_route
            .witness_detail
            .corridor_support
            .value()
            .saturating_add(policy.support_growth_delta_permille)
        || destination_view.support_band >= crate::operational::SupportBand::Strong
        || (promotion_window_score >= 2
            && corridor_support.saturating_add(25)
                >= active_route.witness_detail.corridor_support.value()
            && corridor_retention >= 280
            && corridor_mass >= 260)
        || (confirmation_streak >= 1
            && corridor_support >= 250
            && corridor_retention >= 300
            && corridor_mass >= 300);
    let uncertainty_reduced = destination_state
        .posterior
        .usability_entropy
        .value()
        .saturating_add(policy.uncertainty_reduction_delta_permille)
        <= active_route.witness_detail.usability_entropy.value()
        || destination_state.posterior.usability_entropy.value()
            <= policy.strong_entropy_ceiling_permille
        || (promotion_window_score >= 2
            && corridor_entropy <= 860
            && corridor_retention >= 280
            && corridor_mass >= 260)
        || (confirmation_streak >= 1 && corridor_entropy <= 840 && corridor_mass >= 300);
    let anti_entropy_confirmed = matches!(
        evidence_class_from_state(destination_state),
        EvidenceContributionClass::Direct | EvidenceContributionClass::ReverseFeedback
    ) || destination_state
        .publication
        .last_summary
        .as_ref()
        .is_some_and(|previous_summary| {
            let current_summary = FieldSummary {
                destination: SummaryDestinationKey::from(&DestinationId::from(
                    &destination_state.destination,
                )),
                topology_epoch: previous_summary.topology_epoch,
                freshness_tick: now_tick,
                hop_band: destination_state.corridor_belief.expected_hop_band,
                delivery_support: destination_state.corridor_belief.delivery_support,
                congestion_penalty: destination_state.corridor_belief.congestion_penalty,
                retention_support: destination_state.corridor_belief.retention_affinity,
                uncertainty_penalty: destination_state.posterior.usability_entropy,
                evidence_class: evidence_class_from_state(destination_state),
                uncertainty_class: uncertainty_class_for(
                    destination_state.posterior.usability_entropy.value(),
                ),
            };
            let divergence = summary_divergence(previous_summary, &current_summary).value();
            let recent_publication =
                destination_state
                    .publication
                    .last_sent_at
                    .is_some_and(|tick| {
                        now_tick.0.saturating_sub(tick.0)
                            <= if promotion_window_score >= 2 {
                                policy.anti_entropy_relaxed_recent_publication_ticks
                            } else {
                                policy.anti_entropy_recent_publication_ticks
                            }
                    });
            recent_publication
                && divergence
                    <= if confirmation_streak >= 1 || promotion_window_score >= 2 {
                        policy.anti_entropy_relaxed_divergence_ceiling_permille
                    } else {
                        policy.anti_entropy_divergence_ceiling_permille
                    }
                && previous_summary.retention_support.value()
                    >= if confirmation_streak >= 1 || promotion_window_score >= 2 {
                        policy.anti_entropy_relaxed_retention_floor_permille
                    } else {
                        policy.anti_entropy_retention_floor_permille
                    }
                && previous_summary.delivery_support.value().saturating_add(
                    if confirmation_streak >= 1 || promotion_window_score >= 2 {
                        policy.anti_entropy_relaxed_delivery_bonus_permille
                    } else {
                        policy.anti_entropy_delivery_bonus_permille
                    },
                ) >= destination_state.corridor_belief.delivery_support.value()
                && (confirmation_streak == 0
                    || (corridor_retention >= policy.strong_retention_floor_permille
                        && corridor_mass >= policy.strong_top_mass_floor_permille))
        });
    let continuation_coherent = active_route
        .continuation_neighbors
        .contains(&best_neighbor.neighbor_id)
        || destination_state.frontier.len() <= 2
        || best_neighbor
            .net_value
            .value()
            .saturating_add(policy.coherence_best_neighbor_bonus_permille)
            >= destination_state.corridor_belief.delivery_support.value()
        || (promotion_window_score >= 2 && corridor_mass >= 260)
        || (confirmation_streak >= 1
            && best_neighbor
                .downstream_support
                .value()
                .saturating_add(policy.coherence_relaxed_downstream_bonus_permille)
                >= corridor_support);
    let fresh_enough = route_view.freshness_age_ticks
        <= if confirmation_streak >= 1 || promotion_window_score >= 2 {
            policy.relaxed_fresh_neighbor_ticks
        } else {
            policy.fresh_neighbor_ticks
        };

    FieldPromotionAssessment {
        support_growth,
        uncertainty_reduced,
        anti_entropy_confirmed,
        continuation_coherent,
        fresh_enough,
    }
}
