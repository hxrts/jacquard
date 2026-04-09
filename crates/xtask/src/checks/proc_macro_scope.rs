//! Enforces Jacquard proc-macro annotation coverage across workspace sources.
//!
//! Every non-exempt `.rs` file under `crates/*/src/` must carry at least one
//! Jacquard proc-macro annotation (e.g. `#[public_model]`, `#[purity(...)]`,
//! `#[effect_trait]`). This ensures the macro system's invariants apply
//! broadly and that files are not accidentally left outside the policy surface.
//!
//! Additionally, stale entries in the exemption list that no longer point at
//! real files are reported as violations, keeping the exemption table clean.
//!
//! Scans: all parsed workspace sources via `parse_workspace_sources`. Each
//! source is checked for at least one attribute that resolves to a known
//! Jacquard macro name.
//! Registered as: `cargo xtask check proc-macro-scope`

use anyhow::{bail, Result};

use crate::sources::parse_workspace_sources;

const EXEMPT_FILES: &[&str] = &[
    "crates/adapter/src/lib.rs",
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
    "crates/traits/src/sealed.rs",
    "crates/pathway/src/committee/mod.rs",
    "crates/pathway/src/committee/selection.rs",
    "crates/pathway/src/choreography/artifacts.rs",
    "crates/pathway/src/choreography/mod.rs",
    "crates/pathway/src/engine/mod.rs",
    "crates/pathway/src/engine/planner/admission.rs",
    "crates/pathway/src/engine/planner/candidates.rs",
    "crates/pathway/src/engine/planner/mod.rs",
    "crates/pathway/src/engine/planner/pathing.rs",
    "crates/pathway/src/engine/planner/publishing.rs",
    "crates/pathway/src/engine/planner/scoring.rs",
    "crates/pathway/src/engine/runtime/health.rs",
    "crates/pathway/src/engine/runtime/materialization.rs",
    "crates/pathway/src/engine/runtime/maintenance.rs",
    "crates/pathway/src/engine/runtime/mod.rs",
    "crates/pathway/src/engine/runtime/tick.rs",
    "crates/pathway/src/engine/support.rs",
    "crates/pathway/src/engine/trait_bounds.rs",
    "crates/pathway/src/engine/types.rs",
    "crates/pathway/src/lib.rs",
    "crates/pathway/src/topology.rs",
    "crates/mem-link-profile/src/authoring.rs",
    "crates/mem-link-profile/src/effect.rs",
    "crates/mem-link-profile/src/lib.rs",
    "crates/mem-link-profile/src/network.rs",
    "crates/mem-link-profile/src/retention.rs",
    "crates/mem-link-profile/src/state.rs",
    "crates/mem-link-profile/src/transport.rs",
    "crates/mem-node-profile/src/authoring.rs",
    "crates/mem-node-profile/src/lib.rs",
    "crates/mem-node-profile/src/profile.rs",
    "crates/mem-node-profile/src/service.rs",
    "crates/mem-node-profile/src/state.rs",
    "crates/batman/src/lib.rs",
    "crates/batman/src/gossip.rs",
    "crates/batman/src/planner.rs",
    "crates/batman/src/private_state.rs",
    "crates/batman/src/public_state.rs",
    "crates/batman/src/runtime.rs",
    "crates/batman/src/scoring.rs",
    "crates/field/src/engine.rs",
    "crates/field/src/lib.rs",
    "crates/field/src/planner.rs",
    "crates/field/src/runtime.rs",
    "crates/field/src/summary.rs",
    "crates/field/src/choreography.rs",
    "crates/field/src/control.rs",
    "crates/field/src/state.rs",
    "crates/field/src/observer.rs",
    "crates/field/src/attractor.rs",
    "crates/field/src/route.rs",
    "crates/reference-client/src/clients.rs",
    "crates/reference-client/src/bridge.rs",
    "crates/reference-client/src/lib.rs",
    "crates/reference-client/src/topology.rs",
    "crates/adapter/src/endpoint.rs",
    "crates/adapter/src/topology.rs",
    "crates/router/src/lib.rs",
    "crates/router/src/middleware.rs",
    "crates/router/src/runtime.rs",
    "crates/field-client/src/client.rs",
    "crates/field-client/src/lib.rs",
    "crates/field-client/src/topology.rs",
    "crates/simulator/src/environment.rs",
    "crates/simulator/src/harness.rs",
    "crates/simulator/src/lib.rs",
    "crates/simulator/src/presets.rs",
    "crates/simulator/src/replay.rs",
    "crates/simulator/src/scenario.rs",
];

const MARKERS: &[&str] = &[
    "#[effect_trait",
    "#[effect_handler",
    "#[id_type",
    "#[bounded_value",
    "#[must_use_handle",
    "#[public_model",
    "#[purity",
    "#[jacquard_traits::purity",
    "tell! {",
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
