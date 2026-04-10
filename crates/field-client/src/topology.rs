//! Topology fixture presets for `jacquard-field-client` scenarios.
//!
//! Provides [`FieldTopologyNodePreset`] and [`FieldTopologyLinkPreset`], built
//! through the [`node`] and [`link`] constructor functions. Nodes are
//! registered as route-capable for `FIELD_ENGINE_ID` with `WifiAware` opaque
//! endpoints keyed by a single byte, keeping multi-node topologies cheap to
//! assemble in tests.
//!
//! Node presets support optional state and profile overrides via builder
//! methods. Link presets default to 950‰ delivery confidence with per-link
//! override support. Both produce fully populated `Node` and `Link` records
//! ready for insertion into a `Configuration` observation.

use jacquard_adapter::opaque_endpoint;
use jacquard_core::{
    ByteCount, ControllerId, Link, Node, NodeId, RatioPermille, Tick, TransportKind,
};
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{
    NodeIdentity, NodePreset, NodePresetOptions, NodeStateSnapshot, SimulatedNodeProfile,
};

fn field_endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(256))
}

#[must_use]
pub fn node(node_byte: u8) -> FieldTopologyNodePreset {
    FieldTopologyNodePreset {
        node_byte,
        observed_at_tick: Tick(1),
        state_override: None,
        profile_override: None,
    }
}

#[must_use]
pub fn link(node_byte: u8) -> FieldTopologyLinkPreset {
    FieldTopologyLinkPreset {
        endpoint_byte: node_byte,
        confidence: RatioPermille(950),
        observed_at_tick: Tick(1),
    }
}

#[derive(Clone, Debug)]
pub struct FieldTopologyNodePreset {
    node_byte: u8,
    observed_at_tick: Tick,
    state_override: Option<NodeStateSnapshot>,
    profile_override: Option<SimulatedNodeProfile>,
}

impl FieldTopologyNodePreset {
    #[must_use]
    pub fn observed_at(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub fn with_state(mut self, state: NodeStateSnapshot) -> Self {
        self.state_override = Some(state);
        self
    }

    #[must_use]
    pub fn with_profile(mut self, profile: SimulatedNodeProfile) -> Self {
        self.profile_override = Some(profile);
        self
    }

    #[must_use]
    pub fn build(self) -> Node {
        let mut preset = NodePreset::route_capable(
            NodePresetOptions::new(
                NodeIdentity::new(
                    NodeId([self.node_byte; 32]),
                    ControllerId([self.node_byte; 32]),
                ),
                field_endpoint(self.node_byte),
                self.observed_at_tick,
            ),
            &FIELD_ENGINE_ID,
        );
        if let Some(profile) = self.profile_override {
            preset = preset.with_profile(profile);
        }
        if let Some(state) = self.state_override {
            preset = preset.with_state(state);
        }
        preset.build()
    }
}

#[derive(Clone, Debug)]
pub struct FieldTopologyLinkPreset {
    endpoint_byte: u8,
    confidence: RatioPermille,
    observed_at_tick: Tick,
}

impl FieldTopologyLinkPreset {
    #[must_use]
    pub fn with_confidence(mut self, confidence: RatioPermille) -> Self {
        self.confidence = confidence;
        self
    }

    #[must_use]
    pub fn observed_at(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub fn build(self) -> Link {
        LinkPreset::lossy(
            LinkPresetOptions::new(field_endpoint(self.endpoint_byte), self.observed_at_tick)
                .with_confidence(self.confidence),
        )
        .build()
    }
}
