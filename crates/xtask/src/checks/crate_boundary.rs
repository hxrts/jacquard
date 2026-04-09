//! Enforces the workspace dependency graph. Reads `cargo metadata` and
//! rejects forbidden edges such as `jacquard-core` depending on
//! `jacquard-traits` or either depending on `telltale-runtime`.

use anyhow::{bail, Result};

use crate::util::workspace_metadata;

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
    let forbidden = [
        (
            "jacquard-core",
            vec![
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
            vec![
                "jacquard-pathway",
                "jacquard-router",
                "jacquard-simulator",
                "jacquard-transport",
                "telltale-runtime",
            ],
        ),
        (
            "jacquard-transport",
            vec!["jacquard-pathway", "jacquard-router"],
        ),
    ];

    let mut violations = Vec::new();
    for (package_name, blocked) in forbidden {
        let Some(package) = metadata
            .packages
            .iter()
            .find(|package| package.name == package_name)
        else {
            continue;
        };
        for dependency in &package.dependencies {
            if blocked
                .iter()
                .any(|blocked_name| *blocked_name == dependency.name)
            {
                violations.push(format!(
                    "crate-boundary: {package_name} depends on {} (forbidden)",
                    dependency.name
                ));
            }
        }
    }
    violations
}
