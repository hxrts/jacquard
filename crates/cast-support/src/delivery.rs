use std::collections::BTreeSet;

use jacquard_core::{BroadcastDomainId, ByteCount, NodeId, RatioPermille};
use serde::{Deserialize, Serialize};

use crate::{
    common::{meets_confidence, permille_product, supports_payload},
    BroadcastEvidence, BroadcastSupportKind, CastEvidenceMeta, CastEvidencePolicy, CastGroupId,
    MulticastEvidence, ReceiverCoverageEvidence, UnicastEvidence, UnicastSupportKind,
};

// proc-macro-scope: Cast delivery support shaping is deterministic helper logic over plain data.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CastDeliveryMode {
    Unicast,
    Multicast,
    Broadcast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CastCoverageObjective {
    AnyReceiver,
    AllReceivers,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastReceiverSet {
    receivers: Vec<NodeId>,
}

impl CastReceiverSet {
    #[must_use]
    pub fn new(receivers: impl IntoIterator<Item = NodeId>) -> Self {
        let receivers = receivers.into_iter().collect::<BTreeSet<_>>();
        Self {
            receivers: receivers.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn one(receiver: NodeId) -> Self {
        Self {
            receivers: vec![receiver],
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[NodeId] {
        &self.receivers
    }

    #[must_use]
    pub fn contains(&self, receiver: NodeId) -> bool {
        self.receivers.binary_search(&receiver).is_ok()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.receivers.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.receivers.len()
    }

    #[must_use]
    pub fn covers_all(&self, receivers: &[ReceiverCoverageEvidence]) -> bool {
        self.receivers.iter().all(|receiver| {
            receivers
                .iter()
                .any(|evidence| evidence.receiver == *receiver)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastDeliveryObjective {
    pub mode: CastDeliveryMode,
    pub sender: NodeId,
    pub receivers: CastReceiverSet,
    pub group_id: Option<CastGroupId>,
    pub broadcast_domain_id: Option<BroadcastDomainId>,
    pub coverage: CastCoverageObjective,
}

impl CastDeliveryObjective {
    #[must_use]
    pub fn unicast(sender: NodeId, receiver: NodeId) -> Self {
        Self {
            mode: CastDeliveryMode::Unicast,
            sender,
            receivers: CastReceiverSet::one(receiver),
            group_id: None,
            broadcast_domain_id: None,
            coverage: CastCoverageObjective::AllReceivers,
        }
    }

    #[must_use]
    pub fn multicast(
        sender: NodeId,
        group_id: CastGroupId,
        receivers: impl IntoIterator<Item = NodeId>,
        coverage: CastCoverageObjective,
    ) -> Self {
        Self {
            mode: CastDeliveryMode::Multicast,
            sender,
            receivers: CastReceiverSet::new(receivers),
            group_id: Some(group_id),
            broadcast_domain_id: None,
            coverage,
        }
    }

    #[must_use]
    pub fn broadcast_in_domain(
        sender: NodeId,
        domain_id: BroadcastDomainId,
        receivers: impl IntoIterator<Item = NodeId>,
        coverage: CastCoverageObjective,
    ) -> Self {
        Self {
            mode: CastDeliveryMode::Broadcast,
            sender,
            receivers: CastReceiverSet::new(receivers),
            group_id: None,
            broadcast_domain_id: Some(domain_id),
            coverage,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastDeliveryPolicy {
    pub evidence: CastEvidencePolicy,
    pub require_bidirectional: bool,
    pub allow_partial_multicast: bool,
    pub allow_gateway_assisted_broadcast: bool,
}

impl Default for CastDeliveryPolicy {
    fn default() -> Self {
        Self {
            evidence: CastEvidencePolicy::default(),
            require_bidirectional: false,
            allow_partial_multicast: true,
            allow_gateway_assisted_broadcast: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastDeliveryResourceUse {
    pub receiver_count: u32,
    pub fanout_used: u32,
    pub copy_budget_used: u32,
    pub payload_bytes: ByteCount,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CastDeliveryReport {
    pub omitted_mode_mismatch_count: u32,
    pub omitted_receiver_mismatch_count: u32,
    pub omitted_low_confidence_count: u32,
    pub omitted_capacity_count: u32,
    pub omitted_fanout_count: u32,
    pub omitted_copy_budget_count: u32,
    pub omitted_reverse_confirmation_count: u32,
}

impl CastDeliveryReport {
    fn record_mode_mismatch(&mut self) {
        self.omitted_mode_mismatch_count = self.omitted_mode_mismatch_count.saturating_add(1);
    }

    fn record_receiver_mismatch(&mut self) {
        self.omitted_receiver_mismatch_count =
            self.omitted_receiver_mismatch_count.saturating_add(1);
    }

    fn record_low_confidence(&mut self) {
        self.omitted_low_confidence_count = self.omitted_low_confidence_count.saturating_add(1);
    }

    fn record_capacity(&mut self) {
        self.omitted_capacity_count = self.omitted_capacity_count.saturating_add(1);
    }

    fn record_fanout(&mut self) {
        self.omitted_fanout_count = self.omitted_fanout_count.saturating_add(1);
    }

    fn record_copy_budget(&mut self) {
        self.omitted_copy_budget_count = self.omitted_copy_budget_count.saturating_add(1);
    }

    fn record_reverse_confirmation(&mut self) {
        self.omitted_reverse_confirmation_count =
            self.omitted_reverse_confirmation_count.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicastDeliverySupport {
    pub sender: NodeId,
    pub receiver: NodeId,
    pub confidence_permille: RatioPermille,
    pub bidirectional_confidence_permille: RatioPermille,
    pub payload_bytes_max: ByteCount,
    pub resource_use: CastDeliveryResourceUse,
    pub meta: CastEvidenceMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulticastDeliverySupport {
    pub sender: NodeId,
    pub group_id: CastGroupId,
    pub receivers: Vec<ReceiverCoverageEvidence>,
    pub confidence_permille: RatioPermille,
    pub group_pressure_permille: RatioPermille,
    pub payload_bytes_max: ByteCount,
    pub resource_use: CastDeliveryResourceUse,
    pub meta: CastEvidenceMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastDeliverySupport {
    pub sender: NodeId,
    pub domain_id: BroadcastDomainId,
    pub receivers: Vec<ReceiverCoverageEvidence>,
    pub support: BroadcastSupportKind,
    pub confidence_permille: RatioPermille,
    pub reverse_confirmation_permille: Option<RatioPermille>,
    pub channel_pressure_permille: RatioPermille,
    pub payload_bytes_max: ByteCount,
    pub resource_use: CastDeliveryResourceUse,
    pub meta: CastEvidenceMeta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CastDeliverySupport {
    Unicast(UnicastDeliverySupport),
    Multicast(MulticastDeliverySupport),
    Broadcast(BroadcastDeliverySupport),
}

#[must_use]
pub fn shape_unicast_delivery_support<'a>(
    evidence: impl IntoIterator<Item = &'a UnicastEvidence>,
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
) -> (Vec<UnicastDeliverySupport>, CastDeliveryReport) {
    let mut report = CastDeliveryReport::default();
    let mut support = Vec::new();
    for item in evidence {
        let Some(item_support) = unicast_delivery_support(item, objective, policy, &mut report)
        else {
            continue;
        };
        support.push(item_support);
    }
    support.sort_by_key(|item| std::cmp::Reverse(unicast_delivery_support_rank(*item)));
    (support, report)
}

#[must_use]
pub fn shape_multicast_delivery_support<'a>(
    evidence: impl IntoIterator<Item = &'a MulticastEvidence>,
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
) -> (Vec<MulticastDeliverySupport>, CastDeliveryReport) {
    let mut report = CastDeliveryReport::default();
    let mut support = Vec::new();
    for item in evidence {
        let Some(item_support) = multicast_delivery_support(item, objective, policy, &mut report)
        else {
            continue;
        };
        support.push(item_support);
    }
    support.sort_by_key(|item| std::cmp::Reverse(multicast_delivery_support_rank(item)));
    (support, report)
}

#[must_use]
pub fn shape_broadcast_delivery_support<'a>(
    evidence: impl IntoIterator<Item = &'a BroadcastEvidence>,
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
) -> (Vec<BroadcastDeliverySupport>, CastDeliveryReport) {
    let mut report = CastDeliveryReport::default();
    let mut support = Vec::new();
    for item in evidence {
        let Some(item_support) = broadcast_delivery_support(item, objective, policy, &mut report)
        else {
            continue;
        };
        support.push(item_support);
    }
    support.sort_by_key(|item| std::cmp::Reverse(broadcast_delivery_support_rank(item)));
    (support, report)
}

fn unicast_delivery_support(
    evidence: &UnicastEvidence,
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> Option<UnicastDeliverySupport> {
    if objective.mode != CastDeliveryMode::Unicast {
        report.record_mode_mismatch();
        return None;
    }
    if evidence.from != objective.sender || !objective.receivers.contains(evidence.to) {
        report.record_receiver_mismatch();
        return None;
    }
    if !unicast_reverse_allowed(evidence, policy, report)
        || !capacity_allowed(evidence.payload_bytes_max, policy, report)
    {
        return None;
    }
    let confidence = unicast_confidence(evidence, policy);
    if !meets_confidence(confidence, policy.evidence) {
        report.record_low_confidence();
        return None;
    }
    Some(UnicastDeliverySupport {
        sender: evidence.from,
        receiver: evidence.to,
        confidence_permille: confidence,
        bidirectional_confidence_permille: evidence.bidirectional_confidence_permille,
        payload_bytes_max: evidence.payload_bytes_max,
        resource_use: delivery_resource_use(1, 1, 1, policy),
        meta: evidence.meta,
    })
}

fn multicast_delivery_support(
    evidence: &MulticastEvidence,
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> Option<MulticastDeliverySupport> {
    if !multicast_objective_matches(evidence, objective, report)
        || !capacity_allowed(evidence.payload_bytes_max, policy, report)
    {
        return None;
    }
    let receivers = matching_receivers(&evidence.receivers, &objective.receivers);
    if !coverage_allowed(&receivers, objective, policy, true, report) {
        return None;
    }
    let fanout_used = u32::try_from(receivers.len()).unwrap_or(u32::MAX);
    if fanout_used > evidence.fanout_limit || fanout_used > policy.evidence.bounds.fanout_count_max
    {
        report.record_fanout();
        return None;
    }
    let confidence = coverage_confidence(&receivers);
    if !meets_confidence(confidence, policy.evidence) {
        report.record_low_confidence();
        return None;
    }
    Some(MulticastDeliverySupport {
        sender: evidence.sender,
        group_id: evidence.group_id.clone(),
        receivers,
        confidence_permille: confidence,
        group_pressure_permille: evidence.group_pressure_permille,
        payload_bytes_max: evidence.payload_bytes_max,
        resource_use: delivery_resource_use(fanout_used, fanout_used, 0, policy),
        meta: evidence.meta,
    })
}

fn broadcast_delivery_support(
    evidence: &BroadcastEvidence,
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> Option<BroadcastDeliverySupport> {
    if !broadcast_objective_matches(evidence, objective, report)
        || !broadcast_reverse_allowed(evidence, policy, report)
        || !capacity_allowed(evidence.payload_bytes_max, policy, report)
        || !copy_budget_allowed(evidence, policy, report)
    {
        return None;
    }
    let receivers = matching_receivers(&evidence.receivers, &objective.receivers);
    if !coverage_allowed(&receivers, objective, policy, false, report) {
        return None;
    }
    let confidence = permille_product(
        coverage_confidence(&receivers),
        evidence.transmission_window_quality_permille,
    );
    if !meets_confidence(confidence, policy.evidence) {
        report.record_low_confidence();
        return None;
    }
    Some(BroadcastDeliverySupport {
        sender: evidence.sender,
        domain_id: objective.broadcast_domain_id?,
        receivers,
        support: evidence.support,
        confidence_permille: confidence,
        reverse_confirmation_permille: evidence.reverse_confirmation_permille,
        channel_pressure_permille: evidence.channel_pressure_permille,
        payload_bytes_max: evidence.payload_bytes_max,
        resource_use: delivery_resource_use(1, 0, evidence.copy_budget, policy),
        meta: evidence.meta,
    })
}

fn unicast_reverse_allowed(
    evidence: &UnicastEvidence,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> bool {
    if !policy.require_bidirectional
        || evidence.support == UnicastSupportKind::BidirectionalConfirmed
    {
        return true;
    }
    report.record_reverse_confirmation();
    false
}

fn broadcast_reverse_allowed(
    evidence: &BroadcastEvidence,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> bool {
    if policy.require_bidirectional
        && evidence.support != BroadcastSupportKind::DirectReverseConfirmed
    {
        report.record_reverse_confirmation();
        return false;
    }
    if !policy.allow_gateway_assisted_broadcast
        && evidence.support == BroadcastSupportKind::GatewayAssistedReverse
    {
        report.record_reverse_confirmation();
        return false;
    }
    true
}

fn capacity_allowed(
    payload_bytes_max: ByteCount,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> bool {
    if supports_payload(payload_bytes_max, policy.evidence) {
        return true;
    }
    report.record_capacity();
    false
}

fn copy_budget_allowed(
    evidence: &BroadcastEvidence,
    policy: CastDeliveryPolicy,
    report: &mut CastDeliveryReport,
) -> bool {
    if evidence.copy_budget <= policy.evidence.bounds.copy_budget_max {
        return true;
    }
    report.record_copy_budget();
    false
}

fn multicast_objective_matches(
    evidence: &MulticastEvidence,
    objective: &CastDeliveryObjective,
    report: &mut CastDeliveryReport,
) -> bool {
    if objective.mode != CastDeliveryMode::Multicast {
        report.record_mode_mismatch();
        return false;
    }
    if evidence.sender != objective.sender
        || Some(&evidence.group_id) != objective.group_id.as_ref()
    {
        report.record_receiver_mismatch();
        return false;
    }
    true
}

fn broadcast_objective_matches(
    evidence: &BroadcastEvidence,
    objective: &CastDeliveryObjective,
    report: &mut CastDeliveryReport,
) -> bool {
    if objective.mode != CastDeliveryMode::Broadcast {
        report.record_mode_mismatch();
        return false;
    }
    if evidence.sender != objective.sender {
        report.record_receiver_mismatch();
        return false;
    }
    true
}

fn coverage_allowed(
    receivers: &[ReceiverCoverageEvidence],
    objective: &CastDeliveryObjective,
    policy: CastDeliveryPolicy,
    multicast: bool,
    report: &mut CastDeliveryReport,
) -> bool {
    if receivers.is_empty() {
        report.record_receiver_mismatch();
        return false;
    }
    let all_required = objective.coverage == CastCoverageObjective::AllReceivers
        || (multicast && !policy.allow_partial_multicast);
    if !all_required || objective.receivers.is_empty() {
        return true;
    }
    if objective.receivers.covers_all(receivers) {
        return true;
    }
    report.record_receiver_mismatch();
    false
}

fn matching_receivers(
    evidence: &[ReceiverCoverageEvidence],
    objective: &CastReceiverSet,
) -> Vec<ReceiverCoverageEvidence> {
    if objective.is_empty() {
        return evidence.to_vec();
    }
    evidence
        .iter()
        .copied()
        .filter(|receiver| objective.contains(receiver.receiver))
        .collect()
}

fn unicast_confidence(evidence: &UnicastEvidence, policy: CastDeliveryPolicy) -> RatioPermille {
    if policy.require_bidirectional {
        evidence.bidirectional_confidence_permille
    } else {
        evidence.directional_confidence_permille
    }
}

fn coverage_confidence(receivers: &[ReceiverCoverageEvidence]) -> RatioPermille {
    receivers
        .iter()
        .fold(RatioPermille(1_000), |confidence, receiver| {
            permille_product(confidence, receiver.confidence_permille)
        })
}

fn delivery_resource_use(
    receiver_count: u32,
    fanout_used: u32,
    copy_budget_used: u32,
    policy: CastDeliveryPolicy,
) -> CastDeliveryResourceUse {
    CastDeliveryResourceUse {
        receiver_count,
        fanout_used,
        copy_budget_used,
        payload_bytes: policy.evidence.payload_bytes_required,
    }
}

fn unicast_delivery_support_rank(
    support: UnicastDeliverySupport,
) -> (
    RatioPermille,
    RatioPermille,
    ByteCount,
    jacquard_core::Tick,
    jacquard_core::OrderStamp,
    NodeId,
    NodeId,
) {
    (
        support.confidence_permille,
        support.bidirectional_confidence_permille,
        support.payload_bytes_max,
        support.meta.observed_at_tick,
        support.meta.order,
        support.sender,
        support.receiver,
    )
}

fn multicast_delivery_support_rank(
    support: &MulticastDeliverySupport,
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
        support.resource_use.receiver_count,
        support.confidence_permille,
        std::cmp::Reverse(support.group_pressure_permille),
        support.payload_bytes_max,
        support.meta.observed_at_tick,
        support.meta.order,
        support.sender,
        support.group_id.clone(),
    )
}

fn broadcast_delivery_support_rank(
    support: &BroadcastDeliverySupport,
) -> (
    u8,
    RatioPermille,
    std::cmp::Reverse<RatioPermille>,
    ByteCount,
    jacquard_core::Tick,
    jacquard_core::OrderStamp,
    NodeId,
) {
    (
        broadcast_support_rank(support.support),
        support.confidence_permille,
        std::cmp::Reverse(support.channel_pressure_permille),
        support.payload_bytes_max,
        support.meta.observed_at_tick,
        support.meta.order,
        support.sender,
    )
}

fn broadcast_support_rank(support: BroadcastSupportKind) -> u8 {
    match support {
        BroadcastSupportKind::DirectReverseConfirmed => 3,
        BroadcastSupportKind::GatewayAssistedReverse => 2,
        BroadcastSupportKind::DirectionalOnly => 1,
    }
}
