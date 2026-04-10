//! Helpers for applying and normalizing Rust attributes.
//!
//! Provides three functions used by the model and trait proc macros to safely
//! inject or verify attributes on the item being annotated:
//!
//! - `ensure_derive` — inspects existing `#[derive(...)]` attributes and
//!   injects a single additional `#[derive(...)]` for any required paths that
//!   are missing. Path comparison is whitespace-normalized so `serde ::
//!   Serialize` and `serde::Serialize` are treated as the same path.
//! - `ensure_must_use` — injects `#[must_use = "<message>"]` when no
//!   `#[must_use]` attribute is already present on the item.
//! - `ensure_repr_transparent` — injects `#[repr(transparent)]` when no
//!   `#[repr(transparent)]` is already present, which is required by both
//!   `#[id_type]` and `#[bounded_value]` to guarantee ABI compatibility.

use std::collections::BTreeSet;

use quote::ToTokens;
use syn::{parse_quote, punctuated::Punctuated, Attribute, Path, Token};

pub(crate) fn ensure_derive(attrs: &mut Vec<Attribute>, required: &[Path]) -> syn::Result<()> {
    let mut existing = BTreeSet::new();

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("derive")) {
        let parser = Punctuated::<Path, Token![,]>::parse_terminated;
        for path in attr.parse_args_with(parser)? {
            existing.insert(normalize_path(&path));
        }
    }

    let missing: Vec<Path> = required
        .iter()
        .filter(|path| !existing.contains(&normalize_path(path)))
        .cloned()
        .collect();

    if !missing.is_empty() {
        attrs.push(parse_quote!(#[derive(#(#missing),*)]));
    }

    Ok(())
}

pub(crate) fn ensure_must_use(attrs: &mut Vec<Attribute>, message: &str) {
    if attrs.iter().any(|attr| attr.path().is_ident("must_use")) {
        return;
    }

    attrs.push(parse_quote!(#[must_use = #message]));
}

pub(crate) fn ensure_repr_transparent(attrs: &mut Vec<Attribute>) -> syn::Result<()> {
    // try_fold instead of any() propagates parse errors from each
    // #[repr] attribute. The early-return short-circuits once
    // transparent is found.
    let has_transparent = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("repr"))
        .try_fold(false, |found, attr| -> syn::Result<bool> {
            if found {
                return Ok(true);
            }

            let parser = Punctuated::<Path, Token![,]>::parse_terminated;
            let reprs = attr.parse_args_with(parser)?;
            Ok(reprs.iter().any(|repr| repr.is_ident("transparent")))
        })?;

    if !has_transparent {
        attrs.push(parse_quote!(#[repr(transparent)]));
    }

    Ok(())
}

// to_token_stream() emits spaces around `::` segments; strip them so
// path strings compare equal regardless of whitespace.
fn normalize_path(path: &Path) -> String {
    path.to_token_stream().to_string().replace(' ', "")
}
