use jacquard_cast_support::{
    shape_unicast_evidence, CastEvidenceMeta, CastEvidencePolicy, UnicastEvidence,
    UnicastObservation,
};
use jacquard_core::{ByteCount, DurationMs, NodeId, OrderStamp, RatioPermille, RouteEpoch, Tick};
use jacquard_mercator::{
    evidence::{MercatorEvidenceMeta, MercatorLinkEvidence, MercatorReverseLinkEvidence},
    MercatorEvidenceBounds, MercatorEvidenceGraph,
};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn cast_meta() -> CastEvidenceMeta {
    CastEvidenceMeta::new(Tick(8), DurationMs(10), DurationMs(1_000), OrderStamp(4))
}

fn observation(reverse: Option<u16>) -> UnicastObservation {
    UnicastObservation {
        from: node(1),
        to: node(2),
        directional_confidence_permille: RatioPermille(800),
        reverse_confirmation_permille: reverse.map(RatioPermille),
        payload_bytes_max: ByteCount(512),
        meta: cast_meta(),
    }
}

fn mercator_meta(evidence: UnicastEvidence) -> MercatorEvidenceMeta {
    MercatorEvidenceMeta::new(
        RouteEpoch(3),
        evidence.meta.observed_at_tick,
        evidence.meta.valid_for_ms,
        evidence.meta.order,
    )
}

fn link_evidence(evidence: UnicastEvidence) -> MercatorLinkEvidence {
    MercatorLinkEvidence {
        from: evidence.from,
        to: evidence.to,
        bidirectional_confidence: evidence.bidirectional_confidence_permille.0,
        asymmetric_penalty: 1_000_u16.saturating_sub(evidence.bidirectional_confidence_permille.0),
        meta: mercator_meta(evidence),
    }
}

fn reverse_evidence(evidence: UnicastEvidence) -> Option<MercatorReverseLinkEvidence> {
    evidence
        .reverse_confirmation_permille
        .map(|reverse_confidence| MercatorReverseLinkEvidence {
            from: evidence.to,
            to: evidence.from,
            reverse_confidence: reverse_confidence.0,
            meta: mercator_meta(evidence),
        })
}

#[test]
fn unicast_cast_evidence_can_feed_mercator_link_support() {
    let (cast_evidence, report) =
        shape_unicast_evidence([observation(Some(750))], CastEvidencePolicy::default());
    assert_eq!(report.omitted_stale_count, 0);

    let mut graph = MercatorEvidenceGraph::new(MercatorEvidenceBounds::default());
    let evidence = cast_evidence[0];
    graph.record_link_evidence(link_evidence(evidence));
    graph.record_reverse_link_support(reverse_evidence(evidence).expect("reverse confirmation"));

    let stored = graph.link_evidence();
    assert_eq!(stored[0].from, node(1));
    assert_eq!(stored[0].to, node(2));
    assert_eq!(stored[0].bidirectional_confidence, 750);
}

#[test]
fn one_way_unicast_cast_evidence_feeds_only_directional_mercator_support() {
    let (cast_evidence, _report) =
        shape_unicast_evidence([observation(None)], CastEvidencePolicy::default());

    let evidence = cast_evidence[0];
    assert!(reverse_evidence(evidence).is_none());
    assert_eq!(link_evidence(evidence).bidirectional_confidence, 0);
}
