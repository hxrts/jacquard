//! Enforces the adapter-support crate split.
//!
//! `jacquard-adapter` provides transport-neutral adapter helpers and
//! observational read-model utilities shared by host bridges. Its boundary
//! rules:
//! - `jacquard-adapter` must stay transport-neutral: no BLE, GATT, L2CAP, Wi-Fi
//!   Aware, or socket-specific terms may appear in its source.
//! - `jacquard-adapter` may host pure observational projectors, but it must not
//!   implement router actions, engine actions, or default async watch/broadcast
//!   frameworks.
//! - Adapter helper shapes (`TransportIngressSender`, `PeerDirectory`, etc.)
//!   must not be reintroduced into `jacquard-core` or `jacquard-traits`; they
//!   belong exclusively in the adapter crate.
//! - `jacquard-reference-client` and `jacquard-simulator` must not define new
//!   local mailbox/dispatch helper abstractions when shared adapter surfaces
//!   already exist.
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

const TRANSPORT_SPECIFIC_TERMS: &[&str] = &[
    "Ble",
    "Gatt",
    "L2cap",
    "WifiAware",
    "socket_",
    "Socket",
    "ble_",
    "wifi_",
];

const FORBIDDEN_ADAPTER_LOGIC_TERMS: &[&str] = &[
    "activate_route(",
    "reselect_route(",
    "maintain_route(",
    "register_engine(",
    "ingest_transport_observation_for_router(",
    "forward_payload_for_router(",
    "tokio::sync::watch",
    "tokio::sync::broadcast",
    "watch::channel(",
    "broadcast::channel(",
];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    violations.extend(scan_adapter_for_transport_specific_terms(&root)?);
    violations.extend(scan_adapter_for_forbidden_logic_terms(&root)?);
    violations.extend(scan_core_and_traits_for_adapter_helpers(&root)?);
    violations.extend(scan_host_layers_for_local_helper_duplicates(&root)?);

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

fn scan_host_layers_for_local_helper_duplicates(root: &Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    for rel in ["crates/reference-client/src", "crates/simulator/src"] {
        let dir = root.join(rel);
        if !dir.exists() {
            continue;
        }
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
            for (index, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//")
                    || trimmed.starts_with("use ")
                    || trimmed.starts_with("pub use ")
                {
                    continue;
                }
                if is_local_helper_duplicate_definition(trimmed) {
                    violations.push(Violation::new(
                        rel.clone(),
                        index + 1,
                        "host/simulator helper duplicates mailbox or dispatch support that belongs in jacquard-adapter",
                    ));
                }
            }
        }
    }

    Ok(violations)
}

fn is_local_helper_duplicate_definition(trimmed: &str) -> bool {
    let Some(defined_name) = defined_name(trimmed) else {
        return false;
    };
    let duplicate_needles = [
        "Mailbox",
        "DispatchSender",
        "DispatchReceiver",
        "dispatch_mailbox",
        "TransportIngressSender",
        "TransportIngressReceiver",
        "PeerDirectory",
        "PendingClaims",
        "ClaimGuard",
    ];
    duplicate_needles
        .iter()
        .any(|needle| defined_name.contains(needle))
}

fn defined_name(trimmed: &str) -> Option<&str> {
    for keyword in ["struct ", "enum ", "type ", "fn "] {
        let Some(rest) = trimmed.strip_prefix(keyword) else {
            continue;
        };
        return Some(
            rest.split(|ch: char| ch == '<' || ch == '(' || ch == ':' || ch.is_whitespace())
                .next()
                .unwrap_or(rest),
        );
    }
    None
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

fn scan_adapter_for_forbidden_logic_terms(root: &Path) -> Result<Vec<Violation>> {
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
            FORBIDDEN_ADAPTER_LOGIC_TERMS,
            "jacquard-adapter must stay observational and must not own router actions or watch/broadcast runtime frameworks",
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
            if rel == "toolkit/checks/adapter_boundary.rs" {
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
            brace_depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
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
