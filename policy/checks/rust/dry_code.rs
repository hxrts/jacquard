//! Enforces a conservative DRY rule over Rust function bodies.
//!
//! Rules:
//! - non-trivial function bodies must not be duplicated verbatim across the
//!   workspace after whitespace/comment normalization
//! - only larger blocks are considered so tiny constructors and trivial
//!   wrappers do not create noise
//!
//! Registered as: `cargo xtask check dry-code`

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Attribute, ImplItemFn, ItemFn, ItemMod,
};

use crate::{
    sources::{parse_workspace_sources, ParsedSource},
    util::Violation,
};

const MIN_BLOCK_LINES: usize = 8;
const MIN_NORMALIZED_CHARS: usize = 120;

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let occurrences = collect_occurrences(&parsed);
    let mut groups: BTreeMap<(String, String), Vec<FunctionOccurrence>> = BTreeMap::new();
    for occurrence in occurrences {
        if occurrence.block_lines < MIN_BLOCK_LINES
            || occurrence.normalized_body.len() < MIN_NORMALIZED_CHARS
        {
            continue;
        }
        groups
            .entry((
                occurrence.crate_prefix.clone(),
                occurrence.normalized_body.clone(),
            ))
            .or_default()
            .push(occurrence);
    }

    let mut violations = Vec::new();
    for group in groups.into_values() {
        if group.len() < 2 {
            continue;
        }
        let peers = group
            .iter()
            .map(|occurrence| {
                format!(
                    "{}:{} ({})",
                    occurrence.rel_path, occurrence.start_line, occurrence.symbol
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        for occurrence in group {
            violations.push(Violation::new(
                occurrence.rel_path,
                occurrence.start_line,
                format!(
                    "function `{}` duplicates a non-trivial Rust block; factor shared logic instead of copying it ({peers})",
                    occurrence.symbol
                ),
            ));
        }
    }

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!("dry-code: found {} duplicate block(s)", violations.len());
        bail!("dry-code failed");
    }

    println!("dry-code: no duplicate non-trivial Rust blocks found");
    Ok(())
}

fn collect_occurrences(parsed: &[ParsedSource]) -> Vec<FunctionOccurrence> {
    let mut out = Vec::new();
    for source in parsed {
        let crate_prefix = crate_prefix(&source.rel_path);
        let module_prefix = module_prefix(&source.rel_path);
        let source_lines: Vec<&str> = source.source.lines().collect();
        let mut collector = FunctionCollector {
            rel_path: &source.rel_path,
            crate_prefix: &crate_prefix,
            module_stack: module_prefix,
            source_lines: &source_lines,
            test_depth: 0,
            functions: Vec::new(),
        };
        collector.visit_file(&source.file);
        out.extend(collector.functions);
    }
    out
}

fn crate_prefix(rel_path: &str) -> String {
    let Some(crate_segment) = rel_path.split('/').nth(1) else {
        return "workspace".to_string();
    };
    format!("jacquard_{}", crate_segment.replace('-', "_"))
}

fn module_prefix(rel_path: &str) -> Vec<String> {
    let Some(src_index) = rel_path.find("/src/") else {
        return Vec::new();
    };
    let after_src = &rel_path[src_index + 5..];
    let without_ext = after_src.strip_suffix(".rs").unwrap_or(after_src);
    if without_ext == "lib" {
        return Vec::new();
    }
    if let Some(dir) = without_ext.strip_suffix("/mod") {
        return dir
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect();
    }
    without_ext
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

struct FunctionCollector<'a> {
    rel_path: &'a str,
    crate_prefix: &'a str,
    module_stack: Vec<String>,
    source_lines: &'a [&'a str],
    test_depth: usize,
    functions: Vec<FunctionOccurrence>,
}

impl<'ast> Visit<'ast> for FunctionCollector<'_> {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        let is_test_context = item.ident == "tests" || has_cfg_test(&item.attrs);
        if let Some((_, items)) = &item.content {
            self.module_stack.push(item.ident.to_string());
            if is_test_context {
                self.test_depth += 1;
            }
            for nested in items {
                self.visit_item(nested);
            }
            if is_test_context {
                self.test_depth -= 1;
            }
            self.module_stack.pop();
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if self.test_depth > 0 || has_cfg_test(&item.attrs) {
            return;
        }
        self.functions.push(FunctionOccurrence {
            rel_path: self.rel_path.to_string(),
            crate_prefix: self.crate_prefix.to_string(),
            symbol: symbol_name(
                self.crate_prefix,
                &self.module_stack,
                &item.sig.ident.to_string(),
            ),
            start_line: item.block.span().start().line,
            block_lines: item
                .block
                .span()
                .end()
                .line
                .saturating_sub(item.block.span().start().line)
                + 1,
            normalized_body: normalized_block(
                self.source_lines,
                item.block.span().start().line,
                item.block.span().end().line,
            ),
        });
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if self.test_depth > 0 || has_cfg_test(&item.attrs) {
            return;
        }
        self.functions.push(FunctionOccurrence {
            rel_path: self.rel_path.to_string(),
            crate_prefix: self.crate_prefix.to_string(),
            symbol: symbol_name(
                self.crate_prefix,
                &self.module_stack,
                &item.sig.ident.to_string(),
            ),
            start_line: item.block.span().start().line,
            block_lines: item
                .block
                .span()
                .end()
                .line
                .saturating_sub(item.block.span().start().line)
                + 1,
            normalized_body: normalized_block(
                self.source_lines,
                item.block.span().start().line,
                item.block.span().end().line,
            ),
        });
        visit::visit_impl_item_fn(self, item);
    }
}

#[derive(Clone)]
struct FunctionOccurrence {
    rel_path: String,
    crate_prefix: String,
    symbol: String,
    start_line: usize,
    block_lines: usize,
    normalized_body: String,
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .meta
                .require_list()
                .is_ok_and(|list| list.tokens.to_string().contains("test"))
    })
}

fn normalized_block(source_lines: &[&str], start_line: usize, end_line: usize) -> String {
    let start = start_line.saturating_sub(1);
    let end = end_line.min(source_lines.len());
    source_lines[start..end]
        .iter()
        .map(|line| line.split("//").next().unwrap_or_default())
        .flat_map(|line| line.chars())
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

fn symbol_name(crate_prefix: &str, module_stack: &[String], name: &str) -> String {
    let mut parts = Vec::with_capacity(module_stack.len() + 2);
    parts.push(crate_prefix.to_string());
    parts.extend(module_stack.iter().cloned());
    parts.push(name.to_string());
    parts.join("::")
}
