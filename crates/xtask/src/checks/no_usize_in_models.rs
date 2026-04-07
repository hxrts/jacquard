//! Rejects `usize` field types in public structs and enums under
//! `crates/core` and `crates/traits`. The style guide requires
//! explicitly-sized integers in stored and protocol types.

use anyhow::{bail, Result};
use syn::{visit::Visit, Fields, Item, Type};

use crate::{sources::parse_workspace_sources, util::Violation};

struct UsizeVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for UsizeVisitor {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if path
            .path
            .segments
            .iter()
            .any(|segment| segment.ident == "usize")
        {
            self.found = true;
        }
        syn::visit::visit_type_path(self, path);
    }
}

fn type_has_usize(ty: &Type) -> bool {
    let mut visitor = UsizeVisitor { found: false };
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

        for item in &source.file.items {
            match item {
                Item::Struct(item_struct) => {
                    collect_struct_fields(
                        &source.rel_path,
                        item_struct.ident.to_string(),
                        &item_struct.fields,
                        &mut violations,
                    );
                }
                Item::Enum(item_enum) => {
                    for variant in &item_enum.variants {
                        collect_struct_fields(
                            &source.rel_path,
                            format!("{}::{}", item_enum.ident, variant.ident),
                            &variant.fields,
                            &mut violations,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("no-usize-in-models: found usize in model source files:");
        for violation in &violations {
            eprintln!("  {}", violation.render());
        }
        eprintln!();
        eprintln!("no-usize-in-models: use explicitly-sized integers (u8, u16, u32, u64) instead");
        bail!("no-usize-in-models failed");
    }

    println!("no-usize-in-models: no usize found in model source files");
    Ok(())
}

fn collect_struct_fields(rel_path: &str, owner: String, fields: &Fields, out: &mut Vec<Violation>) {
    for field in fields {
        if type_has_usize(&field.ty) {
            out.push(Violation::new(
                rel_path,
                1,
                format!("{owner} contains usize in a stored field"),
            ));
        }
    }
}
