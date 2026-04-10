//! Validates that high-consequence decisions are returned as typed
//! proof/evidence.
//!
//! Enforces that public methods returning `Result<MaterializedRoute, _>`,
//! `Result<RouteInstallation, _>`, `Result<RouteMaintenanceResult, _>`, or
//! other high-consequence types carry explicit doc comments that explain the
//! proof semantics of the returned value. These are control-plane surfaces
//! where silently dropping the returned proof loses audit evidence.
//!
//! Uses a `syn` AST visitor over `ItemImpl` blocks, inspecting each public
//! method's return type and attribute list. Methods returning one of the
//! `HIGH_CONSEQUENCE_TYPES` without a `///` doc comment are reported.
//! Registered as: `cargo xtask check proof-bearing-actions`

use anyhow::{bail, Result};
use syn::visit::Visit;

use crate::{sources::parse_workspace_sources, util::Violation};

const HIGH_CONSEQUENCE_TYPES: &[&str] = &[
    "MaterializedRoute",
    "RouteInstallation",
    "RouteMaintenanceResult",
    "RouteAdmission",
    "RouterMaintenanceOutcome",
];

struct ProofBearingVisitor<'a> {
    rel_path: &'a str,
    violations: &'a mut Vec<Violation>,
}

impl<'a, 'ast> Visit<'ast> for ProofBearingVisitor<'a> {
    fn visit_item_impl(&mut self, item_impl: &'ast syn::ItemImpl) {
        for impl_item in &item_impl.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                self.check_method(&method.sig, &method.attrs, &method.vis);
            }
        }
        syn::visit::visit_item_impl(self, item_impl);
    }

    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        self.check_method(&item_fn.sig, &item_fn.attrs, &item_fn.vis);
        syn::visit::visit_item_fn(self, item_fn);
    }
}

impl<'a> ProofBearingVisitor<'a> {
    fn check_method(
        &mut self,
        sig: &syn::Signature,
        attrs: &[syn::Attribute],
        vis: &syn::Visibility,
    ) {
        // Only check public methods
        if !matches!(vis, syn::Visibility::Public(_)) {
            return;
        }

        if let syn::ReturnType::Type(_, ty) = &sig.output {
            if let Some(ok_type) = extract_result_ok_type(ty) {
                if HIGH_CONSEQUENCE_TYPES.iter().any(|t| ok_type.contains(t)) {
                    // Check for doc comment
                    let doc_text = extract_doc(attrs);
                    let lower = doc_text.to_lowercase();

                    let has_proof_language = lower.contains("proof")
                        || lower.contains("evidence")
                        || lower.contains("witness")
                        || lower.contains("canonical")
                        || lower.contains("handle")
                        || lower.contains("admission")
                        || lower.contains("installation");

                    if doc_text.is_empty() {
                        self.violations.push(Violation::new(
                            self.rel_path,
                            1,
                            format!(
                                "fn {} returns {} but has no doc comment explaining proof semantics",
                                sig.ident, ok_type
                            ),
                        ));
                    } else if !has_proof_language {
                        self.violations.push(Violation::new(
                            self.rel_path,
                            1,
                            format!(
                                "fn {} returns {} but doc comment does not mention proof/evidence/witness/canonical/handle semantics",
                                sig.ident, ok_type
                            ),
                        ));
                    }
                }
            }
        }
    }
}

fn extract_doc(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if nv.path.is_ident("doc") {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) = &nv.value
                    {
                        return Some(lit_str.value());
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_result_ok_type(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(type_path) = ty {
        let last = type_path.path.segments.last()?;
        if last.ident != "Result" {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
            if let Some(syn::GenericArgument::Type(ok_ty)) = args.args.first() {
                return Some(type_to_string(ok_ty));
            }
        }
    }
    None
}

fn type_to_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        _ => String::new(),
    }
}

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut violations = Vec::new();

    for source in parsed {
        if !(source.rel_path.starts_with("crates/router/src/")
            || source.rel_path.starts_with("crates/pathway/src/"))
        {
            continue;
        }

        let mut visitor = ProofBearingVisitor {
            rel_path: &source.rel_path,
            violations: &mut violations,
        };

        for item in &source.file.items {
            visitor.visit_item(item);
        }
    }

    if violations.is_empty() {
        println!("proof-bearing-actions: all high-consequence methods have proof semantics docs");
        return Ok(());
    }

    eprintln!("proof-bearing-actions: found violations:");
    for v in &violations {
        eprintln!("  {}", v.render());
    }
    bail!("proof-bearing-actions failed");
}
