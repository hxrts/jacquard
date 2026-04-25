#[path = "../../../checks/rust/adapter_boundary.rs"]
pub mod adapter_boundary;
#[path = "../../../checks/rust/annotation_semantics.rs"]
pub mod annotation_semantics;
#[path = "../../../checks/rust/checkpoint_namespacing.rs"]
pub mod checkpoint_namespacing;
#[path = "../../../checks/rust/crate_boundary.rs"]
pub mod crate_boundary;
#[path = "../../../checks/rust/dry_code.rs"]
pub mod dry_code;
#[path = "../../../checks/rust/dx_surface.rs"]
pub mod dx_surface;
#[path = "../../../checks/rust/engine_service_boundary.rs"]
pub mod engine_service_boundary;
#[path = "../../../checks/rust/fail_closed_ordering.rs"]
pub mod fail_closed_ordering;
#[path = "../../../checks/rust/invariant_specs.rs"]
pub mod invariant_specs;
#[path = "../../../checks/rust/long_file.rs"]
pub mod long_file;
#[path = "../../../checks/rust/no_scratch_refs_in_rust.rs"]
pub mod no_scratch_refs_in_rust;
#[path = "../../../checks/rust/no_usize_in_models.rs"]
pub mod no_usize_in_models;
#[path = "../../../checks/rust/ownership_invariants.rs"]
pub mod ownership_invariants;
#[path = "../../../checks/rust/pathway_async_boundary.rs"]
pub mod pathway_async_boundary;
#[path = "../../../checks/rust/pathway_choreography.rs"]
pub mod pathway_choreography;
#[path = "../../../checks/pre_commit.rs"]
pub mod pre_commit;
#[path = "../../../checks/rust/proof_bearing_actions.rs"]
pub mod proof_bearing_actions;
#[path = "../../../checks/rust/reference_bridge_boundary.rs"]
pub mod reference_bridge_boundary;
#[path = "../../../checks/rust/router_round_boundary.rs"]
pub mod router_round_boundary;
#[path = "../../../checks/rust/routing_invariants.rs"]
pub mod routing_invariants;
#[path = "../../../checks/rust/rust_style_guide.rs"]
pub mod rust_style_guide;
#[path = "../../../checks/rust/simulator_boundary.rs"]
pub mod simulator_boundary;
#[path = "../../../checks/rust/surface_classification.rs"]
pub mod surface_classification;
#[path = "../../../checks/rust/trait_purity.rs"]
pub mod trait_purity;
#[path = "../../../checks/rust/transport_authoring_boundary.rs"]
pub mod transport_authoring_boundary;
#[path = "../../../checks/rust/transport_ownership_boundary.rs"]
pub mod transport_ownership_boundary;

use anyhow::{bail, Result};

pub fn run(args: Vec<String>) -> Result<()> {
    let Some(name) = args.first().map(String::as_str) else {
        bail!("jacquard-toolkit-xtask: usage: cargo run --manifest-path toolkit/xtask/Cargo.toml -- check <name> [args]");
    };
    let rest = &args[1..];
    match name {
        "adapter-boundary" => adapter_boundary::run(),
        "annotation-semantics" => annotation_semantics::run(),
        "checkpoint-namespacing" => checkpoint_namespacing::run(),
        "crate-boundary" => crate_boundary::run(),
        "dx-surface" => dx_surface::run(),
        "dry-code" => dry_code::run(),
        "engine-service-boundary" => engine_service_boundary::run(),
        "fail-closed-ordering" => fail_closed_ordering::run(),
        "invariant-specs" => invariant_specs::run(),
        "long-file" => long_file::run(),
        "no-scratch-refs-in-rust" => no_scratch_refs_in_rust::run(),
        "no-usize-in-models" => no_usize_in_models::run(),
        "ownership-invariants" => ownership_invariants::run(),
        "pathway-async-boundary" => pathway_async_boundary::run(),
        "pathway-choreography" => pathway_choreography::run(rest),
        "proof-bearing-actions" => proof_bearing_actions::run(),
        "reference-bridge-boundary" => reference_bridge_boundary::run(),
        "router-round-boundary" => router_round_boundary::run(),
        "routing-invariants" => routing_invariants::run(rest),
        "rust-style-guide" => rust_style_guide::run(),
        "simulator-boundary" => simulator_boundary::run(),
        "surface-classification" => surface_classification::run(),
        "trait-purity" => trait_purity::run(),
        "transport-authoring-boundary" => transport_authoring_boundary::run(),
        "transport-ownership-boundary" => transport_ownership_boundary::run(),
        other => bail!("jacquard-toolkit-xtask: unknown check: {other}"),
    }
}
