//! Enforces semantic consistency for Jacquard's annotation vocabulary.
//!
//! This check focuses on the intended meaning of the annotations rather than on
//! inventory bookkeeping:
//! - `#[effect_trait]` belongs on shared effect vocabulary traits in `traits`
//! - `#[effect_handler]` belongs on impls of actual effect traits, and effect
//!   trait impls must use it consistently
//! - `#[public_model]` belongs on shared public model vocabulary in `core` and
//!   the small transport-neutral adapter vocabulary in `adapter`
//! - `#[id_type]` belongs on shared identifier/time vocabulary in `core`
//! - `#[must_use_handle]` belongs on shared routing capability/handle types and
//!   must travel with `#[public_model]`
//!
//! Registered as: `cargo xtask check annotation-semantics`

use anyhow::{bail, Result};
use syn::{
    Attribute, Fields, Item, ItemEnum, ItemImpl, ItemMod, ItemStruct, ItemTrait, Visibility,
};

use crate::{
    sources::{attributes_match, parse_workspace_sources, ParsedSource},
    util::{layer_for_rel_path, Violation},
};

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let effect_traits = collect_effect_traits(&parsed);
    let mut violations = Vec::new();

    for source in &parsed {
        walk_items(
            &source.rel_path,
            &source.file.items,
            false,
            &effect_traits,
            &mut violations,
        );
    }

    if !violations.is_empty() {
        violations.sort();
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "annotation-semantics: found {} semantic annotation violation(s)",
            violations.len()
        );
        bail!("annotation-semantics failed");
    }

    println!("annotation-semantics: annotation usage is semantically consistent");
    Ok(())
}

fn collect_effect_traits(parsed: &[ParsedSource]) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    for source in parsed {
        collect_effect_traits_from_items(&source.file.items, false, &mut names);
    }
    names
}

fn collect_effect_traits_from_items(
    items: &[Item],
    in_test_context: bool,
    names: &mut std::collections::BTreeSet<String>,
) {
    for item in items {
        let is_test_context = in_test_context || attrs_have_cfg_test(item_attrs(item));
        match item {
            Item::Trait(item_trait) if !is_test_context => {
                if attributes_match(&item_trait.attrs, "effect_trait") {
                    names.insert(item_trait.ident.to_string());
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    collect_effect_traits_from_items(items, is_test_context, names);
                }
            }
            _ => {}
        }
    }
}

fn walk_items(
    rel_path: &str,
    items: &[Item],
    in_test_context: bool,
    effect_traits: &std::collections::BTreeSet<String>,
    violations: &mut Vec<Violation>,
) {
    for item in items {
        let is_test_context = in_test_context || attrs_have_cfg_test(item_attrs(item));
        match item {
            Item::Trait(item_trait) => {
                check_trait(rel_path, item_trait, is_test_context, violations)
            }
            Item::Impl(item_impl) => check_impl(
                rel_path,
                item_impl,
                is_test_context,
                effect_traits,
                violations,
            ),
            Item::Struct(item_struct) => {
                check_struct(rel_path, item_struct, is_test_context, violations)
            }
            Item::Enum(item_enum) => check_enum(rel_path, item_enum, is_test_context, violations),
            Item::Mod(ItemMod {
                content: Some((_, items)),
                ..
            }) => {
                walk_items(rel_path, items, is_test_context, effect_traits, violations);
            }
            _ => {}
        }
    }
}

fn check_trait(
    rel_path: &str,
    item_trait: &ItemTrait,
    is_test_context: bool,
    violations: &mut Vec<Violation>,
) {
    if is_test_context || !attributes_match(&item_trait.attrs, "effect_trait") {
        return;
    }

    if !rel_path.starts_with("crates/traits/src/") {
        violations.push(v(
            rel_path,
            item_trait,
            "`#[effect_trait]` must live under `crates/traits/src/`",
        ));
    }

    if !matches!(item_trait.vis, Visibility::Public(_)) {
        violations.push(v(
            rel_path,
            item_trait,
            "`#[effect_trait]` must annotate a public trait",
        ));
    }

    if item_trait.ident.to_string().ends_with("Driver") {
        violations.push(v(
            rel_path,
            item_trait,
            "driver supervision traits should use `#[purity(effectful)]`, not `#[effect_trait]`",
        ));
    }
}

fn check_impl(
    rel_path: &str,
    item_impl: &ItemImpl,
    is_test_context: bool,
    effect_traits: &std::collections::BTreeSet<String>,
    violations: &mut Vec<Violation>,
) {
    let Some((_, trait_path, _)) = &item_impl.trait_ else {
        if attributes_match(&item_impl.attrs, "effect_handler") {
            violations.push(v(
                rel_path,
                item_impl,
                "`#[effect_handler]` must annotate a trait impl, not an inherent impl",
            ));
        }
        return;
    };

    let Some(trait_name) = trait_path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
    else {
        return;
    };
    let implements_effect_trait = effect_traits.contains(&trait_name);
    let has_effect_handler = attributes_match(&item_impl.attrs, "effect_handler");

    if has_effect_handler && !implements_effect_trait {
        violations.push(v(
            rel_path,
            item_impl,
            format!("`#[effect_handler]` may only annotate impls of `#[effect_trait]` traits; `{trait_name}` is not one"),
        ));
    }

    if implements_effect_trait && !has_effect_handler && !is_test_context {
        violations.push(v(
            rel_path,
            item_impl,
            format!("impl of effect trait `{trait_name}` is missing `#[effect_handler]`"),
        ));
    }
}

fn check_struct(
    rel_path: &str,
    item_struct: &ItemStruct,
    is_test_context: bool,
    violations: &mut Vec<Violation>,
) {
    if is_test_context {
        return;
    }
    check_modelish_item(
        rel_path,
        &item_struct.attrs,
        &item_struct.vis,
        item_struct,
        violations,
    );

    if attributes_match(&item_struct.attrs, "id_type") && !is_tuple_newtype(&item_struct.fields) {
        violations.push(v(
            rel_path,
            item_struct,
            "`#[id_type]` must annotate a public tuple newtype",
        ));
    }
}

fn check_enum(
    rel_path: &str,
    item_enum: &ItemEnum,
    is_test_context: bool,
    violations: &mut Vec<Violation>,
) {
    if is_test_context {
        return;
    }
    check_modelish_item(
        rel_path,
        &item_enum.attrs,
        &item_enum.vis,
        item_enum,
        violations,
    );

    if attributes_match(&item_enum.attrs, "id_type") {
        violations.push(v(
            rel_path,
            item_enum,
            "`#[id_type]` is reserved for public tuple newtypes, not enums",
        ));
    }
}

fn check_modelish_item<T: syn::spanned::Spanned>(
    rel_path: &str,
    attrs: &[Attribute],
    vis: &Visibility,
    item: &T,
    violations: &mut Vec<Violation>,
) {
    let has_public_model = attributes_match(attrs, "public_model");
    let has_id_type = attributes_match(attrs, "id_type");
    let has_must_use_handle = attributes_match(attrs, "must_use_handle");
    let is_public = matches!(vis, Visibility::Public(_));

    if has_public_model && !public_model_scope_ok(rel_path) {
        violations.push(v(
            rel_path,
            item,
            "`#[public_model]` is reserved for shared vocabulary under `core` and transport-neutral adapter model surfaces",
        ));
    }

    if has_id_type && !rel_path.starts_with("crates/core/src/") {
        violations.push(v(
            rel_path,
            item,
            "`#[id_type]` is reserved for shared identifier/time vocabulary under `crates/core/src/`",
        ));
    }

    if has_must_use_handle && !rel_path.starts_with("crates/core/src/routing/") {
        violations.push(v(
            rel_path,
            item,
            "`#[must_use_handle]` is reserved for shared routing handle/lease capability types under `crates/core/src/routing/`",
        ));
    }

    if has_must_use_handle && !has_public_model {
        violations.push(v(
            rel_path,
            item,
            "`#[must_use_handle]` types must also carry `#[public_model]`",
        ));
    }

    if (has_public_model || has_id_type || has_must_use_handle) && !is_public {
        violations.push(v(
            rel_path,
            item,
            "semantic model annotations are reserved for public shared surfaces",
        ));
    }
}

fn public_model_scope_ok(rel_path: &str) -> bool {
    rel_path.starts_with("crates/core/src/")
        || matches!(
            rel_path,
            "crates/adapter/src/claims.rs"
                | "crates/adapter/src/dispatch.rs"
                | "crates/adapter/src/mailbox.rs"
                | "crates/adapter/src/peers.rs"
        )
}

fn is_tuple_newtype(fields: &Fields) -> bool {
    match fields {
        Fields::Unnamed(fields) => fields.unnamed.len() == 1,
        Fields::Named(_) | Fields::Unit => false,
    }
}

fn attrs_have_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found_test = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found_test = true;
            }
            Ok(())
        });
        found_test
    })
}

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(it) => &it.attrs,
        Item::Enum(it) => &it.attrs,
        Item::ExternCrate(it) => &it.attrs,
        Item::Fn(it) => &it.attrs,
        Item::ForeignMod(it) => &it.attrs,
        Item::Impl(it) => &it.attrs,
        Item::Macro(it) => &it.attrs,
        Item::Mod(it) => &it.attrs,
        Item::Static(it) => &it.attrs,
        Item::Struct(it) => &it.attrs,
        Item::Trait(it) => &it.attrs,
        Item::TraitAlias(it) => &it.attrs,
        Item::Type(it) => &it.attrs,
        Item::Union(it) => &it.attrs,
        Item::Use(it) => &it.attrs,
        _ => &[],
    }
}

fn v<T: syn::spanned::Spanned>(rel_path: &str, item: &T, message: impl Into<String>) -> Violation {
    let line = item.span().start().line;
    Violation::with_layer(rel_path, line, message, layer_for_rel_path(rel_path))
}
