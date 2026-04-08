//! `SimulatedNodeProfile`, a builder for the stable capability half of a
//! node. Attaches endpoints and services, sets connection and transfer
//! budgets and hold capacity, and emits a shared `NodeProfile` or a full
//! `Node` bound to a `(NodeId, ControllerId)` pair.

use jacquard_core::{
    ByteCount, ControllerId, HoldItemCount, LinkEndpoint, MaintenanceWorkBudget, Node,
    NodeId, NodeProfile, RelayWorkBudget, RouteServiceKind, RoutingEngineId,
    ServiceScope, Tick, TimeWindow,
};

use crate::{services::SimulatedServiceDescriptor, state::NodeStateSnapshot};

/// Default maximum hold capacity for a simulated node.
pub const DEFAULT_HOLD_CAPACITY_BYTES: ByteCount = ByteCount(4096);

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
    observed_at_tick: Tick,
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
            hold_capacity_bytes_max: DEFAULT_HOLD_CAPACITY_BYTES,
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
    pub fn with_connection_limits(
        mut self,
        connection_count_max: u32,
        neighbor_state_count_max: u32,
        simultaneous_transfer_count_max: u32,
        active_route_count_max: u32,
    ) -> Self {
        self.connection_count_max = connection_count_max;
        self.neighbor_state_count_max = neighbor_state_count_max;
        self.simultaneous_transfer_count_max = simultaneous_transfer_count_max;
        self.active_route_count_max = active_route_count_max;
        self
    }

    #[must_use]
    pub fn with_work_budgets(
        mut self,
        relay_work_budget_max: u32,
        maintenance_work_budget_max: u32,
    ) -> Self {
        self.relay_work_budget_max = relay_work_budget_max;
        self.maintenance_work_budget_max = maintenance_work_budget_max;
        self
    }

    #[must_use]
    pub fn with_observed_at_tick(mut self, tick: Tick) -> Self {
        self.observed_at_tick = tick;
        self
    }

    #[must_use]
    pub fn with_hold_limits(
        mut self,
        hold_item_count_max: u32,
        hold_capacity_bytes_max: ByteCount,
    ) -> Self {
        self.hold_item_count_max = hold_item_count_max;
        self.hold_capacity_bytes_max = hold_capacity_bytes_max;
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
            relay_work_budget_max: RelayWorkBudget(self.relay_work_budget_max),
            maintenance_work_budget_max: MaintenanceWorkBudget(
                self.maintenance_work_budget_max,
            ),
            hold_item_count_max: HoldItemCount(self.hold_item_count_max),
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
        let observed_at_tick = self.observed_at_tick;
        Node {
            controller_id,
            profile: self.build(node_id, controller_id),
            state: state
                .clone()
                .with_observed_at_tick(observed_at_tick)
                .build(),
        }
    }

    #[must_use]
    pub fn route_capable(
        endpoint: LinkEndpoint,
        routing_engine: &RoutingEngineId,
        scope: ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::new()
            .with_endpoint(endpoint.clone())
            .with_connection_limits(8, 8, 4, 4)
            .with_work_budgets(10, 10)
            .with_hold_limits(8, ByteCount(8192))
            .with_service(
                SimulatedServiceDescriptor::advertised(
                    RouteServiceKind::Discover,
                    endpoint.clone(),
                    scope.clone(),
                    valid_for,
                    observed_at_tick,
                )
                .with_capacity_profile(4, None)
                .with_routing_engine(routing_engine),
            )
            .with_service(
                SimulatedServiceDescriptor::advertised(
                    RouteServiceKind::Move,
                    endpoint.clone(),
                    scope.clone(),
                    valid_for,
                    observed_at_tick,
                )
                .with_capacity_profile(4, None)
                .with_routing_engine(routing_engine),
            )
            .with_service(
                SimulatedServiceDescriptor::advertised(
                    RouteServiceKind::Hold,
                    endpoint,
                    scope,
                    valid_for,
                    observed_at_tick,
                )
                .with_capacity_profile(4, Some(ByteCount(4096)))
                .with_routing_engine(routing_engine),
            )
            .with_observed_at_tick(observed_at_tick)
    }
}
