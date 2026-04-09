use jacquard_core::{
    ByteCount, EndpointLocator, NodeId, TransportIngressEvent, TransportKind,
};
use jacquard_mem_link_profile::{InMemoryTransport, SharedInMemoryNetwork};
use jacquard_traits::{TransportDriver, TransportSenderEffects};

fn endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    jacquard_core::LinkEndpoint::new(
        TransportKind::WifiAware,
        EndpointLocator::Opaque(vec![byte]),
        ByteCount(128),
    )
}

#[test]
fn sender_capability_and_driver_ingress_are_owned_separately() {
    let network = SharedInMemoryNetwork::default();
    let mut sender =
        InMemoryTransport::attach(NodeId([1; 32]), [endpoint(1)], network.clone());
    let mut receiver =
        InMemoryTransport::attach(NodeId([2; 32]), [endpoint(2)], network);

    sender
        .send_transport(&endpoint(2), b"frame")
        .expect("send transport frame");

    let ingress = receiver
        .drain_transport_ingress()
        .expect("drain transport ingress");

    assert_eq!(ingress.len(), 1);
    match &ingress[0] {
        | TransportIngressEvent::PayloadReceived { from_node_id, payload, .. } => {
            assert_eq!(from_node_id, &NodeId([1; 32]));
            assert_eq!(payload, b"frame");
        },
        | other => panic!("unexpected ingress event: {other:?}"),
    }
}

#[test]
fn shared_adapter_mailbox_drains_multiple_ingress_frames() {
    let network = SharedInMemoryNetwork::default();
    let mut sender =
        InMemoryTransport::attach(NodeId([1; 32]), [endpoint(1)], network.clone());
    let mut receiver =
        InMemoryTransport::attach(NodeId([2; 32]), [endpoint(2)], network);

    sender
        .send_transport(&endpoint(2), b"first")
        .expect("send first transport frame");
    sender
        .send_transport(&endpoint(2), b"second")
        .expect("send second transport frame");

    let ingress = receiver
        .drain_transport_ingress()
        .expect("drain transport ingress");

    assert_eq!(ingress.len(), 2);
}
