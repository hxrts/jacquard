use jacquard_mem_link_profile::{InMemoryRetentionStore, InMemoryTransport};
use jacquard_traits::{
    jacquard_core::{
        Blake3Digest, ByteCount, ContentId, EndpointLocator, LinkEndpoint,
        TransportKind,
    },
    EffectHandler, RetentionStore, TransportEffects,
};

fn sample_endpoint() -> LinkEndpoint {
    LinkEndpoint {
        transport_kind: TransportKind::WifiAware,
        locator: EndpointLocator::Opaque(vec![1, 2]),
        mtu_bytes: ByteCount(512),
    }
}

#[test]
fn transport_effects_send_and_poll_without_engine_specific_traits() {
    let endpoint = sample_endpoint();
    let mut transport = InMemoryTransport::default();

    transport
        .send_transport(&endpoint, b"frame")
        .expect("send transport payload");
    let observations = transport
        .poll_transport()
        .expect("poll transport observations");

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
fn transport_effect_handlers_do_not_require_engine_specific_traits() {
    fn assert_transport_handler<T>()
    where
        T: TransportEffects + EffectHandler<dyn TransportEffects>,
    {
    }

    assert_transport_handler::<InMemoryTransport>();
}
