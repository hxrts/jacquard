//! Runtime adapter mocks for the mesh integration tests.
//!
//! `TestRuntimeEffects` aggregates the time, ordering, storage, and
//! route-event-log effect handlers behind one struct so a `MeshEngine`
//! can be wired up with a single value. `TestTransport` and
//! `TestRetentionStore` are the matching mesh-side effectful
//! subcomponents.

use std::collections::BTreeMap;

use jacquard_traits::{
    effect_handler,
    jacquard_core::{
        Blake3Digest, ContentId, LinkEndpoint, OrderStamp, RetentionError,
        RouteEventLogError, RouteEventStamped, StorageError, Tick, TransportError,
        TransportObservation, TransportProtocol,
    },
    MeshFrame, MeshTransport, OrderEffects, RetentionStore, RouteEventLogEffects,
    StorageEffects, TimeEffects,
};

#[derive(Default)]
pub struct TestRuntimeEffects {
    pub now: Tick,
    pub next_order: u64,
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub store_bytes_call_count: u32,
    pub events: Vec<RouteEventStamped>,
    pub fail_store_bytes: bool,
    pub fail_record_route_event: bool,
}

#[effect_handler]
impl TimeEffects for TestRuntimeEffects {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

#[effect_handler]
impl OrderEffects for TestRuntimeEffects {
    fn next_order_stamp(&mut self) -> OrderStamp {
        self.next_order += 1;
        OrderStamp(self.next_order)
    }
}

#[effect_handler]
impl StorageEffects for TestRuntimeEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.storage.get(key).cloned())
    }

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        if self.fail_store_bytes {
            return Err(StorageError::Unavailable);
        }
        self.store_bytes_call_count = self.store_bytes_call_count.saturating_add(1);
        self.storage.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.storage.remove(key);
        Ok(())
    }
}

#[effect_handler]
impl RouteEventLogEffects for TestRuntimeEffects {
    fn record_route_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), RouteEventLogError> {
        if self.fail_record_route_event {
            return Err(RouteEventLogError::Unavailable);
        }
        self.events.push(event);
        Ok(())
    }
}

#[derive(Default)]
pub struct TestTransport {
    pub sent_frames: Vec<(LinkEndpoint, Vec<u8>)>,
    pub observations: Vec<TransportObservation>,
}

impl MeshTransport for TestTransport {
    fn transport_id(&self) -> TransportProtocol {
        TransportProtocol::BleGatt
    }

    fn send_frame(&mut self, frame: MeshFrame<'_>) -> Result<(), TransportError> {
        self.sent_frames
            .push((frame.endpoint.clone(), frame.payload.to_vec()));
        Ok(())
    }

    fn poll_observations(
        &mut self,
    ) -> Result<Vec<TransportObservation>, TransportError> {
        Ok(std::mem::take(&mut self.observations))
    }
}

#[derive(Default)]
pub struct TestRetentionStore {
    pub payloads: BTreeMap<ContentId<Blake3Digest>, Vec<u8>>,
}

impl RetentionStore for TestRetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError> {
        self.payloads.insert(object_id, payload);
        Ok(())
    }

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError> {
        Ok(self.payloads.remove(object_id))
    }

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError> {
        Ok(self.payloads.contains_key(object_id))
    }
}
