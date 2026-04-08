mod common;

use common::{
    build_router, objective, NullCandidateEngine, FAR_NODE_ID, LOCAL_NODE_ID,
    PEER_NODE_ID,
};
use jacquard_core::{
    CapabilityError, DestinationId, ReceiptId, RouteSemanticHandoff, Tick,
};
use jacquard_mem_link_profile::{
    InMemoryMeshTransport, InMemoryRetentionStore, InMemoryRuntimeEffects,
};
use jacquard_mesh::{DeterministicMeshTopologyModel, MeshEngine, MESH_ENGINE_ID};
use jacquard_traits::{Blake3Hashing, Router};

#[test]
fn multi_engine_router_rejects_duplicate_mesh_registration() {
    let mut router = build_router(Tick(2));
    let duplicate_engine = MeshEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicMeshTopologyModel::new(),
        InMemoryMeshTransport::new(jacquard_core::TransportProtocol::BleGatt),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects { now: Tick(2), ..Default::default() },
        Blake3Hashing,
    );

    let error = router
        .register_engine(Box::new(duplicate_engine))
        .expect_err("duplicate mesh engine should be rejected");

    assert_eq!(error, CapabilityError::Rejected.into());
}

#[test]
fn multi_engine_router_registers_multiple_engines_and_selects_mesh_candidate() {
    let mut router = build_router(Tick(2));
    let auxiliary = NullCandidateEngine::new(
        LOCAL_NODE_ID,
        jacquard_core::RoutingEngineId::External {
            name: "aux".to_string(),
            contract_id: jacquard_core::RoutingEngineContractId([6; 16]),
        },
    );

    router
        .register_engine(Box::new(auxiliary))
        .expect("register auxiliary engine");

    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("router activation");

    assert_eq!(
        router.registered_engine_ids(),
        vec![
            MESH_ENGINE_ID,
            jacquard_core::RoutingEngineId::External {
                name: "aux".to_string(),
                contract_id: jacquard_core::RoutingEngineContractId([6; 16]),
            },
        ],
    );
    assert_eq!(route.identity.admission.summary.engine, MESH_ENGINE_ID);
}

#[test]
fn transfer_route_lease_updates_router_owned_lease() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(
        &mut router,
        objective(DestinationId::Node(FAR_NODE_ID)),
    )
    .expect("activation");

    let handoff = RouteSemanticHandoff {
        route_id: route.identity.handle.route_id,
        from_node_id: LOCAL_NODE_ID,
        to_node_id: PEER_NODE_ID,
        handoff_epoch: jacquard_core::RouteEpoch(3),
        receipt_id: ReceiptId([9; 16]),
    };

    let transferred = router
        .transfer_route_lease(&route.identity.handle.route_id, handoff.clone())
        .expect("lease transfer");

    assert_eq!(transferred.identity.lease.owner_node_id, PEER_NODE_ID);
    assert_eq!(
        transferred.identity.lease.lease_epoch,
        handoff.handoff_epoch
    );
}
