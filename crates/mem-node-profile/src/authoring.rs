//! Developer-facing in-memory node authoring.
//!
//! Most implementers should start here. This module exposes the intended
//! authoring flow for in-memory nodes:
//! - choose or construct a shared `LinkEndpoint`
//! - choose a node capability shape such as route-capable
//! - optionally override profile or state details
//! - build the shared `Node`
//!
//! [`NodePreset`] keeps [`SimulatedNodeProfile`] and [`NodeStateSnapshot`]
//! as the low-level escape hatches when a test needs exact control over the
//! profile/state split. Callers bring their own shared `LinkEndpoint` values;
//! this crate stays endpoint-agnostic.

use jacquard_core::{
    ControllerId, DiscoveryScopeId, LinkEndpoint, Node, NodeId, RoutingEngineId,
    ServiceScope, Tick, TimeWindow,
};

use crate::{NodeStateSnapshot, SimulatedNodeProfile};

/// Default discovery scope token used by the standard route-capable node
/// preset.
pub const DEFAULT_ROUTE_SERVICE_SCOPE_ID: [u8; 16] = DiscoveryScopeId([7; 16]).0;
/// Default route-service window length used by the standard node preset.
pub const DEFAULT_ROUTE_SERVICE_WINDOW_TICKS: u64 = 20;

#[must_use]
pub fn default_route_service_window(observed_at_tick: Tick) -> TimeWindow {
    TimeWindow::new(
        observed_at_tick,
        Tick(
            observed_at_tick
                .0
                .saturating_add(DEFAULT_ROUTE_SERVICE_WINDOW_TICKS.saturating_sub(1)),
        ),
    )
    .expect("reference node defaults use a valid service window")
}

/// Typed identity for the common node preset path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeIdentity {
    pub node_id: NodeId,
    pub controller_id: ControllerId,
}

impl NodeIdentity {
    #[must_use]
    pub fn new(node_id: NodeId, controller_id: ControllerId) -> Self {
        Self { node_id, controller_id }
    }
}

/// Typed setup for the common node preset path.
#[derive(Clone, Debug)]
pub struct NodePresetOptions {
    pub identity: NodeIdentity,
    pub endpoint: LinkEndpoint,
    pub observed_at_tick: Tick,
}

impl NodePresetOptions {
    #[must_use]
    pub fn new(
        identity: NodeIdentity,
        endpoint: LinkEndpoint,
        observed_at_tick: Tick,
    ) -> Self {
        Self { identity, endpoint, observed_at_tick }
    }
}

/// Preset-first wrapper around `SimulatedNodeProfile` plus `NodeStateSnapshot`.
#[derive(Clone, Debug)]
pub struct NodePreset {
    identity: NodeIdentity,
    profile: SimulatedNodeProfile,
    state: NodeStateSnapshot,
}

impl NodePreset {
    #[must_use]
    pub fn route_capable_for_engines(
        options: NodePresetOptions,
        routing_engines: &[RoutingEngineId],
    ) -> Self {
        let NodePresetOptions { identity, endpoint, observed_at_tick } = options;
        let valid_for = default_route_service_window(observed_at_tick);
        let scope =
            ServiceScope::Discovery(DiscoveryScopeId(DEFAULT_ROUTE_SERVICE_SCOPE_ID));
        let profile = SimulatedNodeProfile::route_capable_for_engines(
            &endpoint,
            routing_engines,
            &scope,
            valid_for,
            observed_at_tick,
        );
        let state = NodeStateSnapshot::route_capable(observed_at_tick);
        Self { identity, profile, state }
    }

    #[must_use]
    pub fn route_capable(
        options: NodePresetOptions,
        routing_engine: &RoutingEngineId,
    ) -> Self {
        Self::route_capable_for_engines(options, std::slice::from_ref(routing_engine))
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
        self.identity = NodeIdentity::new(node_id, controller_id);
        self
    }

    #[must_use]
    pub fn build(self) -> Node {
        self.profile.build_node(
            self.identity.node_id,
            self.identity.controller_id,
            &self.state,
        )
    }
}

#[cfg(test)]
mod tests {
    use jacquard_adapter::opaque_endpoint;
    use jacquard_core::{
        ByteCount, LinkEndpoint, NodeId, RoutingEngineId, Tick, TransportKind,
    };

    use super::*;

    fn endpoint(byte: u8) -> LinkEndpoint {
        opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(512))
    }

    #[test]
    fn route_capable_builds_node_with_matching_identity() {
        let routing_engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
        let identity = NodeIdentity::new(NodeId([3; 32]), ControllerId([3; 32]));
        let node = NodePreset::route_capable(
            NodePresetOptions::new(identity, endpoint(3), Tick(1)),
            &routing_engine,
        )
        .build();

        assert_eq!(node.controller_id, identity.controller_id);
        assert_eq!(node.profile.endpoints.len(), 1);
        assert_eq!(node.profile.services.len(), 3);
    }

    #[test]
    fn route_capable_for_multiple_engines_emits_service_triples_per_engine() {
        let engines = [
            RoutingEngineId::from_contract_bytes(*b"reference-mem-01"),
            RoutingEngineId::from_contract_bytes(*b"reference-mem-02"),
        ];
        let node = NodePreset::route_capable_for_engines(
            NodePresetOptions::new(
                NodeIdentity::new(NodeId([3; 32]), ControllerId([3; 32])),
                endpoint(3),
                Tick(1),
            ),
            &engines,
        )
        .build();

        assert_eq!(node.profile.services.len(), 6);
    }

    #[test]
    fn endpoint_first_route_capable_uses_supplied_endpoint() {
        let routing_engine = RoutingEngineId::from_contract_bytes(*b"reference-mem-01");
        let endpoint =
            opaque_endpoint(TransportKind::WifiAware, vec![9, 8, 7], ByteCount(512));
        let node = NodePreset::route_capable(
            NodePresetOptions::new(
                NodeIdentity::new(NodeId([3; 32]), ControllerId([3; 32])),
                endpoint.clone(),
                Tick(1),
            ),
            &routing_engine,
        )
        .build();

        assert_eq!(node.profile.endpoints, vec![endpoint]);
    }
}
