//! `SharedInMemoryNetwork`, the in-memory carrier fabric that multiple
//! `InMemoryTransport` instances attach to. Owns endpoint-to-node ownership
//! and per-node inbox queues so tests can compose several device runtimes
//! without a real radio.

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
