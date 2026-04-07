//! Integration tests for the mesh admission rejection paths.
//!
//! `mesh_admission_check` has three rejection branches and one admit
//! branch. The unit tests in `engine.rs` exercise the function directly
//! against synthetic summaries. These tests drive the same branches end
//! to end through `candidate_routes`, `check_candidate`, and `admit_route`
//! so that engine wiring, candidate caching, and the admission check are
//! covered together.

mod common;

use common::{build_engine, objective_with_floor, profile_with_connectivity, sample_configuration};
use jacquard_mesh::MESH_ENGINE_ID;
use jacquard_traits::{
    jacquard_core::{
        AdmissionDecision, DestinationId, NodeId, RouteAdmissionRejection, RoutePartitionClass,
        RouteProtectionClass, RouteRepairClass, RouteSelectionError,
    },
    RoutingEnginePlanner,
};

// A candidate produced by the mesh engine always carries LinkProtected
// summary protection. Asking for a TopologyProtected floor must drive
// the admission check into ProtectionFloorUnsatisfied.
#[test]
fn admit_route_rejects_when_summary_protection_is_below_floor() {
    let engine = build_engine();
    let topology = sample_configuration();
    let objective = objective_with_floor(
        DestinationId::Node(NodeId([3; 32])),
        RouteProtectionClass::TopologyProtected,
        RouteProtectionClass::TopologyProtected,
    );
    let profile = profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::PartitionTolerant,
    );

    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("a candidate should still be produced before admission filtering");
    let check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("check_candidate should succeed even when the decision is rejection");
    assert!(matches!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::ProtectionFloorUnsatisfied)
    ));

    let admission_error = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect_err("admit_route must return Inadmissible for protection floor regression");
    assert!(matches!(
        admission_error,
        jacquard_traits::jacquard_core::RouteError::Selection(RouteSelectionError::Inadmissible(
            RouteAdmissionRejection::ProtectionFloorUnsatisfied
        ))
    ));
}

// Mesh candidates always advertise repairable connectivity, so the
// BranchingInfeasible branch is unreachable through `candidate_routes`.
// The unit test in engine.rs covers it directly. Here we drive the
// partition mismatch branch by asking for a 1-hop Direct route, which
// the engine produces with ConnectedOnly partition support, against a
// PartitionTolerant profile.
#[test]
fn admit_route_rejects_when_profile_requires_partition_tolerance_and_summary_does_not() {
    let engine = build_engine();
    let topology = sample_configuration();
    // A direct (1-hop) Node destination is built with route_class Direct,
    // which produces ConnectedOnly partition support. The profile asks
    // for partition tolerance, so admission must fail.
    let objective = objective_with_floor(
        DestinationId::Node(NodeId([2; 32])),
        RouteProtectionClass::LinkProtected,
        RouteProtectionClass::LinkProtected,
    );
    let profile = profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::PartitionTolerant,
    );

    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("direct path candidate should be produced");
    let check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("check_candidate should succeed");
    assert!(matches!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    ));

    let admission_error = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect_err("admit_route must reject partition mismatch");
    assert!(matches!(
        admission_error,
        jacquard_traits::jacquard_core::RouteError::Selection(RouteSelectionError::Inadmissible(
            RouteAdmissionRejection::BackendUnavailable
        ))
    ));
}

// Sanity check that a profile and objective the engine fully supports
// produces an admissible decision. The two-hop service path is
// classified DeferredDelivery, which advertises PartitionTolerant
// connectivity, so a partition-tolerant profile must be admitted.
#[test]
fn admit_route_succeeds_for_partition_tolerant_deferred_delivery_path() {
    let engine = build_engine();
    let topology = sample_configuration();
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("multi-hop candidate should be produced");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("admit_route should succeed");
    assert!(matches!(
        admission.admission_check.decision,
        AdmissionDecision::Admissible
    ));
}

// Repeated admission checks on the same candidate must agree, and the
// admission record must carry the topology epoch and the mesh engine id
// in its witness and summary so that the router can attribute the route.
#[test]
fn admission_emits_stable_check_and_witness_values() {
    let engine = build_engine();
    let topology = sample_configuration();
    let objective = common::objective(DestinationId::Node(NodeId([3; 32])));
    let profile = common::profile();

    let candidate = engine
        .candidate_routes(&objective, &profile, &topology)
        .into_iter()
        .next()
        .expect("node destination should yield a candidate");
    let first_check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("candidate check");
    let second_check = engine
        .check_candidate(&objective, &profile, &candidate, &topology)
        .expect("candidate check");
    let admission = engine
        .admit_route(&objective, &profile, candidate, &topology)
        .expect("route admission");

    assert_eq!(first_check, second_check);
    assert_eq!(admission.admission_check, first_check);
    assert_eq!(admission.witness.topology_epoch, topology.value.epoch);
    assert_eq!(admission.summary.engine, MESH_ENGINE_ID);
}
