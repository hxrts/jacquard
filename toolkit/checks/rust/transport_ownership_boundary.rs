//! Enforces the transport ownership split between effects and drivers.
//!
//! The transport layer is divided into two distinct surfaces that must not
//! bleed into each other:
//! - `TransportSenderEffects` is the only shared synchronous send capability;
//!   it must not grow ingress or supervision methods.
//! - `TransportDriver` is the host-owned ingress and supervision surface;
//!   ingress draining must live here, not inside the effect vocabulary.
//! - Concrete driver implementations must not stamp Jacquard time (`Tick`,
//!   `DurationMs`) internally; time assignment belongs to the host bridge.
//!
//! Scans: the effect-capability file, the driver-contract file, driver
//! implementations, and adapter helpers across the workspace.
//! Registered as: `cargo xtask check transport-ownership-boundary`

use anyhow::{bail, Context, Result};

use crate::util::{
    collect_rust_files, layer_for_rel_path, normalize_rel_path, workspace_root, Violation,
};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = scan_effect_capability_file(&root)?;
    violations.extend(scan_driver_contract_file(&root)?);
    violations.extend(scan_driver_impls(&root)?);
    violations.extend(scan_adapter_helpers(&root)?);

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

fn scan_adapter_helpers(root: &std::path::Path) -> Result<Vec<Violation>> {
    let adapter_path = root.join("crates/adapter/src");
    let mut violations = Vec::new();
    if !adapter_path.exists() {
        return Ok(violations);
    }

    for path in collect_rust_files(root)? {
        if !path.starts_with(&adapter_path) {
            continue;
        }
        let rel = normalize_rel_path(root, &path);
        if rel == "crates/adapter/src/topology.rs" {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (index, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if line.contains("Tick")
                || line.contains("OrderStamp")
                || line.contains("observed_at_tick")
                || line.contains(".observe_at(")
                || line.contains("now_tick(")
                || line.contains("next_order_stamp(")
            {
                violations.push(Violation::with_layer(
                    rel.clone(),
                    index + 1,
                    "adapter helpers must stay free of Jacquard time and ordering assignment",
                    layer_for_rel_path(&rel),
                ));
            }
        }
    }

    Ok(violations)
}

fn scan_effect_capability_file(root: &std::path::Path) -> Result<Vec<Violation>> {
    let effects_path = root.join("crates/traits/src/effects.rs");
    let effects = std::fs::read_to_string(&effects_path)
        .with_context(|| format!("reading {}", effects_path.display()))?;
    let mut violations = Vec::new();

    for (index, line) in effects.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("TransportEffects")
            || line.contains("poll_transport(")
            || line.contains("Stream")
            || line.contains("subscribe")
            || line.contains("watch")
        {
            violations.push(Violation::with_layer(
                "crates/traits/src/effects.rs",
                index + 1,
                "transport effect capabilities must stay free of ingress-stream and supervision vocabulary",
                layer_for_rel_path("crates/traits/src/effects.rs"),
            ));
        }
    }

    Ok(violations)
}

fn scan_driver_contract_file(root: &std::path::Path) -> Result<Vec<Violation>> {
    let drivers_path = root.join("crates/traits/src/drivers.rs");
    let drivers = std::fs::read_to_string(&drivers_path)
        .with_context(|| format!("reading {}", drivers_path.display()))?;
    let mut violations = Vec::new();

    for (index, line) in drivers.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("Tick")
            || line.contains("OrderStamp")
            || line.contains("now_tick(")
            || line.contains("next_order_stamp(")
        {
            violations.push(Violation::with_layer(
                "crates/traits/src/drivers.rs",
                index + 1,
                "transport drivers must not assign Jacquard time or ordering",
                layer_for_rel_path("crates/traits/src/drivers.rs"),
            ));
        }
    }

    Ok(violations)
}

fn scan_driver_impls(root: &std::path::Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    for path in collect_rust_files(root)? {
        let rel = normalize_rel_path(root, &path);
        if rel == "toolkit/checks/transport_ownership_boundary.rs" {
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
