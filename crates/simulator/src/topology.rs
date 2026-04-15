//! Reusable topology fixture presets for simulator scenarios.
//!
//! The intended path is builder-style:
//! - `node(byte).pathway().build()`
//! - `node(byte).for_engines(&[...]).build()`
//! - `link(byte).with_confidence(...).build()`

#![allow(dead_code)]

use jacquard_adapter::opaque_endpoint;
use jacquard_babel::BABEL_ENGINE_ID;
use jacquard_batman_bellman::BATMAN_BELLMAN_ENGINE_ID;
use jacquard_batman_classic::BATMAN_CLASSIC_ENGINE_ID;
use jacquard_core::{
    ByteCount, ControllerId, Link, Node, NodeId, RatioPermille, RoutingEngineId, Tick,
    TransportKind,
};
use jacquard_field::FIELD_ENGINE_ID;
use jacquard_mem_link_profile::{LinkPreset, LinkPresetOptions};
use jacquard_mem_node_profile::{NodeIdentity, NodePreset, NodePresetOptions};
use jacquard_olsrv2::OLSRV2_ENGINE_ID;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_scatter::SCATTER_ENGINE_ID;

// Stable WifiAware endpoint keyed by a single byte — used as a compact,
// collision-free node identity in fixture topologies (byte 1 → node 1, etc.).
fn reference_endpoint(byte: u8) -> jacquard_core::LinkEndpoint {
    opaque_endpoint(TransportKind::WifiAware, vec![byte], ByteCount(256))
}

#[must_use]
pub(crate) fn node(node_byte: u8) -> TopologyNodePreset {
    TopologyNodePreset {
        node_byte,
        routing_engines: vec![PATHWAY_ENGINE_ID],
        observed_at_tick: Tick(1),
    }
}

#[must_use]
pub(crate) fn link(node_byte: u8) -> TopologyLinkPreset {
    TopologyLinkPreset {
        endpoint_byte: node_byte,
        confidence: RatioPermille(950),
        observed_at_tick: Tick(1),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TopologyNodePreset {
    node_byte: u8,
    routing_engines: Vec<RoutingEngineId>,
    observed_at_tick: Tick,
}

impl TopologyNodePreset {
    #[must_use]
    pub(crate) fn for_engine(mut self, engine: &RoutingEngineId) -> Self {
        self.routing_engines = vec![engine.clone()];
        self
    }

    #[must_use]
    pub(crate) fn for_engines(mut self, engines: &[RoutingEngineId]) -> Self {
        self.routing_engines = engines.to_vec();
        self
    }

    #[must_use]
    pub(crate) fn pathway(self) -> Self {
        self.for_engine(&PATHWAY_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn batman_bellman(self) -> Self {
        self.for_engine(&BATMAN_BELLMAN_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn babel(self) -> Self {
        self.for_engine(&BABEL_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn batman_classic(self) -> Self {
        self.for_engine(&BATMAN_CLASSIC_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn olsrv2(self) -> Self {
        self.for_engine(&OLSRV2_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn pathway_and_batman_bellman(self) -> Self {
        self.for_engines(&[PATHWAY_ENGINE_ID, BATMAN_BELLMAN_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn pathway_and_babel(self) -> Self {
        self.for_engines(&[PATHWAY_ENGINE_ID, BABEL_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn babel_and_batman_bellman(self) -> Self {
        self.for_engines(&[BABEL_ENGINE_ID, BATMAN_BELLMAN_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn pathway_and_olsrv2(self) -> Self {
        self.for_engines(&[PATHWAY_ENGINE_ID, OLSRV2_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn olsrv2_and_batman_bellman(self) -> Self {
        self.for_engines(&[OLSRV2_ENGINE_ID, BATMAN_BELLMAN_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn field(self) -> Self {
        self.for_engine(&FIELD_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn pathway_and_field(self) -> Self {
        self.for_engines(&[PATHWAY_ENGINE_ID, FIELD_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn field_and_batman_bellman(self) -> Self {
        self.for_engines(&[FIELD_ENGINE_ID, BATMAN_BELLMAN_ENGINE_ID])
    }

    #[must_use]
    pub(crate) fn scatter(self) -> Self {
        self.for_engine(&SCATTER_ENGINE_ID)
    }

    #[must_use]
    pub(crate) fn all_engines(self) -> Self {
        self.for_engines(&[
            PATHWAY_ENGINE_ID,
            FIELD_ENGINE_ID,
            BATMAN_BELLMAN_ENGINE_ID,
            BATMAN_CLASSIC_ENGINE_ID,
            BABEL_ENGINE_ID,
            OLSRV2_ENGINE_ID,
            SCATTER_ENGINE_ID,
        ])
    }

    #[must_use]
    pub(crate) fn observed_at(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub(crate) fn build(self) -> Node {
        NodePreset::route_capable_for_engines(
            NodePresetOptions::new(
                NodeIdentity::new(
                    NodeId([self.node_byte; 32]),
                    ControllerId([self.node_byte; 32]),
                ),
                reference_endpoint(self.node_byte),
                self.observed_at_tick,
            ),
            &self.routing_engines,
        )
        .build()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TopologyLinkPreset {
    endpoint_byte: u8,
    confidence: RatioPermille,
    observed_at_tick: Tick,
}

impl TopologyLinkPreset {
    #[must_use]
    pub(crate) fn with_confidence(mut self, confidence: RatioPermille) -> Self {
        self.confidence = confidence;
        self
    }

    #[must_use]
    pub(crate) fn observed_at(mut self, observed_at_tick: Tick) -> Self {
        self.observed_at_tick = observed_at_tick;
        self
    }

    #[must_use]
    pub(crate) fn build(self) -> Link {
        LinkPreset::lossy(
            LinkPresetOptions::new(
                reference_endpoint(self.endpoint_byte),
                self.observed_at_tick,
            )
            .with_confidence(self.confidence),
        )
        .build()
    }
}
