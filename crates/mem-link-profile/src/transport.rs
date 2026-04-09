//! In-memory transport adapter for reference composition and tests.
//!
//! This module provides a concrete in-memory implementation of the shared
//! `TransportEffects` capability. `InMemoryTransport` attaches one local node
//! to a [`SharedInMemoryNetwork`](crate::SharedInMemoryNetwork), records sent
//! frames for inspection, and replays ingress observations through
//! `poll_transport()` at a configurable tick.
//!
//! It is intentionally reference-only and in-memory. It exists to support
//! tests, examples, and the reference client, not to model a production
//! transport backend.

use jacquard_core::{LinkEndpoint, NodeId, Tick, TransportError, TransportObservation};
use jacquard_traits::{effect_handler, TransportEffects};

use crate::network::SharedInMemoryNetwork;

/// In-memory transport adapter backed by one shared network.
pub struct InMemoryTransport {
    local_node_id: Option<NodeId>,
    ingress_tick: Tick,
    network: Option<SharedInMemoryNetwork>,
    pub sent_frames: Vec<(jacquard_core::LinkEndpoint, Vec<u8>)>,
    pub observations: Vec<TransportObservation>,
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
            ingress_tick: Tick(0),
            network: None,
            sent_frames: Vec::new(),
            observations: Vec::new(),
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
            ingress_tick: Tick(0),
            network: Some(network),
            sent_frames: Vec::new(),
            observations: Vec::new(),
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

    pub fn set_ingress_tick(&mut self, tick: Tick) {
        self.ingress_tick = tick;
    }
}

#[effect_handler]
impl TransportEffects for InMemoryTransport {
    fn send_transport(
        &mut self,
        endpoint: &jacquard_core::LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.sent_frames.push((endpoint.clone(), payload.to_vec()));
        if let (Some(network), Some(local_node_id)) =
            (&self.network, self.local_node_id)
        {
            network.deliver(
                local_node_id,
                endpoint.clone(),
                payload.to_vec(),
                self.ingress_tick,
            );
        }
        Ok(())
    }

    fn poll_transport(&mut self) -> Result<Vec<TransportObservation>, TransportError> {
        if let (Some(network), Some(local_node_id)) =
            (&self.network, self.local_node_id)
        {
            self.observations.extend(network.take_for(local_node_id));
        }
        Ok(std::mem::take(&mut self.observations))
    }
}
