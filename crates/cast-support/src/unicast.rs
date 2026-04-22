use alloc::{collections::BTreeMap, vec::Vec};
use core::cmp::Reverse;

use jacquard_core::{ByteCount, NodeId, RatioPermille};
use serde::{Deserialize, Serialize};

use crate::{
    common::{meets_confidence, supports_payload, CastEvidenceReport},
    CastEvidenceMeta, CastEvidencePolicy,
};

// proc-macro-scope: Unicast profile evidence shaping is plain helper logic.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicastObservation {
    pub from: NodeId,
    pub to: NodeId,
    pub directional_confidence_permille: RatioPermille,
    pub reverse_confirmation_permille: Option<RatioPermille>,
    pub payload_bytes_max: ByteCount,
    pub meta: CastEvidenceMeta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UnicastSupportKind {
    DirectionalOnly,
    BidirectionalConfirmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicastEvidence {
    pub from: NodeId,
    pub to: NodeId,
    pub support: UnicastSupportKind,
    pub directional_confidence_permille: RatioPermille,
    pub reverse_confirmation_permille: Option<RatioPermille>,
    pub bidirectional_confidence_permille: RatioPermille,
    pub payload_bytes_max: ByteCount,
    pub meta: CastEvidenceMeta,
}

#[must_use]
pub fn shape_unicast_evidence(
    observations: impl IntoIterator<Item = UnicastObservation>,
    policy: CastEvidencePolicy,
) -> (Vec<UnicastEvidence>, CastEvidenceReport) {
    let mut report = CastEvidenceReport::default();
    let mut by_link = BTreeMap::new();
    for observation in observations {
        let Some(evidence) = shape_one_unicast(observation, policy, &mut report) else {
            continue;
        };
        by_link
            .entry((evidence.from, evidence.to))
            .and_modify(|current| {
                if unicast_rank(evidence) > unicast_rank(*current) {
                    *current = evidence;
                }
            })
            .or_insert(evidence);
    }
    let mut evidence = by_link.into_values().collect::<Vec<_>>();
    evidence.sort_by_key(|item| Reverse(unicast_rank(*item)));
    prune(&mut evidence, policy.bounds.receiver_count_max, &mut report);
    (evidence, report)
}

fn shape_one_unicast(
    observation: UnicastObservation,
    policy: CastEvidencePolicy,
    report: &mut CastEvidenceReport,
) -> Option<UnicastEvidence> {
    if !observation.meta.is_fresh_for(policy.bounds) {
        report.record_stale();
        return None;
    }
    if !meets_confidence(observation.directional_confidence_permille, policy) {
        report.record_low_confidence();
        return None;
    }
    if !supports_payload(observation.payload_bytes_max, policy) {
        report.record_capacity();
        return None;
    }
    Some(UnicastEvidence::from_observation(observation))
}

impl UnicastEvidence {
    #[must_use]
    fn from_observation(observation: UnicastObservation) -> Self {
        let bidirectional = observation
            .reverse_confirmation_permille
            .map(|reverse| {
                RatioPermille(reverse.0.min(observation.directional_confidence_permille.0))
            })
            .unwrap_or(RatioPermille(0));
        Self {
            from: observation.from,
            to: observation.to,
            support: support_kind(bidirectional),
            directional_confidence_permille: observation.directional_confidence_permille,
            reverse_confirmation_permille: observation.reverse_confirmation_permille,
            bidirectional_confidence_permille: bidirectional,
            payload_bytes_max: observation.payload_bytes_max,
            meta: observation.meta,
        }
    }
}

fn support_kind(bidirectional: RatioPermille) -> UnicastSupportKind {
    if bidirectional.0 > 0 {
        UnicastSupportKind::BidirectionalConfirmed
    } else {
        UnicastSupportKind::DirectionalOnly
    }
}

fn prune(evidence: &mut Vec<UnicastEvidence>, cap: u32, report: &mut CastEvidenceReport) {
    let cap = usize::try_from(cap).unwrap_or(usize::MAX);
    if evidence.len() <= cap {
        return;
    }
    let omitted = evidence.len().saturating_sub(cap);
    evidence.truncate(cap);
    for _ in 0..omitted {
        report.record_bound();
    }
}

fn unicast_rank(
    evidence: UnicastEvidence,
) -> (
    u8,
    RatioPermille,
    RatioPermille,
    ByteCount,
    jacquard_core::Tick,
    jacquard_core::OrderStamp,
    NodeId,
    NodeId,
) {
    (
        support_rank(evidence.support),
        evidence.bidirectional_confidence_permille,
        evidence.directional_confidence_permille,
        evidence.payload_bytes_max,
        evidence.meta.observed_at_tick,
        evidence.meta.order,
        evidence.from,
        evidence.to,
    )
}

fn support_rank(support: UnicastSupportKind) -> u8 {
    match support {
        UnicastSupportKind::BidirectionalConfirmed => 2,
        UnicastSupportKind::DirectionalOnly => 1,
    }
}
