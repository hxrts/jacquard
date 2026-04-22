use jacquard_core::{
    BroadcastDomainId, ByteCount, DeliveryCompatibilityPolicy, DeliveryCoverageObjective,
    EndpointLocator, MulticastGroupId, NodeId, RouteAdmissionRejection, RouteDeliveryObjective,
    TransportDeliveryMode, TransportDeliverySupport, TransportKind,
};
use jacquard_router::admitted_delivery_intent;

fn node(byte: u8) -> NodeId {
    NodeId([byte; 32])
}

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    jacquard_core::LinkEndpoint::new(
        TransportKind::BleGatt,
        EndpointLocator::Opaque(vec![byte]),
        ByteCount(128),
    )
}

#[test]
fn router_delivery_admission_rejects_fanout_for_node_unicast() {
    let objective = RouteDeliveryObjective::unicast_node(node(2));
    let support = TransportDeliverySupport::Multicast {
        endpoint: endpoint(9),
        group_id: MulticastGroupId([1; 16]),
        receivers: vec![node(2), node(3)],
    };

    let result = admitted_delivery_intent(
        &objective,
        &support,
        DeliveryCompatibilityPolicy::ExactDeliveryOnly,
    );

    assert_eq!(
        result,
        Err(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
    );
}

#[test]
fn router_delivery_admission_preserves_multicast_intent_after_compatibility() {
    let objective = RouteDeliveryObjective::MulticastGroup {
        group_id: MulticastGroupId([4; 16]),
        receivers: vec![node(2), node(3)],
    };
    let support = TransportDeliverySupport::Multicast {
        endpoint: endpoint(9),
        group_id: MulticastGroupId([4; 16]),
        receivers: vec![node(2), node(3)],
    };

    let intent = admitted_delivery_intent(
        &objective,
        &support,
        DeliveryCompatibilityPolicy::ExactDeliveryOnly,
    )
    .expect("admit multicast support");

    assert_eq!(intent.mode(), TransportDeliveryMode::Multicast);
}

#[test]
fn router_delivery_admission_requires_policy_for_lossy_broadcast() {
    let objective = RouteDeliveryObjective::broadcast_domain(
        BroadcastDomainId([5; 16]),
        [node(2), node(3)],
        DeliveryCoverageObjective::AllReceivers,
    );
    let support = TransportDeliverySupport::Broadcast {
        endpoint: endpoint(9),
        domain_id: BroadcastDomainId([5; 16]),
        receivers: vec![node(2), node(3)],
        reverse_confirmation: jacquard_core::ReverseDeliveryConfirmation::Unconfirmed,
    };

    assert_eq!(
        admitted_delivery_intent(
            &objective,
            &support,
            DeliveryCompatibilityPolicy::ExactDeliveryOnly
        ),
        Err(RouteAdmissionRejection::DeliveryAssumptionUnsupported)
    );
    assert_eq!(
        admitted_delivery_intent(
            &objective,
            &support,
            DeliveryCompatibilityPolicy::AllowLossyBroadcast,
        )
        .expect("admit lossy broadcast")
        .mode(),
        TransportDeliveryMode::Broadcast
    );
}
