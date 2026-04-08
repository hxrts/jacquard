//! Source-level helpers for routing-invariant scans.

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
