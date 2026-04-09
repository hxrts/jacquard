use jacquard_mem_link_profile::{InMemoryRetentionStore, InMemoryTransport};
use jacquard_traits::{
    jacquard_core::{
        Blake3Digest, ByteCount, ContentId, EndpointLocator, LinkEndpoint,
        TransportKind,
    },
    EffectHandler, RetentionStore, TransportDriver, TransportSenderEffects,
};

fn sample_endpoint() -> LinkEndpoint {
    LinkEndpoint {
        transport_kind: TransportKind::WifiAware,
        locator: EndpointLocator::Opaque(vec![1, 2]),
        mtu_bytes: ByteCount(512),
    }
}

#[test]
fn transport_sender_and_driver_split_without_engine_specific_traits() {
    let endpoint = sample_endpoint();
    let mut transport = InMemoryTransport::default();

    transport
        .send_transport(&endpoint, b"frame")
        .expect("send transport payload");
    let observations = transport
        .drain_transport_ingress()
        .expect("drain transport ingress");

    assert!(observations.is_empty());
    assert_eq!(transport.sent_frames, vec![(endpoint, b"frame".to_vec())]);
}

#[test]
fn retention_store_retains_and_releases_opaque_payloads() {
    let object_id = ContentId { digest: Blake3Digest([7; 32]) };
    let mut retention = InMemoryRetentionStore::default();

    retention
        .retain_payload(object_id, b"payload".to_vec())
        .expect("put payload");
    assert!(retention
        .contains_retained_payload(&object_id)
        .expect("contains payload"));

    let payload = retention
        .take_retained_payload(&object_id)
        .expect("take payload");
    assert_eq!(payload, Some(b"payload".to_vec()));
    assert!(!retention
        .contains_retained_payload(&object_id)
        .expect("payload removed"));
}

#[test]
fn transport_sender_effect_handlers_do_not_require_engine_specific_traits() {
    fn assert_transport_handler<T>()
    where
        T: TransportSenderEffects + EffectHandler<dyn TransportSenderEffects>,
    {
    }

    assert_transport_handler::<InMemoryTransport>();
}

#[test]
fn transport_driver_stays_outside_effect_handler_vocabulary() {
    fn assert_transport_driver<T: TransportDriver>() {}

    assert_transport_driver::<InMemoryTransport>();
}
