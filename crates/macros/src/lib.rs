#![forbid(unsafe_code)]

mod bounded_value_macro;
mod effect_handler_macro;
mod effect_trait_macro;
mod id_type_macro;
mod must_use_handle_macro;
mod public_model_macro;
mod purity_macro;
mod support;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn id_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    id_type_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn bounded_value(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_value_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn must_use_handle(attr: TokenStream, item: TokenStream) -> TokenStream {
    must_use_handle_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn public_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    public_model_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn purity(attr: TokenStream, item: TokenStream) -> TokenStream {
    purity_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    effect_trait_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    effect_handler_macro::expand(attr, item)
}
