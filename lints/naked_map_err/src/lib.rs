//! Dylint entry point for the naked map_err policy check.
//!
//! Any `.map_err(|_| RouteError::Runtime(...))` call where the original error
//! is discarded (closure arg is `_`) must be replaced with the appropriate
//! `ResultExt` method: `.storage_invalid()`, `.choreography_failed()`, or
//! `.maintenance_failed()`. Discarding the original error makes failures
//! harder to debug and violates the workspace error-mapping convention.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod lint;

pub use lint::NAKED_MAP_ERR_ROUTE_ERROR;
