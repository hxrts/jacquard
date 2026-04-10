//! Expansion logic for the `#[public_model]` proc macro.
//!
//! `#[public_model]` applies canonical model derives to a struct or enum and
//! rejects forbidden field types at compile time. It accepts no arguments and
//! may be applied to any struct or enum item.
//!
//! The injected derives are: `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`,
//! `Deserialize`. Callers may add additional derives (e.g. `Copy`, `Ord`,
//! `Hash`) on the annotated item directly; existing derives are preserved.
//!
//! Forbidden field types include raw `bool`, `f32`, `f64`, `usize`, `isize`,
//! `std::time::Instant`, `std::time::SystemTime`, and `Option<T>` where `T`
//! is any of the above. Validation is delegated to `support::validation` and
//! is applied before any derive injection.

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::Item;

use crate::support::{
    ensure_derive, public_model_derives, validate_public_model_enum, validate_public_model_struct,
};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[public_model] does not take arguments",
        )
        .to_compile_error()
        .into();
    }

    let item = syn::parse_macro_input!(item as Item);

    match item {
        Item::Struct(mut item_struct) => {
            if let Err(error) = validate_public_model_struct(&item_struct) {
                return error.to_compile_error().into();
            }
            if let Err(error) = ensure_derive(&mut item_struct.attrs, &public_model_derives()) {
                return error.to_compile_error().into();
            }

            item_struct.into_token_stream().into()
        }
        Item::Enum(mut item_enum) => {
            if let Err(error) = validate_public_model_enum(&item_enum) {
                return error.to_compile_error().into();
            }
            if let Err(error) = ensure_derive(&mut item_enum.attrs, &public_model_derives()) {
                return error.to_compile_error().into();
            }

            item_enum.into_token_stream().into()
        }
        other => syn::Error::new_spanned(
            other,
            "#[public_model] can only be applied to structs or enums",
        )
        .to_compile_error()
        .into(),
    }
}
