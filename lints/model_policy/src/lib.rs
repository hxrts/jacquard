//! Dylint entry point for Jacquard workspace policy checks.
//!
//! These lints cover explicit workspace policy that should stay out of generic
//! proc macros, such as annotation requirements driven by naming and ownership
//! conventions, shared-crate vocabulary boundaries, and workspace-wide code
//! style rules like the long-block ceiling.

#![feature(rustc_private)]
#![deny(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_errors;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod handle_like;
mod long_block;
mod long_file;
mod shared_boundary;
mod source_scan;
mod world_error;

dylint_linting::dylint_library!();

use rustc_lint::LintStore;
use rustc_session::Session;

#[allow(unsafe_code)]
#[expect(clippy::no_mangle_with_rust_abi)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &Session, lint_store: &mut LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        handle_like::HANDLE_LIKE_MUST_USE,
        long_block::LONG_BLOCK,
        long_file::LONG_FILE,
        shared_boundary::SHARED_PRIVATE_BOUNDARY,
        world_error::WORLD_EXTENSION_ERROR_PURITY,
    ]);
    lint_store.register_late_pass(|_| Box::new(handle_like::HandleLikeMustUse));
    lint_store.register_late_pass(|_| Box::new(long_block::LongBlock));
    lint_store.register_late_pass(|_| Box::new(long_file::LongFile::default()));
    lint_store.register_late_pass(|_| Box::new(shared_boundary::SharedPrivateBoundary));
    lint_store.register_late_pass(|_| Box::new(world_error::WorldExtensionErrorPurity::default()));
}
