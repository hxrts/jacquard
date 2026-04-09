//! Integration tests for the pathway admission rejection paths.
//!
//! `pathway_admission_check` has three rejection branches and one admit
//! branch. The unit tests in `engine.rs` exercise the function directly
//! against synthetic summaries. These tests drive the same branches end
//! to end through `candidate_routes`, `check_candidate`, and
//! `admit_route` so that engine wiring, candidate caching, and the
//! admission check are covered together.

mod common;

use common::{
    engine::{
        build_engine, objective, objective_with_floor, profile,
        profile_with_connectivity,
    },
    fixtures::sample_configuration,
};
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_traits::{
    jacquard_core::{
        AdmissionDecision, Belief, ByteCount, DestinationId, Estimate, NodeId,
        RouteAdmissionRejection, RouteError, RoutePartitionClass, RouteProtectionClass,
        RouteRepairClass, RouteSelectionError, RouteServiceKind, Tick,
    },
    RoutingEnginePlanner,
};

fn keep_only_move_service(
    topology: &mut jacquard_traits::jacquard_core::Observation<
        jacquard_traits::jacquard_core::Configuration,
    >,
    node_id: NodeId,
) {
    let node = topology
        .value
        .nodes
        .get_mut(&node_id)
        .expect("destination node present");
    node.profile
        .services
        .retain(|service| service.service_kind == RouteServiceKind::Move);
}

// A candidate produced by the pathway engine always carries LinkProtected
// summary protection. Asking for a TopologyProtected floor must drive
// the admission check into ProtectionFloorUnsatisfied.
#[test]
fn admit_route_rejects_when_summary_protection_is_below_floor() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective_with_floor(
        DestinationId::Node(NodeId([3; 32])),
        RouteProtectionClass::TopologyProtected,
        RouteProtectionClass::TopologyProtected,
    );
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::PartitionTolerant,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("a candidate should still be produced before admission filtering");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("check_candidate should succeed even when the decision is rejection");
    assert!(matches!(
        check.decision,
        AdmissionDecision::Rejected(
            RouteAdmissionRejection::ProtectionFloorUnsatisfied
        )
    ));

    let admission_error = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect_err(
            "admit_route must return Inadmissible for protection floor regression",
        );
    assert!(matches!(
        admission_error,
        RouteError::Selection(RouteSelectionError::Inadmissible(
            RouteAdmissionRejection::ProtectionFloorUnsatisfied
        ))
    ));
}

// Direct routes without credible patch space are now classified
// BestEffort. A repair-demanding profile must reject them through the
// same end-to-end admission path the router uses.
#[test]
fn admit_route_rejects_when_profile_requires_repair_and_candidate_is_best_effort() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective_with_floor(
        DestinationId::Node(NodeId([2; 32])),
        RouteProtectionClass::LinkProtected,
        RouteProtectionClass::LinkProtected,
    );
    let policy = profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::ConnectedOnly,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("direct path candidate should be produced");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("check_candidate should succeed");
    assert!(matches!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BranchingInfeasible)
    ));

    let admission_error = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect_err("admit_route must reject best-effort repair mismatch");
    assert!(matches!(
        admission_error,
        RouteError::Selection(RouteSelectionError::Inadmissible(
            RouteAdmissionRejection::BranchingInfeasible
        ))
    ));
}

// The same direct route remains admissible when the profile honestly asks for
// BestEffort repair semantics. This locks in the boundary that "best effort"
// and "repairable" remain distinct rather than being silently collapsed.
#[test]
fn admit_route_accepts_best_effort_candidate_when_profile_matches() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective_with_floor(
        DestinationId::Node(NodeId([2; 32])),
        RouteProtectionClass::LinkProtected,
        RouteProtectionClass::LinkProtected,
    );
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::ConnectedOnly,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("direct path candidate should be produced");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("best-effort profile should admit best-effort candidate");

    assert_eq!(
        admission.witness.connectivity.delivered.repair,
        RouteRepairClass::BestEffort
    );
    assert!(matches!(
        admission.admission_check.decision,
        AdmissionDecision::Admissible
    ));
}

// Asking for partition tolerance against a direct route still exercises
// the partition mismatch path independently of the new repair
// classification.
#[test]
fn admit_route_rejects_when_profile_requires_partition_tolerance_and_summary_does_not()
{
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective_with_floor(
        DestinationId::Node(NodeId([2; 32])),
        RouteProtectionClass::LinkProtected,
        RouteProtectionClass::LinkProtected,
    );
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::PartitionTolerant,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("direct path candidate should be produced");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("check_candidate should succeed");
    assert_eq!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    );

    let admission_error = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect_err("admit_route must reject partition mismatch");
    assert!(matches!(
        admission_error,
        RouteError::Selection(RouteSelectionError::Inadmissible(
            RouteAdmissionRejection::BackendUnavailable
        ))
    ));
}

#[test]
fn move_only_destination_is_still_reachable_when_hold_fallback_is_forbidden() {
    let engine = build_engine();
    let mut topology = sample_configuration();
    keep_only_move_service(&mut topology, NodeId([4; 32]));

    let mut goal = objective(DestinationId::Node(NodeId([4; 32])));
    goal.hold_fallback_policy =
        jacquard_traits::jacquard_core::HoldFallbackPolicy::Forbidden;
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::ConnectedOnly,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("move-only direct destination should still produce a candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("non-deferred move-only destination should be admissible");
    assert!(matches!(
        admission.admission_check.decision,
        AdmissionDecision::Admissible
    ));
}

#[test]
fn move_only_destination_is_still_reachable_when_hold_is_allowed_but_not_needed() {
    let engine = build_engine();
    let mut topology = sample_configuration();
    keep_only_move_service(&mut topology, NodeId([4; 32]));

    let goal = objective(DestinationId::Node(NodeId([4; 32])));
    let policy = profile_with_connectivity(
        RouteRepairClass::BestEffort,
        RoutePartitionClass::ConnectedOnly,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("move-only direct destination should still produce a candidate");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("direct non-deferred path should not require hold");
    assert!(matches!(
        admission.admission_check.decision,
        AdmissionDecision::Admissible
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
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("multi-hop candidate should be produced");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("admit_route should succeed");
    assert!(matches!(
        admission.admission_check.decision,
        AdmissionDecision::Admissible
    ));
}

// Repeated admission checks on the same candidate must agree, and the
// admission record must carry the topology epoch and the pathway engine id
// in its witness and summary so that the router can attribute the route.
#[test]
fn admission_emits_stable_check_and_witness_values() {
    let engine = build_engine();
    let topology = sample_configuration();
    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("node destination should yield a candidate");
    let first_check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("candidate check");
    let second_check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("candidate check");
    let admission = engine
        .admit_route(&goal, &policy, candidate, &topology)
        .expect("route admission");

    assert_eq!(first_check, second_check);
    assert_eq!(admission.admission_check, first_check);
    assert_eq!(admission.witness.topology_epoch, topology.value.epoch);
    assert_eq!(admission.summary.engine, PATHWAY_ENGINE_ID);
}

// A Hold advertisement alone is not enough for deferred delivery. The
// destination must also report positive currently available hold
// capacity in shared node state, otherwise the route falls back to a
// non-partition-tolerant class and admission rejects it under the
// standard partition-tolerant profile.
#[test]
fn hold_advertised_without_available_capacity_is_not_deferred_delivery_capable() {
    let engine = build_engine();
    let mut topology = sample_configuration();
    let destination = topology
        .value
        .nodes
        .get_mut(&NodeId([3; 32]))
        .expect("destination node present");
    destination.state.hold_capacity_available_bytes = Belief::Estimated(Estimate {
        value: ByteCount(0),
        confidence_permille: jacquard_traits::jacquard_core::RatioPermille(1000),
        updated_at_tick: Tick(2),
    });

    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile();

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("multi-hop candidate should still be produced");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("candidate check should succeed");

    assert!(matches!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    ));
}

#[test]
fn deferred_delivery_still_requires_real_hold_service() {
    let engine = build_engine();
    let mut topology = sample_configuration();
    keep_only_move_service(&mut topology, NodeId([3; 32]));

    let goal = objective(DestinationId::Node(NodeId([3; 32])));
    let policy = profile_with_connectivity(
        RouteRepairClass::Repairable,
        RoutePartitionClass::PartitionTolerant,
    );

    let candidate = engine
        .candidate_routes(&goal, &policy, &topology)
        .into_iter()
        .next()
        .expect("multi-hop candidate should still be produced");
    let check = engine
        .check_candidate(&goal, &policy, &candidate, &topology)
        .expect("candidate check should succeed");
    assert!(matches!(
        check.decision,
        AdmissionDecision::Rejected(RouteAdmissionRejection::BackendUnavailable)
    ));
}
