//! Dylint entry point for trait-purity policy checks.
//!
//! Companion to `scripts/check/trait-purity.sh`. The script provides a fast
//! grep-based structural check on stable; this crate provides AST-aware linting
//! for the same policy under nightly `cargo-dylint`.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod lint;
mod source_scan;
