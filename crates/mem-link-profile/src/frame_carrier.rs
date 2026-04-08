use jacquard_core::{
    NodeId, Tick, TransportError, TransportObservation, TransportProtocol,
};
use jacquard_traits::{effect_handler, TransportEffects};

use crate::endpoint::SharedInMemoryNetwork;

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
    pub fn attached(
        transport_protocol: &TransportProtocol,
        local_node_id: NodeId,
        endpoints: impl IntoIterator<Item = jacquard_core::LinkEndpoint>,
        network: SharedInMemoryNetwork,
    ) -> Self {
        let endpoints = endpoints.into_iter().collect::<Vec<_>>();
        debug_assert!(endpoints
            .iter()
            .all(|endpoint| endpoint.protocol == *transport_protocol));
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
