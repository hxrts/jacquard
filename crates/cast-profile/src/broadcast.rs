use std::collections::BTreeMap;

use jacquard_core::{ByteCount, NodeId, RatioPermille};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        bounded_len, meets_confidence, permille_product, supports_payload, CastEvidenceReport,
    },
    CastEvidenceMeta, CastEvidencePolicy, ReceiverCoverageEvidence, ReceiverCoverageObservation,
};

// proc-macro-scope: Broadcast profile evidence shaping is plain helper logic.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BroadcastReverseConfirmation {
    Unavailable,
    GatewayAssisted(RatioPermille),
    Direct(RatioPermille),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BroadcastSupportKind {
    DirectionalOnly,
    GatewayAssistedReverse,
    DirectReverseConfirmed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastObservation {
    pub sender: NodeId,
    pub receivers: Vec<ReceiverCoverageObservation>,
    pub reverse_confirmation: BroadcastReverseConfirmation,
    pub transmission_window_quality_permille: RatioPermille,
    pub channel_pressure_permille: RatioPermille,
    pub copy_budget: u32,
    pub payload_bytes_max: ByteCount,
    pub meta: CastEvidenceMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastEvidence {
    pub sender: NodeId,
    pub receivers: Vec<ReceiverCoverageEvidence>,
    pub support: BroadcastSupportKind,
    pub coverage_confidence_permille: RatioPermille,
    pub reverse_confirmation_permille: Option<RatioPermille>,
    pub transmission_window_quality_permille: RatioPermille,
    pub channel_pressure_permille: RatioPermille,
    pub copy_budget: u32,
    pub payload_bytes_max: ByteCount,
    pub meta: CastEvidenceMeta,
}

#[must_use]
pub fn shape_broadcast_evidence(
    observations: impl IntoIterator<Item = BroadcastObservation>,
    policy: CastEvidencePolicy,
) -> (Vec<BroadcastEvidence>, CastEvidenceReport) {
    let mut report = CastEvidenceReport::default();
    let mut evidence = Vec::new();
    for observation in observations {
        let Some(shaped) = shape_one_broadcast(observation, policy, &mut report) else {
            continue;
        };
        evidence.push(shaped);
    }
    evidence.sort_by_key(|item| std::cmp::Reverse(broadcast_rank(item)));
    (evidence, report)
}

fn shape_one_broadcast(
    mut observation: BroadcastObservation,
    policy: CastEvidencePolicy,
    report: &mut CastEvidenceReport,
) -> Option<BroadcastEvidence> {
    if !broadcast_bounds_hold(&observation, policy, report) {
        return None;
    }
    if !meets_confidence(observation.transmission_window_quality_permille, policy) {
        report.record_low_confidence();
        return None;
    }
    let receivers = eligible_receivers(std::mem::take(&mut observation.receivers), policy);
    if receivers.is_empty() {
        report.record_low_confidence();
        return None;
    }
    Some(BroadcastEvidence::from_observation(&observation, receivers))
}

fn broadcast_bounds_hold(
    observation: &BroadcastObservation,
    policy: CastEvidencePolicy,
    report: &mut CastEvidenceReport,
) -> bool {
    if !observation.meta.is_fresh_for(policy.bounds) {
        report.record_stale();
        return false;
    }
    if !supports_payload(observation.payload_bytes_max, policy) {
        report.record_capacity();
        return false;
    }
    if observation.copy_budget > policy.bounds.copy_budget_max {
        report.record_bound();
        return false;
    }
    if !bounded_len(
        observation.receivers.len(),
        policy.bounds.receiver_count_max,
    ) {
        report.record_bound();
        return false;
    }
    true
}

fn eligible_receivers(
    receivers: Vec<ReceiverCoverageObservation>,
    policy: CastEvidencePolicy,
) -> Vec<ReceiverCoverageEvidence> {
    let mut by_receiver = BTreeMap::new();
    for receiver in receivers {
        if !meets_confidence(receiver.confidence_permille, policy) {
            continue;
        }
        by_receiver
            .entry(receiver.receiver)
            .and_modify(|current: &mut ReceiverCoverageObservation| {
                if receiver.confidence_permille > current.confidence_permille {
                    *current = receiver;
                }
            })
            .or_insert(receiver);
    }
    by_receiver
        .into_values()
        .map(|receiver| ReceiverCoverageEvidence {
            receiver: receiver.receiver,
            confidence_permille: receiver.confidence_permille,
        })
        .collect()
}

impl BroadcastEvidence {
    #[must_use]
    fn from_observation(
        observation: &BroadcastObservation,
        receivers: Vec<ReceiverCoverageEvidence>,
    ) -> Self {
        Self {
            sender: observation.sender,
            coverage_confidence_permille: coverage_confidence(&receivers),
            receivers,
            support: support_kind(observation.reverse_confirmation),
            reverse_confirmation_permille: reverse_confidence(observation.reverse_confirmation),
            transmission_window_quality_permille: observation.transmission_window_quality_permille,
            channel_pressure_permille: observation.channel_pressure_permille,
            copy_budget: observation.copy_budget,
            payload_bytes_max: observation.payload_bytes_max,
            meta: observation.meta,
        }
    }

    #[must_use]
    pub fn connected_bidirectional_confidence(&self) -> RatioPermille {
        match self.support {
            BroadcastSupportKind::DirectReverseConfirmed => self
                .reverse_confirmation_permille
                .unwrap_or(RatioPermille(0)),
            BroadcastSupportKind::DirectionalOnly
            | BroadcastSupportKind::GatewayAssistedReverse => RatioPermille(0),
        }
    }

    #[must_use]
    pub fn custody_improvement_score(&self) -> RatioPermille {
        let useful_window = permille_product(
            self.coverage_confidence_permille,
            self.transmission_window_quality_permille,
        );
        RatioPermille(
            useful_window
                .0
                .saturating_sub(self.channel_pressure_permille.0),
        )
    }
}

fn coverage_confidence(receivers: &[ReceiverCoverageEvidence]) -> RatioPermille {
    receivers
        .iter()
        .fold(RatioPermille(1_000), |confidence, receiver| {
            permille_product(confidence, receiver.confidence_permille)
        })
}

fn support_kind(confirmation: BroadcastReverseConfirmation) -> BroadcastSupportKind {
    match confirmation {
        BroadcastReverseConfirmation::Unavailable => BroadcastSupportKind::DirectionalOnly,
        BroadcastReverseConfirmation::GatewayAssisted(_) => {
            BroadcastSupportKind::GatewayAssistedReverse
        }
        BroadcastReverseConfirmation::Direct(_) => BroadcastSupportKind::DirectReverseConfirmed,
    }
}

fn reverse_confidence(confirmation: BroadcastReverseConfirmation) -> Option<RatioPermille> {
    match confirmation {
        BroadcastReverseConfirmation::Unavailable => None,
        BroadcastReverseConfirmation::GatewayAssisted(confidence)
        | BroadcastReverseConfirmation::Direct(confidence) => Some(confidence),
    }
}

fn broadcast_rank(
    evidence: &BroadcastEvidence,
) -> (
    u8,
    RatioPermille,
    RatioPermille,
    std::cmp::Reverse<RatioPermille>,
    ByteCount,
    jacquard_core::Tick,
    jacquard_core::OrderStamp,
    NodeId,
) {
    (
        support_rank(evidence.support),
        evidence.coverage_confidence_permille,
        evidence.transmission_window_quality_permille,
        std::cmp::Reverse(evidence.channel_pressure_permille),
        evidence.payload_bytes_max,
        evidence.meta.observed_at_tick,
        evidence.meta.order,
        evidence.sender,
    )
}

fn support_rank(support: BroadcastSupportKind) -> u8 {
    match support {
        BroadcastSupportKind::DirectReverseConfirmed => 3,
        BroadcastSupportKind::GatewayAssistedReverse => 2,
        BroadcastSupportKind::DirectionalOnly => 1,
    }
}
