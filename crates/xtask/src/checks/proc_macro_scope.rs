//! Enforces that every non-exempt `.rs` file under `crates/*/src`
//! uses at least one Jacquard proc macro annotation. Stale exempt
//! entries that no longer point at real files also fail.

use anyhow::{bail, Result};

use crate::sources::parse_workspace_sources;

const EXEMPT_FILES: &[&str] = &[
    "crates/core/src/lib.rs",
    "crates/core/src/base/mod.rs",
    "crates/core/src/base/constants.rs",
    "crates/core/src/model/mod.rs",
    "crates/core/src/routing/mod.rs",
    "crates/macros/src/lib.rs",
    "crates/macros/src/model/bounded_value.rs",
    "crates/macros/src/model/id_type.rs",
    "crates/macros/src/model/mod.rs",
    "crates/macros/src/model/must_use_handle.rs",
    "crates/macros/src/model/public_model.rs",
    "crates/macros/src/support/attrs.rs",
    "crates/macros/src/support/derives.rs",
    "crates/macros/src/support/mod.rs",
    "crates/macros/src/support/parsing.rs",
    "crates/macros/src/support/validation.rs",
    "crates/macros/src/traits/effect_handler.rs",
    "crates/macros/src/traits/effect_trait.rs",
    "crates/macros/src/traits/mod.rs",
    "crates/macros/src/traits/purity.rs",
    "crates/traits/src/hashing.rs",
    "crates/traits/src/lib.rs",
    "crates/traits/src/routing.rs",
    "crates/mesh/src/committee.rs",
    "crates/mesh/src/engine/mod.rs",
    "crates/mesh/src/engine/planner/candidates.rs",
    "crates/mesh/src/engine/planner/metrics.rs",
    "crates/mesh/src/engine/planner/mod.rs",
    "crates/mesh/src/engine/planner/admission.rs",
    "crates/mesh/src/engine/planner/pathing.rs",
    "crates/mesh/src/engine/planner/publishing.rs",
    "crates/mesh/src/engine/runtime/health.rs",
    "crates/mesh/src/engine/runtime/materialization.rs",
    "crates/mesh/src/engine/runtime/maintenance.rs",
    "crates/mesh/src/engine/runtime/mod.rs",
    "crates/mesh/src/engine/runtime/tick.rs",
    "crates/mesh/src/engine/support.rs",
    "crates/mesh/src/engine/trait_bounds.rs",
    "crates/mesh/src/engine/types.rs",
    "crates/mesh/src/lib.rs",
    "crates/mesh/src/topology.rs",
];

const MARKERS: &[&str] = &[
    "#[effect_trait",
    "#[effect_handler",
    "#[id_type",
    "#[bounded_value",
    "#[must_use_handle",
    "#[public_model",
    "#[purity",
];

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut missing = Vec::new();
    let mut stale = Vec::new();

    for exempt in EXEMPT_FILES {
        if !parsed.iter().any(|source| source.rel_path == *exempt) {
            stale.push((*exempt).to_string());
        }
    }

    for source in parsed {
        if source.rel_path.starts_with("crates/xtask/src/") {
            continue;
        }
        if EXEMPT_FILES.contains(&source.rel_path.as_str()) {
            continue;
        }
        if !MARKERS.iter().any(|marker| source.source.contains(marker)) {
            missing.push(source.rel_path.clone());
        }
    }

    if !stale.is_empty() {
        eprintln!("stale proc-macro exemptions:");
        for entry in &stale {
            eprintln!("  {entry}");
        }
        bail!("proc-macro-scope failed");
    }
    if !missing.is_empty() {
        eprintln!("missing proc-macro file coverage:");
        for entry in &missing {
            eprintln!("  {entry}");
        }
        bail!("proc-macro-scope failed");
    }
    println!("proc-macro file coverage is maximal for crate source files");
    Ok(())
}
