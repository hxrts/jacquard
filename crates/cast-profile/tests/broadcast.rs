use jacquard_cast_profile::{
    shape_broadcast_evidence, BroadcastObservation, BroadcastReverseConfirmation,
    BroadcastSupportKind, CastEvidenceBounds, CastEvidenceMeta, CastEvidencePolicy,
    ReceiverCoverageObservation,
};
use jacquard_core::{ByteCount, DurationMs, NodeId, OrderStamp, RatioPermille, Tick};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn meta(age_ms: u32, order: u64) -> CastEvidenceMeta {
    CastEvidenceMeta::new(
        Tick(10),
        DurationMs(age_ms),
        DurationMs(1_000),
        OrderStamp(order),
    )
}

fn policy() -> CastEvidencePolicy {
    CastEvidencePolicy {
        confidence_floor: RatioPermille(500),
        payload_bytes_required: ByteCount(128),
        bounds: CastEvidenceBounds {
            receiver_count_max: 4,
            copy_budget_max: 2,
            evidence_age_ms_max: DurationMs(1_000),
            ..Default::default()
        },
    }
}

fn receiver(byte: u8, confidence: u16) -> ReceiverCoverageObservation {
    ReceiverCoverageObservation {
        receiver: node(byte),
        confidence_permille: RatioPermille(confidence),
    }
}

fn observation(
    receivers: Vec<ReceiverCoverageObservation>,
    reverse_confirmation: BroadcastReverseConfirmation,
    window: u16,
    pressure: u16,
    payload: u64,
    order: u64,
) -> BroadcastObservation {
    BroadcastObservation {
        sender: node(1),
        receivers,
        reverse_confirmation,
        transmission_window_quality_permille: RatioPermille(window),
        channel_pressure_permille: RatioPermille(pressure),
        copy_budget: 1,
        payload_bytes_max: ByteCount(payload),
        meta: meta(10, order),
    }
}

#[test]
fn broadcast_lora_like_one_way_evidence_is_directional_only() {
    let (evidence, _report) = shape_broadcast_evidence(
        [observation(
            vec![receiver(2, 800), receiver(3, 700)],
            BroadcastReverseConfirmation::Unavailable,
            800,
            100,
            256,
            1,
        )],
        policy(),
    );

    assert_eq!(evidence[0].support, BroadcastSupportKind::DirectionalOnly);
    assert_eq!(
        evidence[0].connected_bidirectional_confidence(),
        RatioPermille(0)
    );
    assert!(evidence[0].custody_improvement_score() > RatioPermille(0));
}

#[test]
fn broadcast_gateway_assisted_confirmation_does_not_become_connected_support() {
    let (evidence, _report) = shape_broadcast_evidence(
        [observation(
            vec![receiver(2, 800)],
            BroadcastReverseConfirmation::GatewayAssisted(RatioPermille(900)),
            800,
            100,
            256,
            1,
        )],
        policy(),
    );

    assert_eq!(
        evidence[0].support,
        BroadcastSupportKind::GatewayAssistedReverse
    );
    assert_eq!(
        evidence[0].connected_bidirectional_confidence(),
        RatioPermille(0)
    );
}

#[test]
fn broadcast_payload_too_large_and_poor_window_are_omitted() {
    let (evidence, report) = shape_broadcast_evidence(
        [
            observation(
                vec![receiver(2, 800)],
                BroadcastReverseConfirmation::Unavailable,
                800,
                100,
                127,
                1,
            ),
            observation(
                vec![receiver(3, 800)],
                BroadcastReverseConfirmation::Unavailable,
                499,
                100,
                256,
                2,
            ),
        ],
        policy(),
    );

    assert!(evidence.is_empty());
    assert_eq!(report.omitted_capacity_count, 1);
    assert_eq!(report.omitted_low_confidence_count, 1);
}

#[test]
fn broadcast_receiver_coverage_changes_are_visible_and_stably_ordered() {
    let (evidence, _report) = shape_broadcast_evidence(
        [
            observation(
                vec![receiver(4, 700), receiver(2, 900)],
                BroadcastReverseConfirmation::Unavailable,
                800,
                100,
                256,
                1,
            ),
            observation(
                vec![receiver(3, 850)],
                BroadcastReverseConfirmation::Unavailable,
                800,
                100,
                256,
                2,
            ),
        ],
        policy(),
    );

    assert_eq!(
        evidence[0]
            .receivers
            .iter()
            .map(|receiver| receiver.receiver)
            .collect::<Vec<_>>(),
        vec![node(3)]
    );
    assert_eq!(
        evidence[1]
            .receivers
            .iter()
            .map(|receiver| receiver.receiver)
            .collect::<Vec<_>>(),
        vec![node(2), node(4)]
    );
}

#[test]
fn broadcast_channel_pressure_reduces_custody_score() {
    let (evidence, _report) = shape_broadcast_evidence(
        [
            observation(
                vec![receiver(2, 800)],
                BroadcastReverseConfirmation::Unavailable,
                800,
                100,
                256,
                1,
            ),
            observation(
                vec![receiver(3, 800)],
                BroadcastReverseConfirmation::Unavailable,
                800,
                700,
                256,
                2,
            ),
        ],
        policy(),
    );

    assert!(evidence[0].custody_improvement_score() > evidence[1].custody_improvement_score());
}
