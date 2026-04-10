//! Source-level helpers for routing-invariant scans.
//!
//! Provides utilities shared across all routing-invariant lint passes for
//! reading and searching raw source text. Because many routing invariants are
//! structural patterns in source (e.g., ordering of mutations relative to
//! persistence calls, or literal patterns like unscoped storage key strings),
//! the lint passes operate on file text rather than purely on the HIR.
//!
//! Key functions:
//! - `source_file_contents` — resolves a HIR item's source file path and reads
//!   the full file text, returning both.
//! - `rel_path` — normalizes a path to a forward-slash-separated string for
//!   consistent cross-platform substring matching.
//! - `first_line_matching` — returns the 1-based line number of the first line
//!   in a file that matches a given regex.
//! - `line_position` — returns the 1-based line number of the first line
//!   containing a given literal substring.

use std::path::{Path, PathBuf};

use regex::Regex;
use rustc_hir::Item;
use rustc_span::source_map::SourceMap;

pub(crate) fn source_file_path(source_map: &SourceMap, item: &Item<'_>) -> PathBuf {
    PathBuf::from(format!(
        "{}",
        source_map
            .lookup_source_file(item.span.lo())
            .name
            .prefer_remapped_unconditionally()
    ))
}

pub(crate) fn source_file_contents(
    source_map: &SourceMap,
    item: &Item<'_>,
) -> Option<(PathBuf, String)> {
    let path = source_file_path(source_map, item);
    let contents = std::fs::read_to_string(&path).ok()?;
    Some((path, contents))
}

pub(crate) fn rel_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn first_line_matching(contents: &str, re: &Regex) -> Option<usize> {
    contents
        .lines()
        .enumerate()
        .find(|(_, line)| re.is_match(line))
        .map(|(idx, _)| idx + 1)
}

pub(crate) fn line_position(contents: &str, needle: &str) -> Option<usize> {
    contents
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .map(|(idx, _)| idx + 1)
}
