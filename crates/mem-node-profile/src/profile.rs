use jacquard_core::{ByteCount, ControllerId, LinkEndpoint, Node, NodeId, NodeProfile};

use crate::{services::SimulatedServiceDescriptor, state::NodeStateSnapshot};

/// Builder for the stable capability half of one node.
#[derive(Clone, Debug, Default)]
pub struct SimulatedNodeProfile {
    services: Vec<SimulatedServiceDescriptor>,
    endpoints: Vec<LinkEndpoint>,
    connection_count_max: u32,
    neighbor_state_count_max: u32,
    simultaneous_transfer_count_max: u32,
    active_route_count_max: u32,
    relay_work_budget_max: u32,
    maintenance_work_budget_max: u32,
    hold_item_count_max: u32,
    hold_capacity_bytes_max: ByteCount,
}

impl SimulatedNodeProfile {
    #[must_use]
    pub fn new() -> Self {
        Self {
            connection_count_max: 4,
            neighbor_state_count_max: 4,
            simultaneous_transfer_count_max: 2,
            active_route_count_max: 2,
            relay_work_budget_max: 4,
            maintenance_work_budget_max: 4,
            hold_item_count_max: 4,
            hold_capacity_bytes_max: ByteCount(4096),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_service(mut self, service: SimulatedServiceDescriptor) -> Self {
        self.services.push(service);
        self
    }

    #[must_use]
    pub fn with_endpoint(mut self, endpoint: LinkEndpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    #[must_use]
    pub fn with_connection_count_max(mut self, count: u32) -> Self {
        self.connection_count_max = count;
        self
    }

    #[must_use]
    pub fn with_neighbor_state_count_max(mut self, count: u32) -> Self {
        self.neighbor_state_count_max = count;
        self
    }

    #[must_use]
    pub fn with_simultaneous_transfer_count_max(mut self, count: u32) -> Self {
        self.simultaneous_transfer_count_max = count;
        self
    }

    #[must_use]
    pub fn with_active_route_count_max(mut self, count: u32) -> Self {
        self.active_route_count_max = count;
        self
    }

    #[must_use]
    pub fn with_relay_budget(mut self, budget: u32) -> Self {
        self.relay_work_budget_max = budget;
        self
    }

    #[must_use]
    pub fn with_maintenance_budget(mut self, budget: u32) -> Self {
        self.maintenance_work_budget_max = budget;
        self
    }

    #[must_use]
    pub fn with_hold_item_count(mut self, count: u32) -> Self {
        self.hold_item_count_max = count;
        self
    }

    #[must_use]
    pub fn with_hold_capacity(mut self, bytes: ByteCount) -> Self {
        self.hold_capacity_bytes_max = bytes;
        self
    }

    #[must_use]
    pub fn build(self, node_id: NodeId, controller_id: ControllerId) -> NodeProfile {
        NodeProfile {
            services: self
                .services
                .into_iter()
                .map(|service| service.build(node_id, controller_id))
                .collect(),
            endpoints: self.endpoints,
            connection_count_max: self.connection_count_max,
            neighbor_state_count_max: self.neighbor_state_count_max,
            simultaneous_transfer_count_max: self.simultaneous_transfer_count_max,
            active_route_count_max: self.active_route_count_max,
            relay_work_budget_max: self.relay_work_budget_max,
            maintenance_work_budget_max: self.maintenance_work_budget_max,
            hold_item_count_max: self.hold_item_count_max,
            hold_capacity_bytes_max: self.hold_capacity_bytes_max,
        }
    }

    #[must_use]
    pub fn build_node(
        self,
        node_id: NodeId,
        controller_id: ControllerId,
        state: &NodeStateSnapshot,
    ) -> Node {
        Node {
            controller_id,
            profile: self.build(node_id, controller_id),
            state: state.clone().build(),
        }
    }
}
