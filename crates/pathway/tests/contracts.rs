//! Pathway-owned contract tests for the public read-only extension seams.
//!
//! These tests live in `jacquard-pathway` because `PathwayTopologyModel` and
//! `PathwayRoutingEngine` are pathway-specific contracts, not shared `traits`
//! boundaries. They verify that the extension seams exposed on
//! `PathwayRoutingEngine` — `topology_model()` and `retention` — delegate
//! correctly to the underlying subcomponents rather than returning stale
//! copies or empty stubs. A test that accidentally reads a stale topology
//! digest or a zero-object retention count would mask a broken subcomponent
//! reference, so each assertion is driven by a concrete write first.

mod common;

use common::{
    engine::{build_engine, LOCAL_NODE_ID},
    fixtures::sample_configuration,
};
use jacquard_pathway::{PathwayRoutingEngine, PathwayTopologyModel};
use jacquard_traits::{
    jacquard_core::{Blake3Digest, ContentId},
    RetentionStore,
};

#[test]
fn pathway_routing_engine_exposes_explicit_pathway_owned_subcomponents() {
    let mut engine = build_engine();
    let object_id = ContentId { digest: Blake3Digest([8; 32]) };
    engine
        .retention
        .retain_payload(object_id, b"payload".to_vec())
        .expect("retain payload");

    assert_eq!(
        engine
            .topology_model()
            .adjacent_links(&LOCAL_NODE_ID, &sample_configuration().value)
            .len(),
        2
    );
    assert!(engine
        .retention_store()
        .contains_retained_payload(&object_id)
        .expect("payload present"));
}
