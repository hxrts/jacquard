//! `SharedInMemoryNetwork`, the in-memory carrier fabric that multiple
//! `InMemoryTransport` instances attach to.
//!
//! This module owns the shared routing table that maps `LinkEndpoint` values to
//! `NodeId` owners, and maintains per-node inbox queues for in-flight frames.
//! When one `InMemoryTransport` sends a frame to a known endpoint, the network
//! places a `TransportIngressEvent::PayloadReceived` into the destination
//! node's inbox. The receiving transport drains that inbox on the next call to
//! `drain_transport_ingress`.
//!
//! The network is cloneable and reference-counted (`Arc<Mutex<_>>`) so multiple
//! transport handles can share it across a test without unsafe aliasing. All
//! internal state is protected by a single `Mutex` for deterministic ordering.
//!
//! Used by test harnesses, fixtures, and the reference client to compose
//! several simulated device runtimes without requiring a real radio or socket
//! layer.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use jacquard_core::{LinkEndpoint, NodeId, TransportIngressEvent};

#[derive(Default)]
struct SharedNetworkState {
    endpoint_owners: BTreeMap<LinkEndpoint, NodeId>,
    inboxes: BTreeMap<NodeId, Vec<TransportIngressEvent>>,
}

/// Shared in-memory observation network used by frame-carrier tests.
#[derive(Clone, Default)]
pub struct SharedInMemoryNetwork {
    inner: Arc<Mutex<SharedNetworkState>>,
}

impl SharedInMemoryNetwork {
    pub fn attach_endpoint(&self, node_id: NodeId, endpoint: LinkEndpoint) {
        let mut guard = self.inner.lock().expect("shared network lock");
        guard.endpoint_owners.insert(endpoint, node_id);
    }

    pub(crate) fn deliver(
        &self,
        from_node_id: NodeId,
        endpoint: LinkEndpoint,
        payload: Vec<u8>,
    ) {
        let mut guard = self.inner.lock().expect("shared network lock");
        if let Some(remote_node_id) = guard.endpoint_owners.get(&endpoint).copied() {
            guard.inboxes.entry(remote_node_id).or_default().push(
                TransportIngressEvent::PayloadReceived {
                    from_node_id,
                    endpoint,
                    payload,
                },
            );
        }
    }

    pub(crate) fn take_for(&self, node_id: NodeId) -> Vec<TransportIngressEvent> {
        let mut guard = self.inner.lock().expect("shared network lock");
        guard.inboxes.remove(&node_id).unwrap_or_default()
    }
}
