//! Integration tests for the deterministic mesh committee selector.
//!
//! Unit tests in `committee.rs` cover the four return-`None` guard
//! branches in isolation. This file exercises the public selector
//! through `CommitteeSelector::select_committee` against a configured
//! topology and confirms the result is deterministic across repeated
//! calls and non-empty under the standard mesh fixture.

mod common;

use jacquard_mesh::DeterministicCommitteeSelector;
use jacquard_traits::{
    jacquard_core::{DestinationId, NodeId, ServiceId},
    CommitteeSelector,
};

use common::engine::{objective, profile};
use common::fixtures::sample_configuration;

// Two calls to the selector on the same inputs must return the same
// `Option<CommitteeSelection>`. The standard sample fixture should
// produce a `Some` result so the determinism check is meaningful.
#[test]
fn committee_selection_is_optional_and_deterministic() {
    let selector = DeterministicCommitteeSelector::new(NodeId([1; 32]));
    let topology = sample_configuration();
    let goal = objective(DestinationId::Service(ServiceId(vec![9, 9])));
    let policy = profile();

    let first = selector
        .select_committee(&goal, &policy, &topology)
        .expect("selector result");
    let second = selector
        .select_committee(&goal, &policy, &topology)
        .expect("selector result");

    assert_eq!(first, second);
    assert!(first.is_some());
}
