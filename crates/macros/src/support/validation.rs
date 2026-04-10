//! Validation helpers for public-model macros.
//!
//! Rejects forbidden field types in `#[public_model]`-annotated structs and
//! enums at compile time, enforcing Jacquard's determinism model constraint:
//!
//! - `validate_public_model_struct` — iterates all named and unnamed fields of
//!   a struct and rejects any forbidden type.
//! - `validate_public_model_enum` — iterates all variant fields across the enum
//!   and applies the same rejection logic.
//!
//! Forbidden types: raw `bool` (use an enum), `f32`, `f64` (no floating-point
//! in deterministic model types), `usize`, `isize` (use fixed-width integers),
//! `std::time::Instant`, `std::time::SystemTime` (use the Jacquard typed time
//! model), and `Option<T>` where `T` is itself a forbidden type.

use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, Error, Fields, GenericArgument, ItemEnum, ItemStruct, Path,
    PathArguments, Type, TypePath,
};

pub(crate) fn validate_public_model_struct(item_struct: &ItemStruct) -> syn::Result<()> {
    reject_bad_field_types(&item_struct.fields, "public_model")
}

pub(crate) fn validate_public_model_enum(item_enum: &ItemEnum) -> syn::Result<()> {
    for variant in &item_enum.variants {
        reject_bad_field_types(&variant.fields, "public_model")?;
    }

    Ok(())
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
            return Some("`std::time::Instant`; use the typed Jacquard time model instead")
        }
        "std::time::SystemTime" => {
            return Some("`std::time::SystemTime`; use the typed Jacquard time model instead")
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

// Recurse into bad_type_reason so that Option<bool>, Option<f64>, and
// other Option-wrapped forbidden types are caught transitively, not
// just bare forbidden types at the top level.
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
