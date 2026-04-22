use jacquard_cast_support::{
    shape_broadcast_evidence, BroadcastEvidence, BroadcastObservation,
    BroadcastReverseConfirmation, CastEvidenceMeta, CastEvidencePolicy,
    ReceiverCoverageObservation,
};
use jacquard_core::{
    ByteCount, DestinationId, DurationMs, NodeId, OrderStamp, RatioPermille, RouteEpoch, Tick,
};
use jacquard_mercator::{
    evidence::{MercatorCustodyOpportunity, MercatorEvidenceMeta, MercatorObjectiveKey},
    MercatorEvidenceBounds, MercatorEvidenceGraph,
};

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn cast_meta() -> CastEvidenceMeta {
    CastEvidenceMeta::new(Tick(6), DurationMs(10), DurationMs(1_000), OrderStamp(9))
}

fn broadcast() -> BroadcastObservation {
    BroadcastObservation {
        sender: node(1),
        receivers: vec![ReceiverCoverageObservation {
            receiver: node(2),
            confidence_permille: RatioPermille(800),
        }],
        reverse_confirmation: BroadcastReverseConfirmation::Unavailable,
        transmission_window_quality_permille: RatioPermille(800),
        channel_pressure_permille: RatioPermille(100),
        copy_budget: 1,
        payload_bytes_max: ByteCount(512),
        meta: cast_meta(),
    }
}

fn custody_opportunity(evidence: &BroadcastEvidence) -> MercatorCustodyOpportunity {
    MercatorCustodyOpportunity {
        objective: MercatorObjectiveKey::destination(DestinationId::Node(node(9))),
        carrier: evidence.receivers[0].receiver,
        improvement_score: evidence.custody_improvement_score().0,
        custody_pressure: evidence.channel_pressure_permille.0,
        meta: MercatorEvidenceMeta::new(
            RouteEpoch(1),
            evidence.meta.observed_at_tick,
            evidence.meta.valid_for_ms,
            evidence.meta.order,
        ),
    }
}

#[test]
fn broadcast_cast_custody_opportunity_does_not_publish_connected_support() {
    let (broadcast_evidence, _report) =
        shape_broadcast_evidence([broadcast()], CastEvidencePolicy::default());
    assert_eq!(
        broadcast_evidence[0].connected_bidirectional_confidence(),
        RatioPermille(0)
    );

    let mut graph = MercatorEvidenceGraph::new(MercatorEvidenceBounds::default());
    graph.record_custody_opportunity(custody_opportunity(&broadcast_evidence[0]));

    assert!(graph.link_evidence().is_empty());
    assert_eq!(graph.custody_opportunities().len(), 1);
}
