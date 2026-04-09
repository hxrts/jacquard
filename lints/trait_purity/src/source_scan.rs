//! Source-level helpers for scanning explicit trait annotation markers.
//!
//! Provides `source_has_trait_purity_marker`, which checks whether a public
//! trait definition in source code is immediately preceded by a purity or
//! effect annotation. The scan walks backward from the trait's opening line,
//! skipping blank lines and doc comments, and returns `true` only if the first
//! non-skipped line is `#[purity(...)]` or `#[effect_trait]`.
//!
//! This source-level scan is necessary because the relevant proc-macro
//! annotations (`#[purity(...)]`, `#[effect_trait]`) are applied before
//! expansion and may not be directly visible through the HIR attribute list at
//! the lint site. Reading the file on disk provides a reliable detection path
//! independent of how the compiler represents those attributes internally.

use rustc_hir::Item;
use rustc_span::source_map::SourceMap;

pub(crate) fn source_has_trait_purity_marker(source_map: &SourceMap, item: &Item<'_>) -> bool {
    let file = source_map.lookup_source_file(item.span.lo());
    let path = format!("{}", file.name.prefer_remapped_unconditionally());
    let Ok(contents) = std::fs::read_to_string(path) else {
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

    for line in lines[..line_index].iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("///") {
            continue;
        }

        return trimmed.starts_with("#[purity(") || trimmed == "#[effect_trait]";
    }

    false
}
