//! In-memory transport, retention, and runtime-effect harnesses for tests.
//!
//! Control flow intuition: these types carry bytes, store retained payloads,
//! and record deterministic runtime effects without inventing routing
//! semantics. They are intended for router/mesh/device integration tests and
//! examples, not as production transports.
//!
//! Ownership:
//! - `Observed`: carrier and observation surface only
//! - never mints canonical route truth or repairs canonical state

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use jacquard_core::{
    Blake3Digest, ContentId, LinkEndpoint, NodeId, OrderStamp, RetentionError,
    RouteEventLogError, RouteEventStamped, StorageError, Tick, TransportError,
    TransportObservation, TransportProtocol,
};
use jacquard_traits::{
    effect_handler, MeshFrame, MeshTransport, OrderEffects, RetentionStore,
    RouteEventLogEffects, StorageEffects, TimeEffects,
};

#[derive(Default)]
struct SharedNetworkState {
    endpoint_owners: BTreeMap<LinkEndpoint, NodeId>,
    inboxes:         BTreeMap<NodeId, Vec<TransportObservation>>,
}

#[derive(Clone, Default)]
pub struct SharedInMemoryMeshNetwork {
    inner: Arc<Mutex<SharedNetworkState>>,
}

impl SharedInMemoryMeshNetwork {
    pub fn attach_endpoint(&self, node_id: NodeId, endpoint: LinkEndpoint) {
        let mut guard = self.inner.lock().expect("shared network lock");
        guard.endpoint_owners.insert(endpoint, node_id);
    }

    fn deliver(
        &self,
        from_node_id: NodeId,
        endpoint: LinkEndpoint,
        payload: Vec<u8>,
        observed_at_tick: Tick,
    ) {
        let mut guard = self.inner.lock().expect("shared network lock");
        if let Some(remote_node_id) = guard.endpoint_owners.get(&endpoint).copied() {
            guard.inboxes.entry(remote_node_id).or_default().push(
                TransportObservation::PayloadReceived {
                    from_node_id,
                    endpoint,
                    payload,
                    observed_at_tick,
                },
            );
        }
    }

    fn take_for(&self, node_id: NodeId) -> Vec<TransportObservation> {
        let mut guard = self.inner.lock().expect("shared network lock");
        guard.inboxes.remove(&node_id).unwrap_or_default()
    }
}

pub struct InMemoryMeshTransport {
    transport_id:     TransportProtocol,
    local_node_id:    Option<NodeId>,
    ingress_tick:     Tick,
    network:          Option<SharedInMemoryMeshNetwork>,
    pub sent_frames:  Vec<(LinkEndpoint, Vec<u8>)>,
    pub observations: Vec<TransportObservation>,
}

impl Default for InMemoryMeshTransport {
    fn default() -> Self {
        Self::new(TransportProtocol::BleGatt)
    }
}

impl InMemoryMeshTransport {
    #[must_use]
    pub fn new(transport_id: TransportProtocol) -> Self {
        Self {
            transport_id,
            local_node_id: None,
            ingress_tick: Tick(0),
            network: None,
            sent_frames: Vec::new(),
            observations: Vec::new(),
        }
    }

    #[must_use]
    pub fn attached(
        transport_id: TransportProtocol,
        local_node_id: NodeId,
        endpoints: impl IntoIterator<Item = LinkEndpoint>,
        network: SharedInMemoryMeshNetwork,
    ) -> Self {
        let endpoints = endpoints.into_iter().collect::<Vec<_>>();
        for endpoint in &endpoints {
            network.attach_endpoint(local_node_id, endpoint.clone());
        }

        Self {
            transport_id,
            local_node_id: Some(local_node_id),
            ingress_tick: Tick(0),
            network: Some(network),
            sent_frames: Vec::new(),
            observations: Vec::new(),
        }
    }

    pub fn set_ingress_tick(&mut self, tick: Tick) {
        self.ingress_tick = tick;
    }
}

impl MeshTransport for InMemoryMeshTransport {
    fn transport_id(&self) -> TransportProtocol {
        self.transport_id.clone()
    }

    fn send_frame(&mut self, frame: MeshFrame<'_>) -> Result<(), TransportError> {
        self.sent_frames
            .push((frame.endpoint.clone(), frame.payload.to_vec()));
        if let (Some(network), Some(local_node_id)) =
            (&self.network, self.local_node_id)
        {
            network.deliver(
                local_node_id,
                frame.endpoint.clone(),
                frame.payload.to_vec(),
                self.ingress_tick,
            );
        }
        Ok(())
    }

    fn poll_observations(
        &mut self,
    ) -> Result<Vec<TransportObservation>, TransportError> {
        if let (Some(network), Some(local_node_id)) =
            (&self.network, self.local_node_id)
        {
            self.observations.extend(network.take_for(local_node_id));
        }
        Ok(std::mem::take(&mut self.observations))
    }
}

#[derive(Default)]
pub struct InMemoryRetentionStore {
    pub payloads: BTreeMap<ContentId<Blake3Digest>, Vec<u8>>,
}

impl RetentionStore for InMemoryRetentionStore {
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

#[derive(Clone, Default)]
pub struct InMemoryRuntimeEffects {
    pub now:                     Tick,
    pub next_order:              u64,
    pub storage:                 BTreeMap<Vec<u8>, Vec<u8>>,
    pub store_bytes_call_count:  u32,
    pub events:                  Vec<RouteEventStamped>,
    pub fail_store_bytes:        bool,
    pub fail_record_route_event: bool,
}

#[effect_handler]
impl TimeEffects for InMemoryRuntimeEffects {
    fn now_tick(&self) -> Tick {
        self.now
    }
}

#[effect_handler]
impl OrderEffects for InMemoryRuntimeEffects {
    fn next_order_stamp(&mut self) -> OrderStamp {
        self.next_order = self.next_order.saturating_add(1);
        OrderStamp(self.next_order)
    }
}

#[effect_handler]
impl StorageEffects for InMemoryRuntimeEffects {
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
impl RouteEventLogEffects for InMemoryRuntimeEffects {
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
