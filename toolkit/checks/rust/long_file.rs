//! Rejects Rust source files that exceed a hard line-count budget.
//!
//! Oversized files usually mix several concerns into one module and
//! hurt navigation, review, and incremental recompile time. When a
//! file grows past [`MAX_FILE_LINES`] total source lines, split it
//! into submodules grouped by concern rather than by size: types in a
//! `types.rs`, pure reducers separate from effectful entry points,
//! protocol/choreography code separate from runtime plumbing, per-
//! object state under a `<name>/` directory, etc.
//!
//! Two escape hatches:
//! - a `long-file-exception: <reason>` comment inside the first
//!   [`MARKER_SCAN_LINES`] lines of the file, for deliberate long-
//!   lived exceptions,
//! - a `[[toolkit.exemptions.long_file]]` entry in
//!   `toolkit/toolkit.toml`, for inherited oversized files that are
//!   tracked for a future split.
//!
//! Registered as: `cargo xtask check long-file`

use std::fs;

use anyhow::{bail, Context, Result};

use crate::{
    exemptions::long_file_exemptions,
    util::{collect_rust_files, normalize_rel_path, workspace_root, Violation},
};

/// Upper bound on `.rs` source file length, in total lines including
/// blanks and comments.
pub const MAX_FILE_LINES: usize = 800;

const EXCEPTION_MARKER: &str = "long-file-exception:";
const MARKER_SCAN_LINES: usize = 40;

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let exemptions = long_file_exemptions()?;
    let mut violations = Vec::new();

    for path in collect_rust_files(&root)? {
        let rel = normalize_rel_path(&root, &path);
        let contents =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let line_count = contents.lines().count();
        if line_count <= MAX_FILE_LINES {
            continue;
        }
        if exemptions.iter().any(|(exempt, _)| exempt == &rel) {
            continue;
        }
        if has_exception_marker(&contents) {
            continue;
        }

        violations.push(Violation::new(
            &rel,
            1,
            format!(
                "file is {line_count} lines; limit is {MAX_FILE_LINES}. \
                 Split this module into submodules grouped by concern \
                 (types, pure reducers, effectful entry points, protocol/ \
                 choreography, per-object state) rather than by size. \
                 To keep the file as-is, add a `// long-file-exception: \
                 <reason>` comment within the first {MARKER_SCAN_LINES} lines, \
                 or a `[[toolkit.exemptions.long_file]]` entry in \
                 toolkit/toolkit.toml with a non-empty reason."
            ),
        ));
    }

    if violations.is_empty() {
        println!("long-file: no Rust source file exceeds {MAX_FILE_LINES} lines");
        return Ok(());
    }

    eprintln!("long-file: found {} oversized file(s):", violations.len());
    for violation in &violations {
        eprintln!("  {}", violation.render());
    }
    bail!("long-file failed");
}

fn has_exception_marker(contents: &str) -> bool {
    contents
        .lines()
        .take(MARKER_SCAN_LINES)
        .any(|line| match line.find(EXCEPTION_MARKER) {
            Some(idx) => !line[idx + EXCEPTION_MARKER.len()..].trim().is_empty(),
            None => false,
        })
}
