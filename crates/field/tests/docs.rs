//! Drift checks for field docs and maintained parity surfaces.

use std::path::PathBuf;

fn repo_text(relative_path: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(relative_path),
    )
    .unwrap_or_else(|error| panic!("read {relative_path}: {error}"))
}

#[test]
fn field_docs_reference_current_pages_and_parity_ledger() {
    let summary = repo_text("docs/SUMMARY.md");
    let crate_docs = repo_text("crates/field/src/lib.rs");
    let guide = repo_text("verification/Field/Docs/Guide.md");

    assert!(!summary.contains("406_field_routing.md"));
    assert!(!summary.contains("409_coded_diffusion_research.md"));
    assert!(!summary.contains("410_active_belief_strong_paper.md"));
    assert!(summary.contains("305_profile_reference.md"));
    assert!(!summary.contains("404_field_routing.md"));
    assert!(!summary.contains("403_field_routing.md"));
    assert!(!summary.contains("403_profile_reference.md"));
    assert!(!summary.contains("404_profile_reference.md"));

    assert!(crate_docs.contains("verification/Field/Docs/Parity.md"));
    assert!(guide.contains("Docs/Parity.md"));
}

#[test]
fn field_docs_keep_the_current_proof_boundary_explicit() {
    let adequacy = repo_text("verification/Field/Docs/Adequacy.md");
    let parity = repo_text("verification/Field/Docs/Parity.md");
    let protocol = repo_text("verification/Field/Docs/Protocol.md");
    let guide = repo_text("verification/Field/Docs/Guide.md");
    let replay_fixtures = repo_text("verification/Field/Adequacy/ReplayFixtures.lean");
    let coded_diffusion = repo_text("analysis_2/research_boundary.md");

    assert!(coded_diffusion.contains("active implementation boundary"));
    assert!(coded_diffusion.contains("Field.CodedDiffusion"));
    assert!(coded_diffusion.contains("PayloadBudgetMetadata"));
    assert!(coded_diffusion.contains("ContributionLedgerRecord"));
    assert!(coded_diffusion.contains("reconstruction monotonicity"));
    assert!(coded_diffusion.contains("AnomalyLandscape"));
    assert!(coded_diffusion.contains("EvidenceVectorRecord"));
    assert!(coded_diffusion.contains("DecisionCommitmentState"));
    assert!(coded_diffusion.contains("ReceiverInferenceQualitySummary"));
    assert!(coded_diffusion.contains("CodedInferenceLandscapeEvent"));
    assert!(coded_diffusion.contains("artifacts/coded-inference/readiness"));
    assert!(coded_diffusion.contains("artifacts/coded-inference/baselines"));
    assert!(coded_diffusion.contains("uncoded-replication"));
    assert!(coded_diffusion.contains("epidemic-forwarding"));
    assert!(coded_diffusion.contains("spray-and-wait"));
    assert!(coded_diffusion.contains("uncontrolled-coded-diffusion"));
    assert!(coded_diffusion.contains("controlled-coded-diffusion"));
    assert!(coded_diffusion.contains("equal-payload-bytes"));
    assert!(coded_diffusion.contains("4096"));
    assert!(coded_diffusion.contains("direct delivery"));
    assert!(coded_diffusion.contains("PRoPHET/contact-frequency forwarding"));
    assert!(coded_diffusion.contains("R_est"));
    assert!(coded_diffusion.contains("R_low"));
    assert!(coded_diffusion.contains("R_high"));
    assert!(coded_diffusion.contains("W_infer"));
    assert!(coded_diffusion.contains("W_diff"));
    assert!(coded_diffusion.contains("controller ablation"));
    assert!(coded_diffusion.contains("target-band and budget sweep"));
    assert!(coded_diffusion.contains("Plot-ready rows"));
    assert!(coded_diffusion.contains("expected_innovation_gain"));
    assert!(coded_diffusion.contains("bridge_value"));
    assert!(coded_diffusion.contains("landscape_value"));
    assert!(coded_diffusion.contains("duplicate_risk"));
    assert!(coded_diffusion.contains("reproduction_pressure_penalty"));
    assert!(coded_diffusion.contains("deterministic-random-forwarding"));
    assert!(coded_diffusion.contains("local-evidence-policy-no-reproduction-control"));
    assert!(adequacy.contains("FieldReplaySnapshot"));
    assert!(adequacy.contains("reduced_protocol_replay()"));
    assert!(parity.contains("field is a single private-selector engine"));
    assert!(protocol.contains("observational-only reconfiguration"));
    assert!(guide.contains("Field/Search/API.lean"));
    assert!(guide.contains("Field/Adequacy/Search.lean"));
    assert!(protocol.contains("Field/Adequacy/ReplayFixtures.lean"));
    assert!(adequacy.contains("FieldExportedReplayBundle"));
    assert!(parity.contains("replay-derived fixture vocabulary"));
    assert!(replay_fixtures.contains("checkpoint-restore"));
}

#[test]
fn field_surfaces_ban_stale_route_vocabulary() {
    let sources = [
        repo_text("crates/field/src/attractor.rs"),
        repo_text("crates/field/src/planner/mod.rs"),
        repo_text("crates/field/src/planner/admission.rs"),
        repo_text("crates/field/src/planner/publication.rs"),
        repo_text("crates/field/src/planner/promotion.rs"),
        repo_text("crates/field/src/route.rs"),
        repo_text("crates/field/src/runtime/mod.rs"),
        repo_text("crates/field/src/runtime/control.rs"),
        repo_text("crates/field/src/runtime/observer.rs"),
        repo_text("crates/field/src/runtime/continuation.rs"),
        repo_text("crates/field/src/runtime/routing.rs"),
        repo_text("crates/field/src/runtime/sessions.rs"),
        repo_text("crates/field/src/state.rs"),
    ];

    for source in &sources {
        assert!(!source.contains("primary_neighbor"));
        assert!(!source.contains("alternates"));
        assert!(!source.contains("MAX_ALTERNATE_COUNT"));
    }
}

#[test]
fn field_docs_keep_runtime_boundary_reduced() {
    let adequacy = repo_text("verification/Field/Docs/Adequacy.md");
    let parity = repo_text("verification/Field/Docs/Parity.md");

    assert!(adequacy.contains("selected witness"));
    assert!(
        parity.contains("protocol artifacts and protocol reconfiguration are observational-only")
    );
    assert!(parity.contains("participant-set change stays outside"));
}

#[test]
fn field_docs_mark_corridor_routing_as_legacy_engine_baseline() {
    let crate_docs = repo_text("crates/field/src/lib.rs");
    let planner = repo_text("crates/field/src/planner/mod.rs");
    let search = repo_text("crates/field/src/search/mod.rs");
    let route = repo_text("crates/field/src/route.rs");

    assert!(crate_docs.contains("coded-diffusion research"));
    assert!(crate_docs.contains("coded evidence"));
    assert!(crate_docs.contains("contribution ledgers"));
    assert!(planner.contains("Baseline-only"));
    assert!(search.contains("Baseline-only"));
    assert!(route.contains("Baseline-only"));
}

#[test]
fn field_route_exports_are_grouped_under_baseline_boundary() {
    let crate_docs = repo_text("crates/field/src/lib.rs");

    assert!(crate_docs.contains("pub mod baseline"));
    assert!(crate_docs.contains("Baseline-only corridor-routing compatibility exports"));
    assert!(crate_docs.contains("FieldSearchConfig"));
    assert!(crate_docs.contains("FieldRuntimeRouteArtifact"));
    assert!(crate_docs.contains("FieldRouteRecoveryState"));
    assert!(crate_docs.contains("New coded-diffusion research work should use"));
}

#[test]
fn retained_scaffolding_documents_coded_diffusion_roles() {
    let research = repo_text("crates/field/src/research.rs");
    let control = repo_text("crates/field/src/control.rs");
    let observer = repo_text("crates/field/src/observer.rs");
    let choreography = repo_text("crates/field/src/choreography.rs");
    let runtime_observer = repo_text("crates/field/src/runtime/observer.rs");
    let runtime_control = repo_text("crates/field/src/runtime/control.rs");
    let runtime_sessions = repo_text("crates/field/src/runtime/sessions.rs");

    for required in [
        "FragmentSpreadBelief",
        "DiffusionOrderParameters",
        "NearCriticalControlState",
        "FragmentRetentionPolicy",
        "DelayedFragmentEvent",
        "FragmentReplayEvent",
        "PrivateProtocolRole",
    ] {
        assert!(research.contains(required));
    }
    assert!(control.contains("rank deficit"));
    assert!(observer.contains("FragmentSpreadBelief"));
    assert!(choreography.contains("PrivateProtocolRole"));
    assert!(runtime_observer.contains("fragment custody"));
    assert!(runtime_control.contains("fragment-control coordination"));
    assert!(runtime_sessions.contains("delayed"));
}
