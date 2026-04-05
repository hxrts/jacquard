//! Source-level helpers for scanning explicit model-policy annotations.

use rustc_hir::Item;
use rustc_span::source_map::SourceMap;

pub(crate) fn source_has_attribute(
    source_map: &SourceMap,
    item: &Item<'_>,
    attr_name: &str,
) -> bool {
    let file = source_map.lookup_source_file(item.span.lo());
    let Ok(contents) = std::fs::read_to_string(&file.name.prefer_remapped_unconditionally()) else {
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
