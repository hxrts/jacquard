//! Enforces the adapter-support crate split.
//!
//! `jacquard-adapter` provides transport-neutral ingress/peer-directory
//! helpers shared by host bridges. Its boundary rules:
//! - `jacquard-adapter` must stay transport-neutral: no BLE, GATT, L2CAP, Wi-Fi
//!   Aware, or socket-specific terms may appear in its source.
//! - Adapter helper shapes (`TransportIngressSender`, `PeerDirectory`, etc.)
//!   must not be reintroduced into `jacquard-core` or `jacquard-traits`; they
//!   belong exclusively in the adapter crate.
//!
//! Scans: `crates/adapter/src/` for transport-specific terms, and
//! `crates/core/src/` and `crates/traits/src/` for adapter helper names.
//! Registered as: `cargo xtask check adapter-boundary`

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::util::{normalize_rel_path, workspace_root, Violation};

const ADAPTER_HELPER_NAMES: &[&str] = &[
    "TransportIngressSender",
    "TransportIngressReceiver",
    "TransportIngressNotifier",
    "TransportIngressDrain",
    "transport_ingress_mailbox(",
    "PeerDirectory",
    "PeerIdentityState",
    "PendingClaims",
    "ClaimGuard",
];

const TRANSPORT_SPECIFIC_TERMS: &[&str] =
    &["Ble", "Gatt", "L2cap", "WifiAware", "socket_", "Socket", "ble_", "wifi_"];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    violations.extend(scan_adapter_for_transport_specific_terms(&root)?);
    violations.extend(scan_core_and_traits_for_adapter_helpers(&root)?);

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "adapter-boundary: found {} boundary violation(s)",
            violations.len()
        );
        bail!("adapter-boundary failed");
    }

    println!("adapter-boundary: adapter split is valid");
    Ok(())
}

fn scan_adapter_for_transport_specific_terms(root: &Path) -> Result<Vec<Violation>> {
    let adapter_src = root.join("crates/adapter/src");
    let mut violations = Vec::new();

    for entry in walkdir::WalkDir::new(&adapter_src)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let contents = std::fs::read_to_string(entry.path())
            .with_context(|| format!("reading {}", entry.path().display()))?;
        let rel = normalize_rel_path(root, entry.path());
        violations.extend(scan_non_test_lines(
            &rel,
            &contents,
            TRANSPORT_SPECIFIC_TERMS,
            "jacquard-adapter must stay transport-neutral",
        ));
    }

    Ok(violations)
}

fn scan_core_and_traits_for_adapter_helpers(root: &Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    for rel in ["crates/core/src", "crates/traits/src"] {
        let dir = root.join(rel);
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let contents = std::fs::read_to_string(entry.path())
                .with_context(|| format!("reading {}", entry.path().display()))?;
            let rel = normalize_rel_path(root, entry.path());
            if rel == "crates/xtask/src/checks/adapter_boundary.rs" {
                continue;
            }
            for (index, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                if ADAPTER_HELPER_NAMES
                    .iter()
                    .any(|needle| line.contains(needle))
                {
                    violations.push(Violation::new(
                        rel.clone(),
                        index + 1,
                        "adapter-support helpers belong in jacquard-adapter, not in core or traits",
                    ));
                }
            }
        }
    }

    Ok(violations)
}

fn scan_non_test_lines(
    rel: &str,
    contents: &str,
    needles: &[&str],
    message: &str,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut in_test_module = false;
    let mut brace_depth = 0_i32;
    let mut saw_test_module_open = false;
    let mut saw_cfg_test = false;

    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("#[cfg(test)]") {
            saw_cfg_test = true;
            continue;
        }
        if saw_cfg_test && trimmed.starts_with("mod tests") {
            in_test_module = true;
            saw_test_module_open = trimmed.contains('{');
            brace_depth =
                line.matches('{').count() as i32 - line.matches('}').count() as i32;
            saw_cfg_test = false;
            continue;
        }

        if in_test_module {
            if !saw_test_module_open && line.contains('{') {
                saw_test_module_open = true;
            }
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
            if saw_test_module_open && brace_depth <= 0 {
                in_test_module = false;
            }
            continue;
        }

        if trimmed.starts_with("//") {
            continue;
        }
        if needles.iter().any(|needle| line.contains(needle)) {
            violations.push(Violation::new(rel.to_owned(), index + 1, message));
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::scan_non_test_lines;

    #[test]
    fn ignores_transport_specific_terms_inside_test_modules() {
        let contents = r#"
pub struct AdapterType;

#[cfg(test)]
mod tests {
    fn helper() {
        let _ = "Ble";
    }
}
"#;

        let violations = scan_non_test_lines(
            "crates/adapter/src/example.rs",
            contents,
            &["Ble"],
            "adapter must stay transport-neutral",
        );

        assert!(violations.is_empty());
    }
}
