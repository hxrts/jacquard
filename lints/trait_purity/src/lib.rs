//! Dylint entry point for trait-purity policy checks.
//!
//! Companion to `cargo xtask check trait-purity`. The xtask provides the
//! stable fast path; this crate provides AST-aware linting for the same policy
//! under nightly `cargo dylint`.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod lint;
mod source_scan;
