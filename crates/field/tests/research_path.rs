//! Guards for the coded-diffusion research boundary.

fn research_source() -> &'static str {
    include_str!("../src/research.rs")
}

#[test]
fn research_boundary_does_not_import_legacy_route_stack() {
    let source = research_source();
    let forbidden_imports = [
        "crate::planner",
        "crate::search",
        "crate::route",
        "planner::",
        "search::",
        "route::",
        "RouteShapeVisibility",
        "CorridorBeliefEnvelope",
        "FieldSearch",
        "FieldPlanner",
        "FieldBootstrap",
        "FieldRoute",
    ];

    for forbidden in forbidden_imports {
        assert!(
            !source.contains(forbidden),
            "research boundary must not depend on legacy route stack token `{forbidden}`"
        );
    }
}

#[test]
fn research_boundary_exposes_coded_diffusion_vocabulary() {
    let source = research_source();
    let required_terms = [
        "DiffusionMessageId",
        "CodedTargetId",
        "CodedEvidenceId",
        "DiffusionFragmentId",
        "CodingRankId",
        "LocalObservationId",
        "ContributionLedgerId",
        "ContributionLedgerKind",
        "ContributionLedgerRecord",
        "InferenceTaskId",
        "AnomalyClusterId",
        "AnomalyHypothesisSet",
        "AnomalyHypothesisScore",
        "AnomalyDecisionGuard",
        "AnomalyLandscape",
        "AnomalyLandscapeSummary",
        "EvidenceOriginMode",
        "CodedEvidenceRecord",
        "CodingWindow",
        "PayloadBudgetKind",
        "PayloadBudgetMetadata",
        "FragmentCustody",
        "FragmentArrivalClass",
        "ReceiverRankError",
        "ReceiverRankState",
        "ReconstructionQuorum",
        "DiffusionPressure",
        "FragmentSpreadBelief",
        "DiffusionOrderParameters",
        "NearCriticalControlState",
        "FragmentRetentionPolicy",
        "DelayedFragmentEvent",
        "FragmentReplayEvent",
        "PrivateProtocolRole",
    ];

    for required in required_terms {
        assert!(
            source.contains(required),
            "research boundary is missing `{required}`"
        );
    }
}
