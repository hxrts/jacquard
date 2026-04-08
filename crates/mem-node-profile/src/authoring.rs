//! Developer-facing in-memory node authoring.
//!
//! Most implementers should start here. This module exposes the intended
//! authoring flow for in-memory nodes:
//! - choose an endpoint shape such as BLE or opaque
//! - choose a node capability shape such as route-capable BLE
//! - optionally override profile or state details
//! - build the shared `Node`
//!
//! [`ReferenceNode`] keeps [`SimulatedNodeProfile`] and [`NodeStateSnapshot`]
//! as the low-level escape hatches when a test needs exact control over the
//! profile/state split.

use jacquard_core::{
    ControllerId, DiscoveryScopeId, Node, NodeId, RoutingEngineId, ServiceScope, Tick,
    TimeWindow,
};

use crate::{ble_endpoint, NodeStateSnapshot, SimulatedNodeProfile};

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
    pub fn ble_route_capable(
        node_byte: u8,
        routing_engine: &RoutingEngineId,
        observed_at_tick: Tick,
    ) -> Self {
        let node_id = NodeId([node_byte; 32]);
        let controller_id = ControllerId([node_byte; 32]);
        let endpoint = ble_endpoint(node_byte);
        let valid_for = TimeWindow::new(
            observed_at_tick,
            Tick(observed_at_tick.0.saturating_add(19)),
        )
        .expect("reference node uses a valid service window");
        let profile = SimulatedNodeProfile::route_capable(
            endpoint,
            routing_engine,
            ServiceScope::Discovery(DiscoveryScopeId([7; 16])),
            valid_for,
            observed_at_tick,
        );
        let state = NodeStateSnapshot::route_capable(observed_at_tick);
        Self { node_id, controller_id, profile, state }
    }

    #[must_use]
    pub fn route_capable(
        node_byte: u8,
        routing_engine: &RoutingEngineId,
        observed_at_tick: Tick,
    ) -> Self {
        Self::ble_route_capable(node_byte, routing_engine, observed_at_tick)
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
    use jacquard_core::{RoutingEngineId, Tick};

    use super::*;

    #[test]
    fn route_capable_builds_node_with_matching_identity() {
        let routing_engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
        let node =
            ReferenceNode::ble_route_capable(3, &routing_engine, Tick(1)).build();

        assert_eq!(node.controller_id, ControllerId([3; 32]));
        assert_eq!(node.profile.endpoints.len(), 1);
        assert_eq!(node.profile.services.len(), 3);
    }
}
