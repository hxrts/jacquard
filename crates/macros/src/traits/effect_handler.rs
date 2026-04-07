//! Expansion logic for the `#[effect_handler]` proc macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemImpl};

pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = parse_macro_input!(item as ItemImpl);
    let trait_path = match &item_impl.trait_ {
        | Some((_, path, _)) => path.clone(),
        | None => {
            return syn::Error::new_spanned(
                &item_impl.self_ty,
                "#[effect_handler] can only be applied to trait impls",
            )
            .to_compile_error()
            .into();
        },
    };

    let mut item_impl = item_impl;
    let self_ty = item_impl.self_ty.clone();
    let generics = item_impl.generics.clone();
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    item_impl.items.push(parse_quote! {
        fn __jacquard_handler_marker(
            &self,
        ) -> ::jacquard_traits::__private::HandlerToken<Self, dyn #trait_path>
        where
            Self: Sized,
        {
            ::jacquard_traits::__private::HandlerToken(::core::marker::PhantomData)
        }
    });

    let expanded = quote! {
        #item_impl

        impl #impl_generics ::jacquard_traits::__private::HandlerDefinition<dyn #trait_path> for #self_ty #where_clause {}
    };

    expanded.into()
}
