//! Expansion logic for the `#[bounded_value]` proc macro.

use proc_macro::TokenStream;
use quote::quote;

use crate::support::{
    ensure_derive, ensure_repr_transparent, id_type_derives, parse_max_expr,
    parse_single_field_tuple_struct, tuple_struct_field_type,
};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let max = match parse_max_expr(attr.into()) {
        Ok(max) => max,
        Err(error) => return error.to_compile_error().into(),
    };

    let mut item_struct = match parse_single_field_tuple_struct(item.into(), "bounded_value") {
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
            pub const MAX: #field_ty = #max;

            pub const fn new(value: #field_ty) -> ::core::option::Option<Self> {
                if value <= Self::MAX {
                    ::core::option::Option::Some(Self(value))
                } else {
                    ::core::option::Option::None
                }
            }

            pub const fn get(self) -> #field_ty {
                self.0
            }
        }
    };

    expanded.into()
}
