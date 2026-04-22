//! In-memory transport adapter for reference composition and tests.
//!
//! This module provides a concrete in-memory implementation of the shared
//! transport sender capability plus the host-owned transport driver surface.
//! `InMemoryTransport` attaches one local node to a
//! [`SharedInMemoryNetwork`](crate::SharedInMemoryNetwork), records sent frames
//! for inspection, and exposes raw ingress events through the shared
//! `jacquard-host-support` mailbox primitives so the host bridge can stamp them with
//! Jacquard time.
//!
//! It is intentionally reference-only and in-memory. It exists to support
//! tests, examples, and the reference client, not to model a production
//! transport backend.
//!
//! Non-unicast `send_transport_to` calls are recorded in `sent_intents` with
//! their requested delivery mode. The backing `SharedInMemoryNetwork` still
//! delivers to the single endpoint embedded in the intent; it does not model
//! multicast or broadcast fanout.

use jacquard_core::{
    LinkEndpoint, NodeId, TransportDeliveryIntent, TransportError, TransportIngressEvent,
};
use jacquard_host_support::{
    transport_ingress_mailbox, TransportIngressClass, TransportIngressReceiver,
    TransportIngressSender,
};
use jacquard_traits::{effect_handler, TransportDriver, TransportSenderEffects};

use crate::network::SharedInMemoryNetwork;

const DEFAULT_TRANSPORT_INGRESS_CAPACITY: usize = 1024;

/// In-memory transport adapter backed by one shared network.
pub struct InMemoryTransport {
    local_node_id: Option<NodeId>,
    network: Option<SharedInMemoryNetwork>,
    pub sent_frames: Vec<(jacquard_core::LinkEndpoint, Vec<u8>)>,
    pub sent_intents: Vec<(TransportDeliveryIntent, Vec<u8>)>,
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
            sent_intents: Vec::new(),
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
            debug_assert!(endpoints
                .iter()
                .all(|endpoint| endpoint.transport_kind == first_endpoint.transport_kind));
        }
        for endpoint in &endpoints {
            network.attach_endpoint(local_node_id, endpoint.clone());
        }

        Self {
            local_node_id: Some(local_node_id),
            network: Some(network),
            sent_frames: Vec::new(),
            sent_intents: Vec::new(),
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
        self.ingress_sender
            .emit(TransportIngressClass::Payload, event)
            .expect("push ingress event to in-memory transport mailbox");
    }

    fn send_intent(&mut self, intent: &TransportDeliveryIntent, payload: &[u8]) {
        let endpoint = intent.endpoint().clone();
        self.sent_frames.push((endpoint.clone(), payload.to_vec()));
        self.sent_intents.push((intent.clone(), payload.to_vec()));
        if let (Some(network), Some(local_node_id)) = (&self.network, self.local_node_id) {
            network.deliver(local_node_id, endpoint, payload.to_vec());
        }
    }
}

#[effect_handler]
impl TransportSenderEffects for InMemoryTransport {
    fn send_transport(
        &mut self,
        endpoint: &jacquard_core::LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.send_intent(&TransportDeliveryIntent::unicast(endpoint.clone()), payload);
        Ok(())
    }

    fn send_transport_to(
        &mut self,
        intent: &TransportDeliveryIntent,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.send_intent(intent, payload);
        Ok(())
    }
}

impl TransportDriver for InMemoryTransport {
    fn drain_transport_ingress(&mut self) -> Result<Vec<TransportIngressEvent>, TransportError> {
        if let (Some(network), Some(local_node_id)) = (&self.network, self.local_node_id) {
            for event in network.take_for(local_node_id) {
                self.ingress_sender
                    .emit(TransportIngressClass::Payload, event)
                    .expect("in-memory transport payload ingress mailbox");
            }
        }
        Ok(self.ingress_receiver.drain().events)
    }
}
