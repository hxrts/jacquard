//! Shared in-memory runtime handles used by the pathway integration tests.
//!
//! The integration tests should exercise the real pathway public boundary, not
//! engine-private escape hatches. These wrappers keep the concrete in-memory
//! adapters shared with the reference client, while exposing separate test
//! handles so assertions can inspect sent frames, stored bytes, and retained
//! payloads without requiring public pathway-engine getters for those
//! internals.

const EFFECTS_LOCK: &str = "test effects lock";
const TRANSPORT_LOCK: &str = "test transport lock";
const RETENTION_LOCK: &str = "test retention lock";

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use jacquard_core::{
    Blake3Digest, ContentId, OrderStamp, RetentionError, RouteEventLogError,
    RouteEventStamped, StorageError, Tick, TransportError, TransportIngressEvent,
    TransportObservation,
};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
};
use jacquard_traits::{
    effect_handler, OrderEffects, RetentionStore, RouteEventLogEffects, StorageEffects,
    TimeEffects, TransportSenderEffects,
};

#[derive(Clone, Default)]
pub struct TestTransport(Arc<Mutex<InMemoryTransport>>);

impl TestTransport {
    pub fn push_observation(&self, observation: TransportObservation) {
        let event = match observation {
            | TransportObservation::PayloadReceived {
                from_node_id,
                endpoint,
                payload,
                ..
            } => TransportIngressEvent::PayloadReceived {
                from_node_id,
                endpoint,
                payload,
            },
            | TransportObservation::LinkObserved { remote_node_id, observation } => {
                TransportIngressEvent::LinkObserved {
                    remote_node_id,
                    link: observation.value,
                    source_class: observation.source_class,
                    evidence_class: observation.evidence_class,
                    origin_authentication: observation.origin_authentication,
                }
            },
        };
        self.0
            .lock()
            .expect(TRANSPORT_LOCK)
            .push_ingress_event(event);
    }

    #[must_use]
    pub fn sent_frames(&self) -> Vec<(jacquard_core::LinkEndpoint, Vec<u8>)> {
        self.0.lock().expect(TRANSPORT_LOCK).sent_frames.clone()
    }
}

#[effect_handler]
impl TransportSenderEffects for TestTransport {
    fn send_transport(
        &mut self,
        endpoint: &jacquard_core::LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.0
            .lock()
            .expect(TRANSPORT_LOCK)
            .send_transport(endpoint, payload)
    }
}

#[derive(Clone, Default)]
pub struct TestRuntimeEffects(Arc<Mutex<InMemoryRuntimeEffects>>);

impl TestRuntimeEffects {
    #[must_use]
    pub fn with_now(now: Tick) -> Self {
        let effects = Self::default();
        effects.set_now(now);
        effects
    }

    pub fn set_now(&self, now: Tick) {
        self.0.lock().expect(EFFECTS_LOCK).now = now;
    }

    pub fn set_fail_store_bytes(&self, fail: bool) {
        self.0.lock().expect(EFFECTS_LOCK).fail_store_bytes = fail;
    }

    pub fn set_fail_record_route_event(&self, fail: bool) {
        self.0.lock().expect(EFFECTS_LOCK).fail_record_route_event = fail;
    }

    #[must_use]
    pub fn events(&self) -> Vec<RouteEventStamped> {
        self.0.lock().expect(EFFECTS_LOCK).events.clone()
    }

    #[must_use]
    pub fn storage_clone(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        self.0.lock().expect(EFFECTS_LOCK).storage.clone()
    }

    pub fn replace_storage(&self, storage: BTreeMap<Vec<u8>, Vec<u8>>) {
        self.0.lock().expect(EFFECTS_LOCK).storage = storage;
    }

    #[must_use]
    pub fn storage_value(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.lock().expect(EFFECTS_LOCK).storage.get(key).cloned()
    }

    #[must_use]
    pub fn storage_keys(&self) -> Vec<Vec<u8>> {
        self.0
            .lock()
            .expect(EFFECTS_LOCK)
            .storage
            .keys()
            .cloned()
            .collect()
    }
}

#[effect_handler]
impl TimeEffects for TestRuntimeEffects {
    fn now_tick(&self) -> Tick {
        self.0.lock().expect(EFFECTS_LOCK).now_tick()
    }
}

#[effect_handler]
impl OrderEffects for TestRuntimeEffects {
    fn next_order_stamp(&mut self) -> OrderStamp {
        self.0.lock().expect(EFFECTS_LOCK).next_order_stamp()
    }
}

#[effect_handler]
impl StorageEffects for TestRuntimeEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        self.0.lock().expect(EFFECTS_LOCK).load_bytes(key)
    }

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.0.lock().expect(EFFECTS_LOCK).store_bytes(key, value)
    }

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.0.lock().expect(EFFECTS_LOCK).remove_bytes(key)
    }
}

#[effect_handler]
impl RouteEventLogEffects for TestRuntimeEffects {
    fn record_route_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), RouteEventLogError> {
        self.0.lock().expect(EFFECTS_LOCK).record_route_event(event)
    }
}

#[derive(Clone, Default)]
pub struct TestRetentionStore(Arc<Mutex<InMemoryRetentionStore>>);

impl TestRetentionStore {
    #[must_use]
    pub fn payload_count(&self) -> usize {
        self.0.lock().expect(RETENTION_LOCK).payloads.len()
    }
}

#[effect_handler]
impl RetentionStore for TestRetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError> {
        self.0
            .lock()
            .expect(RETENTION_LOCK)
            .retain_payload(object_id, payload)
    }

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError> {
        self.0
            .lock()
            .expect(RETENTION_LOCK)
            .take_retained_payload(object_id)
    }

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError> {
        self.0
            .lock()
            .expect(RETENTION_LOCK)
            .contains_retained_payload(object_id)
    }
}
