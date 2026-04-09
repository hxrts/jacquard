//! Developer-facing in-memory node authoring.
//!
//! Most implementers should start here. This module exposes the intended
//! authoring flow for in-memory nodes:
//! - choose or construct a shared `LinkEndpoint`
//! - choose a node capability shape such as route-capable
//! - optionally override profile or state details
//! - build the shared `Node`
//!
//! [`ReferenceNode`] keeps [`SimulatedNodeProfile`] and [`NodeStateSnapshot`]
//! as the low-level escape hatches when a test needs exact control over the
//! profile/state split. Transport-specific endpoint helpers are provided by
//! `jacquard-core`; this crate stays endpoint-agnostic.

use jacquard_core::{
    ControllerId, DiscoveryScopeId, LinkEndpoint, Node, NodeId, RoutingEngineId,
    ServiceScope, Tick, TimeWindow,
};

use crate::{NodeStateSnapshot, SimulatedNodeProfile, SimulatedServiceDescriptor};

/// Preset-first wrapper around `SimulatedNodeProfile` plus `NodeStateSnapshot`.
#[derive(Clone, Debug)]
pub struct ReferenceNode {
    node_id: NodeId,
    controller_id: ControllerId,
    profile: SimulatedNodeProfile,
    state: NodeStateSnapshot,
}

impl ReferenceNode {
    #[must_use]
    pub fn route_capable_for_engines(
        node_id: NodeId,
        controller_id: ControllerId,
        endpoint: LinkEndpoint,
        routing_engines: &[RoutingEngineId],
        observed_at_tick: Tick,
    ) -> Self {
        let valid_for = TimeWindow::new(
            observed_at_tick,
            Tick(observed_at_tick.0.saturating_add(19)),
        )
        .expect("reference node uses a valid service window");
        let scope = ServiceScope::Discovery(DiscoveryScopeId([7; 16]));
        let service_endpoint = endpoint.clone();
        let mut profile = SimulatedNodeProfile::new()
            .with_endpoint(endpoint)
            .with_connection_limits(8, 8, 4, 4)
            .with_work_budgets(10, 10)
            .with_hold_limits(8, jacquard_core::ByteCount(8192))
            .with_observed_at_tick(observed_at_tick);
        for routing_engine in routing_engines {
            profile = profile
                .with_service(
                    SimulatedServiceDescriptor::discover_service(
                        service_endpoint.clone(),
                        scope.clone(),
                        valid_for,
                        observed_at_tick,
                    )
                    .with_routing_engine(routing_engine),
                )
                .with_service(
                    SimulatedServiceDescriptor::move_service(
                        service_endpoint.clone(),
                        scope.clone(),
                        valid_for,
                        observed_at_tick,
                    )
                    .with_routing_engine(routing_engine),
                )
                .with_service(
                    SimulatedServiceDescriptor::hold_service(
                        service_endpoint.clone(),
                        scope.clone(),
                        valid_for,
                        observed_at_tick,
                    )
                    .with_routing_engine(routing_engine),
                );
        }
        let state = NodeStateSnapshot::route_capable(observed_at_tick);
        Self { node_id, controller_id, profile, state }
    }

    #[must_use]
    pub fn route_capable(
        node_id: NodeId,
        controller_id: ControllerId,
        endpoint: LinkEndpoint,
        routing_engine: &RoutingEngineId,
        observed_at_tick: Tick,
    ) -> Self {
        Self::route_capable_for_engines(
            node_id,
            controller_id,
            endpoint,
            std::slice::from_ref(routing_engine),
            observed_at_tick,
        )
    }

    #[must_use]
    pub fn with_profile(mut self, profile: SimulatedNodeProfile) -> Self {
        self.profile = profile;
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: NodeStateSnapshot) -> Self {
        self.state = state;
        self
    }

    #[must_use]
    pub fn with_identity(
        mut self,
        node_id: NodeId,
        controller_id: ControllerId,
    ) -> Self {
        self.node_id = node_id;
        self.controller_id = controller_id;
        self
    }

    #[must_use]
    pub fn build(self) -> Node {
        self.profile
            .build_node(self.node_id, self.controller_id, &self.state)
    }
}

#[cfg(test)]
mod tests {
    use jacquard_core::{
        opaque_endpoint, ByteCount, ControllerId, EndpointAddress, LinkEndpoint,
        NodeId, RoutingEngineId, Tick, TransportProtocol,
    };

    use super::*;

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportProtocol::WifiAware, vec![byte], ByteCount(512))
    }

    #[test]
    fn route_capable_builds_node_with_matching_identity() {
        let routing_engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
        let node = ReferenceNode::route_capable(
            NodeId([3; 32]),
            ControllerId([3; 32]),
            endpoint(3),
            &routing_engine,
            Tick(1),
        )
        .build();

        assert_eq!(node.controller_id, ControllerId([3; 32]));
        assert_eq!(node.profile.endpoints.len(), 1);
        assert_eq!(node.profile.services.len(), 3);
    }

    #[test]
    fn route_capable_for_multiple_engines_emits_service_triples_per_engine() {
        let engines = [
            RoutingEngineId::from_contract_bytes(*b"reference-mem-01"),
            RoutingEngineId::from_contract_bytes(*b"reference-mem-02"),
        ];
        let node = ReferenceNode::route_capable_for_engines(
            NodeId([3; 32]),
            ControllerId([3; 32]),
            endpoint(3),
            &engines,
            Tick(1),
        )
        .build();

        assert_eq!(node.profile.services.len(), 6);
    }

    #[test]
    fn endpoint_first_route_capable_uses_supplied_endpoint() {
        let routing_engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
        let endpoint = LinkEndpoint {
            protocol: TransportProtocol::WifiAware,
            address: EndpointAddress::Opaque(vec![9, 8, 7]),
            mtu_bytes: ByteCount(512),
        };
        let node = ReferenceNode::route_capable(
            NodeId([3; 32]),
            ControllerId([3; 32]),
            endpoint.clone(),
            &routing_engine,
            Tick(1),
        )
        .build();

        assert_eq!(node.profile.endpoints, vec![endpoint]);
    }
}
