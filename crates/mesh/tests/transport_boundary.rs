//! Regression test for the shared transport-capability boundary.
//!
//! Control flow: this test instantiates `MeshEngine` with a local transport
//! that implements only `TransportEffects`. If route activation and
//! forwarding succeed, mesh is still generic over the shared transport effect
//! surface rather than over a mesh-specific transport trait.

mod common;

use common::{
    effects::{TestRetentionStore, TestRuntimeEffects},
    engine::{lease, materialization_input, objective, profile, LOCAL_NODE_ID},
    fixtures::sample_configuration,
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine};
use jacquard_traits::{
    effect_handler,
    jacquard_core::{
        DestinationId, LinkEndpoint, Tick, TransportError, TransportObservation,
    },
    Blake3Hashing, RoutingEngine, RoutingEnginePlanner, TransportEffects,
};

#[derive(Default)]
struct SharedOnlyTransport {
    sent_frames: Vec<(LinkEndpoint, Vec<u8>)>,
    observations: Vec<TransportObservation>,
}

#[effect_handler]
impl TransportEffects for SharedOnlyTransport {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.sent_frames.push((endpoint.clone(), payload.to_vec()));
        Ok(())
    }

    fn poll_transport(&mut self) -> Result<Vec<TransportObservation>, TransportError> {
        Ok(std::mem::take(&mut self.observations))
    }
}

#[test]
fn mesh_engine_accepts_transport_effects_without_a_mesh_specific_transport_trait() {
    let topology = sample_configuration();
    let mut engine = MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        SharedOnlyTransport::default(),
        TestRetentionStore::default(),
        TestRuntimeEffects { now: Tick(2), ..Default::default() },
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
    let admission = engine
        .admit_route(&goal, &profile, candidate, &topology)
        .expect("admission");
    let input = materialization_input(admission, lease(Tick(2), Tick(12)));
    let route_id = input.handle.route_id;
    engine
        .materialize_route(input)
        .expect("materialization succeeds");
    engine
        .forward_payload(&route_id, b"payload")
        .expect("forwarding succeeds");

    assert_eq!(engine.transport_adapter().sent_frames.len(), 1);
}
