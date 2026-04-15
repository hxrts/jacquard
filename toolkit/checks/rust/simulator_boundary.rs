//! Enforces the simulator ownership boundary.
//!
//! `jacquard-simulator` must reuse the host bridge and adapter helpers rather
//! than reaching into transport-driver internals or assigning ordering/event
//! stamps itself.
//!
//! Boundary rules:
//! - simulator code must not drain transport ingress directly
//! - simulator code must not stamp observations or assign order stamps
//! - simulator code must not record route events directly; it should consume
//!   the existing runtime-effect log surfaces
//! - simulator code should expose an explicit harness / adapter boundary
//!
//! Registered as: `cargo xtask check simulator-boundary`

use anyhow::{bail, Context, Result};

use crate::util::{
    collect_rust_files, layer_for_rel_path, normalize_rel_path, workspace_root, Violation,
};

const HARNESS_FILE: &str = "crates/simulator/src/harness/mod.rs";
const REQUIRED_TOKENS: &[&str] = &[
    "pub trait JacquardHostAdapter",
    "pub struct JacquardSimulationHarness",
    "pub struct JacquardSimulator",
];
const FORBIDDEN_PATTERNS: &[&str] = &[
    "drain_transport_ingress(",
    ".observe_at(",
    "OrderStamp(",
    "next_order_stamp(",
    "record_route_event(",
];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let harness = std::fs::read_to_string(root.join(HARNESS_FILE))
        .with_context(|| format!("reading {}", root.join(HARNESS_FILE).display()))?;
    let mut violations = Vec::new();

    for token in REQUIRED_TOKENS {
        if !harness.contains(token) {
            violations.push(Violation::with_layer(
                HARNESS_FILE,
                1,
                format!("missing required simulator harness token `{token}`"),
                layer_for_rel_path(HARNESS_FILE),
            ));
        }
    }

    for path in collect_rust_files(&root)? {
        let rel = normalize_rel_path(&root, &path);
        if !rel.starts_with("crates/simulator/") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (index, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            for pattern in FORBIDDEN_PATTERNS {
                if line.contains(pattern) {
                    violations.push(Violation::with_layer(
                        rel.clone(),
                        index + 1,
                        format!("simulator boundary pattern `{pattern}` is forbidden"),
                        layer_for_rel_path(&rel),
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "simulator-boundary: found {} boundary violation(s)",
            violations.len()
        );
        bail!("simulator-boundary failed");
    }

    println!("simulator-boundary: simulator bridge ownership is valid");
    Ok(())
}
