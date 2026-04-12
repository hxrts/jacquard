//! Drift checks for field docs and maintained parity surfaces.

#[test]
fn field_docs_reference_current_pages_and_parity_ledger() {
    let summary = include_str!("../../../docs/SUMMARY.md");
    let crate_docs = include_str!("../src/lib.rs");
    let guide = include_str!("../../../verification/Field/Docs/Guide.md");

    assert!(summary.contains("403_field_routing.md"));
    assert!(summary.contains("404_profile_reference.md"));
    assert!(!summary.contains("404_field_routing.md"));
    assert!(!summary.contains("403_profile_reference.md"));

    assert!(crate_docs.contains("verification/Field/Docs/Parity.md"));
    assert!(guide.contains("Docs/Parity.md"));
}

#[test]
fn field_docs_keep_the_current_proof_boundary_explicit() {
    let field_routing = include_str!("../../../docs/403_field_routing.md");
    let adequacy = include_str!("../../../verification/Field/Docs/Adequacy.md");
    let parity = include_str!("../../../verification/Field/Docs/Parity.md");
    let protocol = include_str!("../../../verification/Field/Docs/Protocol.md");
    let guide = include_str!("../../../verification/Field/Docs/Guide.md");

    assert!(field_routing.contains("Lean covers:"));
    assert!(adequacy.contains("FieldReplaySnapshot"));
    assert!(adequacy.contains("reduced_protocol_replay()"));
    assert!(parity.contains("field is a single private-selector engine"));
    assert!(field_routing.contains("support-then-hop-then-stable"));
    assert!(protocol.contains("observational-only reconfiguration"));
    assert!(guide.contains("Field/Search/API.lean"));
    assert!(guide.contains("Field/Adequacy/Search.lean"));
}

#[test]
fn field_surfaces_ban_stale_route_vocabulary() {
    let sources = [
        include_str!("../src/attractor.rs"),
        include_str!("../src/planner.rs"),
        include_str!("../src/route.rs"),
        include_str!("../src/runtime.rs"),
        include_str!("../src/state.rs"),
        include_str!("../../../docs/403_field_routing.md"),
    ];

    for source in sources {
        assert!(!source.contains("primary_neighbor"));
        assert!(!source.contains("alternates"));
        assert!(!source.contains("MAX_ALTERNATE_COUNT"));
    }
}

#[test]
fn field_docs_keep_runtime_boundary_reduced() {
    let field_routing = include_str!("../../../docs/403_field_routing.md");
    let adequacy = include_str!("../../../verification/Field/Docs/Adequacy.md");
    let parity = include_str!("../../../verification/Field/Docs/Parity.md");

    assert!(field_routing.contains("expose the selected witness"));
    assert!(adequacy.contains("selected witness"));
    assert!(
        parity.contains("protocol artifacts and protocol reconfiguration are observational-only")
    );
}
