//! Enforces the reference-client host-bridge boundary.
//!
//! `jacquard-reference-client` is the host-side integration layer that
//! composes router, pathway/batman engine, and in-memory profiles for
//! end-to-end tests. Its internal boundary rules are:
//! - `TransportDriver` may only appear inside `bridge.rs` or `clients.rs`.
//! - Direct in-memory transport attachment is allowed only in bridge/builders.
//! - Tests must advance the bridge (which owns time and ingress), not the
//!   router directly, preserving the host-owns-time invariant.
//!
//! Scans: all `.rs` files under `crates/reference-client/src/` for forbidden
//! patterns outside the designated bridge files.
//! Registered as: `cargo xtask check reference-bridge-boundary`

use anyhow::{bail, Context, Result};

use crate::util::{
    collect_rust_files, layer_for_rel_path, normalize_rel_path, workspace_root,
    Violation,
};

const BRIDGE_FILE: &str = "crates/reference-client/src/bridge.rs";
const CLIENT_BUILDERS_FILE: &str = "crates/reference-client/src/clients.rs";

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    for path in collect_rust_files(&root)? {
        let rel = normalize_rel_path(&root, &path);
        if !rel.starts_with("crates/reference-client/") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        violations.extend(scan_file(&rel, &contents));
    }

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "reference-bridge-boundary: found {} boundary violation(s)",
            violations.len()
        );
        bail!("reference-bridge-boundary failed");
    }

    println!("reference-bridge-boundary: reference client bridge ownership is valid");
    Ok(())
}

fn scan_file(rel: &str, contents: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let allow_driver = rel == BRIDGE_FILE;
    let allow_attach = rel == BRIDGE_FILE || rel == CLIENT_BUILDERS_FILE;

    for (index, line) in contents.lines().enumerate() {
        if !allow_driver
            && (line.contains("TransportDriver")
                || line.contains("drain_transport_ingress("))
        {
            violations.push(Violation::with_layer(
                rel.to_owned(),
                index + 1,
                "reference-client transport-driver ownership must stay inside the bridge layer",
                layer_for_rel_path(rel),
            ));
        }

        if !allow_attach && line.contains("InMemoryTransport::attach(") {
            violations.push(Violation::with_layer(
                rel.to_owned(),
                index + 1,
                "reference-client may attach concrete in-memory transports only in bridge/builders",
                layer_for_rel_path(rel),
            ));
        }

        if rel != BRIDGE_FILE
            && (line.contains("router_mut().advance_round(")
                || line.contains(".router.advance_round("))
        {
            violations.push(Violation::with_layer(
                rel.to_owned(),
                index + 1,
                "reference-client must advance host bridges, not routers directly",
                layer_for_rel_path(rel),
            ));
        }
    }

    violations
}
