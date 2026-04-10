//! Enforces the mechanized subset of the Rust style guide.
//!
//! Current rules:
//! - functions and methods longer than 60 lines are rejected unless they carry
//!   an explicit `long-block-exception:` or `long-fn-exception:` marker near
//!   the top of the body, or are listed in the centralized style exemptions
//! - fields named `*_visible` / `visible_*` must not use raw `bool`; encode the
//!   visibility state in the type shape rather than a redundant flag
//!
//! Registered as: `cargo xtask check rust-style-guide`

use anyhow::{bail, Result};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Field, ImplItemFn, ItemEnum, ItemFn, ItemMod, ItemStruct, TraitItemFn, Type,
};

use crate::{
    exemptions::style_guide_exceptions,
    sources::{parse_workspace_sources, ParsedSource},
    util::Violation,
};

const FUNCTION_LINE_MAX: usize = 60;
const EXCEPTION_MARKERS: &[&str] = &["long-block-exception:", "long-fn-exception:"];

pub fn run() -> Result<()> {
    let violations = collect_violations()?;
    report_violations(&violations)?;

    println!("rust-style-guide: Rust style invariants are valid");
    Ok(())
}

fn collect_violations() -> Result<Vec<Violation>> {
    let parsed = parse_workspace_sources()?;
    let style_exceptions = style_guide_exceptions()?;
    let mut violations = Vec::new();
    for source in &parsed {
        violations.extend(scan_function_lengths(source, &style_exceptions));
        violations.extend(scan_visibility_bools(source));
    }
    Ok(violations)
}

fn report_violations(violations: &[Violation]) -> Result<()> {
    if violations.is_empty() {
        return Ok(());
    }
    for violation in violations {
        eprintln!("{}", violation.render());
    }
    eprintln!();
    eprintln!(
        "rust-style-guide: found {} style violation(s)",
        violations.len()
    );
    bail!("rust-style-guide failed");
}

fn scan_function_lengths(
    source: &ParsedSource,
    style_exceptions: &[(String, String)],
) -> Vec<Violation> {
    let crate_prefix = crate_prefix(&source.rel_path);
    let module_prefix = module_prefix(&source.rel_path);
    let source_lines: Vec<&str> = source.source.lines().collect();
    let mut fn_collector = FunctionCollector {
        crate_prefix: &crate_prefix,
        module_stack: module_prefix,
        functions: Vec::new(),
    };
    fn_collector.visit_file(&source.file);

    let mut violations = Vec::new();
    for function in fn_collector.functions {
        let function_line_count = function.end_line.saturating_sub(function.start_line) + 1;
        if function_line_count <= FUNCTION_LINE_MAX {
            continue;
        }
        if has_nearby_exception_marker(&source_lines, function.start_line, function.end_line)
            || is_style_exempt(style_exceptions, &function.symbol)
        {
            continue;
        }
        violations.push(Violation::new(
            source.rel_path.clone(),
            function.start_line,
            format!(
                "function `{}` is {} lines; split it or add a documented style exception",
                function.symbol, function_line_count
            ),
        ));
    }
    violations
}

fn scan_visibility_bools(source: &ParsedSource) -> Vec<Violation> {
    let mut visitor = VisibleBoolVisitor::default();
    visitor.visit_file(&source.file);
    visitor
        .violations
        .into_iter()
        .map(|violation| {
            Violation::new(
                source.rel_path.clone(),
                violation.line,
                format!(
                    "field `{}` uses raw `bool` for visibility; encode visibility in the type shape instead",
                    violation.field_name
                ),
            )
        })
        .collect()
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

fn has_nearby_exception_marker(source_lines: &[&str], start_line: usize, end_line: usize) -> bool {
    let before = start_line.saturating_sub(4).max(1);
    let after = end_line.min(start_line.saturating_add(12));
    for line_no in before..=after {
        let Some(line) = source_lines.get(line_no - 1) else {
            continue;
        };
        if EXCEPTION_MARKERS.iter().any(|marker| line.contains(marker)) {
            return true;
        }
    }
    false
}

fn is_style_exempt(exemptions: &[(String, String)], symbol: &str) -> bool {
    exemptions
        .iter()
        .any(|(exempt_symbol, _)| exempt_symbol == symbol)
}

#[derive(Default)]
struct VisibleBoolVisitor {
    violations: Vec<VisibleBoolViolation>,
}

impl<'ast> Visit<'ast> for VisibleBoolVisitor {
    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        for field in &item.fields {
            self.record_field(field);
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        for variant in &item.variants {
            for field in &variant.fields {
                self.record_field(field);
            }
        }
        visit::visit_item_enum(self, item);
    }
}

impl VisibleBoolVisitor {
    fn record_field(&mut self, field: &Field) {
        let Some(field_ident) = field.ident.as_ref() else {
            return;
        };
        let field_name = field_ident.to_string();
        if !is_visibility_field_name(&field_name) || !is_bool_type(&field.ty) {
            return;
        }
        self.violations.push(VisibleBoolViolation {
            field_name,
            line: field.span().start().line,
        });
    }
}

struct VisibleBoolViolation {
    field_name: String,
    line: usize,
}

fn is_visibility_field_name(field_name: &str) -> bool {
    field_name.ends_with("_visible") || field_name.starts_with("visible_")
}

fn is_bool_type(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    type_path.qself.is_none() && type_path.path.is_ident("bool")
}

struct FunctionCollector<'a> {
    crate_prefix: &'a str,
    module_stack: Vec<String>,
    functions: Vec<FunctionOccurrence>,
}

impl<'ast> Visit<'ast> for FunctionCollector<'_> {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if let Some((_, items)) = &item.content {
            self.module_stack.push(item.ident.to_string());
            for nested in items {
                self.visit_item(nested);
            }
            self.module_stack.pop();
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.functions.push(function_occurrence(
            self.crate_prefix,
            &self.module_stack,
            item,
        ));
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.functions.push(FunctionOccurrence {
            symbol: symbol_name(
                self.crate_prefix,
                &self.module_stack,
                &item.sig.ident.to_string(),
            ),
            start_line: item.span().start().line,
            end_line: item.span().end().line,
        });
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        if item.default.is_none() {
            return;
        }
        self.functions.push(FunctionOccurrence {
            symbol: symbol_name(
                self.crate_prefix,
                &self.module_stack,
                &item.sig.ident.to_string(),
            ),
            start_line: item.span().start().line,
            end_line: item.span().end().line,
        });
        visit::visit_trait_item_fn(self, item);
    }
}

fn function_occurrence(
    crate_prefix: &str,
    module_stack: &[String],
    item: &ItemFn,
) -> FunctionOccurrence {
    FunctionOccurrence {
        symbol: symbol_name(crate_prefix, module_stack, &item.sig.ident.to_string()),
        start_line: item.span().start().line,
        end_line: item.span().end().line,
    }
}

fn symbol_name(crate_prefix: &str, module_stack: &[String], fn_name: &str) -> String {
    if module_stack.is_empty() {
        format!("{crate_prefix}::{fn_name}")
    } else {
        format!("{crate_prefix}::{}::{fn_name}", module_stack.join("::"))
    }
}

struct FunctionOccurrence {
    symbol: String,
    start_line: usize,
    end_line: usize,
}
