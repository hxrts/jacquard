//! Enforces the explicit router-ingress and round-advancement vocabulary.
//!
//! Phase 2 invariants:
//! - router/control traits must expose `advance_round`
//! - router middleware must expose explicit ingress APIs
//! - `anti_entropy_tick`, `replace_topology`, and `replace_policy_inputs` names
//!   must be gone from tracked Rust sources
//! - transport ingress must not be drained directly from router/pathway engine
//!   runtime code
//!
//! Scans: all workspace sources via `parse_workspace_sources`, filtered to
//! router, traits, reference-client, batman, and pathway crate paths.
//! Registered as: `cargo xtask check router-round-boundary`

use anyhow::{bail, Result};

use crate::{sources::parse_workspace_sources, util::Violation};

const FORBIDDEN_PATTERNS: &[(&str, &[&str])] = &[
    (
        "anti_entropy_tick(",
        &[
            "crates/router/",
            "crates/traits/",
            "crates/reference-client/",
            "crates/batman/",
            "crates/pathway/",
        ],
    ),
    (
        "replace_topology(",
        &[
            "crates/router/",
            "crates/traits/",
            "crates/reference-client/",
        ],
    ),
    (
        "replace_policy_inputs(",
        &[
            "crates/router/",
            "crates/traits/",
            "crates/reference-client/",
        ],
    ),
    (
        "drain_transport_ingress(",
        &["crates/router/src/", "crates/pathway/src/engine/"],
    ),
];

fn required_trait_tokens(source: &str) -> Vec<&'static str> {
    let mut missing = Vec::new();
    for token in [
        "fn ingest_topology_observation(",
        "fn ingest_policy_inputs(",
        "fn ingest_transport_observation(",
        "fn advance_round(",
    ] {
        if !source.contains(token) {
            missing.push(token);
        }
    }
    missing
}

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut violations = Vec::new();
    let mut saw_routing_trait_file = false;

    for source in parsed {
        let rel_path = source.rel_path.as_str();
        let text = &source.source;

        if rel_path == "crates/traits/src/routing.rs" {
            saw_routing_trait_file = true;
            for token in required_trait_tokens(text) {
                violations.push(Violation::new(
                    rel_path,
                    1,
                    format!("missing required explicit round token `{token}`"),
                ));
            }
        }

        for (pattern, prefixes) in FORBIDDEN_PATTERNS {
            if !prefixes.iter().any(|prefix| rel_path.starts_with(prefix)) {
                continue;
            }
            for (line_idx, line) in text.lines().enumerate() {
                if line.contains(*pattern) {
                    violations.push(Violation::new(
                        rel_path,
                        line_idx + 1,
                        format!("router-round pattern `{pattern}` is forbidden"),
                    ));
                }
            }
        }
    }

    if !saw_routing_trait_file {
        bail!("router-round-boundary: did not inspect crates/traits/src/routing.rs");
    }

    if violations.is_empty() {
        println!("router-round-boundary: explicit ingress and round vocabulary is valid");
        return Ok(());
    }

    eprintln!("router-round-boundary: found violations:");
    for violation in &violations {
        eprintln!("  {}", violation.render());
    }
    bail!("router-round-boundary failed");
}
