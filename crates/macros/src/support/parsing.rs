//! Parsing helpers for proc-macro inputs.

use proc_macro2::TokenStream;
use syn::{parse::Parser, Error, Expr, Fields, ItemStruct, Type};

pub(crate) fn parse_single_field_tuple_struct(
    item: TokenStream,
    macro_name: &str,
) -> syn::Result<ItemStruct> {
    let item_struct = syn::parse2::<ItemStruct>(item)?;
    validate_single_field_tuple_struct(&item_struct, macro_name)?;
    Ok(item_struct)
}

pub(crate) fn tuple_struct_field_type(item_struct: &ItemStruct) -> Type {
    match &item_struct.fields {
        | Fields::Unnamed(fields) => fields.unnamed[0].ty.clone(),
        | _ => unreachable!("validated before extracting tuple struct field type"),
    }
}

pub(crate) fn parse_max_expr(attr: TokenStream) -> syn::Result<Expr> {
    let mut max_expr = None;
    let parser = syn::meta::parser(|meta| {
        if !meta.path.is_ident("max") {
            return Err(meta.error("expected `max = ...`"));
        }

        max_expr = Some(meta.value()?.parse()?);
        Ok(())
    });

    parser.parse2(attr)?;
    max_expr.ok_or_else(|| {
        Error::new(proc_macro2::Span::call_site(), "expected `max = ...`")
    })
}

fn validate_single_field_tuple_struct(
    item_struct: &ItemStruct,
    macro_name: &str,
) -> syn::Result<()> {
    if !item_struct.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &item_struct.generics,
            format!("{macro_name} does not support generic tuple structs"),
        ));
    }

    match &item_struct.fields {
        | Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(()),
        | _ => Err(Error::new_spanned(
            &item_struct.fields,
            format!("{macro_name} requires a tuple struct with exactly one field"),
        )),
    }
}
