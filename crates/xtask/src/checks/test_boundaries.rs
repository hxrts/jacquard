//! Enforces unit-test boundary hygiene for in-source test modules.
//!
//! Two rules are enforced across all workspace crates (excluding xtask
//! itself, target directories, and dedicated test/bench trees):
//! - Standalone `tests.rs` files directly under `src/` sub-paths are rejected;
//!   unit tests must be co-located in the module they cover via `#[cfg(test)]`
//!   blocks, not in separate files.
//! - Importing from the external `tests/` directory via `#[path = ...]` or
//!   `include_str!` inside source files is forbidden; integration tests must
//!   remain self-contained in their own compilation unit.
//!
//! Scans: all `.rs` files under `crates/*/src/` using regex patterns.
//! Registered as: `cargo xtask check test-boundaries`

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::{layer_for_rel_path, normalize_rel_path, workspace_root, Violation};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    let standalone_tests = Regex::new(r"/src/.+/tests\.rs$")?;
    let import_from_tests = Regex::new(
        r#"(#\[\s*path\s*=\s*".*tests/)|(include_(str|bytes)?!\s*\(\s*".*tests/)"#,
    )?;

    for path in rust_files(root.join("crates"))? {
        let rel = normalize_rel_path(&root, &path);
        if rel.starts_with("crates/xtask/")
            || rel.contains("/target/")
            || rel.contains("/tests/")
            || rel.contains("/benches/")
            || rel.contains("/examples/")
        {
            continue;
        }

        if standalone_tests.is_match(&rel) {
            violations.push(Violation::with_layer(
                rel.clone(),
                1,
                "standalone unit-test source files under src/ are forbidden; colocate unit tests in the owning file",
                layer_for_rel_path(&rel),
            ));
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (idx, line) in contents.lines().enumerate() {
            if import_from_tests.is_match(line) {
                violations.push(Violation::with_layer(
                    rel.clone(),
                    idx + 1,
                    "source-tree unit tests must not import helpers out of tests/",
                    layer_for_rel_path(&rel),
                ));
            }
        }
    }

    if violations.is_empty() {
        println!("test-boundaries: OK");
    } else {
        eprintln!("test-boundaries: violation(s)");
        for violation in &violations {
            eprintln!("  {}", violation.render());
        }
        anyhow::bail!("test-boundaries failed");
    }

    Ok(())
}

fn rust_files(dir: PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.into_path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
