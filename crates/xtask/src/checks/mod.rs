//! Check registry for `cargo xtask check <name>`.
//!
//! Each public submodule implements exactly one named policy check. The `run`
//! function dispatches on the first CLI argument, routing to the matching
//! submodule or returning an informative error for unknown names.
//!
//! Registered check names:
//!   adapter-boundary, checkpoint-namespacing, crate-boundary,
//!   docs-link-check, docs-semantic-drift, dx-surface, engine-service-boundary,
//!   fail-closed-ordering, invariant-specs, no-scratch-refs-in-rust,
//!   no-usize-in-models, ownership-invariants, pathway-async-boundary,
//!   pathway-choreography, proc-macro-scope, proof-bearing-actions,
//!   reference-bridge-boundary, result-must-use, router-round-boundary,
//!   routing-invariants, rust-style-guide, simulator-boundary,
//! surface-classification, test-boundaries,   trait-purity,
//! transport-authoring-boundary, transport-ownership-boundary.

pub mod adapter_boundary;
pub mod checkpoint_namespacing;
pub mod crate_boundary;
pub mod docs_link_check;
pub mod docs_semantic_drift;
pub mod dx_surface;
pub mod engine_service_boundary;
pub mod fail_closed_ordering;
pub mod invariant_specs;
pub mod no_scratch_refs_in_rust;
pub mod no_usize_in_models;
pub mod ownership_invariants;
pub mod pathway_async_boundary;
pub mod pathway_choreography;
pub mod pre_commit;
pub mod proc_macro_scope;
pub mod proof_bearing_actions;
pub mod reference_bridge_boundary;
pub mod result_must_use;
pub mod router_round_boundary;
pub mod routing_invariants;
pub mod rust_style_guide;
pub mod simulator_boundary;
pub mod surface_classification;
pub mod test_boundaries;
pub mod trait_purity;
pub mod transport_authoring_boundary;
pub mod transport_ownership_boundary;

use anyhow::{bail, Result};

pub fn run(args: Vec<String>) -> Result<()> {
    let Some(name) = args.first().map(String::as_str) else {
        bail!("xtask: usage: cargo xtask check <name> [args]");
    };
    let rest = &args[1..];
    match name {
        | "adapter-boundary" => adapter_boundary::run(),
        | "checkpoint-namespacing" => checkpoint_namespacing::run(),
        | "crate-boundary" => crate_boundary::run(),
        | "docs-link-check" => docs_link_check::run(),
        | "docs-semantic-drift" => docs_semantic_drift::run(),
        | "dx-surface" => dx_surface::run(),
        | "engine-service-boundary" => engine_service_boundary::run(),
        | "fail-closed-ordering" => fail_closed_ordering::run(),
        | "invariant-specs" => invariant_specs::run(),
        | "pathway-async-boundary" => pathway_async_boundary::run(),
        | "pathway-choreography" => pathway_choreography::run(rest),
        | "no-scratch-refs-in-rust" => no_scratch_refs_in_rust::run(),
        | "no-usize-in-models" => no_usize_in_models::run(),
        | "ownership-invariants" => ownership_invariants::run(),
        | "proc-macro-scope" => proc_macro_scope::run(),
        | "proof-bearing-actions" => proof_bearing_actions::run(),
        | "reference-bridge-boundary" => reference_bridge_boundary::run(),
        | "result-must-use" => result_must_use::run(),
        | "router-round-boundary" => router_round_boundary::run(),
        | "routing-invariants" => routing_invariants::run(rest),
        | "rust-style-guide" => rust_style_guide::run(),
        | "simulator-boundary" => simulator_boundary::run(),
        | "surface-classification" => surface_classification::run(),
        | "test-boundaries" => test_boundaries::run(),
        | "transport-authoring-boundary" => transport_authoring_boundary::run(),
        | "transport-ownership-boundary" => transport_ownership_boundary::run(),
        | "trait-purity" => trait_purity::run(),
        | other => bail!("xtask: unknown check: {other}"),
    }
}
