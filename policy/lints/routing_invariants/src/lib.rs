//! Dylint entry point for routing-invariant policy checks.
//!
//! These lints mirror the stable `cargo xtask check routing-invariants` lane
//! with compiler-hosted source scans under nightly. Where the xtask performs
//! fast text-only scans on staged files, this crate runs the same checks with
//! full HIR context, ensuring correctness under macro expansion and allowing
//! per-item span diagnostics rather than raw line numbers.
//!
//! The lint passes enforce fail-closed mutation ordering, typed time-wrapper
//! discipline, storage key namespacing, and a set of structural patterns that
//! have historically been sites of routing correctness bugs. See `lint.rs` for
//! the full list of registered passes and what each one detects.

#![feature(rustc_private)]
#![deny(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod source_scan;
mod lint;
