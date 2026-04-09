//! In-memory transport adapter for reference composition and tests.
//!
//! This module provides a concrete in-memory implementation of the shared
//! transport sender capability plus the host-owned transport driver surface.
//! `InMemoryTransport` attaches one local node to a
//! [`SharedInMemoryNetwork`](crate::SharedInMemoryNetwork), records sent frames
//! for inspection, and exposes raw ingress events through the shared
//! `jacquard-adapter` mailbox primitives so the host bridge can stamp them with
//! Jacquard time.
//!
//! It is intentionally reference-only and in-memory. It exists to support
//! tests, examples, and the reference client, not to model a production
//! transport backend.

use jacquard_adapter::{
    transport_ingress_mailbox, TransportIngressClass, TransportIngressReceiver,
    TransportIngressSender,
};
use jacquard_core::{LinkEndpoint, NodeId, TransportError, TransportIngressEvent};
use jacquard_traits::{effect_handler, TransportDriver, TransportSenderEffects};

use crate::network::SharedInMemoryNetwork;

const DEFAULT_TRANSPORT_INGRESS_CAPACITY: usize = 1024;

/// In-memory transport adapter backed by one shared network.
pub struct InMemoryTransport {
    local_node_id: Option<NodeId>,
    network: Option<SharedInMemoryNetwork>,
    pub sent_frames: Vec<(jacquard_core::LinkEndpoint, Vec<u8>)>,
    ingress_sender: TransportIngressSender,
    ingress_receiver: TransportIngressReceiver,
}

impl Default for InMemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTransport {
    #[must_use]
    pub fn new() -> Self {
        let (ingress_sender, ingress_receiver, _) =
            transport_ingress_mailbox(DEFAULT_TRANSPORT_INGRESS_CAPACITY);
        Self {
            local_node_id: None,
            network: None,
            sent_frames: Vec::new(),
            ingress_sender,
            ingress_receiver,
        }
    }

    #[must_use]
    pub fn attach(
        local_node_id: NodeId,
        endpoints: impl IntoIterator<Item = LinkEndpoint>,
        network: SharedInMemoryNetwork,
    ) -> Self {
        let (ingress_sender, ingress_receiver, _) =
            transport_ingress_mailbox(DEFAULT_TRANSPORT_INGRESS_CAPACITY);
        let endpoints = endpoints.into_iter().collect::<Vec<_>>();
        if let Some(first_endpoint) = endpoints.first() {
            debug_assert!(endpoints.iter().all(
                |endpoint| endpoint.transport_kind == first_endpoint.transport_kind
            ));
        }
        for endpoint in &endpoints {
            network.attach_endpoint(local_node_id, endpoint.clone());
        }

        Self {
            local_node_id: Some(local_node_id),
            network: Some(network),
            sent_frames: Vec::new(),
            ingress_sender,
            ingress_receiver,
        }
    }

    #[must_use]
    pub fn attached(
        local_node_id: NodeId,
        endpoints: impl IntoIterator<Item = LinkEndpoint>,
        network: SharedInMemoryNetwork,
    ) -> Self {
        Self::attach(local_node_id, endpoints, network)
    }

    /// Inject a raw ingress event directly into this transport's mailbox.
    ///
    /// Used by test harnesses that need to simulate incoming transport
    /// observations without going through the network layer.
    pub fn push_ingress_event(&mut self, event: TransportIngressEvent) {
        let _ = self
            .ingress_sender
            .emit(TransportIngressClass::Payload, event)
            .expect("push ingress event to in-memory transport mailbox");
    }
}

#[effect_handler]
impl TransportSenderEffects for InMemoryTransport {
    fn send_transport(
        &mut self,
        endpoint: &jacquard_core::LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.sent_frames.push((endpoint.clone(), payload.to_vec()));
        if let (Some(network), Some(local_node_id)) =
            (&self.network, self.local_node_id)
        {
            network.deliver(local_node_id, endpoint.clone(), payload.to_vec());
        }
        Ok(())
    }
}

impl TransportDriver for InMemoryTransport {
    fn drain_transport_ingress(
        &mut self,
    ) -> Result<Vec<TransportIngressEvent>, TransportError> {
        if let (Some(network), Some(local_node_id)) =
            (&self.network, self.local_node_id)
        {
            for event in network.take_for(local_node_id) {
                let _ = self
                    .ingress_sender
                    .emit(TransportIngressClass::Payload, event)
                    .expect("in-memory transport payload ingress mailbox");
            }
        }
        Ok(self.ingress_receiver.drain().events)
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        ByteCount, EndpointLocator, NodeId, TransportIngressEvent, TransportKind,
    };
    use jacquard_traits::{TransportDriver, TransportSenderEffects};

    use super::{InMemoryTransport, SharedInMemoryNetwork};

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
            | TransportIngressEvent::PayloadReceived {
                from_node_id, payload, ..
            } => {
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
}
