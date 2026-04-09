//! Validates that transport-related traits are classified in their doc
//! comments.
//!
//! Traits whose name contains "Transport" describe a physical or network
//! boundary and must declare their surface kind in their module-level docs:
//! - `connectivity surface` — the trait carries opaque bytes across a link.
//! - `service surface` — the trait carries typed semantic operations.
//!
//! Scans: all parsed workspace sources via `parse_workspace_sources`. Each
//! public trait whose name matches `is_transport_trait` is inspected for the
//! required classification phrase in its doc-comment text.
//!
//! Registered as: `cargo xtask check surface-classification`

use anyhow::{bail, Result};

use crate::{sources::parse_workspace_sources, util::Violation};

fn is_transport_trait(name: &str) -> bool {
    // Only traits describing physical transport/network boundaries
    name.contains("Transport") && !name.contains("TransportObservation")
}

fn extract_doc_text(attrs: &[syn::Attribute]) -> String {
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

fn check_trait(
    trait_item: &syn::ItemTrait,
    rel_path: &str,
    violations: &mut Vec<Violation>,
) {
    let trait_name = trait_item.ident.to_string();
    if !is_transport_trait(&trait_name) {
        return;
    }
    let doc_text = extract_doc_text(&trait_item.attrs).to_lowercase();
    if !doc_text.contains("connectivity surface")
        && !doc_text.contains("service surface")
    {
        violations.push(Violation::new(
            rel_path,
            1,
            format!(
                "trait {trait_name} missing surface classification (\"connectivity surface\" or \"service surface\") in doc comment"
            ),
        ));
    }
}

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut violations = Vec::new();

    for source in parsed {
        if !source.rel_path.starts_with("crates/traits/src/") {
            continue;
        }
        for item in &source.file.items {
            if let syn::Item::Trait(trait_item) = item {
                check_trait(trait_item, &source.rel_path, &mut violations);
            }
        }
    }

    if violations.is_empty() {
        println!("surface-classification: all transport traits properly classified");
        return Ok(());
    }

    eprintln!("surface-classification: found violations:");
    for v in &violations {
        eprintln!("  {}", v.render());
    }
    bail!("surface-classification failed");
}
