//! Shared helpers for parsing, attributes, derives, and validation.
//!
//! This module is private to the `jacquard-macros` crate and provides the
//! building blocks used by all proc-macro expand functions:
//!
//! - `attrs` — helpers for applying or normalizing Rust attributes, including
//!   `ensure_derive`, `ensure_must_use`, and `ensure_repr_transparent`.
//! - `derives` — canonical derive lists for `#[id_type]` and `#[public_model]`
//!   annotated types, shared across the model macros.
//! - `parsing` — input parsing helpers for single-field tuple structs and the
//!   `max = <expr>` argument form used by `#[bounded_value]`.
//! - `validation` — rejection logic for forbidden field types in public model
//!   structs and enums (raw `bool`, floats, `usize`, wall-clock time types).

mod attrs;
mod derives;
mod parsing;
mod validation;

pub(crate) use attrs::{ensure_derive, ensure_must_use, ensure_repr_transparent};
pub(crate) use derives::{id_type_derives, public_model_derives};
pub(crate) use parsing::{
    parse_max_expr, parse_single_field_tuple_struct, tuple_struct_field_type,
};
pub(crate) use validation::{validate_public_model_enum, validate_public_model_struct};
