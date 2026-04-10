//! Integration tests for engine registration, selection precedence, and
//! router-owned lease management.
//!
//! These tests exercise the `MultiEngineRouter::register_engine` path and the
//! router's handling of multiple simultaneously registered engines. The
//! `NullCandidateEngine` stub is used as an auxiliary engine that never
//! produces candidates, confirming that the router correctly falls back to
//! pathway when an auxiliary engine provides no route opinions.
//!
//! Key behaviors covered:
//! - Duplicate engine registration for the same `RoutingEngineId` is rejected
//!   with `CapabilityError::Rejected`.
//! - Multiple engines can be registered; the pathway engine wins candidate
//!   selection when all other engines produce no candidates.
//! - `transfer_route_lease` updates the router-owned lease to reflect the new
//!   owner node and increments the `lease_epoch`, enforcing that lease
//!   ownership is tracked exclusively in canonical router state.

mod common;

use common::{
    build_router, objective, NullCandidateEngine, FAR_NODE_ID, LOCAL_NODE_ID, PEER_NODE_ID,
};
use jacquard_core::{CapabilityError, DestinationId, ReceiptId, RouteSemanticHandoff, Tick};
use jacquard_mem_link_profile::{
    InMemoryRetentionStore, InMemoryRuntimeEffects, InMemoryTransport,
};
use jacquard_pathway::{DeterministicPathwayTopologyModel, PathwayEngine, PATHWAY_ENGINE_ID};
use jacquard_traits::{Blake3Hashing, Router};

#[test]
fn multi_engine_router_rejects_duplicate_pathway_registration() {
    let mut router = build_router(Tick(2));
    let duplicate_engine = PathwayEngine::without_committee_selector(
        LOCAL_NODE_ID,
        DeterministicPathwayTopologyModel::new(),
        InMemoryTransport::new(),
        InMemoryRetentionStore::default(),
        InMemoryRuntimeEffects {
            now: Tick(2),
            ..Default::default()
        },
        Blake3Hashing,
    );

    let error = router
        .register_engine(Box::new(duplicate_engine))
        .expect_err("duplicate pathway engine should be rejected");

    assert_eq!(error, CapabilityError::Rejected.into());
}

#[test]
fn multi_engine_router_registers_multiple_engines_and_selects_pathway_candidate() {
    let mut router = build_router(Tick(2));
    let auxiliary_engine_id = jacquard_core::RoutingEngineId::from_contract_bytes([6; 16]);
    let auxiliary = NullCandidateEngine::new(LOCAL_NODE_ID, auxiliary_engine_id.clone());

    router
        .register_engine(Box::new(auxiliary))
        .expect("register auxiliary engine");

    let route = Router::activate_route(&mut router, objective(DestinationId::Node(FAR_NODE_ID)))
        .expect("router activation");

    assert_eq!(
        router.registered_engine_ids(),
        vec![auxiliary_engine_id, PATHWAY_ENGINE_ID],
    );
    assert_eq!(route.identity.admission.summary.engine, PATHWAY_ENGINE_ID);
}

#[test]
fn transfer_route_lease_updates_router_owned_lease() {
    let mut router = build_router(Tick(2));
    let route = Router::activate_route(&mut router, objective(DestinationId::Node(FAR_NODE_ID)))
        .expect("activation");

    let handoff = RouteSemanticHandoff {
        route_id: route.identity.stamp.route_id,
        from_node_id: LOCAL_NODE_ID,
        to_node_id: PEER_NODE_ID,
        handoff_epoch: jacquard_core::RouteEpoch(3),
        receipt_id: ReceiptId([9; 16]),
    };

    let transferred = router
        .transfer_route_lease(&route.identity.stamp.route_id, handoff.clone())
        .expect("lease transfer");

    assert_eq!(transferred.identity.lease.owner_node_id, PEER_NODE_ID);
    assert_eq!(
        transferred.identity.lease.lease_epoch,
        handoff.handoff_epoch
    );
}
