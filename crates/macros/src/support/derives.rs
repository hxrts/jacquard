//! Canonical derive lists used by Jacquard proc macros.
//!
//! Exports two functions that return the fixed sets of derives injected by the
//! model attribute macros:
//!
//! - `id_type_derives` — the full derive set for `#[id_type]` and
//!   `#[bounded_value]` types: `Clone`, `Copy`, `Debug`, `Default`,
//!   `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Serialize`,
//!   `Deserialize`. All eleven are required to give identifier and
//!   bounded-value newtypes the ergonomic surface expected across the
//!   workspace.
//! - `public_model_derives` — the smaller set for `#[public_model]` types:
//!   `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`. Callers
//!   may add extra derives (e.g. `Copy`, `Ord`) on the annotated item itself.

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
