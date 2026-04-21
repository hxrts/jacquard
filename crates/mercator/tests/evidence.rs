use jacquard_core::{
    DestinationId, DurationMs, NodeId, OrderStamp, RouteEpoch, RouteId, ServiceId, Tick,
};
use jacquard_mercator::evidence::{
    MercatorBrokerPressure, MercatorCustodyOpportunity, MercatorEvidenceGraph,
    MercatorEvidenceMeta, MercatorLinkEvidence, MercatorObjectiveKey, MercatorReverseLinkEvidence,
    MercatorRouteSupport, MercatorServiceSupport, MercatorSupportState,
};
use jacquard_mercator::MercatorEvidenceBounds;

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn route(byte: u8) -> RouteId {
    RouteId([byte; 16])
}

fn objective(byte: u8) -> MercatorObjectiveKey {
    MercatorObjectiveKey::destination(DestinationId::Service(ServiceId(vec![byte; 4])))
}

fn meta(epoch: u64, tick: u64, order: u64) -> MercatorEvidenceMeta {
    MercatorEvidenceMeta::new(
        RouteEpoch(epoch),
        Tick(tick),
        DurationMs(1_000),
        OrderStamp(order),
    )
}

fn tight_bounds() -> MercatorEvidenceBounds {
    MercatorEvidenceBounds {
        neighbor_count_max: 2,
        candidate_broker_count_max: 2,
        service_evidence_count_max: 2,
        corridor_alternate_count_max: 2,
        custody_opportunity_count_max: 2,
        custody_record_count_max: 2,
    }
}

fn link(to: u8, confidence: u16, penalty: u16, order: u64) -> MercatorLinkEvidence {
    MercatorLinkEvidence {
        from: node(1),
        to: node(to),
        bidirectional_confidence: confidence,
        asymmetric_penalty: penalty,
        meta: meta(4, 10, order),
    }
}

fn support(route_byte: u8, state: MercatorSupportState, score: u16) -> MercatorRouteSupport {
    MercatorRouteSupport {
        route_id: route(route_byte),
        objective: objective(route_byte),
        state,
        support_score: score,
        last_loss_epoch: None,
        stale_started_at: None,
        meta: meta(4, 10, u64::from(route_byte)),
    }
}

fn graph_with_links_in_order(order: &[MercatorLinkEvidence]) -> MercatorEvidenceGraph {
    let mut graph = MercatorEvidenceGraph::new(tight_bounds());
    for evidence in order {
        graph.record_link_evidence(*evidence);
    }
    graph
}

#[test]
fn evidence_pruning_is_deterministic_under_reordered_input() {
    let candidates = [
        link(2, 700, 10, 1),
        link(3, 900, 30, 2),
        link(4, 900, 20, 3),
        link(5, 500, 0, 4),
    ];
    let forward = graph_with_links_in_order(&candidates);
    let reverse =
        graph_with_links_in_order(&[candidates[3], candidates[2], candidates[1], candidates[0]]);

    let forward_kept = forward
        .link_evidence()
        .into_iter()
        .map(|evidence| evidence.to)
        .collect::<Vec<_>>();
    let reverse_kept = reverse
        .link_evidence()
        .into_iter()
        .map(|evidence| evidence.to)
        .collect::<Vec<_>>();

    assert_eq!(forward_kept, vec![node(3), node(4)]);
    assert_eq!(reverse_kept, forward_kept);
}

#[test]
fn evidence_epoch_invalidation_withdraws_pre_disruption_support() {
    let mut graph = MercatorEvidenceGraph::new(tight_bounds());
    let mut old_link = link(2, 950, 0, 1);
    old_link.meta = meta(2, 8, 1);
    graph.record_link_evidence(old_link);
    graph.record_reverse_link_support(MercatorReverseLinkEvidence {
        from: node(2),
        to: node(1),
        reverse_confidence: 900,
        meta: meta(2, 8, 2),
    });
    let mut old_route_support = support(7, MercatorSupportState::Fresh, 880);
    old_route_support.meta = meta(2, 8, 6);
    graph.record_route_support(old_route_support);
    graph.record_broker_pressure(MercatorBrokerPressure {
        broker: node(8),
        participation_count: 3,
        pressure_score: 100,
        meta: meta(2, 8, 3),
    });
    graph.record_service_support(MercatorServiceSupport {
        objective: objective(1),
        provider: node(9),
        support_score: 820,
        meta: meta(2, 8, 4),
    });
    graph.record_custody_opportunity(MercatorCustodyOpportunity {
        objective: objective(1),
        carrier: node(10),
        improvement_score: 780,
        custody_pressure: 50,
        meta: meta(2, 8, 5),
    });

    graph.invalidate_disruption_epoch(RouteEpoch(3), Tick(12));

    assert_eq!(graph.latest_disruption_epoch(), Some(RouteEpoch(3)));
    assert_eq!(graph.link_evidence()[0].bidirectional_confidence, 0);
    assert_eq!(
        graph.route_support()[0].state,
        MercatorSupportState::Withdrawn
    );
    assert_eq!(graph.route_support()[0].stale_started_at, Some(Tick(12)));
    assert_eq!(graph.diagnostics().support_withdrawal_count, 6);
}

#[test]
fn epoch_pre_disruption_loss_is_not_counted_as_post_disruption_stale_persistence() {
    let mut graph = MercatorEvidenceGraph::new(tight_bounds());
    let mut lost_before_disruption = support(3, MercatorSupportState::Withdrawn, 0);
    lost_before_disruption.last_loss_epoch = Some(RouteEpoch(2));
    lost_before_disruption.stale_started_at = None;
    graph.record_route_support(lost_before_disruption);

    graph.invalidate_disruption_epoch(RouteEpoch(3), Tick(12));

    assert_eq!(graph.diagnostics().stale_persistence_rounds, 0);
    assert_eq!(graph.route_support()[0].stale_started_at, None);
}

#[test]
fn evidence_support_state_pruning_prefers_fresh_and_repairing_support() {
    let mut graph = MercatorEvidenceGraph::new(tight_bounds());
    graph.record_route_support(support(1, MercatorSupportState::Withdrawn, 900));
    graph.record_route_support(support(2, MercatorSupportState::Suspect, 100));
    graph.record_route_support(support(3, MercatorSupportState::Repairing, 200));
    graph.record_route_support(support(4, MercatorSupportState::Fresh, 150));

    let kept = graph
        .route_support()
        .into_iter()
        .map(|support| support.state)
        .collect::<Vec<_>>();

    assert_eq!(
        kept,
        vec![MercatorSupportState::Repairing, MercatorSupportState::Fresh]
    );
}
