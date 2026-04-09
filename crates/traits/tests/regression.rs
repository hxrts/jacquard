//! Regression tests for jacquard-traits.
//!
//! This test binary collects regression test modules that guard against
//! previously identified defects or policy violations re-emerging. Each
//! sub-module focuses on one narrow behavioral or compile-time invariant.
//!
//! Modules included here:
//! - `domain_separation` — verifies that tagged hashing separates ambiguous
//!   domain/payload pairs so collisions cannot occur across domains.
//! - `effect_handler_enforcement` — verifies that the `#[effect_handler]`
//!   attribute is required; implementations without it fail to compile.
//! - `route_identity_immutability` — verifies that routing engines cannot
//!   mutate router-owned canonical identity fields during maintenance.

#[path = "regression/domain_separation.rs"]
mod domain_separation;

#[path = "regression/effect_handler_enforcement.rs"]
mod effect_handler_enforcement;

#[path = "regression/route_identity_immutability.rs"]
mod route_identity_immutability;
