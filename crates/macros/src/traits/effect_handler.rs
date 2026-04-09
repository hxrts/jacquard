//! Expansion logic for the `#[effect_handler]` proc macro.
//!
//! `#[effect_handler]` stamps a trait impl block as a concrete handler for
//! an effect trait. It may only be applied to trait impl blocks (not inherent
//! impls) and accepts no arguments.
//!
//! The macro injects two things into the annotated impl:
//!
//! - A hidden `__jacquard_handler_marker` method that returns a
//!   `HandlerToken<Self, dyn TraitPath>` backed by `PhantomData`. This method
//!   is `where Self: Sized` to preserve object safety and exists solely to
//!   prove the impl relationship at compile time with no runtime cost.
//! - A blanket `HandlerDefinition<dyn TraitPath>` impl for the concrete type,
//!   which allows the effect system to verify that a handler is registered for
//!   a given trait object at compile time.

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

    // PhantomData satisfies HandlerToken's type parameter. No runtime
    // data is held; this method exists only to prove the impl
    // relationship at compile time.
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
