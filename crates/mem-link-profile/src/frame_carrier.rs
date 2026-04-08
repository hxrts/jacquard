use jacquard_core::{
    NodeId, Tick, TransportError, TransportObservation, TransportProtocol,
};
use jacquard_traits::{MeshFrame, MeshTransport};

use crate::endpoint::SharedInMemoryNetwork;

/// In-memory `MeshTransport` implementation backed by one shared network.
pub struct InMemoryMeshTransport {
    transport_id: TransportProtocol,
    local_node_id: Option<NodeId>,
    ingress_tick: Tick,
    network: Option<SharedInMemoryNetwork>,
    pub sent_frames: Vec<(jacquard_core::LinkEndpoint, Vec<u8>)>,
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
        endpoints: impl IntoIterator<Item = jacquard_core::LinkEndpoint>,
        network: SharedInMemoryNetwork,
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
