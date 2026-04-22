use std::collections::{BTreeMap, BTreeSet};

use jacquard_core::{ByteCount, MulticastGroupId, NodeId, RatioPermille};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        bounded_len, eligible_receivers, permille_product, supports_payload, CastEvidenceReport,
    },
    CastEvidenceMeta, CastEvidencePolicy, ReceiverCoverageEvidence, ReceiverCoverageObservation,
};

// proc-macro-scope: Multicast profile evidence shaping is plain helper logic.

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CastGroupId(pub MulticastGroupId);

impl CastGroupId {
    #[must_use]
    pub const fn new(group_id: MulticastGroupId) -> Self {
        Self(group_id)
    }

    #[must_use]
    pub const fn to_route_group_id(&self) -> MulticastGroupId {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulticastObservation {
    pub sender: NodeId,
    pub group_id: CastGroupId,
    pub receivers: Vec<ReceiverCoverageObservation>,
    pub group_pressure_permille: RatioPermille,
    pub fanout_limit: u32,
    pub payload_bytes_max: ByteCount,
    pub meta: CastEvidenceMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulticastEvidence {
    pub sender: NodeId,
    pub group_id: CastGroupId,
    pub receivers: Vec<ReceiverCoverageEvidence>,
    pub covered_receiver_count: u32,
    pub partial_delivery_confidence_permille: RatioPermille,
    pub group_pressure_permille: RatioPermille,
    pub fanout_limit: u32,
    pub payload_bytes_max: ByteCount,
    pub meta: CastEvidenceMeta,
}

#[must_use]
pub fn shape_multicast_evidence(
    observations: impl IntoIterator<Item = MulticastObservation>,
    policy: CastEvidencePolicy,
) -> (Vec<MulticastEvidence>, CastEvidenceReport) {
    let mut report = CastEvidenceReport::default();
    let mut by_group = BTreeMap::new();
    for observation in observations {
        let Some(evidence) = shape_one_multicast(observation, policy, &mut report) else {
            continue;
        };
        by_group
            .entry((evidence.sender, evidence.group_id.clone()))
            .and_modify(|current| replace_if_better(current, evidence.clone()))
            .or_insert(evidence);
    }
    let mut evidence = by_group.into_values().collect::<Vec<_>>();
    evidence.sort_by_key(|item| std::cmp::Reverse(multicast_rank(item)));
    (evidence, report)
}

fn shape_one_multicast(
    mut observation: MulticastObservation,
    policy: CastEvidencePolicy,
    report: &mut CastEvidenceReport,
) -> Option<MulticastEvidence> {
    if !multicast_bounds_hold(&observation, policy, report) {
        return None;
    }
    let receivers = eligible_receivers(std::mem::take(&mut observation.receivers), policy);
    if receivers.is_empty() {
        report.record_low_confidence();
        return None;
    }
    Some(MulticastEvidence::from_observation(observation, receivers))
}

fn multicast_bounds_hold(
    observation: &MulticastObservation,
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
    if observation.fanout_limit > policy.bounds.fanout_count_max {
        report.record_bound();
        return false;
    }
    if !bounded_len(
        observation.receivers.len(),
        policy.bounds.group_coverage_count_max,
    ) {
        report.record_bound();
        return false;
    }
    true
}

impl MulticastEvidence {
    #[must_use]
    fn from_observation(
        observation: MulticastObservation,
        receivers: Vec<ReceiverCoverageEvidence>,
    ) -> Self {
        let covered_receiver_count = u32::try_from(receivers.len()).unwrap_or(u32::MAX);
        Self {
            sender: observation.sender,
            group_id: observation.group_id,
            partial_delivery_confidence_permille: partial_delivery_confidence(&receivers),
            receivers,
            covered_receiver_count,
            group_pressure_permille: observation.group_pressure_permille,
            fanout_limit: observation.fanout_limit,
            payload_bytes_max: observation.payload_bytes_max,
            meta: observation.meta,
        }
    }

    #[must_use]
    pub fn can_satisfy_receiver_objective(&self, required: &BTreeSet<NodeId>) -> bool {
        required.iter().all(|node| {
            self.receivers
                .iter()
                .any(|receiver| receiver.receiver == *node)
        })
    }
}

fn partial_delivery_confidence(receivers: &[ReceiverCoverageEvidence]) -> RatioPermille {
    receivers
        .iter()
        .fold(RatioPermille(1_000), |confidence, receiver| {
            permille_product(confidence, receiver.confidence_permille)
        })
}

fn replace_if_better(current: &mut MulticastEvidence, proposed: MulticastEvidence) {
    if multicast_rank(&proposed) > multicast_rank(current) {
        *current = proposed;
    }
}

fn multicast_rank(
    evidence: &MulticastEvidence,
) -> (
    u32,
    RatioPermille,
    std::cmp::Reverse<RatioPermille>,
    ByteCount,
    jacquard_core::Tick,
    jacquard_core::OrderStamp,
    NodeId,
    CastGroupId,
) {
    (
        evidence.covered_receiver_count,
        evidence.partial_delivery_confidence_permille,
        std::cmp::Reverse(evidence.group_pressure_permille),
        evidence.payload_bytes_max,
        evidence.meta.observed_at_tick,
        evidence.meta.order,
        evidence.sender,
        evidence.group_id.clone(),
    )
}
