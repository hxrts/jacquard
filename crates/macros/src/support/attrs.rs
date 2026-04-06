//! Helpers for applying and normalizing Rust attributes.

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

fn normalize_path(path: &Path) -> String {
    path.to_token_stream().to_string().replace(' ', "")
}
