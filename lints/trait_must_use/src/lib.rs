//! Dylint entry point for the trait-method `#[must_use]` policy check.
//!
//! Public trait methods that return a meaningful value (not `()` and not
//! `Result<(), _>`) must carry `#[must_use]` or a `#[must_use = "..."]`
//! attribute. Without it, callers can silently discard results in ways that
//! are hard to audit.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod lint;

pub use lint::TRAIT_METHOD_MUST_USE;
