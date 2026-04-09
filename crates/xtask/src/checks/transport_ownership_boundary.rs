//! Enforces the transport ownership split:
//! - `TransportSenderEffects` is the only shared transport effect capability
//! - ingress draining must live on `TransportDriver`, outside effect vocabulary
//! - concrete driver implementations must not stamp Jacquard time internally

use anyhow::{bail, Context, Result};

use crate::util::{
    collect_rust_files, layer_for_rel_path, normalize_rel_path, workspace_root,
    Violation,
};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = scan_effect_capability_file(&root)?;
    violations.extend(scan_driver_impls(&root)?);

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "transport-ownership-boundary: found {} boundary violation(s)",
            violations.len()
        );
        bail!("transport-ownership-boundary failed");
    }

    println!("transport-ownership-boundary: transport ownership split is valid");
    Ok(())
}

fn scan_effect_capability_file(root: &std::path::Path) -> Result<Vec<Violation>> {
    let effects_path = root.join("crates/traits/src/effects.rs");
    let effects = std::fs::read_to_string(&effects_path)
        .with_context(|| format!("reading {}", effects_path.display()))?;
    let mut violations = Vec::new();

    for (index, line) in effects.lines().enumerate() {
        if line.contains("TransportEffects") || line.contains("poll_transport(") {
            violations.push(Violation::with_layer(
                "crates/traits/src/effects.rs",
                index + 1,
                "transport ingress ownership must stay out of effect capabilities",
                layer_for_rel_path("crates/traits/src/effects.rs"),
            ));
        }
    }

    Ok(violations)
}

fn scan_driver_impls(root: &std::path::Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    for path in collect_rust_files(root)? {
        let rel = normalize_rel_path(root, &path);
        if rel == "crates/xtask/src/checks/transport_ownership_boundary.rs" {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        violations.extend(scan_transport_driver_blocks(&rel, &contents));
    }

    Ok(violations)
}

fn scan_transport_driver_blocks(rel: &str, contents: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    for (index, line) in contents.lines().enumerate() {
        if !line.contains("impl TransportDriver") {
            continue;
        }
        let mut depth = 0_i32;
        let mut saw_open = false;
        for (inner_index, inner_line) in contents.lines().enumerate().skip(index) {
            depth += inner_line.matches('{').count() as i32;
            if inner_line.contains('{') {
                saw_open = true;
            }
            if saw_open
                && (inner_line.contains("observed_at_tick")
                    || inner_line.contains(".observe_at(")
                    || inner_line.contains("now_tick("))
            {
                violations.push(Violation::with_layer(
                    rel.to_owned(),
                    inner_index + 1,
                    "transport drivers must emit raw ingress and must not stamp Jacquard time internally",
                    layer_for_rel_path(rel),
                ));
            }
            depth -= inner_line.matches('}').count() as i32;
            if saw_open && depth <= 0 {
                break;
            }
        }
    }

    violations
}
