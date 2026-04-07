//! Dylint entry point for Jacquard shared-model policy checks.
//!
//! These lints cover explicit workspace policy that should stay out of generic
//! proc macros, such as annotation requirements driven by naming and ownership
//! conventions across files.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod handle_like;
mod source_scan;
