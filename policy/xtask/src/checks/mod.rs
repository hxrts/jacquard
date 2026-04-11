#[path = "../../../checks/adapter_boundary.rs"]
pub mod adapter_boundary;
#[path = "../../../checks/checkpoint_namespacing.rs"]
pub mod checkpoint_namespacing;
#[path = "../../../checks/crate_boundary.rs"]
pub mod crate_boundary;
#[path = "../../../checks/dry_code.rs"]
pub mod dry_code;
#[path = "../../../checks/dx_surface.rs"]
pub mod dx_surface;
#[path = "../../../checks/engine_service_boundary.rs"]
pub mod engine_service_boundary;
#[path = "../../../checks/fail_closed_ordering.rs"]
pub mod fail_closed_ordering;
#[path = "../../../checks/field_code_map.rs"]
pub mod field_code_map;
#[path = "../../../checks/invariant_specs.rs"]
pub mod invariant_specs;
#[path = "../../../checks/no_scratch_refs_in_rust.rs"]
pub mod no_scratch_refs_in_rust;
#[path = "../../../checks/no_usize_in_models.rs"]
pub mod no_usize_in_models;
#[path = "../../../checks/ownership_invariants.rs"]
pub mod ownership_invariants;
#[path = "../../../checks/pathway_async_boundary.rs"]
pub mod pathway_async_boundary;
#[path = "../../../checks/pathway_choreography.rs"]
pub mod pathway_choreography;
#[path = "../../../checks/pre_commit.rs"]
pub mod pre_commit;
#[path = "../../../checks/proof_bearing_actions.rs"]
pub mod proof_bearing_actions;
#[path = "../../../checks/reference_bridge_boundary.rs"]
pub mod reference_bridge_boundary;
#[path = "../../../checks/router_round_boundary.rs"]
pub mod router_round_boundary;
#[path = "../../../checks/routing_invariants.rs"]
pub mod routing_invariants;
#[path = "../../../checks/rust_style_guide.rs"]
pub mod rust_style_guide;
#[path = "../../../checks/simulator_boundary.rs"]
pub mod simulator_boundary;
#[path = "../../../checks/surface_classification.rs"]
pub mod surface_classification;
#[path = "../../../checks/trait_purity.rs"]
pub mod trait_purity;
#[path = "../../../checks/transport_authoring_boundary.rs"]
pub mod transport_authoring_boundary;
#[path = "../../../checks/transport_ownership_boundary.rs"]
pub mod transport_ownership_boundary;

use anyhow::{bail, Result};

pub fn run(args: Vec<String>) -> Result<()> {
    let Some(name) = args.first().map(String::as_str) else {
        bail!("policy-xtask: usage: cargo run --manifest-path policy/xtask/Cargo.toml -- check <name> [args]");
    };
    let rest = &args[1..];
    match name {
        "adapter-boundary" => adapter_boundary::run(),
        "checkpoint-namespacing" => checkpoint_namespacing::run(),
        "crate-boundary" => crate_boundary::run(),
        "dx-surface" => dx_surface::run(),
        "dry-code" => dry_code::run(),
        "engine-service-boundary" => engine_service_boundary::run(),
        "field-code-map" => field_code_map::run(),
        "fail-closed-ordering" => fail_closed_ordering::run(),
        "invariant-specs" => invariant_specs::run(),
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
        other => bail!("policy-xtask: unknown check: {other}"),
    }
}
