use jacquard_cast_support::{
    shape_broadcast_delivery_support, shape_broadcast_evidence, shape_multicast_delivery_support,
    shape_multicast_evidence, shape_unicast_delivery_support, shape_unicast_evidence,
    BroadcastObservation, BroadcastReverseConfirmation, CastCoverageObjective,
    CastDeliveryObjective, CastDeliveryPolicy, CastEvidenceBounds, CastEvidenceMeta,
    CastEvidencePolicy, CastGroupId, CastReceiverSet, MulticastObservation,
    ReceiverCoverageObservation, UnicastObservation,
};
use jacquard_core::{
    BroadcastDomainId, ByteCount, DurationMs, MulticastGroupId, NodeId, OrderStamp, RatioPermille,
    Tick,
};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn meta(order: u64) -> CastEvidenceMeta {
    CastEvidenceMeta::new(
        Tick(10),
        DurationMs(10),
        DurationMs(1_000),
        OrderStamp(order),
    )
}

fn evidence_policy() -> CastEvidencePolicy {
    CastEvidencePolicy {
        confidence_floor: RatioPermille(500),
        payload_bytes_required: ByteCount(128),
        bounds: CastEvidenceBounds {
            receiver_count_max: 8,
            group_coverage_count_max: 8,
            fanout_count_max: 4,
            copy_budget_max: 3,
            evidence_age_ms_max: DurationMs(1_000),
        },
    }
}

fn delivery_policy() -> CastDeliveryPolicy {
    CastDeliveryPolicy {
        evidence: evidence_policy(),
        require_bidirectional: false,
        allow_partial_multicast: true,
        allow_gateway_assisted_broadcast: true,
    }
}

fn receiver(byte: u8, confidence: u16) -> ReceiverCoverageObservation {
    ReceiverCoverageObservation {
        receiver: node(byte),
        confidence_permille: RatioPermille(confidence),
    }
}

fn unicast_observation(to: u8, confidence: u16, reverse: Option<u16>) -> UnicastObservation {
    UnicastObservation {
        from: node(1),
        to: node(to),
        directional_confidence_permille: RatioPermille(confidence),
        reverse_confirmation_permille: reverse.map(RatioPermille),
        payload_bytes_max: ByteCount(512),
        meta: meta(u64::from(to)),
    }
}

fn group(byte: u8) -> CastGroupId {
    CastGroupId::new(MulticastGroupId([byte; 16]))
}

fn domain(byte: u8) -> BroadcastDomainId {
    BroadcastDomainId([byte; 16])
}

#[test]
fn receiver_set_is_sorted_deduped_and_queryable() {
    let receivers = CastReceiverSet::new([node(4), node(2), node(4), node(3)]);

    assert_eq!(receivers.as_slice(), &[node(2), node(3), node(4)]);
    assert_eq!(receivers.len(), 3);
    assert!(receivers.contains(node(3)));
    assert!(!receivers.contains(node(5)));
}

#[test]
fn unicast_delivery_can_require_bidirectional_confirmation() {
    let (evidence, _) = shape_unicast_evidence(
        [
            unicast_observation(2, 900, None),
            unicast_observation(3, 800, Some(700)),
        ],
        evidence_policy(),
    );
    let objective = CastDeliveryObjective::unicast(node(1), node(3));
    let policy = CastDeliveryPolicy {
        require_bidirectional: true,
        ..delivery_policy()
    };

    let (support, report) = shape_unicast_delivery_support(evidence.iter(), &objective, policy);

    assert_eq!(report.omitted_reverse_confirmation_count, 0);
    assert_eq!(report.omitted_receiver_mismatch_count, 1);
    assert_eq!(support.len(), 1);
    assert_eq!(support[0].receiver, node(3));
    assert_eq!(support[0].confidence_permille, RatioPermille(700));
    assert_eq!(support[0].resource_use.payload_bytes, ByteCount(128));
}

#[test]
fn multicast_delivery_respects_partial_coverage_policy() {
    let (evidence, _) = shape_multicast_evidence(
        [MulticastObservation {
            sender: node(1),
            group_id: group(1),
            receivers: vec![receiver(2, 900), receiver(3, 800)],
            group_pressure_permille: RatioPermille(100),
            fanout_limit: 3,
            payload_bytes_max: ByteCount(512),
            meta: meta(1),
        }],
        evidence_policy(),
    );
    let objective = CastDeliveryObjective::multicast(
        node(1),
        group(1),
        [node(2), node(4)],
        CastCoverageObjective::AnyReceiver,
    );

    let (partial, partial_report) =
        shape_multicast_delivery_support(evidence.iter(), &objective, delivery_policy());
    let strict_policy = CastDeliveryPolicy {
        allow_partial_multicast: false,
        ..delivery_policy()
    };
    let (strict, strict_report) =
        shape_multicast_delivery_support(evidence.iter(), &objective, strict_policy);

    assert_eq!(partial_report.omitted_receiver_mismatch_count, 0);
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].receivers[0].receiver, node(2));
    assert_eq!(partial[0].resource_use.fanout_used, 1);
    assert!(strict.is_empty());
    assert_eq!(strict_report.omitted_receiver_mismatch_count, 1);
}

#[test]
fn broadcast_delivery_can_reject_gateway_assisted_reverse_support() {
    let (evidence, _) = shape_broadcast_evidence(
        [
            BroadcastObservation {
                sender: node(1),
                receivers: vec![receiver(2, 900)],
                reverse_confirmation: BroadcastReverseConfirmation::GatewayAssisted(RatioPermille(
                    900,
                )),
                transmission_window_quality_permille: RatioPermille(900),
                channel_pressure_permille: RatioPermille(100),
                copy_budget: 1,
                payload_bytes_max: ByteCount(512),
                meta: meta(1),
            },
            BroadcastObservation {
                sender: node(1),
                receivers: vec![receiver(2, 800)],
                reverse_confirmation: BroadcastReverseConfirmation::Direct(RatioPermille(800)),
                transmission_window_quality_permille: RatioPermille(800),
                channel_pressure_permille: RatioPermille(100),
                copy_budget: 1,
                payload_bytes_max: ByteCount(512),
                meta: meta(2),
            },
        ],
        evidence_policy(),
    );
    let objective = CastDeliveryObjective::broadcast_in_domain(
        node(1),
        domain(1),
        [node(2)],
        CastCoverageObjective::AllReceivers,
    );
    let policy = CastDeliveryPolicy {
        require_bidirectional: true,
        allow_gateway_assisted_broadcast: false,
        ..delivery_policy()
    };

    let (support, report) = shape_broadcast_delivery_support(evidence.iter(), &objective, policy);

    assert_eq!(report.omitted_reverse_confirmation_count, 1);
    assert_eq!(support.len(), 1);
    assert_eq!(
        support[0].reverse_confirmation_permille,
        Some(RatioPermille(800))
    );
    assert_eq!(support[0].resource_use.copy_budget_used, 1);
}

#[test]
fn broadcast_delivery_ordering_is_deterministic() {
    let (evidence, _) = shape_broadcast_evidence(
        [
            BroadcastObservation {
                sender: node(1),
                receivers: vec![receiver(3, 800)],
                reverse_confirmation: BroadcastReverseConfirmation::Unavailable,
                transmission_window_quality_permille: RatioPermille(800),
                channel_pressure_permille: RatioPermille(100),
                copy_budget: 1,
                payload_bytes_max: ByteCount(512),
                meta: meta(1),
            },
            BroadcastObservation {
                sender: node(1),
                receivers: vec![receiver(2, 900)],
                reverse_confirmation: BroadcastReverseConfirmation::Unavailable,
                transmission_window_quality_permille: RatioPermille(900),
                channel_pressure_permille: RatioPermille(100),
                copy_budget: 1,
                payload_bytes_max: ByteCount(512),
                meta: meta(2),
            },
        ],
        evidence_policy(),
    );
    let objective = CastDeliveryObjective::broadcast_in_domain(
        node(1),
        domain(1),
        [node(2), node(3)],
        CastCoverageObjective::AnyReceiver,
    );

    let (support, report) =
        shape_broadcast_delivery_support(evidence.iter(), &objective, delivery_policy());

    assert_eq!(report.omitted_receiver_mismatch_count, 0);
    assert_eq!(support.len(), 2);
    assert_eq!(support[0].receivers[0].receiver, node(2));
    assert_eq!(support[1].receivers[0].receiver, node(3));
}
