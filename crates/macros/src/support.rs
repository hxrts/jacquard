//! Shared parsing and attribute utilities for Contour proc macros.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parser, parse_quote, punctuated::Punctuated, AngleBracketedGenericArguments, Attribute,
    Error, Expr, Fields, GenericArgument, ItemEnum, ItemStruct, Path, PathArguments, Token, Type,
    TypePath,
};

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

pub(crate) fn id_type_derives() -> [Path; 11] {
    [
        parse_quote!(Clone),
        parse_quote!(Copy),
        parse_quote!(Debug),
        parse_quote!(Default),
        parse_quote!(PartialEq),
        parse_quote!(Eq),
        parse_quote!(PartialOrd),
        parse_quote!(Ord),
        parse_quote!(Hash),
        parse_quote!(Serialize),
        parse_quote!(Deserialize),
    ]
}

pub(crate) fn public_model_derives() -> [Path; 6] {
    [
        parse_quote!(Clone),
        parse_quote!(Debug),
        parse_quote!(PartialEq),
        parse_quote!(Eq),
        parse_quote!(Serialize),
        parse_quote!(Deserialize),
    ]
}

pub(crate) fn parse_single_field_tuple_struct(
    item: TokenStream,
    macro_name: &str,
) -> syn::Result<ItemStruct> {
    let item_struct = syn::parse2::<ItemStruct>(item)?;
    validate_single_field_tuple_struct(&item_struct, macro_name)?;
    Ok(item_struct)
}

pub(crate) fn validate_single_field_tuple_struct(
    item_struct: &ItemStruct,
    macro_name: &str,
) -> syn::Result<()> {
    if !item_struct.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &item_struct.generics,
            format!("{macro_name} does not support generic tuple structs"),
        ));
    }

    match &item_struct.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(()),
        _ => Err(Error::new_spanned(
            &item_struct.fields,
            format!("{macro_name} requires a tuple struct with exactly one field"),
        )),
    }
}

pub(crate) fn tuple_struct_field_type(item_struct: &ItemStruct) -> Type {
    match &item_struct.fields {
        Fields::Unnamed(fields) => fields.unnamed[0].ty.clone(),
        _ => unreachable!("validated before extracting tuple struct field type"),
    }
}

pub(crate) fn validate_public_model_struct(item_struct: &ItemStruct) -> syn::Result<()> {
    reject_bad_field_types(&item_struct.fields, "public_model")
}

pub(crate) fn validate_public_model_enum(item_enum: &ItemEnum) -> syn::Result<()> {
    for variant in &item_enum.variants {
        reject_bad_field_types(&variant.fields, "public_model")?;
    }

    Ok(())
}

pub(crate) fn parse_max_expr(attr: TokenStream) -> syn::Result<Expr> {
    let mut max_expr = None;
    let parser = syn::meta::parser(|meta| {
        if !meta.path.is_ident("max") {
            return Err(meta.error("expected `max = ...`"));
        }

        max_expr = Some(meta.value()?.parse()?);
        Ok(())
    });

    parser.parse2(attr)?;
    max_expr.ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "expected `max = ...`"))
}

fn reject_bad_field_types(fields: &Fields, macro_name: &str) -> syn::Result<()> {
    match fields {
        Fields::Named(fields) => {
            for field in &fields.named {
                reject_bad_type(&field.ty, macro_name)?;
            }
        }
        Fields::Unnamed(fields) => {
            for field in &fields.unnamed {
                reject_bad_type(&field.ty, macro_name)?;
            }
        }
        Fields::Unit => {}
    }

    Ok(())
}

fn reject_bad_type(ty: &Type, macro_name: &str) -> syn::Result<()> {
    if let Some(reason) = bad_type_reason(ty) {
        return Err(Error::new_spanned(
            ty,
            format!("{macro_name} does not allow {reason} in public model fields"),
        ));
    }

    Ok(())
}

fn bad_type_reason(ty: &Type) -> Option<&'static str> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let normalized = normalize_path(path);
    let leaf = path.segments.last()?.ident.to_string();

    match normalized.as_str() {
        "std::time::Instant" => {
            return Some("`std::time::Instant`; use the typed Contour time model instead")
        }
        "std::time::SystemTime" => {
            return Some("`std::time::SystemTime`; use the typed Contour time model instead")
        }
        _ => {}
    }

    match leaf.as_str() {
        "bool" => Some("raw `bool`; model public semantics with an enum instead"),
        "f32" => Some("`f32`; floating-point types are not allowed in deterministic model types"),
        "f64" => Some("`f64`; floating-point types are not allowed in deterministic model types"),
        "usize" => Some("`usize`; use a fixed-width integer type instead"),
        "isize" => Some("`isize`; use a fixed-width integer type instead"),
        "Option" => option_inner_reason(path),
        _ => None,
    }
}

fn option_inner_reason(path: &Path) -> Option<&'static str> {
    let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
        &path.segments.last()?.arguments
    else {
        return None;
    };

    let inner = args.iter().find_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })?;

    bad_type_reason(inner)
}

fn normalize_path(path: &Path) -> String {
    path.to_token_stream().to_string().replace(' ', "")
}
