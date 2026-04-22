use std::collections::BTreeSet;

use jacquard_cast_support::{
    shape_multicast_evidence, CastEvidenceBounds, CastEvidenceMeta, CastEvidencePolicy,
    CastGroupId, MulticastObservation, ReceiverCoverageObservation,
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
        payload_bytes_required: ByteCount(64),
        bounds: CastEvidenceBounds {
            group_coverage_count_max: 4,
            fanout_count_max: 3,
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

fn group(name: &[u8]) -> CastGroupId {
    CastGroupId(name.to_vec())
}

fn observation(
    name: &[u8],
    receivers: Vec<ReceiverCoverageObservation>,
    pressure: u16,
    fanout: u32,
    order: u64,
) -> MulticastObservation {
    MulticastObservation {
        sender: node(1),
        group_id: group(name),
        receivers,
        group_pressure_permille: RatioPermille(pressure),
        fanout_limit: fanout,
        payload_bytes_max: ByteCount(256),
        meta: meta(10, order),
    }
}

#[test]
fn multicast_full_and_partial_coverage_are_explicit() {
    let (evidence, _report) = shape_multicast_evidence(
        [observation(
            b"team",
            vec![receiver(2, 800), receiver(3, 700)],
            100,
            2,
            1,
        )],
        policy(),
    );

    let full = BTreeSet::from([node(2), node(3)]);
    let partial = BTreeSet::from([node(2), node(4)]);

    assert_eq!(evidence[0].covered_receiver_count, 2);
    assert!(evidence[0].can_satisfy_receiver_objective(&full));
    assert!(!evidence[0].can_satisfy_receiver_objective(&partial));
}

#[test]
fn multicast_group_pressure_reduces_ranking() {
    let (evidence, _report) = shape_multicast_evidence(
        [
            observation(b"quiet", vec![receiver(2, 800)], 100, 1, 1),
            observation(b"loaded", vec![receiver(3, 800)], 900, 1, 2),
        ],
        policy(),
    );

    assert_eq!(evidence[0].group_id, group(b"quiet"));
    assert_eq!(evidence[1].group_id, group(b"loaded"));
}

#[test]
fn multicast_stale_group_evidence_and_bounded_fanout_are_omitted() {
    let mut stale = observation(b"stale", vec![receiver(2, 800)], 100, 1, 1);
    stale.meta = meta(1_001, 1);

    let (evidence, report) = shape_multicast_evidence(
        [
            stale,
            observation(b"wide", vec![receiver(3, 800)], 100, 4, 2),
        ],
        policy(),
    );

    assert!(evidence.is_empty());
    assert_eq!(report.omitted_stale_count, 1);
    assert_eq!(report.omitted_bound_count, 1);
}

#[test]
fn multicast_receiver_order_is_stable_across_input_ordering() {
    let first = observation(
        b"group",
        vec![receiver(4, 700), receiver(2, 900), receiver(3, 800)],
        100,
        3,
        1,
    );
    let second = observation(
        b"group",
        vec![receiver(3, 800), receiver(2, 900), receiver(4, 700)],
        100,
        3,
        1,
    );

    let (forward, _) = shape_multicast_evidence([first], policy());
    let (reordered, _) = shape_multicast_evidence([second], policy());

    assert_eq!(forward[0].receivers, reordered[0].receivers);
    assert_eq!(
        forward[0]
            .receivers
            .iter()
            .map(|receiver| receiver.receiver)
            .collect::<Vec<_>>(),
        vec![node(2), node(3), node(4)]
    );
}

#[test]
fn multicast_low_confidence_receivers_do_not_force_all_or_nothing_delivery() {
    let (evidence, _report) = shape_multicast_evidence(
        [observation(
            b"mixed",
            vec![receiver(2, 800), receiver(3, 400), receiver(4, 700)],
            100,
            3,
            1,
        )],
        policy(),
    );

    assert_eq!(evidence[0].covered_receiver_count, 2);
    assert_eq!(
        evidence[0]
            .receivers
            .iter()
            .map(|receiver| receiver.receiver)
            .collect::<Vec<_>>(),
        vec![node(2), node(4)]
    );
}
