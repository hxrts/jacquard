//! Regression test for the shared transport-capability boundary.
//!
//! Control flow: this test instantiates `PathwayEngine` with a local transport
//! that implements the shared send capability plus host-owned ingress driver,
//! but no pathway-specific transport trait. If route activation and
//! forwarding succeed, mesh is still generic over the shared transport effect
//! surface rather than over a mesh-specific transport trait.

mod common;

use std::sync::{Arc, Mutex};

use common::{
    effects::{TestRetentionStore, TestRuntimeEffects},
    engine::{lease, materialization_input, objective, profile, LOCAL_NODE_ID},
    fixtures::sample_configuration,
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine};
use jacquard_traits::{
    effect_handler,
    jacquard_core::{
        DestinationId, LinkEndpoint, Tick, TransportError, TransportIngressEvent,
    },
    Blake3Hashing, RouterManagedEngine, RoutingEngine, RoutingEnginePlanner,
    TransportDriver, TransportSenderEffects,
};

#[derive(Default)]
struct SharedOnlyTransportState {
    sent_frames: Vec<(LinkEndpoint, Vec<u8>)>,
    ingress_events: Vec<TransportIngressEvent>,
}

#[derive(Clone, Default)]
struct SharedOnlyTransport(Arc<Mutex<SharedOnlyTransportState>>);

impl SharedOnlyTransport {
    #[must_use]
    fn sent_frame_count(&self) -> usize {
        self.0
            .lock()
            .expect("shared-only transport lock")
            .sent_frames
            .len()
    }
}

#[effect_handler]
impl TransportSenderEffects for SharedOnlyTransport {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.0
            .lock()
            .expect("shared-only transport lock")
            .sent_frames
            .push((endpoint.clone(), payload.to_vec()));
        Ok(())
    }
}

impl TransportDriver for SharedOnlyTransport {
    fn drain_transport_ingress(
        &mut self,
    ) -> Result<Vec<TransportIngressEvent>, TransportError> {
        Ok(std::mem::take(
            &mut self
                .0
                .lock()
                .expect("shared-only transport lock")
                .ingress_events,
        ))
    }
}

#[test]
fn mesh_engine_accepts_shared_transport_sender_and_driver_without_a_pathway_transport_trait(
) {
    let topology = sample_configuration();
    let transport = SharedOnlyTransport::default();
    let mut engine = PathwayEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        transport.clone(),
        TestRetentionStore::default(),
        TestRuntimeEffects::with_now(Tick(2)),
        Blake3Hashing,
    );
    let goal = objective(DestinationId::Node(jacquard_traits::jacquard_core::NodeId(
        [3; 32],
    )));
    let profile = profile();

    engine
        .engine_tick(&jacquard_traits::jacquard_core::RoutingTickContext::new(
            topology.clone(),
        ))
        .expect("engine tick");
    let candidate = engine
        .candidate_routes(&goal, &profile, &topology)
        .into_iter()
        .next()
        .expect("candidate");
    let route_id = candidate.route_id;
    let admission = engine
        .admit_route(&goal, &profile, candidate, &topology)
        .expect("admission");
    let input = materialization_input(route_id, admission, lease(Tick(2), Tick(12)));
    engine
        .materialize_route(input)
        .expect("materialization succeeds");
    engine
        .forward_payload_for_router(&route_id, b"payload")
        .expect("forwarding succeeds");

    assert_eq!(transport.sent_frame_count(), 1);
}
