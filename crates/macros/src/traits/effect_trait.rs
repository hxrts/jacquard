//! Expansion logic for the `#[effect_trait]` proc macro.
//!
//! `#[effect_trait]` stamps a trait as an effect surface and injects the
//! required plumbing for the effect-handler linkage mechanism. It accepts no
//! arguments and is applied to trait definitions.
//!
//! The macro injects three things:
//!
//! - `Send + Sync + 'static` supertraits, required by the effect system so that
//!   trait objects can be stored and passed across thread boundaries.
//! - A hidden `__jacquard_handler_marker` required method that returns
//!   `HandlerToken<Self, dyn TraitName>`. The method is `#[doc(hidden)]` and
//!   `where Self: Sized`, preserving object safety while acting as a sealed
//!   compile-time witness that a concrete handler is linked to the trait.
//! - A `EffectDefinition` impl for `dyn TraitName`, which registers the trait
//!   object with the effect system.

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
    // Inject a sealed marker method that binds the concrete type to the
    // trait at compile time. `where Self: Sized` keeps the trait
    // object-safe; `#[doc(hidden)]` hides it from public docs.
    item_trait.items.push(parse_quote! {
        #[doc(hidden)]
        fn __jacquard_handler_marker(
            &self,
        ) -> ::jacquard_traits::__private::HandlerToken<Self, dyn #ident>
        where
            Self: Sized;
    });

    let expanded = quote! {
        #item_trait

        impl ::jacquard_traits::__private::EffectDefinition for dyn #ident {}
    };

    expanded.into()
}
