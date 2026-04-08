//! Shared model authoring helpers for node/link extensions.
//!
//! These builders reduce structural boilerplate when assembling shared world
//! objects. They intentionally do not choose transport- or product-specific
//! semantic defaults; reference crates such as `mem-link-profile` and
//! `mem-node-profile` layer presets on top.

use jacquard_macros::public_model;
use serde::{Deserialize, Serialize};

use crate::{
    Belief, ByteCount, CapacityHint, ControllerId, DurationMs, HoldItemCount,
    InformationSetSummary, Link, LinkEndpoint, LinkProfile, LinkRuntimeState,
    LinkState, MaintenanceWorkBudget, Node, NodeProfile, NodeRelayBudget, NodeState,
    RatioPermille, RelayWorkBudget, RepairCapability, RoutingEngineId,
    ServiceDescriptor, ServiceScope, Tick, TimeWindow,
};

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkBuilder {
    endpoint: LinkEndpoint,
    profile: LinkProfile,
    state: LinkState,
}

impl LinkBuilder {
    #[must_use]
    pub fn new(endpoint: LinkEndpoint) -> Self {
        Self {
            endpoint,
            profile: LinkProfile {
                latency_floor_ms: DurationMs(0),
                repair_capability: RepairCapability::None,
                partition_recovery: crate::PartitionRecoveryClass::None,
            },
            state: LinkState {
                state: LinkRuntimeState::Suspended,
                median_rtt_ms: Belief::Absent,
                transfer_rate_bytes_per_sec: Belief::Absent,
                stability_horizon_ms: Belief::Absent,
                loss_permille: RatioPermille(0),
                delivery_confidence_permille: Belief::Absent,
                symmetry_permille: Belief::Absent,
            },
        }
    }

    #[must_use]
    pub fn with_profile(
        mut self,
        latency_floor_ms: DurationMs,
        repair_capability: RepairCapability,
        partition_recovery: crate::PartitionRecoveryClass,
    ) -> Self {
        self.profile = LinkProfile {
            latency_floor_ms,
            repair_capability,
            partition_recovery,
        };
        self
    }

    #[must_use]
    pub fn with_runtime_state(mut self, state: LinkRuntimeState) -> Self {
        self.state.state = state;
        self
    }

    #[must_use]
    pub fn with_runtime_observation(
        mut self,
        median_rtt_ms: DurationMs,
        transfer_rate_bytes_per_sec: u32,
        stability_horizon_ms: DurationMs,
        observed_at_tick: Tick,
    ) -> Self {
        self.state.median_rtt_ms = Belief::certain(median_rtt_ms, observed_at_tick);
        self.state.transfer_rate_bytes_per_sec =
            Belief::certain(transfer_rate_bytes_per_sec, observed_at_tick);
        self.state.stability_horizon_ms =
            Belief::certain(stability_horizon_ms, observed_at_tick);
        self
    }

    #[must_use]
    pub fn with_quality(
        mut self,
        loss_permille: RatioPermille,
        delivery_confidence_permille: RatioPermille,
        symmetry_permille: RatioPermille,
        observed_at_tick: Tick,
    ) -> Self {
        self.state.loss_permille = loss_permille;
        self.state.delivery_confidence_permille =
            Belief::certain(delivery_confidence_permille, observed_at_tick);
        self.state.symmetry_permille =
            Belief::certain(symmetry_permille, observed_at_tick);
        self
    }

    #[must_use]
    pub fn build(self) -> Link {
        Link {
            endpoint: self.endpoint,
            profile: self.profile,
            state: self.state,
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceDescriptorBuilder {
    provider_node_id: crate::NodeId,
    controller_id: ControllerId,
    service_kind: crate::RouteServiceKind,
    endpoints: Vec<LinkEndpoint>,
    routing_engines: Vec<RoutingEngineId>,
    scope: ServiceScope,
    valid_for: TimeWindow,
    capacity: Belief<CapacityHint>,
}

impl ServiceDescriptorBuilder {
    #[must_use]
    pub fn new(
        provider_node_id: crate::NodeId,
        controller_id: ControllerId,
        service_kind: crate::RouteServiceKind,
    ) -> Self {
        Self {
            provider_node_id,
            controller_id,
            service_kind,
            endpoints: Vec::new(),
            routing_engines: Vec::new(),
            scope: ServiceScope::Introduction { scope_token: vec![1] },
            valid_for: TimeWindow::new(Tick(0), Tick(1))
                .expect("shared builder uses a valid default window"),
            capacity: Belief::Absent,
        }
    }

    #[must_use]
    pub fn with_endpoint(mut self, endpoint: LinkEndpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    #[must_use]
    pub fn with_routing_engine(mut self, routing_engine: &RoutingEngineId) -> Self {
        self.routing_engines.push(routing_engine.clone());
        self
    }

    #[must_use]
    pub fn with_scope(mut self, scope: ServiceScope) -> Self {
        self.scope = scope;
        self
    }

    #[must_use]
    pub fn with_valid_for(mut self, valid_for: TimeWindow) -> Self {
        self.valid_for = valid_for;
        self
    }

    #[must_use]
    pub fn with_capacity(
        mut self,
        capacity: CapacityHint,
        observed_at_tick: Tick,
    ) -> Self {
        self.capacity = Belief::certain(capacity, observed_at_tick);
        self
    }

    #[must_use]
    pub fn build(self) -> ServiceDescriptor {
        ServiceDescriptor {
            provider_node_id: self.provider_node_id,
            controller_id: self.controller_id,
            service_kind: self.service_kind,
            endpoints: self.endpoints,
            routing_engines: self.routing_engines,
            scope: self.scope,
            valid_for: self.valid_for,
            capacity: self.capacity,
        }
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeProfileBuilder {
    services: Vec<ServiceDescriptor>,
    endpoints: Vec<LinkEndpoint>,
    connection_count_max: u32,
    neighbor_state_count_max: u32,
    simultaneous_transfer_count_max: u32,
    active_route_count_max: u32,
    relay_work_budget_max: RelayWorkBudget,
    maintenance_work_budget_max: MaintenanceWorkBudget,
    hold_item_count_max: HoldItemCount,
    hold_capacity_bytes_max: ByteCount,
}

impl NodeProfileBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            endpoints: Vec::new(),
            connection_count_max: 0,
            neighbor_state_count_max: 0,
            simultaneous_transfer_count_max: 0,
            active_route_count_max: 0,
            relay_work_budget_max: RelayWorkBudget(0),
            maintenance_work_budget_max: MaintenanceWorkBudget(0),
            hold_item_count_max: HoldItemCount(0),
            hold_capacity_bytes_max: ByteCount(0),
        }
    }

    #[must_use]
    pub fn with_service(mut self, service: ServiceDescriptor) -> Self {
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
        relay_work_budget_max: RelayWorkBudget,
        maintenance_work_budget_max: MaintenanceWorkBudget,
    ) -> Self {
        self.relay_work_budget_max = relay_work_budget_max;
        self.maintenance_work_budget_max = maintenance_work_budget_max;
        self
    }

    #[must_use]
    pub fn with_hold_limits(
        mut self,
        hold_item_count_max: HoldItemCount,
        hold_capacity_bytes_max: ByteCount,
    ) -> Self {
        self.hold_item_count_max = hold_item_count_max;
        self.hold_capacity_bytes_max = hold_capacity_bytes_max;
        self
    }

    #[must_use]
    pub fn build(self) -> NodeProfile {
        NodeProfile {
            services: self.services,
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
}

impl Default for NodeProfileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeStateBuilder {
    relay_budget: Belief<NodeRelayBudget>,
    available_connection_count: Belief<u32>,
    hold_capacity_available_bytes: Belief<ByteCount>,
    information_summary: Belief<InformationSetSummary>,
}

impl NodeStateBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            relay_budget: Belief::Absent,
            available_connection_count: Belief::Absent,
            hold_capacity_available_bytes: Belief::Absent,
            information_summary: Belief::Absent,
        }
    }

    #[must_use]
    pub fn with_relay_budget(
        mut self,
        relay_work_budget: RelayWorkBudget,
        relay_utilization_permille: RatioPermille,
        retention_horizon_ms: DurationMs,
        observed_at_tick: Tick,
    ) -> Self {
        self.relay_budget = Belief::certain(
            NodeRelayBudget::observed(
                relay_work_budget,
                relay_utilization_permille,
                retention_horizon_ms,
                observed_at_tick,
            ),
            observed_at_tick,
        );
        self
    }

    #[must_use]
    pub fn with_available_connections(
        mut self,
        available_connection_count: u32,
        observed_at_tick: Tick,
    ) -> Self {
        self.available_connection_count =
            Belief::certain(available_connection_count, observed_at_tick);
        self
    }

    #[must_use]
    pub fn with_hold_capacity(
        mut self,
        hold_capacity_available_bytes: ByteCount,
        observed_at_tick: Tick,
    ) -> Self {
        self.hold_capacity_available_bytes =
            Belief::certain(hold_capacity_available_bytes, observed_at_tick);
        self
    }

    #[must_use]
    pub fn with_information_summary(
        mut self,
        item_count: HoldItemCount,
        byte_count: ByteCount,
        false_positive_permille: RatioPermille,
        observed_at_tick: Tick,
    ) -> Self {
        self.information_summary = Belief::certain(
            InformationSetSummary::bloom_filter(
                item_count,
                byte_count,
                false_positive_permille,
                observed_at_tick,
            ),
            observed_at_tick,
        );
        self
    }

    #[must_use]
    pub fn build(self) -> NodeState {
        NodeState {
            relay_budget: self.relay_budget,
            available_connection_count: self.available_connection_count,
            hold_capacity_available_bytes: self.hold_capacity_available_bytes,
            information_summary: self.information_summary,
        }
    }
}

impl Default for NodeStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[public_model]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeBuilder {
    controller_id: ControllerId,
    profile: NodeProfile,
    state: NodeState,
}

impl NodeBuilder {
    #[must_use]
    pub fn new(
        controller_id: ControllerId,
        profile: NodeProfile,
        state: NodeState,
    ) -> Self {
        Self { controller_id, profile, state }
    }

    #[must_use]
    pub fn build(self) -> Node {
        Node {
            controller_id: self.controller_id,
            profile: self.profile,
            state: self.state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EndpointAddress, PartitionRecoveryClass, RouteServiceKind, ServiceScope,
        TransportProtocol,
    };

    #[test]
    fn link_builder_builds_shared_link() {
        let endpoint = LinkEndpoint {
            protocol: TransportProtocol::BleGatt,
            address: EndpointAddress::Opaque(vec![1]),
            mtu_bytes: ByteCount(256),
        };
        let link = LinkBuilder::new(endpoint.clone())
            .with_profile(
                DurationMs(8),
                RepairCapability::TransportRetransmit,
                PartitionRecoveryClass::LocalReconnect,
            )
            .with_runtime_state(LinkRuntimeState::Active)
            .with_runtime_observation(DurationMs(40), 2048, DurationMs(500), Tick(2))
            .with_quality(
                RatioPermille(50),
                RatioPermille(950),
                RatioPermille(900),
                Tick(2),
            )
            .build();
        assert_eq!(link.endpoint, endpoint);
        assert_eq!(link.state.state, LinkRuntimeState::Active);
    }

    #[test]
    fn node_builders_build_shared_node() {
        let endpoint = LinkEndpoint {
            protocol: TransportProtocol::BleGatt,
            address: EndpointAddress::Opaque(vec![2]),
            mtu_bytes: ByteCount(256),
        };
        let node_id = crate::NodeId([2; 32]);
        let controller_id = ControllerId([2; 32]);
        let service = ServiceDescriptorBuilder::new(
            node_id,
            controller_id,
            RouteServiceKind::Discover,
        )
        .with_endpoint(endpoint.clone())
        .with_scope(ServiceScope::Introduction { scope_token: vec![1] })
        .with_valid_for(TimeWindow::new(Tick(0), Tick(10)).expect("valid window"))
        .with_capacity(CapacityHint::new(RatioPermille(0)), Tick(0))
        .build();
        let profile = NodeProfileBuilder::new()
            .with_endpoint(endpoint)
            .with_service(service)
            .with_connection_limits(4, 4, 2, 2)
            .with_work_budgets(RelayWorkBudget(4), MaintenanceWorkBudget(4))
            .with_hold_limits(HoldItemCount(4), ByteCount(4096))
            .build();
        let state = NodeStateBuilder::new()
            .with_available_connections(4, Tick(0))
            .with_hold_capacity(ByteCount(4096), Tick(0))
            .build();
        let node = NodeBuilder::new(controller_id, profile, state).build();
        assert_eq!(node.controller_id, controller_id);
        assert_eq!(node.profile.endpoints.len(), 1);
    }
}
