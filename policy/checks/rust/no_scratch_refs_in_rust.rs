//! Rejects references to the private scratch directory in committed Rust
//! sources.
//!
//! Committed sources, inline comments, and string literals must not reference
//! the private `work/` scratch prefix. That directory is local-only and must
//! not appear in published crate, doc, or CI surfaces. References would create
//! broken paths for anyone without the local scratch tree.
//!
//! Scans every `.rs` file in the workspace as raw bytes, searching for the
//! forbidden byte prefix `work/`. Files are scanned including comments because
//! even commented-out scratch paths are disallowed.
//!
//! Exempt files: this check itself and `docs_link_check` may mention the
//! prefix, as they are the enforcement mechanism.
//! Registered as: `cargo xtask check no-scratch-refs-in-rust`

use std::fs;

use anyhow::{bail, Context, Result};

use crate::util::{collect_rust_files, normalize_rel_path, workspace_root, Violation};

/// The prefix that must not appear in committed rust sources or comments.
const FORBIDDEN_PREFIX: &[u8] = b"work/";

const EXEMPT_FILES: &[&str] = &["policy/checks/no_scratch_refs_in_rust.rs"];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    for path in collect_rust_files(&root)? {
        let rel = normalize_rel_path(&root, &path);
        if EXEMPT_FILES.contains(&rel.as_str()) {
            continue;
        }

        let contents = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;

        for (line_idx, line) in contents.split(|b| *b == b'\n').enumerate() {
            if contains_subslice(line, FORBIDDEN_PREFIX) {
                let snippet = String::from_utf8_lossy(line).into_owned();
                violations.push(Violation::new(
                    &rel,
                    line_idx + 1,
                    format!(
                        "rust source must not reference the private scratch directory: {}",
                        snippet.trim()
                    ),
                ));
            }
        }
    }

    if violations.is_empty() {
        println!("no-scratch-refs-in-rust: no scratch directory references found");
        return Ok(());
    }

    eprintln!("no-scratch-refs-in-rust: found violations:");
    for v in &violations {
        eprintln!("  {}", v.render());
    }
    bail!("no-scratch-refs-in-rust failed");
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
