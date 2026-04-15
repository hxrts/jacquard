//! Router integration test for `jacquard-pathway`.

mod common;

use jacquard_core::{AdmissionDecision, Tick};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
};
use jacquard_pathway::{
    DeterministicPathwayTopologyModel, PathwayEngine, PATHWAY_CAPABILITIES, PATHWAY_ENGINE_ID,
};
use jacquard_testkit::router_integration::{
    activate_route_within_rounds, admitted_single_candidate, build_router,
};
use jacquard_traits::Blake3Hashing;

#[test]
fn pathway_materializes_explicit_path_route_within_bound() {
    let topology = common::fixtures::sample_configuration();
    let objective =
        common::engine::objective(jacquard_core::DestinationId::Node(common::FAR_NODE_ID));
    let profile = common::engine::profile();

    let engine = PathwayEngine::without_committee_selector(
        common::LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects {
            now: Tick(1),
            ..Default::default()
        },
        Blake3Hashing,
    );
    let admission = admitted_single_candidate(&engine, &objective, &profile, &topology)
        .expect("admit pathway candidate");
    assert_eq!(
        admission.admission_check.decision,
        AdmissionDecision::Admissible
    );

    let mut router = build_router(
        common::LOCAL_NODE_ID,
        topology.clone(),
        profile,
        topology.observed_at_tick,
        1,
    );
    router
        .register_engine(Box::new(PathwayEngine::without_committee_selector(
            common::LOCAL_NODE_ID,
            DeterministicPathwayTopologyModel::new(),
            InMemoryTransport::new(),
            InMemoryRetentionStore::default(),
            InMemoryRuntimeEffects {
                now: Tick(1),
                ..Default::default()
            },
            Blake3Hashing,
        )))
        .expect("register pathway engine");
    assert_eq!(
        router
            .registered_engine_capabilities(&PATHWAY_ENGINE_ID)
            .expect("pathway capabilities")
            .route_shape_visibility,
        PATHWAY_CAPABILITIES.route_shape_visibility
    );

    let route = activate_route_within_rounds(&mut router, &objective, 1)
        .unwrap_or_else(|err| panic!("expected pathway route within 1 round: {err}"));
    assert_eq!(
        route.identity.admission.backend_ref.engine,
        PATHWAY_ENGINE_ID
    );
    assert_eq!(
        route.identity.admission.admission_check,
        admission.admission_check
    );
}
