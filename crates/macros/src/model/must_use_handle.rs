//! Expansion logic for the `#[must_use_handle]` proc macro.
//!
//! `#[must_use_handle]` applies a descriptive `#[must_use]` annotation to a
//! struct or enum that represents a routing handle or lease. It accepts no
//! arguments and may be applied to any struct or enum item.
//!
//! The generated `#[must_use]` message follows the form:
//!   "dropping `<TypeName>` discards a routing handle or lease without making
//!    that choice explicit"
//!
//! This ensures callers are warned at compile time if a handle type is dropped
//! without being explicitly consumed or released. The macro delegates
//! attribute injection to `support::attrs::ensure_must_use` and rejects
//! application to non-struct and non-enum items with a compile error.

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{Item, LitStr};

use crate::support::ensure_must_use;

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[must_use_handle] does not take arguments",
        )
        .to_compile_error()
        .into();
    }

    let item = syn::parse_macro_input!(item as Item);

    match item {
        | Item::Struct(mut item_struct) => {
            let message = LitStr::new(
                &format!(
                    "dropping `{}` discards a routing handle or lease without making that choice explicit",
                    item_struct.ident
                ),
                item_struct.ident.span(),
            );
            ensure_must_use(&mut item_struct.attrs, &message.value());
            item_struct.into_token_stream().into()
        },
        | Item::Enum(mut item_enum) => {
            let message = LitStr::new(
                &format!(
                    "dropping `{}` discards a routing handle or lease without making that choice explicit",
                    item_enum.ident
                ),
                item_enum.ident.span(),
            );
            ensure_must_use(&mut item_enum.attrs, &message.value());
            item_enum.into_token_stream().into()
        },
        | other => syn::Error::new_spanned(
            other,
            "#[must_use_handle] can only be applied to structs or enums",
        )
        .to_compile_error()
        .into(),
    }
}
