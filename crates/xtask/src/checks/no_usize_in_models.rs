//! Rejects bare `usize` field types in public structs and enums under
//! `crates/core` and `crates/traits`. The style guide requires explicit
//! newtypes or fixed-width integer choices for stored model fields.

use anyhow::{bail, Result};
use syn::{visit::Visit, Fields, Item, Type};

use crate::{
    sources::parse_workspace_sources,
    util::{layer_of, Violation},
};

struct BarePrimitiveVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for BarePrimitiveVisitor {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if path
            .path
            .segments
            .iter()
            .any(|segment| matches!(segment.ident.to_string().as_str(), "usize"))
        {
            self.found = true;
        }
        syn::visit::visit_type_path(self, path);
    }
}

fn type_has_bare_primitive(ty: &Type) -> bool {
    let mut visitor = BarePrimitiveVisitor { found: false };
    visitor.visit_type(ty);
    visitor.found
}

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut violations = Vec::new();

    for source in parsed {
        if !(source.rel_path.starts_with("crates/core/src/")
            || source.rel_path.starts_with("crates/traits/src/"))
        {
            continue;
        }

        let crate_name = extract_crate_name(&source.rel_path);

        for item in &source.file.items {
            match item {
                | Item::Struct(item_struct) => {
                    collect_struct_fields(
                        &source.rel_path,
                        item_struct.ident.to_string(),
                        &item_struct.fields,
                        &mut violations,
                        crate_name,
                    );
                },
                | Item::Enum(item_enum) => {
                    for variant in &item_enum.variants {
                        collect_struct_fields(
                            &source.rel_path,
                            format!("{}::{}", item_enum.ident, variant.ident),
                            &variant.fields,
                            &mut violations,
                            crate_name,
                        );
                    }
                },
                | _ => {},
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("no-usize-in-models: found usize in model source files:");
        for violation in &violations {
            eprintln!("  {}", violation.render());
        }
        eprintln!();
        eprintln!(
            "no-usize-in-models: use explicit newtypes or fixed-width integers instead"
        );
        bail!("no-usize-in-models failed");
    }

    println!("no-usize-in-models: no usize found in model source files");
    Ok(())
}

fn extract_crate_name(rel_path: &str) -> &str {
    // Extract crate name from "crates/core/src/..." -> "jacquard-core"
    if rel_path.starts_with("crates/core/") {
        "jacquard-core"
    } else if rel_path.starts_with("crates/traits/") {
        "jacquard-traits"
    } else {
        "unknown"
    }
}

fn collect_struct_fields(
    rel_path: &str,
    owner: String,
    fields: &Fields,
    out: &mut Vec<Violation>,
    crate_name: &str,
) {
    // Skip newtypes: single field with a bare primitive is the idiomatic newtype
    // pattern
    if is_newtype_pattern(fields) {
        return;
    }

    for field in fields {
        if type_has_bare_primitive(&field.ty) {
            out.push(Violation::with_layer(
                rel_path,
                1,
                format!("{owner} contains usize in a stored field"),
                layer_of(crate_name),
            ));
        }
    }
}

fn is_newtype_pattern(fields: &Fields) -> bool {
    matches!(fields, Fields::Unnamed(f) if f.unnamed.len() == 1)
}
