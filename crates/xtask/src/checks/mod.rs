//! Check registry. Each submodule implements one `cargo xtask check <name>`
//! rule. `run` matches the requested name against the registered set.

pub mod crate_boundary;
pub mod docs_link_check;
pub mod docs_semantic_drift;
pub mod mesh_choreography;
pub mod no_usize_in_models;
pub mod pre_commit;
pub mod proc_macro_scope;
pub mod routing_invariants;
pub mod test_boundaries;
pub mod trait_purity;

use anyhow::{bail, Result};

pub fn run(args: Vec<String>) -> Result<()> {
    let Some(name) = args.first().map(String::as_str) else {
        bail!("xtask: usage: cargo xtask check <name> [args]");
    };
    let rest = &args[1..];
    match name {
        | "crate-boundary" => crate_boundary::run(),
        | "docs-link-check" => docs_link_check::run(),
        | "docs-semantic-drift" => docs_semantic_drift::run(),
        | "mesh-choreography" => mesh_choreography::run(rest),
        | "no-usize-in-models" => no_usize_in_models::run(),
        | "proc-macro-scope" => proc_macro_scope::run(),
        | "routing-invariants" => routing_invariants::run(rest),
        | "test-boundaries" => test_boundaries::run(),
        | "trait-purity" => trait_purity::run(),
        | other => bail!("xtask: unknown check: {other}"),
    }
}
