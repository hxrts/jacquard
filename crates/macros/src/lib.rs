#![forbid(unsafe_code)]

mod effect_handler_macro;
mod effect_trait_macro;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn effect_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    effect_trait_macro::expand(attr, item)
}

#[proc_macro_attribute]
pub fn effect_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    effect_handler_macro::expand(attr, item)
}
