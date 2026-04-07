//! Dylint entry point for routing-invariant policy checks.
//!
//! These lints mirror the stable `cargo xtask check routing-invariants` lane
//! with compiler-hosted source scans under nightly.

#![feature(rustc_private)]
#![deny(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod source_scan;
mod lint;
