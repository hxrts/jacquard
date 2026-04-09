//! Dylint entry point for trait-purity policy checks.
//!
//! Companion to `cargo xtask check trait-purity`. The xtask provides the
//! stable fast path via text scans on staged files; this crate provides
//! AST-aware linting for the same policy under nightly `cargo dylint`, giving
//! accurate per-trait span diagnostics and correct handling of macro-generated
//! trait definitions.
//!
//! The single registered lint pass (`TRAIT_PURITY`) requires every public
//! trait definition to carry a `#[purity(...)]` or `#[effect_trait]` marker
//! in source. Internal support traits (`Sealed`, `EffectDefinition`,
//! `HandlerDefinition`) are exempt from this requirement.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

mod lint;
mod source_scan;
