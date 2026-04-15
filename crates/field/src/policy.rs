//! Internal operational policy for the field engine.
//!
//! This surface centralizes calibrated thresholds without turning Field into a
//! dynamic rule engine. Everything remains deterministic, integer-typed, and
//! cheap to read from hot-path code.

mod defaults;

pub(crate) use defaults::DEFAULT_FIELD_POLICY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldPolicy {
    pub(crate) regime: FieldRegimePolicy,
    pub(crate) posture: FieldPosturePolicy,
    pub(crate) continuity: FieldContinuityPolicy,
    pub(crate) promotion: FieldPromotionPolicy,
    pub(crate) evidence: FieldEvidencePolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldRegimePolicy {
    pub(crate) fallback_risk_pressure_permille: u16,
    pub(crate) destination_count_strength_step_permille: u16,
    pub(crate) dwell_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldPosturePolicy {
    pub(crate) dwell_ticks: u64,
    pub(crate) primary_fast_path_regime_score_floor_permille: u16,
    pub(crate) primary_fast_path_threshold_divisor: u16,
    pub(crate) risk_suppressed_hold_field_strength_floor_permille: u16,
    pub(crate) risk_suppressed_hold_retention_alignment_floor_permille: u16,
    pub(crate) risk_suppressed_hold_relay_alignment_floor_permille: u16,
    pub(crate) risk_suppressed_hold_risk_price_ceiling_permille: u16,
    pub(crate) sparse_opportunistic_bonus_permille: u16,
    pub(crate) sparse_structured_bonus_permille: u16,
    pub(crate) congested_structured_bonus_permille: u16,
    pub(crate) congested_retention_biased_bonus_permille: u16,
    pub(crate) retention_favorable_retention_biased_bonus_permille: u16,
    pub(crate) unstable_risk_suppressed_bonus_permille: u16,
    pub(crate) adversarial_risk_suppressed_bonus_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldContinuityPolicy {
    pub(crate) steady_support_floor_permille: u16,
    pub(crate) steady_entropy_ceiling_permille: u16,
    pub(crate) bootstrap: FieldBootstrapContinuityPolicy,
    pub(crate) continuation: FieldContinuationPolicy,
    pub(crate) runtime: FieldRuntimeContinuityPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldBootstrapContinuityPolicy {
    pub(crate) service_support_floor_permille: u16,
    pub(crate) service_top_mass_floor_permille: u16,
    pub(crate) service_entropy_ceiling_permille: u16,
    pub(crate) service_corroborated_branch_count_min: usize,
    pub(crate) service_corroborated_support_floor_permille: u16,
    pub(crate) service_corroborated_retention_floor_permille: u16,
    pub(crate) service_corroborated_top_mass_floor_permille: u16,
    pub(crate) service_corroborated_entropy_ceiling_permille: u16,
    pub(crate) service_corroborated_support_score_floor_permille: u16,
    pub(crate) reverse_feedback_node_top_mass_floor_permille: u16,
    pub(crate) reverse_feedback_support_relief_permille: u16,
    pub(crate) reverse_feedback_discovery_top_mass_relief_permille: u16,
    pub(crate) reverse_feedback_discovery_support_relief_permille: u16,
    pub(crate) reverse_feedback_retention_floor_permille: u16,
    pub(crate) reverse_feedback_discovery_retention_floor_permille: u16,
    pub(crate) reverse_feedback_coherent_sources_min: usize,
    pub(crate) reverse_feedback_discovery_coherent_sources_min: usize,
    pub(crate) forward_propagated_service_top_mass_floor_permille: u16,
    pub(crate) forward_propagated_service_retention_floor_permille: u16,
    pub(crate) forward_propagated_service_combined_support_floor_permille: u16,
    pub(crate) forward_propagated_shared_top_mass_floor_permille: u16,
    pub(crate) forward_propagated_shared_retention_floor_permille: u16,
    pub(crate) forward_propagated_shared_combined_support_floor_permille: u16,
    pub(crate) forward_propagated_shared_coherent_sources_min: usize,
    pub(crate) forward_propagated_discovery_top_mass_relief_permille: u16,
    pub(crate) forward_propagated_discovery_retention_floor_permille: u16,
    pub(crate) forward_propagated_discovery_combined_support_bonus_permille: u16,
    pub(crate) discovery_service_support_score_floor_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldContinuationPolicy {
    pub(crate) service_pending_retention_floor_permille: u16,
    pub(crate) service_pending_support_floor_permille: u16,
    pub(crate) service_pending_uncertainty_ceiling_permille: u16,
    pub(crate) node_pending_retention_floor_permille: u16,
    pub(crate) node_pending_support_floor_permille: u16,
    pub(crate) node_pending_uncertainty_ceiling_permille: u16,
    pub(crate) service_shift_quality_margin_permille: u16,
    pub(crate) service_shift_downstream_margin_permille: u16,
    pub(crate) node_selection_support_floor_permille: u16,
    pub(crate) service_runtime_support_floor_permille: u16,
    pub(crate) node_runtime_support_floor_permille: u16,
    pub(crate) service_frontier_viability_support_floor_permille: u16,
    pub(crate) service_frontier_viability_retention_floor_permille: u16,
    pub(crate) service_forward_viability_support_floor_permille: u16,
    pub(crate) service_forward_viability_retention_floor_permille: u16,
    pub(crate) service_forward_viability_uncertainty_ceiling_permille: u16,
    pub(crate) service_viable_branch_count_min: usize,
    pub(crate) node_frontier_viability_support_floor_permille: u16,
    pub(crate) node_frontier_viability_retention_floor_permille: u16,
    pub(crate) node_forward_viability_support_floor_permille: u16,
    pub(crate) node_forward_viability_retention_floor_permille: u16,
    pub(crate) node_forward_viability_uncertainty_ceiling_permille: u16,
    pub(crate) node_viable_branch_count_min: usize,
    pub(crate) synthesized_rank_penalty_permille: u16,
    pub(crate) synthesized_reachability_bonus_permille: u16,
    pub(crate) synthesized_selected_neighbor_bonus_permille: u16,
    pub(crate) synthesized_retention_bonus_permille: u16,
    pub(crate) transmission_divergence_trigger_permille: u16,
    pub(crate) transmission_weak_support_floor_permille: u16,
    pub(crate) transmission_retention_affinity_floor_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldRuntimeContinuityPolicy {
    pub(crate) route_failure_support_floor_permille: u16,
    pub(crate) route_weak_support_floor_permille: u16,
    pub(crate) bootstrap_failure_support_floor_permille: u16,
    pub(crate) degraded_steady_failure_support_floor_permille: u16,
    pub(crate) bootstrap_stale_ticks_max: u64,
    pub(crate) degraded_steady_stale_ticks_max: u64,
    pub(crate) envelope_shift_support_delta_max_permille: u16,
    pub(crate) service_shift_delta_bonus_permille: u16,
    pub(crate) discovery_shift_delta_bonus_permille: u16,
    pub(crate) degraded_shift_delta_bonus_permille: u16,
    pub(crate) support_floor_relief_permille: u16,
    pub(crate) retention_biased_hold_congestion_price_floor_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldPromotionPolicy {
    pub(crate) stable_support_floor_permille: u16,
    pub(crate) stable_retention_floor_permille: u16,
    pub(crate) stable_top_mass_floor_permille: u16,
    pub(crate) stable_entropy_ceiling_permille: u16,
    pub(crate) weak_support_growth_bonus_permille: u16,
    pub(crate) strong_support_floor_permille: u16,
    pub(crate) strong_retention_floor_permille: u16,
    pub(crate) strong_top_mass_floor_permille: u16,
    pub(crate) strong_entropy_ceiling_permille: u16,
    pub(crate) support_growth_delta_permille: u16,
    pub(crate) uncertainty_reduction_delta_permille: u16,
    pub(crate) anti_entropy_divergence_ceiling_permille: u16,
    pub(crate) anti_entropy_relaxed_divergence_ceiling_permille: u16,
    pub(crate) anti_entropy_retention_floor_permille: u16,
    pub(crate) anti_entropy_relaxed_retention_floor_permille: u16,
    pub(crate) anti_entropy_delivery_bonus_permille: u16,
    pub(crate) anti_entropy_relaxed_delivery_bonus_permille: u16,
    pub(crate) anti_entropy_recent_publication_ticks: u64,
    pub(crate) anti_entropy_relaxed_recent_publication_ticks: u64,
    pub(crate) coherence_best_neighbor_bonus_permille: u16,
    pub(crate) coherence_relaxed_downstream_bonus_permille: u16,
    pub(crate) fresh_neighbor_ticks: u64,
    pub(crate) relaxed_fresh_neighbor_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldEvidencePolicy {
    pub(crate) publication: FieldPublicationPolicy,
    pub(crate) observer: FieldObserverEvidencePolicy,
    pub(crate) replay: FieldReplayEvidencePolicy,
    pub(crate) summary_decay: FieldSummaryDecayPolicy,
    pub(crate) attractor: FieldAttractorPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldPublicationPolicy {
    pub(crate) service_neighbor_retention_floor_permille: u16,
    pub(crate) service_neighbor_support_floor_permille: u16,
    pub(crate) service_neighbor_uncertainty_ceiling_permille: u16,
    pub(crate) service_evidence_score_bonus_permille: u16,
    pub(crate) service_frontier_score_bonus_permille: u16,
    pub(crate) node_evidence_score_bonus_permille: u16,
    pub(crate) node_frontier_score_bonus_permille: u16,
    pub(crate) node_support_floor_min_permille: u16,
    pub(crate) service_corroborating_frontier_support_floor_permille: u16,
    pub(crate) service_corroborating_frontier_net_value_floor_permille: u16,
    pub(crate) corroboration_branch_bonus_permille: u16,
    pub(crate) service_freshness_weight_min_permille: u16,
    pub(crate) service_freshness_weight_max_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldObserverEvidencePolicy {
    pub(crate) service_carry_forward_freshness_ticks: u64,
    pub(crate) node_carry_forward_freshness_ticks: u64,
    pub(crate) service_carry_forward_retention_floor_permille: u16,
    pub(crate) service_carry_forward_support_floor_permille: u16,
    pub(crate) node_carry_forward_retention_floor_permille: u16,
    pub(crate) node_carry_forward_support_floor_permille: u16,
    pub(crate) service_delivery_bonus_permille: u16,
    pub(crate) service_delivery_decay_step_permille: u16,
    pub(crate) node_delivery_bonus_permille: u16,
    pub(crate) service_retention_bonus_permille: u16,
    pub(crate) service_retention_decay_step_permille: u16,
    pub(crate) node_retention_bonus_permille: u16,
    pub(crate) synthesized_node_publication_staleness_slack_ticks: u64,
    pub(crate) synthesized_node_support_relief_permille: u16,
    pub(crate) synthesized_node_support_floor_min_permille: u16,
    pub(crate) synthesized_node_retention_floor_permille: u16,
    pub(crate) synthesized_node_rank_penalty_permille: u16,
    pub(crate) synthesized_node_selected_neighbor_bonus_permille: u16,
    pub(crate) synthesized_node_reachability_bonus_permille: u16,
    pub(crate) synthesized_node_retention_bonus_permille: u16,
    pub(crate) replay_bridge_support_bonus_permille: u16,
    pub(crate) replay_bridge_retention_bonus_permille: u16,
    pub(crate) replay_high_retention_floor_permille: u16,
    pub(crate) replay_medium_retention_floor_permille: u16,
    pub(crate) replay_high_uncertainty_relief_permille: u16,
    pub(crate) replay_medium_uncertainty_relief_permille: u16,
    pub(crate) replay_low_uncertainty_relief_permille: u16,
    pub(crate) replay_publication_retention_relief_permille: u16,
    pub(crate) prune_extended_retention_floor_permille: u16,
    pub(crate) prune_extended_support_floor_permille: u16,
    pub(crate) prune_moderate_retention_floor_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldReplayEvidencePolicy {
    pub(crate) low_coherence_score_floor_permille: u16,
    pub(crate) retention_replay_support_floor_permille: u16,
    pub(crate) retention_replay_relaxed_support_floor_permille: u16,
    pub(crate) retention_replay_uncertainty_floor_permille: u16,
    pub(crate) stale_primary_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldSummaryDecayPolicy {
    pub(crate) retention_bias_floor_permille: u16,
    pub(crate) retention_bias_cap_permille: u16,
    pub(crate) sparse_relief_uncertainty_ceiling_permille: u16,
    pub(crate) sparse_relief_bonus_permille: u16,
    pub(crate) reverse_feedback_retention_floor_permille: u16,
    pub(crate) reverse_feedback_uncertainty_ceiling_permille: u16,
    pub(crate) reverse_feedback_bonus_permille: u16,
    pub(crate) forward_propagated_retention_floor_permille: u16,
    pub(crate) forward_propagated_support_floor_permille: u16,
    pub(crate) forward_propagated_uncertainty_ceiling_permille: u16,
    pub(crate) forward_propagated_bonus_permille: u16,
    pub(crate) fallback_bonus_permille: u16,
    pub(crate) reflected_uncertainty_penalty_permille: u16,
    pub(crate) reverse_feedback_uncertainty_penalty_permille: u16,
    pub(crate) sparse_regime_uncertainty_penalty_permille: u16,
    pub(crate) congested_regime_uncertainty_penalty_permille: u16,
    pub(crate) retention_favorable_regime_uncertainty_penalty_permille: u16,
    pub(crate) unstable_regime_uncertainty_penalty_permille: u16,
    pub(crate) adversarial_regime_uncertainty_penalty_permille: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldAttractorPolicy {
    pub(crate) sparse_relay_alignment_divisor: u16,
    pub(crate) congested_alignment_divisor: u16,
    pub(crate) retention_affinity_divisor: u16,
    pub(crate) unstable_risk_alignment_divisor: u16,
    pub(crate) opportunistic_risk_relief_divisor: u16,
    pub(crate) structured_alignment_divisor: u16,
    pub(crate) retention_biased_affinity_divisor: u16,
    pub(crate) risk_suppressed_alignment_divisor: u16,
    pub(crate) penalty_divisor: u16,
}

impl Default for FieldPolicy {
    fn default() -> Self {
        DEFAULT_FIELD_POLICY
    }
}
