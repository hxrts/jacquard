//! Mesh-facing subcomponent interfaces.
//!
//! These traits are the high-level contract boundaries for mesh internals that
//! need to remain swappable across crates and runtimes.
//!
//! Effect boundary:
//! - `MeshTopologyModel` is read-only. It should be deterministic with respect
//!   to its inputs and must not mutate canonical route state.
//! - `MeshTransport` is effectful. It carries frames and reports transport
//!   observations, but
//!   it must not impose sequencing, traffic control, or routing truth.
//! - `RetentionStore` is effectful. It stores opaque deferred-delivery payloads,
//!   but it must not interpret higher-level routing semantics.

use jacquard_core::{
    Blake3Digest, Configuration, ContentId, Link, LinkEndpoint, Node, NodeId, RetentionError,
    TransportError, TransportObservation, TransportProtocol,
};
use jacquard_macros::purity;

use crate::{effect_handler, RoutingEngine, TransportEffects};

#[purity(read_only)]
/// Deterministic, read-only topology queries used by the mesh planner/runtime.
///
/// Read-only deterministic boundary.
pub trait MeshTopologyModel {
    #[must_use]
    fn local_node(&self, local_node_id: &NodeId, configuration: &Configuration) -> Option<Node>;

    #[must_use]
    fn neighboring_nodes(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<(NodeId, Node)>;

    #[must_use]
    fn reachable_endpoints(
        &self,
        local_node_id: &NodeId,
        configuration: &Configuration,
    ) -> Vec<LinkEndpoint>;

    #[must_use]
    fn adjacent_links(&self, local_node_id: &NodeId, configuration: &Configuration) -> Vec<Link>;
}

#[purity(effectful)]
/// Effectful frame-carrier boundary for one mesh transport implementation.
///
/// Effectful runtime boundary.
pub trait MeshTransport {
    #[must_use]
    fn transport_id(&self) -> TransportProtocol;

    fn send_frame(&mut self, endpoint: &LinkEndpoint, payload: &[u8])
        -> Result<(), TransportError>;

    fn poll_observations(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}

// A concrete mesh transport adapter is also a concrete transport effect handler.
#[effect_handler]
impl<T> TransportEffects for T
where
    T: MeshTransport + Send + Sync + 'static,
{
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.send_frame(endpoint, payload)
    }

    fn poll_transport(&mut self) -> Result<Vec<TransportObservation>, TransportError> {
        self.poll_observations()
    }
}

#[purity(effectful)]
/// Effectful deferred-delivery retention boundary.
///
/// Effectful runtime boundary.
pub trait RetentionStore {
    fn retain_payload(
        &mut self,
        object_id: ContentId<Blake3Digest>,
        payload: Vec<u8>,
    ) -> Result<(), RetentionError>;

    fn take_retained_payload(
        &mut self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<Option<Vec<u8>>, RetentionError>;

    fn contains_retained_payload(
        &self,
        object_id: &ContentId<Blake3Digest>,
    ) -> Result<bool, RetentionError>;
}

#[purity(effectful)]
/// Mesh-specialized routing-engine boundary with explicit subcomponent ownership.
///
/// Planning purity stays in `RoutingEnginePlanner` plus `MeshTopologyModel`.
/// This trait only binds the effectful routing-engine runtime to its swappable
/// subcomponents.
///
/// Effectful runtime boundary with read-only subcomponent accessors.
pub trait MeshRoutingEngine: RoutingEngine {
    type TopologyModel: MeshTopologyModel;
    type Transport: MeshTransport;
    type Retention: RetentionStore;

    fn topology_model(&self) -> &Self::TopologyModel;

    fn transport(&self) -> &Self::Transport;

    fn transport_mut(&mut self) -> &mut Self::Transport;

    fn retention_store(&self) -> &Self::Retention;

    fn retention_store_mut(&mut self) -> &mut Self::Retention;
}
