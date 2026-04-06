//! Canonical derive lists used by Jacquard proc macros.

use syn::{parse_quote, Path};

pub(crate) fn id_type_derives() -> [Path; 11] {
    [
        parse_quote!(Clone),
        parse_quote!(Copy),
        parse_quote!(Debug),
        parse_quote!(Default),
        parse_quote!(PartialEq),
        parse_quote!(Eq),
        parse_quote!(PartialOrd),
        parse_quote!(Ord),
        parse_quote!(Hash),
        parse_quote!(Serialize),
        parse_quote!(Deserialize),
    ]
}

pub(crate) fn public_model_derives() -> [Path; 6] {
    [
        parse_quote!(Clone),
        parse_quote!(Debug),
        parse_quote!(PartialEq),
        parse_quote!(Eq),
        parse_quote!(Serialize),
        parse_quote!(Deserialize),
    ]
}
