#![forbid(unsafe_code)]

mod model;
mod support;
mod traits;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn id_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::id_type::expand(attr, item)
}

#[proc_macro_attribute]
pub fn bounded_value(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::bounded_value::expand(attr, item)
}

#[proc_macro_attribute]
pub fn must_use_handle(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::must_use_handle::expand(attr, item)
}

#[proc_macro_attribute]
pub fn public_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    model::public_model::expand(attr, item)
}

#[proc_macro_attribute]
pub fn purity(attr: TokenStream, item: TokenStream) -> TokenStream {
    traits::purity::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    traits::effect_trait::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    traits::effect_handler::expand(attr, item)
}
