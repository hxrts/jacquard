//! Enforces the workspace crate-layer dependency graph.
//!
//! Reads `cargo metadata` and rejects any dependency edge that violates the
//! declared layer order. Forbidden edges include:
//! - `jacquard-core` depending on `jacquard-traits` (core must stay
//!   behavior-free and cannot import the traits layer above it)
//! - `jacquard-core` or `jacquard-traits` depending on `telltale-runtime` (the
//!   two lowest layers must remain runtime-free)
//! - any other cross-layer inversion recorded in the forbidden edge table
//!
//! Scans: workspace `Cargo.toml` dependency graph via `cargo_metadata`.
//! Registered as: `cargo xtask check crate-boundary`

use anyhow::{bail, Result};
use cargo_metadata::DependencyKind;

use crate::util::workspace_metadata;

const FORBIDDEN_DEPENDENCIES: &[(&str, &[&str])] = &[
    (
        "jacquard-core",
        &[
            "jacquard-adapter",
            "jacquard-traits",
            "jacquard-pathway",
            "jacquard-router",
            "jacquard-simulator",
            "jacquard-transport",
            "telltale-runtime",
        ],
    ),
    (
        "jacquard-traits",
        &[
            "jacquard-adapter",
            "jacquard-pathway",
            "jacquard-router",
            "jacquard-simulator",
            "jacquard-transport",
            "telltale-runtime",
        ],
    ),
    (
        "jacquard-adapter",
        &[
            "jacquard-traits",
            "jacquard-pathway",
            "jacquard-batman",
            "jacquard-router",
            "jacquard-reference-client",
            "jacquard-mem-link-profile",
            "jacquard-mem-node-profile",
            "jacquard-simulator",
            "jacquard-transport",
            "telltale-runtime",
        ],
    ),
    (
        "jacquard-transport",
        &["jacquard-pathway", "jacquard-router"],
    ),
];

pub fn run() -> Result<()> {
    let metadata = workspace_metadata()?;
    let violations = forbidden_dependency_violations(&metadata);

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{violation}");
        }
        eprintln!();
        eprintln!(
            "crate-boundary: found {} forbidden dependency edge(s)",
            violations.len()
        );
        bail!("crate-boundary failed");
    }

    println!("crate-boundary: dependency graph is valid");
    Ok(())
}

fn forbidden_dependency_violations(metadata: &cargo_metadata::Metadata) -> Vec<String> {
    FORBIDDEN_DEPENDENCIES
        .iter()
        .flat_map(|(package_name, blocked)| {
            package_dependency_violations(metadata, package_name, blocked)
        })
        .collect()
}

fn package_dependency_violations(
    metadata: &cargo_metadata::Metadata,
    package_name: &str,
    blocked: &[&str],
) -> Vec<String> {
    let Some(package) = metadata
        .packages
        .iter()
        .find(|package| package.name == package_name)
    else {
        return Vec::new();
    };

    package
        .dependencies
        .iter()
        .filter(|dependency| {
            dependency.kind != DependencyKind::Development
                && blocked
                    .iter()
                    .any(|blocked_name| *blocked_name == dependency.name)
        })
        .map(|dependency| {
            format!(
                "crate-boundary: {package_name} depends on {} (forbidden)",
                dependency.name
            )
        })
        .collect()
}
