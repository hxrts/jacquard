//! Validates service/engine boundary consistency.

use anyhow::{bail, Result};
use std::fs;

use crate::util::workspace_root;

const FORBIDDEN_PUBLIC_TYPES: &[&str] = &[
    "MeshRouteSegment",
    "DeterministicCommitteeSelector",
    "MeshEngineRuntime",
    "MeshPlanner",
    "MeshCandidate",
    "BackendRouteId",
];

#[allow(dead_code)]
const ALLOWED_PUBLIC_TYPES: &[&str] = &[
    "RoutingEngine",
    "MeshTransport",
    "Configuration",
    "MeshTopologyModel",
    "RetentionStore",
    "MeshRoutingEngine",
];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mesh_lib = root.join("crates/mesh/src/lib.rs");

    if !mesh_lib.exists() {
        println!("engine-service-boundary: no mesh/src/lib.rs found");
        return Ok(());
    }

    let contents = fs::read_to_string(&mesh_lib)?;

    // Check for forbidden public exports
    let mut violations = Vec::new();

    for forbidden in FORBIDDEN_PUBLIC_TYPES {
        if contents.contains(&format!("pub use {}::{}", "crate", forbidden))
            || contents.contains(&format!("pub struct {}", forbidden))
            || contents.contains(&format!("pub enum {}", forbidden))
            || contents.contains(&format!("pub mod {}", forbidden))
        {
            violations.push(format!(
                "{} is publicly exported but should be engine-private (pub(crate))",
                forbidden
            ));
        }
    }

    if violations.is_empty() {
        println!("engine-service-boundary: boundary properly enforced");
        return Ok(());
    }

    eprintln!("engine-service-boundary: found violations:");
    for v in &violations {
        eprintln!("  {}", v);
    }
    bail!("engine-service-boundary failed");
}
