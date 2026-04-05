//! Expansion logic for the `#[purity(...)]` proc macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Error, ItemTrait, Receiver, Token, TraitItem,
};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let purity = parse_macro_input!(attr as PurityClass);
    let item_trait = parse_macro_input!(item as ItemTrait);

    if let Err(error) = validate_trait(&item_trait, purity) {
        return error.to_compile_error().into();
    }

    quote!(#item_trait).into()
}

#[derive(Clone, Copy)]
enum PurityClass {
    Pure,
    ReadOnly,
    Effectful,
}

impl PurityClass {
    fn macro_form(self) -> &'static str {
        match self {
            Self::Pure => "#[purity(pure)]",
            Self::ReadOnly => "#[purity(read_only)]",
            Self::Effectful => "#[purity(effectful)]",
        }
    }
}

impl Parse for PurityClass {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        if !input.is_empty() {
            let _ = input.parse::<Token![,]>()?;
            if !input.is_empty() {
                return Err(Error::new(
                    input.span(),
                    "expected a single purity class: pure, read_only, or effectful",
                ));
            }
        }

        match ident.to_string().as_str() {
            "pure" => Ok(Self::Pure),
            "read_only" => Ok(Self::ReadOnly),
            "effectful" => Ok(Self::Effectful),
            _ => Err(Error::new(
                ident.span(),
                "expected one of: pure, read_only, effectful",
            )),
        }
    }
}

fn validate_trait(item_trait: &ItemTrait, purity: PurityClass) -> syn::Result<()> {
    for item in &item_trait.items {
        let TraitItem::Fn(method) = item else {
            continue;
        };

        let Some(receiver) = method.sig.receiver() else {
            continue;
        };

        validate_receiver(receiver, purity)?;
    }

    Ok(())
}

fn validate_receiver(receiver: &Receiver, purity: PurityClass) -> syn::Result<()> {
    match purity {
        PurityClass::Effectful => Ok(()),
        PurityClass::Pure | PurityClass::ReadOnly => {
            if receiver.reference.is_none() {
                return Err(Error::new_spanned(
                    receiver,
                    format!(
                        "{} does not allow by-value receivers; use `&self` or split the effectful method into a separate trait",
                        purity.macro_form()
                    ),
                ));
            }

            if receiver.mutability.is_some() {
                return Err(Error::new_spanned(
                    receiver,
                    format!(
                        "{} does not allow `&mut self`; split mutable/runtime behavior into a separate trait",
                        purity.macro_form()
                    ),
                ));
            }

            match &receiver.ty.as_ref() {
                syn::Type::Reference(_) => Ok(()),
                _ => Err(Error::new_spanned(
                    receiver,
                    format!(
                        "{} only allows shared receivers such as `&self`",
                        purity.macro_form()
                    ),
                )),
            }
        }
    }
}
