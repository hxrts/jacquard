use jacquard_cast_support::{
    shape_broadcast_evidence, shape_multicast_evidence, shape_unicast_evidence,
    BroadcastObservation, BroadcastReverseConfirmation, CastEvidenceBounds, CastEvidenceMeta,
    CastEvidencePolicy, CastGroupId, MulticastObservation, ReceiverCoverageObservation,
    UnicastObservation, UnicastSupportKind,
};
use jacquard_core::{ByteCount, DurationMs, NodeId, OrderStamp, RatioPermille, Tick};

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

fn policy() -> CastEvidencePolicy {
    CastEvidencePolicy {
        confidence_floor: RatioPermille(500),
        payload_bytes_required: ByteCount(64),
        bounds: CastEvidenceBounds {
            receiver_count_max: 2,
            group_coverage_count_max: 3,
            fanout_count_max: 2,
            copy_budget_max: 2,
            evidence_age_ms_max: DurationMs(1_000),
        },
    }
}

#[test]
fn unicast_surface_keeps_directional_and_reverse_support_separate() {
    let (evidence, report) = shape_unicast_evidence(
        [UnicastObservation {
            from: node(1),
            to: node(2),
            directional_confidence_permille: RatioPermille(800),
            reverse_confirmation_permille: None,
            payload_bytes_max: ByteCount(128),
            meta: meta(1),
        }],
        policy(),
    );

    assert_eq!(report.omitted_stale_count, 0);
    assert_eq!(evidence[0].support, UnicastSupportKind::DirectionalOnly);
    assert_eq!(
        evidence[0].bidirectional_confidence_permille,
        RatioPermille(0)
    );
}

#[test]
fn multicast_surface_enforces_group_coverage_bounds() {
    let receivers = vec![
        ReceiverCoverageObservation {
            receiver: node(2),
            confidence_permille: RatioPermille(700),
        },
        ReceiverCoverageObservation {
            receiver: node(3),
            confidence_permille: RatioPermille(700),
        },
        ReceiverCoverageObservation {
            receiver: node(4),
            confidence_permille: RatioPermille(700),
        },
        ReceiverCoverageObservation {
            receiver: node(5),
            confidence_permille: RatioPermille(700),
        },
    ];

    let (evidence, report) = shape_multicast_evidence(
        [MulticastObservation {
            sender: node(1),
            group_id: CastGroupId(b"group".to_vec()),
            receivers,
            group_pressure_permille: RatioPermille(100),
            fanout_limit: 2,
            payload_bytes_max: ByteCount(128),
            meta: meta(1),
        }],
        policy(),
    );

    assert!(evidence.is_empty());
    assert_eq!(report.omitted_bound_count, 1);
}

#[test]
fn broadcast_surface_keeps_gateway_confirmation_out_of_bidirectional_support() {
    let (evidence, _report) = shape_broadcast_evidence(
        [BroadcastObservation {
            sender: node(1),
            receivers: vec![ReceiverCoverageObservation {
                receiver: node(2),
                confidence_permille: RatioPermille(700),
            }],
            reverse_confirmation: BroadcastReverseConfirmation::GatewayAssisted(RatioPermille(800)),
            transmission_window_quality_permille: RatioPermille(700),
            channel_pressure_permille: RatioPermille(100),
            copy_budget: 1,
            payload_bytes_max: ByteCount(128),
            meta: meta(1),
        }],
        policy(),
    );

    assert_eq!(
        evidence[0].connected_bidirectional_confidence(),
        RatioPermille(0)
    );
    assert!(evidence[0].custody_improvement_score() > RatioPermille(0));
}
