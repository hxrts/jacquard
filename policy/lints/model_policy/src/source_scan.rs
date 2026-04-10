//! Source-level helpers for scanning explicit model-policy annotations.
//!
//! This module provides utilities used by the model-policy lint passes to
//! inspect raw source text alongside the HIR. The compiler's attribute
//! representation does not always expose proc-macro annotations the same
//! way they appear in source, so these helpers read the file on disk and
//! scan lines directly.
//!
//! Key functions:
//! - `source_has_attribute` — checks whether an item is preceded by a given
//!   `#[attr_name]` annotation in the source file, skipping doc comments and
//!   other attributes in between.
//! - `source_file_contents` — reads the full source text for the file
//!   containing a given HIR item, returning the resolved path alongside it.
//! - `line_number` — maps a `Span` to its 1-based source line number.

use std::path::PathBuf;

use rustc_hir::Item;
use rustc_span::{source_map::SourceMap, Span};

pub(crate) fn source_has_attribute(
    source_map: &SourceMap,
    item: &Item<'_>,
    attr_name: &str,
) -> bool {
    let path = source_file_path(source_map, item);
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return false;
    };
    let line_index = source_map
        .lookup_char_pos(item.span.lo())
        .line
        .saturating_sub(1);
    let lines: Vec<&str> = contents.lines().collect();

    if line_index >= lines.len() {
        return false;
    }

    let expected_prefix = format!("#[{attr_name}");

    for line in lines[..line_index].iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("///") {
            continue;
        }

        if trimmed.starts_with("#[") {
            if trimmed.starts_with(&expected_prefix) {
                return true;
            }

            continue;
        }

        return false;
    }

    false
}

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

pub(crate) fn line_number(source_map: &SourceMap, span: Span) -> usize {
    source_map.lookup_char_pos(span.lo()).line
}
