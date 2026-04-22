use jacquard_cast_profile::{
    shape_unicast_evidence, CastEvidenceBounds, CastEvidenceMeta, CastEvidencePolicy,
    UnicastObservation, UnicastSupportKind,
};
use jacquard_core::{ByteCount, DurationMs, NodeId, OrderStamp, RatioPermille, Tick};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn meta(tick: u64, age_ms: u32, order: u64) -> CastEvidenceMeta {
    CastEvidenceMeta::new(
        Tick(tick),
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
            receiver_count_max: 2,
            evidence_age_ms_max: DurationMs(1_000),
            ..Default::default()
        },
    }
}

fn observation(
    to: u8,
    confidence: u16,
    reverse: Option<u16>,
    payload: u64,
    order: u64,
) -> UnicastObservation {
    UnicastObservation {
        from: node(1),
        to: node(to),
        directional_confidence_permille: RatioPermille(confidence),
        reverse_confirmation_permille: reverse.map(RatioPermille),
        payload_bytes_max: ByteCount(payload),
        meta: meta(10, 10, order),
    }
}

#[test]
fn unicast_bidirectional_evidence_ranks_above_one_way_evidence() {
    let (evidence, report) = shape_unicast_evidence(
        [
            observation(2, 800, None, 256, 1),
            observation(3, 700, Some(650), 256, 2),
        ],
        policy(),
    );

    assert_eq!(report.omitted_low_confidence_count, 0);
    assert_eq!(evidence[0].to, node(3));
    assert_eq!(
        evidence[0].support,
        UnicastSupportKind::BidirectionalConfirmed
    );
    assert_eq!(
        evidence[1].bidirectional_confidence_permille,
        RatioPermille(0)
    );
}

#[test]
fn unicast_one_way_evidence_remains_directional() {
    let (evidence, _report) = shape_unicast_evidence([observation(2, 900, None, 256, 1)], policy());

    assert_eq!(evidence[0].support, UnicastSupportKind::DirectionalOnly);
    assert_eq!(evidence[0].reverse_confirmation_permille, None);
}

#[test]
fn unicast_stale_low_confidence_and_low_capacity_evidence_are_omitted() {
    let mut stale = observation(2, 900, Some(900), 256, 1);
    stale.meta = meta(1, 1_001, 1);

    let (evidence, report) = shape_unicast_evidence(
        [
            stale,
            observation(3, 499, Some(499), 256, 2),
            observation(4, 900, Some(900), 127, 3),
        ],
        policy(),
    );

    assert!(evidence.is_empty());
    assert_eq!(report.omitted_stale_count, 1);
    assert_eq!(report.omitted_low_confidence_count, 1);
    assert_eq!(report.omitted_capacity_count, 1);
}

#[test]
fn unicast_output_order_is_stable_under_equal_scores() {
    let observations = [
        observation(4, 800, Some(800), 256, 1),
        observation(2, 800, Some(800), 256, 1),
        observation(3, 800, Some(800), 256, 1),
    ];
    let (forward, _) = shape_unicast_evidence(observations, policy());
    let (reverse, _) = shape_unicast_evidence(
        [observations[2], observations[1], observations[0]],
        policy(),
    );

    let forward_nodes = forward.iter().map(|item| item.to).collect::<Vec<_>>();
    let reverse_nodes = reverse.iter().map(|item| item.to).collect::<Vec<_>>();

    assert_eq!(forward_nodes, vec![node(4), node(3)]);
    assert_eq!(reverse_nodes, forward_nodes);
}
