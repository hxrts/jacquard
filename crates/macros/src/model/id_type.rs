//! Expansion logic for the `#[id_type]` proc macro.
//!
//! `#[id_type]` wraps a single-field tuple struct as an opaque, canonical
//! identifier. It accepts no arguments and rejects generic structs. The macro:
//!
//! - Applies `repr(transparent)` so the wrapper has the same layout as its
//!   inner field.
//! - Injects the canonical id-type derives: `Clone`, `Copy`, `Debug`,
//!   `Default`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Serialize`,
//!   `Deserialize`.
//! - Generates `const fn new(value: <FieldType>) -> Self` and `const fn
//!   get(self) -> <FieldType>` constructors.
//!
//! Parsing and attribute injection are delegated to `support::parsing` and
//! `support::attrs` respectively.

use proc_macro::TokenStream;
use quote::quote;

use crate::support::{
    ensure_derive, ensure_repr_transparent, id_type_derives, parse_single_field_tuple_struct,
    tuple_struct_field_type,
};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[id_type] does not take arguments",
        )
        .to_compile_error()
        .into();
    }

    let mut item_struct = match parse_single_field_tuple_struct(item.into(), "id_type") {
        Ok(item_struct) => item_struct,
        Err(error) => return error.to_compile_error().into(),
    };
    let ident = item_struct.ident.clone();
    let field_ty = tuple_struct_field_type(&item_struct);

    if let Err(error) = ensure_repr_transparent(&mut item_struct.attrs) {
        return error.to_compile_error().into();
    }
    if let Err(error) = ensure_derive(&mut item_struct.attrs, &id_type_derives()) {
        return error.to_compile_error().into();
    }

    let expanded = quote! {
        #item_struct

        impl #ident {
            pub const fn new(value: #field_ty) -> Self {
                Self(value)
            }

            pub const fn get(self) -> #field_ty {
                self.0
            }
        }
    };

    expanded.into()
}
