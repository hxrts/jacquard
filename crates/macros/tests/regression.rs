//! Regression test harness for `jacquard-macros`.
//!
//! Uses `trybuild` to run all `.rs` files under `tests/regression/ui/` as
//! compile-fail tests, verifying that invalid or mis-annotated proc-macro
//! usages produce the expected compiler errors.
//!
//! Each UI test file exercises one specific rejection rule, such as:
//! - `#[id_type]` applied to a multi-field struct
//! - `#[purity(pure)]` applied to a trait with a `&mut self` method
//! - `#[purity(read_only)]` applied to a trait with a by-value receiver
//! - `#[purity(...)]` given an unrecognized purity mode
//! - `#[public_model]` applied to a struct with a `bool` field
//!
//! UI test files are not subject to `//!` header requirements.

#[test]
fn macro_regressions() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/regression/ui/*.rs");
}
