//! Rejects references to the private scratch directory inside the rust
//! codebase. The committed sources and inline comments must not point at
//! files outside the published crate, doc, and CI surfaces.
//!
//! Exemption: this check itself and `docs_link_check` are allowed to mention
//! the prefix because they are the enforcement mechanism.

use anyhow::{bail, Context, Result};
use std::fs;

use crate::util::{collect_rust_files, normalize_rel_path, workspace_root, Violation};

/// The prefix that must not appear in committed rust sources or comments.
const FORBIDDEN_PREFIX: &[u8] = b"work/";

const EXEMPT_FILES: &[&str] = &[
    "crates/xtask/src/checks/no_scratch_refs_in_rust.rs",
    "crates/xtask/src/checks/docs_link_check.rs",
];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    for path in collect_rust_files(&root)? {
        let rel = normalize_rel_path(&root, &path);
        if EXEMPT_FILES.contains(&rel.as_str()) {
            continue;
        }

        let contents = fs::read(&path)
            .with_context(|| format!("reading {}", path.display()))?;

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
    haystack.windows(needle.len()).any(|window| window == needle)
}
