//! In-memory transport adapter for reference composition and tests.
//!
//! This module provides a concrete in-memory implementation of the shared
//! transport sender capability plus the host-owned transport driver surface.
//! `InMemoryTransport` attaches one local node to a
//! [`SharedInMemoryNetwork`](crate::SharedInMemoryNetwork), records sent frames
//! for inspection, and exposes raw ingress events for the host bridge to stamp
//! with Jacquard time.
//!
//! It is intentionally reference-only and in-memory. It exists to support
//! tests, examples, and the reference client, not to model a production
//! transport backend.

use jacquard_core::{LinkEndpoint, NodeId, TransportError, TransportIngressEvent};
use jacquard_traits::{effect_handler, TransportDriver, TransportSenderEffects};

use crate::network::SharedInMemoryNetwork;

/// In-memory transport adapter backed by one shared network.
pub struct InMemoryTransport {
    local_node_id: Option<NodeId>,
    network: Option<SharedInMemoryNetwork>,
    pub sent_frames: Vec<(jacquard_core::LinkEndpoint, Vec<u8>)>,
    pub ingress_events: Vec<TransportIngressEvent>,
}

impl Default for InMemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryTransport {
    #[must_use]
    pub fn new() -> Self {
        Self {
            local_node_id: None,
            network: None,
            sent_frames: Vec::new(),
            ingress_events: Vec::new(),
        }
    }

    #[must_use]
    pub fn attach(
        local_node_id: NodeId,
        endpoints: impl IntoIterator<Item = LinkEndpoint>,
        network: SharedInMemoryNetwork,
    ) -> Self {
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
            ingress_events: Vec::new(),
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
            self.ingress_events.extend(network.take_for(local_node_id));
        }
        Ok(std::mem::take(&mut self.ingress_events))
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
}
