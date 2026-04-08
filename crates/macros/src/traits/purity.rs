//! Expansion logic for the `#[purity(...)]` proc macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Error, ItemTrait, Receiver, TraitItem,
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
            | Self::Pure => "#[purity(pure)]",
            | Self::ReadOnly => "#[purity(read_only)]",
            | Self::Effectful => "#[purity(effectful)]",
        }
    }
}

impl Parse for PurityClass {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        match ident.to_string().as_str() {
            | "pure" => Ok(Self::Pure),
            | "read_only" => Ok(Self::ReadOnly),
            | "effectful" => Ok(Self::Effectful),
            | _ => Err(Error::new_spanned(
                ident,
                "expected one of: `pure`, `read_only`, `effectful`",
            )),
        }
    }
}

fn validate_trait(item_trait: &ItemTrait, purity: PurityClass) -> syn::Result<()> {
    let methods: Vec<_> = item_trait
        .items
        .iter()
        .filter_map(|item| match item {
            | TraitItem::Fn(method) => Some(method),
            | _ => None,
        })
        .collect();

    match purity {
        | PurityClass::Pure => {
            for method in methods {
                reject_disallowed_receiver(method.sig.receiver(), purity)?;
            }
        },
        | PurityClass::ReadOnly => {
            for method in methods {
                reject_disallowed_receiver(method.sig.receiver(), purity)?;
            }
        },
        | PurityClass::Effectful => {
            if methods.is_empty() {
                return Ok(());
            }

            // effectful requires at least one &mut self method. A trait
            // with only &self methods belongs in read_only or pure.
            if !methods.iter().any(|method| {
                matches!(
                    method.sig.receiver(),
                    Some(Receiver { mutability: Some(_), .. })
                )
            }) {
                return Err(Error::new_spanned(
                    &item_trait.ident,
                    format!(
                        "{} requires at least one `&mut self` method",
                        purity.macro_form()
                    ),
                ));
            }
        },
    }

    Ok(())
}

fn reject_disallowed_receiver(
    receiver: Option<&Receiver>,
    purity: PurityClass,
) -> syn::Result<()> {
    if let Some(receiver) = receiver {
        if receiver.mutability.is_some() {
            return Err(Error::new_spanned(
                receiver,
                format!(
                    "{} does not allow `&mut self`; split mutable/runtime behavior into a separate trait",
                    purity.macro_form()
                ),
            ));
        }

        if receiver.reference.is_none() {
            return Err(Error::new_spanned(
                receiver,
                format!(
                    "{} does not allow by-value receivers; use `&self` or split the effectful method into a separate trait",
                    purity.macro_form()
                ),
            ));
        }
    }

    Ok(())
}
