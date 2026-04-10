//! Enforces the transport-authoring boundary across transport-neutral crates.
//!
//! Transport-specific endpoint authoring helpers (BLE, IP socket, etc.) must
//! live in transport-owned profile crates, not in the shared crates:
//! - `jacquard-core` must not contain transport-specific helper functions such
//!   as `ble_endpoint`, `opaque_endpoint`, or `socket_endpoint`.
//! - `jacquard-mem-link-profile`, `jacquard-mem-node-profile`, and
//!   `jacquard-reference-client` must not reintroduce BLE-specific public
//!   authoring vocabulary that belongs in a transport-owned crate.
//!
//! Scans: `crates/core/src/authoring.rs` and the source trees of the three
//! transport-neutral mem/reference crates for forbidden identifier patterns.
//! Registered as: `cargo xtask check transport-authoring-boundary`

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::util::{normalize_rel_path, workspace_root, Violation};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    violations.extend(scan_file(
        &root,
        &root.join("crates/core/src/authoring.rs"),
        &["ble_endpoint(", "opaque_endpoint(", "socket_endpoint("],
        "transport-specific endpoint helper belongs outside jacquard-core",
    )?);

    for rel in [
        "crates/mem-link-profile/src",
        "crates/mem-node-profile/src",
        "crates/reference-client/src",
    ] {
        violations.extend(scan_tree(
            &root,
            &root.join(rel),
            &[
                "ble_endpoint(",
                "BleGatt",
                "BleL2cap",
                "BleDeviceId",
                "BleProfileId",
                "TransportKind::Ble",
            ],
            "BLE-specific authoring belongs outside transport-neutral mem/reference crates",
        )?);
    }

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "transport-authoring-boundary: found {} boundary violation(s)",
            violations.len()
        );
        bail!("transport-authoring-boundary failed");
    }

    println!("transport-authoring-boundary: transport-neutral authoring boundary is valid");
    Ok(())
}

fn scan_tree(root: &Path, dir: &Path, needles: &[&str], message: &str) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();
    if !dir.exists() {
        return Ok(violations);
    }

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        violations.extend(scan_file(root, entry.path(), needles, message)?);
    }

    Ok(violations)
}

fn scan_file(root: &Path, path: &Path, needles: &[&str], message: &str) -> Result<Vec<Violation>> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let rel = normalize_rel_path(root, path);
    let mut violations = Vec::new();

    for (index, line) in contents.lines().enumerate() {
        if needles.iter().any(|needle| line.contains(needle)) {
            violations.push(Violation::new(rel.clone(), index + 1, message));
        }
    }

    Ok(violations)
}
