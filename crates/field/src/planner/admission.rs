//! Admission and continuity classification for field route publication.

use jacquard_core::{
    AdmissionAssumptions, AdmissionDecision, AdversaryRegime, ByteCount, ClaimStrength,
    Configuration, ConnectivityPosture, ConnectivityRegime, FailureModelClass, Limit,
    MessageFlowAssumptionClass, NodeDensityClass, Observation, RouteAdmissionCheck,
    RouteAdmissionRejection, RouteCost, RouteProtectionClass, RouteSummary, RuntimeEnvelopeClass,
    SelectedRoutingParameters,
};

use crate::{
    route::{FieldBootstrapClass, FieldContinuityBand, FieldWitnessDetail},
    state::{
        CorridorBeliefEnvelope, DestinationFieldState, DestinationKey, ObservationClass,
        OperatingRegime, RoutingPosture,
    },
    summary::{EvidenceContributionClass, SummaryUncertaintyClass},
    FIELD_CAPABILITIES,
};

use super::publication::{service_corroborated_support_score, service_corroborating_branch_count};

pub(super) struct AdmissionInputs<'a> {
    pub(super) objective: &'a jacquard_core::RoutingObjective,
    pub(super) profile: &'a SelectedRoutingParameters,
    pub(super) summary: &'a RouteSummary,
    pub(super) destination_state: &'a DestinationFieldState,
    pub(super) delivered_protection: RouteProtectionClass,
    pub(super) delivered_connectivity: ConnectivityPosture,
    pub(super) assumptions: AdmissionAssumptions,
    pub(super) route_cost: RouteCost,
    pub(super) search_config: &'a crate::FieldSearchConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FieldAdmissionClass {
    BootstrapAdmissible,
    SteadyAdmissible,
}

pub(super) fn evidence_class_from_state(
    destination_state: &DestinationFieldState,
) -> EvidenceContributionClass {
    match destination_state.posterior.predicted_observation_class {
        ObservationClass::DirectOnly => EvidenceContributionClass::Direct,
        ObservationClass::ForwardPropagated | ObservationClass::Mixed => {
            EvidenceContributionClass::ForwardPropagated
        }
        ObservationClass::ReverseValidated => EvidenceContributionClass::ReverseFeedback,
    }
}

pub(super) fn selected_neighbor_publishable(
    destination_state: &DestinationFieldState,
    topology: &Observation<Configuration>,
    local_node_id: jacquard_core::NodeId,
    selected_neighbor: jacquard_core::NodeId,
) -> bool {
    destination_state
        .frontier
        .as_slice()
        .iter()
        .any(|entry| entry.neighbor_id == selected_neighbor)
        || destination_state
            .pending_forward_evidence
            .iter()
            .any(|evidence| evidence.from_neighbor == selected_neighbor)
        || topology
            .value
            .links
            .contains_key(&(local_node_id, selected_neighbor))
        || topology
            .value
            .links
            .contains_key(&(selected_neighbor, local_node_id))
}

pub(super) fn admission_check_for(inputs: AdmissionInputs<'_>) -> RouteAdmissionCheck {
    let AdmissionInputs {
        objective,
        profile,
        summary,
        destination_state,
        delivered_protection,
        delivered_connectivity,
        assumptions,
        route_cost,
        search_config,
    } = inputs;

    let decision = if !bootstrap_corridor_admissible_with_config(destination_state, search_config) {
        AdmissionDecision::Rejected(RouteAdmissionRejection::CapacityExceeded)
    } else if objective.protection_floor > FIELD_CAPABILITIES.max_protection
        || profile.selected_protection > FIELD_CAPABILITIES.max_protection
        || delivered_protection < objective.protection_floor
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    } else if !steady_corridor_admissible(destination_state)
        && destination_state.posterior.usability_entropy.value()
            > if search_config.node_discovery_enabled() {
                search_config.node_bootstrap_entropy_ceiling()
            } else {
                925
            }
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
    } else if delivered_connectivity.repair < profile.selected_connectivity.repair
        || delivered_connectivity.partition < profile.selected_connectivity.partition
    {
        AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible)
    } else {
        AdmissionDecision::Admissible
    };

    RouteAdmissionCheck {
        decision,
        profile: assumptions,
        productive_step_bound: Limit::Bounded(u32::from(summary.hop_count_hint.value_or(1))),
        total_step_bound: Limit::Bounded(
            u32::from(summary.hop_count_hint.value_or(1)).saturating_add(2),
        ),
        route_cost,
    }
}

pub(super) fn delivered_protection(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> RouteProtectionClass {
    if bootstrap_corridor_admissible_with_config(destination_state, search_config) {
        RouteProtectionClass::LinkProtected
    } else {
        RouteProtectionClass::None
    }
}

pub(super) fn delivered_connectivity(
    posture: RoutingPosture,
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> ConnectivityPosture {
    let partition = if bootstrap_corridor_admissible_with_config(destination_state, search_config)
        || posture == RoutingPosture::RetentionBiased
    {
        jacquard_core::RoutePartitionClass::PartitionTolerant
    } else {
        jacquard_core::RoutePartitionClass::ConnectedOnly
    };
    let repair = if posture == RoutingPosture::RiskSuppressed
        && destination_state.posterior.usability_entropy.value() > 700
    {
        jacquard_core::RouteRepairClass::BestEffort
    } else {
        jacquard_core::RouteRepairClass::Repairable
    };
    ConnectivityPosture { repair, partition }
}

#[cfg(test)]
pub(crate) fn bootstrap_corridor_admissible(destination_state: &DestinationFieldState) -> bool {
    bootstrap_corridor_admissible_with_config(
        destination_state,
        &crate::FieldSearchConfig::default(),
    )
}

// long-block-exception: bootstrap admission keeps the node and service
// thresholds in one fail-closed gate over the same belief surface.
pub(crate) fn bootstrap_corridor_admissible_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    let support = destination_state.corridor_belief.delivery_support.value();
    let entropy = destination_state.posterior.usability_entropy.value();
    let retention = destination_state.corridor_belief.retention_affinity.value();
    let top_mass = destination_state.posterior.top_corridor_mass.value();
    let evidence_class = evidence_class_from_state(destination_state);
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let discovery_enabled = !service_bias && search_config.node_discovery_enabled();
    let support_floor = if service_bias {
        130
    } else {
        search_config.node_bootstrap_support_floor()
    };
    let top_mass_floor = if service_bias {
        260
    } else {
        search_config.node_bootstrap_top_mass_floor()
    };
    let entropy_ceiling = if service_bias {
        950
    } else {
        search_config.node_bootstrap_entropy_ceiling()
    };
    let coherent_source_count = destination_state
        .frontier
        .len()
        .max(destination_state.pending_forward_evidence.len());
    let service_branch_count = service_corroborating_branch_count(destination_state);
    let service_support_score =
        service_corroborated_support_score(destination_state, &crate::FieldSearchConfig::default());

    if support < support_floor || entropy > entropy_ceiling {
        return false;
    }

    if service_bias
        && service_branch_count >= 2
        && support >= 130
        && retention >= 140
        && top_mass >= 140
        && entropy <= 970
        && service_support_score >= 380
    {
        return true;
    }

    match evidence_class {
        EvidenceContributionClass::Direct => {
            top_mass
                >= if discovery_enabled {
                    top_mass_floor.saturating_sub(80)
                } else {
                    top_mass_floor
                }
        }
        EvidenceContributionClass::ReverseFeedback => {
            top_mass
                >= if discovery_enabled {
                    top_mass_floor.saturating_sub(100)
                } else {
                    180
                }
                && (support >= support_floor.saturating_sub(40)
                    || retention >= if discovery_enabled { 140 } else { 180 }
                    || coherent_source_count >= if discovery_enabled { 1 } else { 2 })
        }
        EvidenceContributionClass::ForwardPropagated => {
            (top_mass >= 260 && retention >= 220 && support.saturating_add(retention) >= 520)
                || (coherent_source_count >= 2
                    && top_mass >= 180
                    && retention >= 160
                    && support.saturating_add(retention) >= 420)
                || (discovery_enabled
                    && coherent_source_count >= 1
                    && top_mass >= top_mass_floor.saturating_sub(90)
                    && retention >= 140
                    && support.saturating_add(retention) >= support_floor.saturating_add(160))
        }
    }
}

pub(crate) fn steady_corridor_admissible(destination_state: &DestinationFieldState) -> bool {
    destination_state.corridor_belief.delivery_support.value() >= 300
        && destination_state.posterior.usability_entropy.value() <= 850
}

#[cfg(test)]
pub(crate) fn promoted_corridor_admissible(
    destination_state: &DestinationFieldState,
    confirmation_streak: u8,
    promotion_window_score: u8,
) -> bool {
    promoted_corridor_admissible_with_config(
        destination_state,
        confirmation_streak,
        promotion_window_score,
        &crate::FieldSearchConfig::default(),
    )
}

pub(crate) fn promoted_corridor_admissible_with_config(
    destination_state: &DestinationFieldState,
    confirmation_streak: u8,
    promotion_window_score: u8,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    if steady_corridor_admissible(destination_state) {
        return true;
    }
    let window_confirmed = confirmation_streak >= 1 || promotion_window_score >= 3;
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let service_branch_count = service_corroborating_branch_count(destination_state);
    let service_support_score =
        service_corroborated_support_score(destination_state, &crate::FieldSearchConfig::default());
    if service_bias
        && service_branch_count >= 2
        && destination_state.corridor_belief.delivery_support.value() >= 150
        && destination_state.posterior.usability_entropy.value() <= 950
        && destination_state.corridor_belief.retention_affinity.value() >= 160
        && service_support_score >= if window_confirmed { 420 } else { 460 }
    {
        return true;
    }
    destination_state.corridor_belief.delivery_support.value()
        >= if search_config.node_discovery_enabled() {
            search_config
                .node_bootstrap_support_floor()
                .saturating_sub(20)
                .max(180)
        } else {
            180
        }
        && destination_state.posterior.usability_entropy.value()
            <= if search_config.node_discovery_enabled() {
                search_config
                    .node_bootstrap_entropy_ceiling()
                    .saturating_sub(if window_confirmed { 20 } else { 35 })
                    .max(if window_confirmed { 940 } else { 925 })
            } else if window_confirmed {
                940
            } else {
                925
            }
        && destination_state.corridor_belief.retention_affinity.value()
            >= if window_confirmed { 220 } else { 240 }
        && destination_state.posterior.top_corridor_mass.value()
            >= if search_config.node_discovery_enabled() {
                search_config
                    .node_bootstrap_top_mass_floor()
                    .saturating_sub(if window_confirmed { 20 } else { 0 })
                    .max(if window_confirmed { 200 } else { 220 })
            } else if window_confirmed {
                200
            } else {
                220
            }
}

pub(super) fn admission_class_for_state_with_config(
    destination_state: &DestinationFieldState,
    _search_config: &crate::FieldSearchConfig,
) -> FieldAdmissionClass {
    if steady_corridor_admissible(destination_state) {
        FieldAdmissionClass::SteadyAdmissible
    } else {
        FieldAdmissionClass::BootstrapAdmissible
    }
}

pub(crate) fn bootstrap_class_for_state(
    destination_state: &DestinationFieldState,
) -> FieldBootstrapClass {
    bootstrap_class_for_state_with_config(destination_state, &crate::FieldSearchConfig::default())
}

pub(crate) fn bootstrap_class_for_state_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> FieldBootstrapClass {
    match admission_class_for_state_with_config(destination_state, search_config) {
        FieldAdmissionClass::BootstrapAdmissible => FieldBootstrapClass::Bootstrap,
        FieldAdmissionClass::SteadyAdmissible => FieldBootstrapClass::Steady,
    }
}

fn degraded_steady_band_admissible_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let discovery_node_route = !service_bias && search_config.node_discovery_enabled();
    let support_floor = if service_bias || discovery_node_route {
        180
    } else {
        220
    };
    let retention_floor = if service_bias {
        240
    } else if discovery_node_route {
        180
    } else {
        220
    };
    let top_mass_floor = if service_bias || discovery_node_route {
        160
    } else {
        180
    };
    destination_state.corridor_belief.delivery_support.value() >= support_floor
        && destination_state.corridor_belief.retention_affinity.value() >= retention_floor
        && destination_state.posterior.top_corridor_mass.value() >= top_mass_floor
        && destination_state.posterior.usability_entropy.value()
            <= if discovery_node_route { 960 } else { 940 }
}

fn steady_route_softening_needed_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> bool {
    let service_bias = matches!(destination_state.destination, DestinationKey::Service(_));
    let discovery_node_route = !service_bias && search_config.node_discovery_enabled();
    let support = destination_state.corridor_belief.delivery_support.value();
    let retention = destination_state.corridor_belief.retention_affinity.value();
    let top_mass = destination_state.posterior.top_corridor_mass.value();
    let entropy = destination_state.posterior.usability_entropy.value();
    support < if discovery_node_route { 320 } else { 360 }
        || retention < if discovery_node_route { 260 } else { 320 }
        || top_mass < if discovery_node_route { 220 } else { 280 }
        || entropy > if discovery_node_route { 820 } else { 760 }
}

pub(crate) fn continuity_band_for_state(
    destination_state: &DestinationFieldState,
) -> FieldContinuityBand {
    continuity_band_for_state_with_config(destination_state, &crate::FieldSearchConfig::default())
}

pub(crate) fn continuity_band_for_state_with_config(
    destination_state: &DestinationFieldState,
    search_config: &crate::FieldSearchConfig,
) -> FieldContinuityBand {
    if steady_corridor_admissible(destination_state)
        && !steady_route_softening_needed_with_config(destination_state, search_config)
    {
        FieldContinuityBand::Steady
    } else if degraded_steady_band_admissible_with_config(destination_state, search_config) {
        FieldContinuityBand::DegradedSteady
    } else {
        FieldContinuityBand::Bootstrap
    }
}

// long-block-exception: service publication narrowing keeps corroboration and
// freshness ordering in one deterministic neighbor-selection pass.

pub(super) fn admission_assumptions(
    witness_detail: &FieldWitnessDetail,
    regime: OperatingRegime,
    admission_class: FieldAdmissionClass,
) -> AdmissionAssumptions {
    AdmissionAssumptions {
        message_flow_assumption: MessageFlowAssumptionClass::BestEffort,
        failure_model: match regime {
            OperatingRegime::Adversarial => FailureModelClass::ByzantineInterface,
            OperatingRegime::Unstable => FailureModelClass::CrashStop,
            _ => FailureModelClass::Benign,
        },
        runtime_envelope: RuntimeEnvelopeClass::EnvelopeAdmitted,
        node_density_class: match regime {
            OperatingRegime::Sparse => NodeDensityClass::Sparse,
            OperatingRegime::Congested => NodeDensityClass::Dense,
            OperatingRegime::RetentionFavorable
            | OperatingRegime::Unstable
            | OperatingRegime::Adversarial => NodeDensityClass::Moderate,
        },
        connectivity_regime: match regime {
            OperatingRegime::Sparse => ConnectivityRegime::Stable,
            OperatingRegime::Congested | OperatingRegime::RetentionFavorable => {
                ConnectivityRegime::PartitionProne
            }
            OperatingRegime::Unstable | OperatingRegime::Adversarial => {
                ConnectivityRegime::HighChurn
            }
        },
        adversary_regime: match regime {
            OperatingRegime::Adversarial => AdversaryRegime::ActiveAdversarial,
            OperatingRegime::Unstable => AdversaryRegime::BenignUntrusted,
            _ => AdversaryRegime::Cooperative,
        },
        claim_strength: match (
            admission_class,
            witness_detail.evidence_class,
            witness_detail.uncertainty_class,
        ) {
            (FieldAdmissionClass::BootstrapAdmissible, _, _) => ClaimStrength::InterfaceOnly,
            (
                FieldAdmissionClass::SteadyAdmissible,
                EvidenceContributionClass::Direct,
                SummaryUncertaintyClass::Low,
            ) => ClaimStrength::ConservativeUnderProfile,
            (_, _, SummaryUncertaintyClass::High) => ClaimStrength::InterfaceOnly,
            _ => ClaimStrength::ConservativeUnderProfile,
        },
    }
}

pub(super) fn route_cost_for(
    corridor: &CorridorBeliefEnvelope,
    continuation_neighbor_count: usize,
    posture: RoutingPosture,
) -> RouteCost {
    let hop_count = corridor.expected_hop_band.max_hops.max(1);
    let hold_bytes_reserved = if posture == RoutingPosture::RetentionBiased {
        ByteCount(256)
    } else {
        ByteCount(0)
    };
    RouteCost {
        message_count_max: Limit::Bounded(u32::from(hop_count)),
        byte_count_max: Limit::Bounded(ByteCount(u64::from(hop_count) * 256)),
        hop_count,
        repair_attempt_count_max: Limit::Bounded(
            u32::try_from(continuation_neighbor_count)
                .expect("continuation neighbor count fits u32"),
        ),
        hold_bytes_reserved: Limit::Bounded(hold_bytes_reserved),
        work_step_count_max: Limit::Bounded(
            u32::from(hop_count)
                .saturating_add(
                    u32::try_from(continuation_neighbor_count)
                        .expect("continuation neighbor count fits u32"),
                )
                .saturating_add(1),
        ),
    }
}

pub(super) fn rejected_check(
    assumptions: AdmissionAssumptions,
    reason: RouteAdmissionRejection,
) -> RouteAdmissionCheck {
    RouteAdmissionCheck {
        decision: AdmissionDecision::Rejected(reason),
        profile: assumptions,
        productive_step_bound: Limit::Bounded(0),
        total_step_bound: Limit::Bounded(0),
        route_cost: RouteCost {
            message_count_max: Limit::Bounded(0),
            byte_count_max: Limit::Bounded(ByteCount(0)),
            hop_count: 0,
            repair_attempt_count_max: Limit::Bounded(0),
            hold_bytes_reserved: Limit::Bounded(ByteCount(0)),
            work_step_count_max: Limit::Bounded(0),
        },
    }
}

pub(super) fn uncertainty_class_for(value: u16) -> SummaryUncertaintyClass {
    match value {
        0..=249 => SummaryUncertaintyClass::Low,
        250..=599 => SummaryUncertaintyClass::Medium,
        _ => SummaryUncertaintyClass::High,
    }
}
