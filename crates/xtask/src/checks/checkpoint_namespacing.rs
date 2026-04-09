//! Validates that checkpoint keys follow the namespace pattern.
//!
//! Only checks string literals passed to storage operations (store_bytes,
//! load_bytes, remove_bytes) — not arbitrary strings or choreography names.

use anyhow::{bail, Result};
use syn::visit::Visit;

use crate::{sources::parse_workspace_sources, util::Violation};

struct CheckpointKeyVisitor {
    crate_type: &'static str,
    violations: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for CheckpointKeyVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method_name = call.method.to_string();

        // Only check storage-related method calls
        if matches!(
            method_name.as_str(),
            "store_bytes" | "load_bytes" | "remove_bytes" | "checkpoint" | "restore"
        ) {
            // First argument should be the key
            if let Some(syn::Expr::Lit(expr_lit)) = call.args.first() {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    self.validate_key(&lit_str.value());
                }
            }
        }

        syn::visit::visit_expr_method_call(self, call);
    }
}

impl CheckpointKeyVisitor {
    fn validate_key(&mut self, key: &str) {
        let valid = match self.crate_type {
            | "mesh" => key.starts_with("engine/mesh/") || key.starts_with("mesh/"),
            | "router" => key.starts_with("router/"),
            | _ => true,
        };

        if !valid {
            let suggested = match self.crate_type {
                | "mesh" => format!("engine/mesh/{}", key),
                | "router" => format!("router/{}", key),
                | _ => key.to_string(),
            };
            self.violations.push((
                1,
                format!(
                    "bare checkpoint key '{}' in storage call; use namespaced key '{}'",
                    key, suggested
                ),
            ));
        }
    }
}

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut violations = Vec::new();

    for source in parsed {
        let crate_type = if source.rel_path.starts_with("crates/pathway/src/") {
            "mesh"
        } else if source.rel_path.starts_with("crates/router/src/") {
            "router"
        } else {
            continue;
        };

        // Skip choreography directories — they contain protocol message names,
        // not checkpoint keys
        if source.rel_path.contains("/choreography/") {
            continue;
        }

        for item in &source.file.items {
            let mut visitor =
                CheckpointKeyVisitor { crate_type, violations: Vec::new() };

            visitor.visit_item(item);

            for (line, msg) in visitor.violations {
                violations.push(Violation::new(&source.rel_path, line, msg));
            }
        }
    }

    if violations.is_empty() {
        println!("checkpoint-namespacing: all checkpoint keys properly namespaced");
        return Ok(());
    }

    eprintln!("checkpoint-namespacing: found violations:");
    for v in &violations {
        eprintln!("  {}", v.render());
    }
    bail!("checkpoint-namespacing failed");
}
