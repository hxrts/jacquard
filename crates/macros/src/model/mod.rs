//! Proc macros that shape shared model and handle types.
//!
//! This module contains four attribute macros:
//!
//! - `id_type` — wraps a single-field newtype tuple struct as an opaque
//!   identifier. Applies canonical derives (`Clone`, `Copy`, `Debug`, `Eq`,
//!   `Ord`, `Hash`, `Serialize`, `Deserialize`, etc.) and `repr(transparent)`,
//!   and generates `new`/`get` const constructors.
//! - `bounded_value` — wraps a single-field newtype tuple struct as a
//!   range-bounded numeric value. Requires a `max = <expr>` argument, applies
//!   the same derive set as `id_type`, and generates a `MAX` constant and a
//!   checked `new` constructor that returns `Option<Self>`.
//! - `must_use_handle` — applies a descriptive `#[must_use]` annotation to a
//!   struct or enum that represents a routing handle or lease.
//! - `public_model` — applies canonical model derives (`Clone`, `Debug`, `Eq`,
//!   `Serialize`, `Deserialize`) to a struct or enum and validates that no
//!   forbidden field types appear.

pub(crate) mod bounded_value;
pub(crate) mod id_type;
pub(crate) mod must_use_handle;
pub(crate) mod public_model;
