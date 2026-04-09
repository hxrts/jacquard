//! `SimulatedNodeProfile`, a builder for the stable capability half of a node.
//!
//! This module assembles the `NodeProfile` portion of a `Node`: endpoint list,
//! service descriptors, connection limits, work budgets, and hold capacity. It
//! delegates service construction to `SimulatedServiceDescriptor` and
//! node-state construction to `NodeStateSnapshot`, then emits a fully specified
//! `Node` via `NodeBuilder` from `jacquard-core`.
//!
//! Two output paths are available:
//! - `build(node_id, controller_id)` returns only the `NodeProfile`, useful
//!   when the caller manages state separately.
//! - `build_node(node_id, controller_id, state)` combines the profile with a
//!   `NodeStateSnapshot` into a complete `Node` value.
//!
//! `DEFAULT_HOLD_CAPACITY_BYTES` (4096 bytes) is the module-level constant used
//! by all default preset constructors.
//!
//! Most callers should prefer `NodePreset` from the `authoring` module,
//! which wraps this builder with preset constructors. Use
//! `SimulatedNodeProfile` directly only when a test needs exact control over
//! the profile fields.

use jacquard_core::{
    ByteCount, ControllerId, HoldItemCount, LinkEndpoint, MaintenanceWorkBudget, Node,
    NodeBuilder, NodeId, NodeProfile, NodeProfileBuilder, RelayWorkBudget,
    RoutingEngineId, ServiceScope, Tick, TimeWindow,
};

use crate::{
    service::{RouteServiceBundle, SimulatedServiceDescriptor},
    state::NodeStateSnapshot,
};

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
    pub fn with_route_service_bundle(mut self, bundle: RouteServiceBundle) -> Self {
        self.services.extend(bundle.into_services());
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
        let mut builder = NodeProfileBuilder::new()
            .with_connection_limits(
                self.connection_count_max,
                self.neighbor_state_count_max,
                self.simultaneous_transfer_count_max,
                self.active_route_count_max,
            )
            .with_work_budgets(
                RelayWorkBudget(self.relay_work_budget_max),
                MaintenanceWorkBudget(self.maintenance_work_budget_max),
            )
            .with_hold_limits(
                HoldItemCount(self.hold_item_count_max),
                self.hold_capacity_bytes_max,
            );
        for endpoint in self.endpoints {
            builder = builder.with_endpoint(endpoint);
        }
        for service in self.services {
            builder = builder.with_service(service.build(node_id, controller_id));
        }
        builder.build()
    }

    #[must_use]
    pub fn build_node(
        self,
        node_id: NodeId,
        controller_id: ControllerId,
        state: &NodeStateSnapshot,
    ) -> Node {
        let observed_at_tick = self.observed_at_tick;
        NodeBuilder::new(
            controller_id,
            self.build(node_id, controller_id),
            state
                .clone()
                .with_observed_at_tick(observed_at_tick)
                .build(),
        )
        .build()
    }

    #[must_use]
    pub fn route_capable(
        endpoint: &LinkEndpoint,
        routing_engine: &RoutingEngineId,
        scope: &ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::route_capable_for_engines(
            endpoint,
            std::slice::from_ref(routing_engine),
            scope,
            valid_for,
            observed_at_tick,
        )
    }

    #[must_use]
    pub fn route_capable_for_engines(
        endpoint: &LinkEndpoint,
        routing_engines: &[RoutingEngineId],
        scope: &ServiceScope,
        valid_for: TimeWindow,
        observed_at_tick: Tick,
    ) -> Self {
        Self::new()
            .with_endpoint(endpoint.clone())
            .with_connection_limits(8, 8, 4, 4)
            .with_work_budgets(10, 10)
            .with_hold_limits(8, ByteCount(8192))
            .with_route_service_bundle(RouteServiceBundle::route_capable(
                endpoint,
                routing_engines,
                scope,
                valid_for,
                observed_at_tick,
            ))
            .with_observed_at_tick(observed_at_tick)
    }
}

#[cfg(test)]
mod tests {
    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        ByteCount, ControllerId, DiscoveryScopeId, NodeId, ServiceScope, TransportKind,
    };

    use super::*;

    #[test]
    fn route_service_bundle_adds_service_triple_per_engine() {
        let engines = [
            RoutingEngineId::from_contract_bytes(*b"reference-mem-01"),
            RoutingEngineId::from_contract_bytes(*b"reference-mem-02"),
        ];
        let scope = ServiceScope::Discovery(DiscoveryScopeId([7; 16]));
        let valid_for = TimeWindow::new(Tick(1), Tick(20)).expect("valid window");
        let profile = SimulatedNodeProfile::route_capable_for_engines(
            &opaque_endpoint(TransportKind::WifiAware, vec![3], ByteCount(512)),
            &engines,
            &scope,
            valid_for,
            Tick(1),
        );

        let node = profile.build(NodeId([3; 32]), ControllerId([3; 32]));

        assert_eq!(node.services.len(), 6);
    }
}
