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

    assert!(summary.contains("406_field_routing.md"));
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
    let field_routing = repo_text("docs/406_field_routing.md");
    let adequacy = repo_text("verification/Field/Docs/Adequacy.md");
    let parity = repo_text("verification/Field/Docs/Parity.md");
    let protocol = repo_text("verification/Field/Docs/Protocol.md");
    let guide = repo_text("verification/Field/Docs/Guide.md");
    let replay_fixtures = repo_text("verification/Field/Adequacy/ReplayFixtures.lean");

    assert!(field_routing.contains("Lean covers:"));
    assert!(adequacy.contains("FieldReplaySnapshot"));
    assert!(adequacy.contains("reduced_protocol_replay()"));
    assert!(parity.contains("field is a single private-selector engine"));
    assert!(field_routing.contains("support-then-hop-then-stable"));
    assert!(protocol.contains("observational-only reconfiguration"));
    assert!(guide.contains("Field/Search/API.lean"));
    assert!(guide.contains("Field/Adequacy/Search.lean"));
    assert!(field_routing.contains("FieldExportedReplayBundle"));
    assert!(field_routing.contains("participant-set change is not supported"));
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
        repo_text("docs/406_field_routing.md"),
    ];

    for source in &sources {
        assert!(!source.contains("primary_neighbor"));
        assert!(!source.contains("alternates"));
        assert!(!source.contains("MAX_ALTERNATE_COUNT"));
    }
}

#[test]
fn field_docs_keep_runtime_boundary_reduced() {
    let field_routing = repo_text("docs/406_field_routing.md");
    let adequacy = repo_text("verification/Field/Docs/Adequacy.md");
    let parity = repo_text("verification/Field/Docs/Parity.md");

    assert!(field_routing.contains("expose the selected witness"));
    assert!(adequacy.contains("selected witness"));
    assert!(
        parity.contains("protocol artifacts and protocol reconfiguration are observational-only")
    );
    assert!(parity.contains("participant-set change stays outside"));
}

#[test]
fn field_docs_mark_corridor_routing_as_baseline_only_for_research() {
    let crate_docs = repo_text("crates/field/src/lib.rs");
    let planner = repo_text("crates/field/src/planner/mod.rs");
    let search = repo_text("crates/field/src/search/mod.rs");
    let route = repo_text("crates/field/src/route.rs");
    let field_routing = repo_text("docs/406_field_routing.md");

    assert!(crate_docs.contains("coded-diffusion research"));
    assert!(crate_docs.contains("message fragments"));
    assert!(planner.contains("Baseline-only"));
    assert!(search.contains("Baseline-only"));
    assert!(route.contains("Baseline-only"));
    assert!(field_routing.contains("baseline-only"));
    assert!(field_routing.contains("not the active coded-diffusion research objective"));
}
