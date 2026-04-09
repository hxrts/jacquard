//! `FieldSummary` encoding and evidence composition for the field engine.
//!
//! `FieldSummary` is a 64-byte fixed-size record carrying delivery support,
//! congestion penalty, retention support, uncertainty penalty, hop band,
//! topology epoch, and evidence and uncertainty classes. Summaries are
//! exchanged between neighbors as compact reachability advertisements.
//! `decay_summary` ages a summary by tick delta, reducing its delivery support
//! and increasing uncertainty. `compose_summary_with_link` builds a direct
//! observation from a link state entry. `merge_neighbor_summaries` takes the
//! element-wise better of two summaries. `discount_reflected_evidence` reduces
//! support on summaries that originated locally. `clamp_corridor_envelope` and
//! `project_posterior_to_claim` convert fused summaries into corridor belief
//! envelopes bounded by the posterior delivery support.

#![expect(
    dead_code,
    reason = "phase-4 summary/evidence contracts are integrated across later phases"
)]

use jacquard_core::{
    DegradationReason, DestinationId, GatewayId, Link, LinkRuntimeState, NodeId,
    RouteDegradation, RouteEpoch, Tick,
};

use crate::state::{
    ControlState, CorridorBeliefEnvelope, DestinationPosterior, DivergenceBucket,
    EntropyBucket, HopBand, OperatingRegime, SupportBucket,
};

pub(crate) const FIELD_SUMMARY_ENCODING_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EvidenceContributionClass {
    Direct,
    ForwardPropagated,
    ReverseFeedback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SummaryUncertaintyClass {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SummaryDestinationKey {
    Node(NodeId),
    Gateway(GatewayId),
    Service([u8; 32]),
}

impl From<&DestinationId> for SummaryDestinationKey {
    fn from(value: &DestinationId) -> Self {
        match value {
            | DestinationId::Node(id) => Self::Node(*id),
            | DestinationId::Gateway(id) => Self::Gateway(*id),
            | DestinationId::Service(id) => {
                let mut bytes = [0_u8; 32];
                let copy_len = id.0.len().min(bytes.len());
                bytes[..copy_len].copy_from_slice(&id.0[..copy_len]);
                Self::Service(bytes)
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldSummary {
    pub(crate) destination: SummaryDestinationKey,
    pub(crate) topology_epoch: RouteEpoch,
    pub(crate) freshness_tick: Tick,
    pub(crate) hop_band: HopBand,
    pub(crate) delivery_support: SupportBucket,
    pub(crate) congestion_penalty: EntropyBucket,
    pub(crate) retention_support: SupportBucket,
    pub(crate) uncertainty_penalty: EntropyBucket,
    pub(crate) evidence_class: EvidenceContributionClass,
    pub(crate) uncertainty_class: SummaryUncertaintyClass,
}

impl FieldSummary {
    #[must_use]
    pub(crate) fn encode(&self) -> [u8; FIELD_SUMMARY_ENCODING_BYTES] {
        let mut bytes = [0_u8; FIELD_SUMMARY_ENCODING_BYTES];
        match self.destination {
            | SummaryDestinationKey::Node(id) => {
                bytes[0] = 0;
                bytes[1..33].copy_from_slice(&id.0);
            },
            | SummaryDestinationKey::Gateway(id) => {
                bytes[0] = 1;
                bytes[1..17].copy_from_slice(&id.0);
            },
            | SummaryDestinationKey::Service(id) => {
                bytes[0] = 2;
                bytes[1..33].copy_from_slice(&id);
            },
        }
        bytes[33..41].copy_from_slice(&self.freshness_tick.0.to_le_bytes());
        bytes[41..49].copy_from_slice(&self.topology_epoch.0.to_le_bytes());
        bytes[49] = self.hop_band.min_hops;
        bytes[50] = self.hop_band.max_hops;
        bytes[51..53].copy_from_slice(&self.delivery_support.value().to_le_bytes());
        bytes[53..55].copy_from_slice(&self.congestion_penalty.value().to_le_bytes());
        bytes[55..57].copy_from_slice(&self.retention_support.value().to_le_bytes());
        bytes[57..59].copy_from_slice(&self.uncertainty_penalty.value().to_le_bytes());
        bytes[59] = evidence_code(self.evidence_class);
        bytes[60] = uncertainty_code(self.uncertainty_class);
        bytes
    }

    pub(crate) fn decode(
        bytes: [u8; FIELD_SUMMARY_ENCODING_BYTES],
    ) -> Result<Self, &'static str> {
        let destination = match bytes[0] {
            | 0 => {
                let mut id = [0_u8; 32];
                id.copy_from_slice(&bytes[1..33]);
                SummaryDestinationKey::Node(NodeId(id))
            },
            | 1 => {
                let mut id = [0_u8; 16];
                id.copy_from_slice(&bytes[1..17]);
                SummaryDestinationKey::Gateway(GatewayId(id))
            },
            | 2 => {
                let mut id = [0_u8; 32];
                id.copy_from_slice(&bytes[1..33]);
                SummaryDestinationKey::Service(id)
            },
            | _ => return Err("unknown destination key"),
        };
        let freshness_tick = Tick(u64::from_le_bytes(
            bytes[33..41].try_into().expect("freshness bytes"),
        ));
        let topology_epoch = RouteEpoch(u64::from_le_bytes(
            bytes[41..49].try_into().expect("epoch bytes"),
        ));
        Ok(Self {
            destination,
            topology_epoch,
            freshness_tick,
            hop_band: HopBand::new(bytes[49], bytes[50]),
            delivery_support: SupportBucket::new(u16::from_le_bytes(
                bytes[51..53].try_into().expect("support bytes"),
            )),
            congestion_penalty: EntropyBucket::new(u16::from_le_bytes(
                bytes[53..55].try_into().expect("congestion bytes"),
            )),
            retention_support: SupportBucket::new(u16::from_le_bytes(
                bytes[55..57].try_into().expect("retention bytes"),
            )),
            uncertainty_penalty: EntropyBucket::new(u16::from_le_bytes(
                bytes[57..59].try_into().expect("uncertainty bytes"),
            )),
            evidence_class: evidence_from_code(bytes[59])?,
            uncertainty_class: uncertainty_from_code(bytes[60])?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FieldEvidence {
    Direct(DirectEvidence),
    Forward(ForwardPropagatedEvidence),
    Reverse(ReverseFeedbackEvidence),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectEvidence {
    pub(crate) neighbor_id: NodeId,
    pub(crate) link: Link,
    pub(crate) observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForwardPropagatedEvidence {
    pub(crate) from_neighbor: NodeId,
    pub(crate) summary: FieldSummary,
    pub(crate) observed_at_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReverseFeedbackEvidence {
    pub(crate) from_neighbor: NodeId,
    pub(crate) delivery_feedback: SupportBucket,
    pub(crate) observed_at_tick: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalOriginTrace {
    pub(crate) local_node_id: NodeId,
    pub(crate) topology_epoch: RouteEpoch,
}

#[must_use]
pub(crate) fn evidence_classification(
    evidence: &FieldEvidence,
) -> EvidenceContributionClass {
    match evidence {
        | FieldEvidence::Direct(_) => EvidenceContributionClass::Direct,
        | FieldEvidence::Forward(_) => EvidenceContributionClass::ForwardPropagated,
        | FieldEvidence::Reverse(_) => EvidenceContributionClass::ReverseFeedback,
    }
}

#[must_use]
pub(crate) fn decay_summary(summary: &FieldSummary, now_tick: Tick) -> FieldSummary {
    let age = now_tick.0.saturating_sub(summary.freshness_tick.0);
    let age_u16 = u16::try_from(age).unwrap_or(u16::MAX);
    let weakened_support = summary
        .delivery_support
        .value()
        .saturating_sub(age_u16.min(200));
    let raised_uncertainty = summary
        .uncertainty_penalty
        .value()
        .saturating_add(age_u16.min(200));
    let hop_penalty = u8::try_from((age / 16).min(2)).unwrap_or(2);
    FieldSummary {
        destination: summary.destination,
        topology_epoch: summary.topology_epoch,
        freshness_tick: summary.freshness_tick,
        hop_band: HopBand::new(
            summary.hop_band.min_hops,
            summary.hop_band.max_hops.saturating_add(hop_penalty),
        ),
        delivery_support: SupportBucket::new(weakened_support),
        congestion_penalty: summary.congestion_penalty,
        retention_support: summary.retention_support,
        uncertainty_penalty: EntropyBucket::new(raised_uncertainty),
        evidence_class: summary.evidence_class,
        uncertainty_class: uncertainty_class_for(raised_uncertainty),
    }
}

#[must_use]
pub(crate) fn compose_summary_with_link(
    summary: &FieldSummary,
    direct_link: &Link,
) -> FieldSummary {
    let link_support = link_support_bucket(direct_link);
    let loss_penalty = EntropyBucket::new(direct_link.state.loss_permille.0);
    let bootstraps_from_direct = summary.delivery_support.value() == 0;
    FieldSummary {
        destination: summary.destination,
        topology_epoch: summary.topology_epoch,
        freshness_tick: summary.freshness_tick,
        hop_band: if bootstraps_from_direct {
            HopBand::new(1, 1)
        } else {
            HopBand::new(
                summary.hop_band.min_hops.saturating_add(1),
                summary.hop_band.max_hops.saturating_add(1),
            )
        },
        delivery_support: if bootstraps_from_direct {
            link_support
        } else {
            SupportBucket::new(
                summary.delivery_support.value().min(link_support.value()),
            )
        },
        congestion_penalty: EntropyBucket::new(
            summary
                .congestion_penalty
                .value()
                .saturating_add(loss_penalty.value())
                .min(1000),
        ),
        retention_support: summary.retention_support,
        uncertainty_penalty: summary.uncertainty_penalty,
        evidence_class: EvidenceContributionClass::Direct,
        uncertainty_class: summary.uncertainty_class,
    }
}

#[must_use]
pub(crate) fn merge_neighbor_summaries(
    left: &FieldSummary,
    right: &FieldSummary,
) -> FieldSummary {
    let preferred = summary_preference(left).cmp(&summary_preference(right));
    let (best, other) = if preferred.is_gt() || preferred.is_eq() {
        (left, right)
    } else {
        (right, left)
    };
    FieldSummary {
        destination: best.destination,
        topology_epoch: if best.topology_epoch >= other.topology_epoch {
            best.topology_epoch
        } else {
            other.topology_epoch
        },
        freshness_tick: if best.freshness_tick >= other.freshness_tick {
            best.freshness_tick
        } else {
            other.freshness_tick
        },
        hop_band: HopBand::new(
            best.hop_band.min_hops.min(other.hop_band.min_hops),
            best.hop_band.max_hops.max(other.hop_band.max_hops),
        ),
        delivery_support: SupportBucket::new(
            best.delivery_support
                .value()
                .max(other.delivery_support.value()),
        ),
        congestion_penalty: EntropyBucket::new(
            best.congestion_penalty
                .value()
                .max(other.congestion_penalty.value()),
        ),
        retention_support: SupportBucket::new(
            best.retention_support
                .value()
                .max(other.retention_support.value()),
        ),
        uncertainty_penalty: EntropyBucket::new(
            best.uncertainty_penalty
                .value()
                .max(other.uncertainty_penalty.value()),
        ),
        evidence_class: best.evidence_class,
        uncertainty_class: best.uncertainty_class.max(other.uncertainty_class),
    }
}

#[must_use]
pub(crate) fn discount_reflected_evidence(
    summary: &FieldSummary,
    local_origin_trace: LocalOriginTrace,
) -> FieldSummary {
    let reflected = matches!(
        summary.destination,
        SummaryDestinationKey::Node(node_id) if node_id == local_origin_trace.local_node_id
    ) || summary.topology_epoch == local_origin_trace.topology_epoch;
    if !reflected {
        return summary.clone();
    }
    FieldSummary {
        destination: summary.destination,
        topology_epoch: summary.topology_epoch,
        freshness_tick: summary.freshness_tick,
        hop_band: summary.hop_band,
        delivery_support: SupportBucket::new(summary.delivery_support.value() / 2),
        congestion_penalty: EntropyBucket::new(
            summary.uncertainty_penalty.value().saturating_add(150),
        ),
        retention_support: summary.retention_support,
        uncertainty_penalty: EntropyBucket::new(
            summary.uncertainty_penalty.value().saturating_add(200),
        ),
        evidence_class: summary.evidence_class,
        uncertainty_class: SummaryUncertaintyClass::High,
    }
}

#[must_use]
pub(crate) fn clamp_corridor_envelope(
    summary: &FieldSummary,
    regime: OperatingRegime,
    control_state: &ControlState,
) -> CorridorBeliefEnvelope {
    let regime_penalty = match regime {
        | OperatingRegime::Sparse => 50,
        | OperatingRegime::Congested => 200,
        | OperatingRegime::RetentionFavorable => 100,
        | OperatingRegime::Unstable => 250,
        | OperatingRegime::Adversarial => 300,
    };
    let congestion = summary
        .congestion_penalty
        .value()
        .saturating_add(control_state.congestion_price.value())
        .saturating_add(regime_penalty)
        .min(1000);
    CorridorBeliefEnvelope {
        expected_hop_band: summary.hop_band,
        delivery_support: SupportBucket::new(
            summary
                .delivery_support
                .value()
                .saturating_sub(regime_penalty.min(summary.delivery_support.value())),
        ),
        congestion_penalty: EntropyBucket::new(congestion),
        retention_affinity: SupportBucket::new(
            summary
                .retention_support
                .value()
                .saturating_sub(control_state.retention_price.value() / 2),
        ),
        validity_window: jacquard_core::TimeWindow::new(
            summary.freshness_tick,
            Tick(summary.freshness_tick.0.saturating_add(4)),
        )
        .expect("field summary validity"),
    }
}

#[must_use]
pub(crate) fn derive_degradation_class(
    summary: &FieldSummary,
    regime: OperatingRegime,
    control_state: &ControlState,
) -> RouteDegradation {
    let total_penalty = summary
        .congestion_penalty
        .value()
        .saturating_add(summary.uncertainty_penalty.value())
        .saturating_add(control_state.risk_price.value());
    if total_penalty < 300 {
        return RouteDegradation::None;
    }
    let reason = match regime {
        | OperatingRegime::Sparse => DegradationReason::SparseTopology,
        | OperatingRegime::Congested => DegradationReason::CapacityPressure,
        | OperatingRegime::RetentionFavorable => DegradationReason::PolicyPreference,
        | OperatingRegime::Unstable => DegradationReason::LinkInstability,
        | OperatingRegime::Adversarial => DegradationReason::PartitionRisk,
    };
    RouteDegradation::Degraded(reason)
}

#[must_use]
pub(crate) fn project_posterior_to_claim(
    posterior: &DestinationPosterior,
    corridor_envelope: &CorridorBeliefEnvelope,
) -> CorridorBeliefEnvelope {
    let support_cap = posterior.top_corridor_mass.value();
    let delivery_value = corridor_envelope.delivery_support.value().min(support_cap);
    let congestion_value = corridor_envelope
        .congestion_penalty
        .value()
        .max(posterior.usability_entropy.value());
    CorridorBeliefEnvelope {
        expected_hop_band: corridor_envelope.expected_hop_band,
        delivery_support: SupportBucket::new(delivery_value),
        congestion_penalty: EntropyBucket::new(congestion_value),
        retention_affinity: corridor_envelope.retention_affinity,
        validity_window: corridor_envelope.validity_window,
    }
}

#[must_use]
pub(crate) fn summary_divergence(
    predicted: &FieldSummary,
    observed: &FieldSummary,
) -> DivergenceBucket {
    let hop_gap = u16::from(
        predicted
            .hop_band
            .max_hops
            .abs_diff(observed.hop_band.max_hops),
    );
    let support_gap = predicted
        .delivery_support
        .value()
        .abs_diff(observed.delivery_support.value());
    DivergenceBucket::new(hop_gap.saturating_mul(100).saturating_add(support_gap))
}

fn link_support_bucket(link: &Link) -> SupportBucket {
    let state_floor = match link.state.state {
        | LinkRuntimeState::Active => 900_u16,
        | LinkRuntimeState::Degraded => 650_u16,
        | LinkRuntimeState::Suspended => 250_u16,
        | LinkRuntimeState::Faulted => 0_u16,
    };
    let confidence = link
        .state
        .delivery_confidence_permille
        .value()
        .map(|value| value.0)
        .unwrap_or(state_floor);
    SupportBucket::new(state_floor.min(confidence))
}

fn summary_preference(summary: &FieldSummary) -> (u16, u16, u8, Tick) {
    (
        summary.delivery_support.value(),
        1000_u16.saturating_sub(summary.uncertainty_penalty.value()),
        summary.hop_band.max_hops,
        summary.freshness_tick,
    )
}

fn evidence_code(value: EvidenceContributionClass) -> u8 {
    match value {
        | EvidenceContributionClass::Direct => 0,
        | EvidenceContributionClass::ForwardPropagated => 1,
        | EvidenceContributionClass::ReverseFeedback => 2,
    }
}

fn evidence_from_code(value: u8) -> Result<EvidenceContributionClass, &'static str> {
    match value {
        | 0 => Ok(EvidenceContributionClass::Direct),
        | 1 => Ok(EvidenceContributionClass::ForwardPropagated),
        | 2 => Ok(EvidenceContributionClass::ReverseFeedback),
        | _ => Err("unknown evidence class"),
    }
}

fn uncertainty_code(value: SummaryUncertaintyClass) -> u8 {
    match value {
        | SummaryUncertaintyClass::Low => 0,
        | SummaryUncertaintyClass::Medium => 1,
        | SummaryUncertaintyClass::High => 2,
    }
}

fn uncertainty_from_code(value: u8) -> Result<SummaryUncertaintyClass, &'static str> {
    match value {
        | 0 => Ok(SummaryUncertaintyClass::Low),
        | 1 => Ok(SummaryUncertaintyClass::Medium),
        | 2 => Ok(SummaryUncertaintyClass::High),
        | _ => Err("unknown uncertainty class"),
    }
}

fn uncertainty_class_for(value: u16) -> SummaryUncertaintyClass {
    match value {
        | 0..=249 => SummaryUncertaintyClass::Low,
        | 250..=599 => SummaryUncertaintyClass::Medium,
        | _ => SummaryUncertaintyClass::High,
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        Belief, ByteCount, DestinationId, DurationMs, EndpointLocator, LinkEndpoint,
        LinkProfile, LinkState, PartitionRecoveryClass, RatioPermille,
        RepairCapability, TransportKind,
    };

    use super::*;
    use crate::state::SupportBucket;

    fn node(byte: u8) -> NodeId {
        NodeId([byte; 32])
    }

    fn summary(destination: &DestinationId) -> FieldSummary {
        FieldSummary {
            destination: SummaryDestinationKey::from(destination),
            topology_epoch: RouteEpoch(2),
            freshness_tick: Tick(10),
            hop_band: HopBand::new(1, 3),
            delivery_support: SupportBucket::new(800),
            congestion_penalty: EntropyBucket::new(100),
            retention_support: SupportBucket::new(200),
            uncertainty_penalty: EntropyBucket::new(150),
            evidence_class: EvidenceContributionClass::ForwardPropagated,
            uncertainty_class: SummaryUncertaintyClass::Medium,
        }
    }

    fn link(confidence: u16, loss: u16) -> Link {
        Link {
            endpoint: LinkEndpoint {
                transport_kind: TransportKind::WifiAware,
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
                loss_permille: RatioPermille(loss),
                delivery_confidence_permille: Belief::certain(
                    RatioPermille(confidence),
                    Tick(10),
                ),
                symmetry_permille: Belief::Absent,
            },
        }
    }

    #[test]
    fn summary_encoding_is_fixed_width_and_round_trips() {
        let encoded = summary(&DestinationId::Node(node(9))).encode();
        assert_eq!(encoded.len(), FIELD_SUMMARY_ENCODING_BYTES);
        let decoded = FieldSummary::decode(encoded).expect("decode summary");
        assert_eq!(decoded, summary(&DestinationId::Node(node(9))));
    }

    #[test]
    fn reflection_discounting_reduces_support_and_raises_uncertainty() {
        let reflected = discount_reflected_evidence(
            &summary(&DestinationId::Node(node(1))),
            LocalOriginTrace {
                local_node_id: node(1),
                topology_epoch: RouteEpoch(2),
            },
        );
        assert!(reflected.delivery_support.value() < 800);
        assert!(reflected.uncertainty_penalty.value() > 150);
    }

    #[test]
    fn direct_composition_has_priority_over_forward_only_support() {
        let composed = compose_summary_with_link(
            &summary(&DestinationId::Node(node(3))),
            &link(650, 50),
        );
        assert_eq!(composed.evidence_class, EvidenceContributionClass::Direct);
        assert_eq!(composed.delivery_support.value(), 650);
    }

    #[test]
    fn direct_composition_bootstraps_unknown_summary_from_link_evidence() {
        let empty = FieldSummary {
            delivery_support: SupportBucket::new(0),
            hop_band: HopBand::new(1, jacquard_core::ROUTE_HOP_COUNT_MAX),
            ..summary(&DestinationId::Node(node(3)))
        };
        let composed = compose_summary_with_link(&empty, &link(900, 50));
        assert_eq!(composed.delivery_support.value(), 900);
        assert_eq!(composed.hop_band, HopBand::new(1, 1));
    }

    #[test]
    fn absent_reverse_feedback_is_not_treated_as_negative_proof() {
        let baseline = summary(&DestinationId::Node(node(4)));
        let decayed = decay_summary(&baseline, Tick(12));
        assert!(decayed.delivery_support.value() > 0);
        assert!(
            decayed.uncertainty_penalty.value() >= baseline.uncertainty_penalty.value()
        );
    }

    #[test]
    fn merge_results_are_deterministic_for_same_inputs() {
        let left = summary(&DestinationId::Node(node(7)));
        let right = FieldSummary {
            freshness_tick: Tick(9),
            delivery_support: SupportBucket::new(700),
            ..summary(&DestinationId::Node(node(7)))
        };
        assert_eq!(
            merge_neighbor_summaries(&left, &right),
            merge_neighbor_summaries(&left, &right)
        );
    }
}
