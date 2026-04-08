//! Validates that transport-related traits are classified as connectivity or
//! service surface in their doc comments.
//!
//! Only targets traits whose name contains "Transport" — these are the
//! traits that describe a physical/network boundary and need explicit
//! classification as connectivity surface (carries opaque bytes) or service
//! surface (carries typed semantic operations).

use anyhow::{bail, Result};

use crate::{sources::parse_workspace_sources, util::Violation};

fn is_transport_trait(name: &str) -> bool {
    // Only traits describing physical transport/network boundaries
    name.contains("Transport") && !name.contains("TransportObservation")
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
                let trait_name = trait_item.ident.to_string();

                if !is_transport_trait(&trait_name) {
                    continue;
                }

                // Get doc comments
                let doc_text = trait_item
                    .attrs
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
                    .join(" ");

                let lower_doc = doc_text.to_lowercase();

                if !lower_doc.contains("connectivity surface")
                    && !lower_doc.contains("service surface")
                {
                    violations.push(Violation::new(
                        &source.rel_path,
                        1,
                        format!(
                            "trait {} missing surface classification (\"connectivity surface\" or \"service surface\") in doc comment",
                            trait_name
                        ),
                    ));
                }
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
