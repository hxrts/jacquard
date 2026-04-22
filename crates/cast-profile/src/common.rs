use jacquard_core::{ByteCount, DurationMs, NodeId, OrderStamp, RatioPermille, Tick};
use serde::{Deserialize, Serialize};

use crate::CastEvidenceBounds;

// proc-macro-scope: Cast evidence helper types stay plain and crate-local.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastEvidenceMeta {
    pub observed_at_tick: Tick,
    pub evidence_age_ms: DurationMs,
    pub valid_for_ms: DurationMs,
    pub order: OrderStamp,
}

impl CastEvidenceMeta {
    #[must_use]
    pub fn new(
        observed_at_tick: Tick,
        evidence_age_ms: DurationMs,
        valid_for_ms: DurationMs,
        order: OrderStamp,
    ) -> Self {
        Self {
            observed_at_tick,
            evidence_age_ms,
            valid_for_ms,
            order,
        }
    }

    #[must_use]
    pub fn is_fresh_for(self, bounds: CastEvidenceBounds) -> bool {
        self.evidence_age_ms <= bounds.evidence_age_ms_max
            && self.evidence_age_ms <= self.valid_for_ms
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastEvidencePolicy {
    pub bounds: CastEvidenceBounds,
    pub confidence_floor: RatioPermille,
    pub payload_bytes_required: ByteCount,
}

impl Default for CastEvidencePolicy {
    fn default() -> Self {
        Self {
            bounds: CastEvidenceBounds::default(),
            confidence_floor: RatioPermille(1),
            payload_bytes_required: ByteCount(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastEvidenceError {
    ReceiverSetTooLarge,
    GroupCoverageTooLarge,
    FanoutLimitExceeded,
    CopyBudgetExceeded,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CastEvidenceReport {
    pub omitted_stale_count: u32,
    pub omitted_low_confidence_count: u32,
    pub omitted_capacity_count: u32,
    pub omitted_bound_count: u32,
}

impl CastEvidenceReport {
    pub(crate) fn record_stale(&mut self) {
        self.omitted_stale_count = self.omitted_stale_count.saturating_add(1);
    }

    pub(crate) fn record_low_confidence(&mut self) {
        self.omitted_low_confidence_count = self.omitted_low_confidence_count.saturating_add(1);
    }

    pub(crate) fn record_capacity(&mut self) {
        self.omitted_capacity_count = self.omitted_capacity_count.saturating_add(1);
    }

    pub(crate) fn record_bound(&mut self) {
        self.omitted_bound_count = self.omitted_bound_count.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiverCoverageObservation {
    pub receiver: NodeId,
    pub confidence_permille: RatioPermille,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiverCoverageEvidence {
    pub receiver: NodeId,
    pub confidence_permille: RatioPermille,
}

#[must_use]
pub(crate) fn bounded_len(count: usize, cap: u32) -> bool {
    let cap = usize::try_from(cap).unwrap_or(usize::MAX);
    count <= cap
}

#[must_use]
pub(crate) fn supports_payload(capacity: ByteCount, policy: CastEvidencePolicy) -> bool {
    capacity >= policy.payload_bytes_required
}

#[must_use]
pub(crate) fn meets_confidence(confidence: RatioPermille, policy: CastEvidencePolicy) -> bool {
    confidence >= policy.confidence_floor
}

#[must_use]
pub(crate) fn permille_product(left: RatioPermille, right: RatioPermille) -> RatioPermille {
    let product = u32::from(left.0).saturating_mul(u32::from(right.0));
    RatioPermille(u16::try_from(product / 1_000).unwrap_or(1_000))
}
