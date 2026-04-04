//! Expansion logic for the `#[effect_trait]` proc macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemTrait};

pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_trait = parse_macro_input!(item as ItemTrait);
    let ident = item_trait.ident.clone();

    item_trait
        .supertraits
        .push(parse_quote!(::core::marker::Send));
    item_trait
        .supertraits
        .push(parse_quote!(::core::marker::Sync));
    item_trait.supertraits.push(parse_quote!('static));
    item_trait.items.push(parse_quote! {
        #[doc(hidden)]
        fn __contour_handler_marker(
            &self,
        ) -> ::contour_traits::__private::HandlerToken<Self, dyn #ident>
        where
            Self: Sized;
    });

    let expanded = quote! {
        #item_trait

        impl ::contour_traits::__private::EffectDefinition for dyn #ident {}
    };

    expanded.into()
}
