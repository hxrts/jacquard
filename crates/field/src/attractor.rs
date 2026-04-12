//! Attractor scoring and neighbor ranking for the field routing engine.
//!
//! For each destination, scores every candidate neighbor continuation using
//! field dynamics. The base score combines net delivery support, downstream
//! support, corridor mass, and field strength. Regime-specific bonuses reward
//! sparse relay alignment, congestion relief, retention, and risk avoidance.
//! Posture-specific bonuses cover opportunistic, structured, retention-biased,
//! and risk-suppressed routing modes, with control penalties applied for
//! congestion, risk, and entropy.
//!
//! `rank_frontier_by_attractor` sorts frontier entries by score.
//! `derive_local_attractor_view` builds a `LocalAttractorView` capturing the
//! leading continuation and coherence margin (gap between top two candidates)
//! for each destination. The coherence margin feeds the control plane as a
//! signal of field stability and informs route replacement decisions in the
//! runtime.

use jacquard_core::NodeId;

use crate::state::{
    ControlState, DestinationFieldState, DestinationKey, FieldEngineState, MeanFieldState,
    NeighborContinuation, OperatingRegime, RoutingPosture, SupportBucket,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalAttractorEntry {
    pub(crate) destination: DestinationKey,
    pub(crate) leading_neighbor: NodeId,
    pub(crate) attractor_score: SupportBucket,
    pub(crate) coherence_margin: SupportBucket,
    pub(crate) regime: OperatingRegime,
    pub(crate) posture: RoutingPosture,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LocalAttractorView {
    pub(crate) entries: Vec<LocalAttractorEntry>,
    pub(crate) coherence_score: SupportBucket,
}

#[must_use]
pub(crate) fn rank_frontier_by_attractor(
    destination_state: &DestinationFieldState,
    mean_field: &MeanFieldState,
    regime: OperatingRegime,
    posture: RoutingPosture,
    control: &ControlState,
) -> Vec<(NeighborContinuation, SupportBucket)> {
    let mut ranked = destination_state
        .frontier
        .as_slice()
        .iter()
        .cloned()
        .map(|continuation| {
            let score = attractor_score_for(
                &continuation,
                destination_state,
                mean_field,
                regime,
                posture,
                control,
            );
            (continuation, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.neighbor_id.cmp(&right.0.neighbor_id))
    });
    ranked
}

#[must_use]
pub(crate) fn derive_local_attractor_view(state: &FieldEngineState) -> LocalAttractorView {
    let mut entries = Vec::new();
    for (destination, destination_state) in &state.destinations {
        let ranked = rank_frontier_by_attractor(
            destination_state,
            &state.mean_field,
            state.regime.current,
            state.posture.current,
            &state.controller,
        );
        let Some((leading, primary_score)) = ranked.first() else {
            continue;
        };
        let secondary_score = ranked.get(1).map(|(_, score)| score.value()).unwrap_or(0);
        entries.push(LocalAttractorEntry {
            destination: destination.clone(),
            leading_neighbor: leading.neighbor_id,
            attractor_score: *primary_score,
            coherence_margin: SupportBucket::new(
                primary_score.value().saturating_sub(secondary_score),
            ),
            regime: state.regime.current,
            posture: state.posture.current,
        });
    }
    let coherence_score = if entries.is_empty() {
        SupportBucket::default()
    } else {
        SupportBucket::new(
            u16::try_from(
                entries
                    .iter()
                    .map(|entry| u32::from(entry.coherence_margin.value()))
                    .sum::<u32>()
                    / u32::try_from(entries.len()).expect("entry count fits u32"),
            )
            .expect("coherence average fits u16"),
        )
    };
    LocalAttractorView {
        entries,
        coherence_score,
    }
}

fn attractor_score_for(
    continuation: &NeighborContinuation,
    destination_state: &DestinationFieldState,
    mean_field: &MeanFieldState,
    regime: OperatingRegime,
    posture: RoutingPosture,
    control: &ControlState,
) -> SupportBucket {
    let base = average_u16(&[
        continuation.net_value.value(),
        continuation.net_value.value(),
        continuation.downstream_support.value(),
        destination_state.posterior.top_corridor_mass.value(),
        mean_field.field_strength.value(),
    ]);
    let regime_bonus = match regime {
        OperatingRegime::Sparse => mean_field.relay_alignment.value() / 4,
        OperatingRegime::Congested => mean_field.congestion_alignment.value() / 3,
        OperatingRegime::RetentionFavorable => {
            destination_state.corridor_belief.retention_affinity.value() / 2
        }
        OperatingRegime::Unstable | OperatingRegime::Adversarial => {
            mean_field.risk_alignment.value() / 3
        }
    };
    let posture_bonus = match posture {
        RoutingPosture::Opportunistic => 1000_u16.saturating_sub(control.risk_price.value()) / 4,
        RoutingPosture::Structured => mean_field.relay_alignment.value() / 4,
        RoutingPosture::RetentionBiased => {
            destination_state.corridor_belief.retention_affinity.value() / 3
        }
        RoutingPosture::RiskSuppressed => mean_field.risk_alignment.value() / 3,
    };
    let penalty = average_u16(&[
        control.congestion_price.value(),
        control.risk_price.value(),
        destination_state.posterior.usability_entropy.value(),
    ]);
    SupportBucket::new(
        base.saturating_add(regime_bonus)
            .saturating_add(posture_bonus)
            .saturating_sub(penalty / 3),
    )
}

fn average_u16(values: &[u16]) -> u16 {
    let sum: u32 = values.iter().map(|value| u32::from(*value)).sum();
    let len = u32::try_from(values.len()).expect("slice length fits u32");
    u16::try_from(sum / len).expect("average fits u16")
}

#[cfg(test)]
mod tests {
    use jacquard_core::{DestinationId, NodeId, Tick};

    use super::*;
    use crate::state::{
        DestinationFieldState, DestinationKey, HopBand, ObservationClass, SupportBucket,
    };

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn destination_state() -> DestinationFieldState {
        let mut state = DestinationFieldState::new(
            DestinationKey::from(&DestinationId::Node(node(9))),
            Tick(1),
        );
        state.posterior.top_corridor_mass = SupportBucket::new(850);
        state.posterior.predicted_observation_class = ObservationClass::DirectOnly;
        state.corridor_belief.retention_affinity = SupportBucket::new(400);
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(1),
            net_value: SupportBucket::new(900),
            downstream_support: SupportBucket::new(900),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(2),
        });
        state.frontier = state.frontier.clone().insert(NeighborContinuation {
            neighbor_id: node(2),
            net_value: SupportBucket::new(150),
            downstream_support: SupportBucket::new(200),
            expected_hop_band: HopBand::new(1, 2),
            freshness: Tick(2),
        });
        state
    }

    #[test]
    fn attractor_prefers_stronger_continuation() {
        let ranked = rank_frontier_by_attractor(
            &destination_state(),
            &MeanFieldState {
                relay_alignment: SupportBucket::new(800),
                congestion_alignment: SupportBucket::new(800),
                retention_alignment: SupportBucket::new(700),
                risk_alignment: SupportBucket::new(700),
                field_strength: SupportBucket::new(850),
            },
            OperatingRegime::Sparse,
            RoutingPosture::Structured,
            &ControlState::default(),
        );
        assert_eq!(ranked[0].0.neighbor_id, node(1));
        assert!(ranked[0].1 > ranked[1].1);
    }

    #[test]
    fn local_attractor_view_reports_coherence_margin() {
        let mut engine_state = FieldEngineState::new();
        engine_state.mean_field = MeanFieldState {
            relay_alignment: SupportBucket::new(800),
            congestion_alignment: SupportBucket::new(800),
            retention_alignment: SupportBucket::new(700),
            risk_alignment: SupportBucket::new(700),
            field_strength: SupportBucket::new(850),
        };
        engine_state.destinations.insert(
            DestinationKey::from(&DestinationId::Node(node(9))),
            destination_state(),
        );
        let view = derive_local_attractor_view(&engine_state);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].leading_neighbor, node(1));
        assert!(view.coherence_score.value() > 0);
    }
}
