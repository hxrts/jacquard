//! Rejects `Result`-returning trait methods that lack `#[must_use]`.
//!
//! Every `fn method(...) -> Result<T, E>` declared inside a public trait
//! under `crates/traits/src/` must carry `#[must_use]`. The style guide
//! requires this annotation to prevent callers from silently discarding errors
//! or important routing evidence at trait-boundary call sites.
//!
//! Uses a `syn` AST visitor over `ItemTrait` blocks, inspecting each trait
//! method's return type. Methods returning `Result<..>` without `#[must_use]`
//! in the `L2` (traits) layer are reported as violations.
//!
//! Registered as: `cargo xtask check result-must-use`

use anyhow::Result;
use syn::{visit::Visit, Item, ReturnType, Type};

use crate::{
    sources::parse_workspace_sources,
    util::{layer_of, Violation},
};

struct ResultMethodVisitor {
    pub trait_name: String,
    pub found_violations: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for ResultMethodVisitor {
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        let trait_name = item.ident.to_string();
        self.trait_name = trait_name;

        for item in &item.items {
            if let syn::TraitItem::Fn(method) = item {
                // Check if method returns Result
                if is_result_return_type(&method.sig.output) {
                    // Check if it has #[must_use]
                    let has_must_use = method
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("must_use"));

                    if !has_must_use {
                        let method_name = method.sig.ident.to_string();
                        self.found_violations.push((1, method_name));
                    }
                }
            }
        }

        syn::visit::visit_item_trait(self, item);
    }
}

fn is_result_return_type(return_type: &ReturnType) -> bool {
    match return_type {
        | ReturnType::Default => false,
        | ReturnType::Type(_, ty) => is_result_type(ty),
    }
}

fn is_result_type(ty: &Type) -> bool {
    match ty {
        | Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .any(|segment| segment.ident == "Result"),
        | _ => false,
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
            if let Item::Trait(trait_item) = item {
                let mut visitor = ResultMethodVisitor {
                    trait_name: trait_item.ident.to_string(),
                    found_violations: Vec::new(),
                };

                visitor.visit_item_trait(trait_item);

                for (line, method_name) in visitor.found_violations {
                    violations.push(Violation::with_layer(
                        source.rel_path.clone(),
                        line,
                        format!(
                            "trait {} method {} returns Result without #[must_use]",
                            visitor.trait_name, method_name
                        ),
                        layer_of("jacquard-traits"),
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        violations.sort_by(|a, b| {
            if a.file != b.file {
                a.file.cmp(&b.file)
            } else {
                a.line.cmp(&b.line)
            }
        });
        for v in &violations {
            eprintln!("{}:{}: {}", v.file, v.line, v.message);
        }
        std::process::exit(1);
    }

    Ok(())
}
