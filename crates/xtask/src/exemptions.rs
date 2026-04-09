//! Centralized exemption tables for xtask policy checks.
//!
//! Collects every place where a style-guide rule has a documented, intentional
//! exception so that exemptions are auditable in code review rather than
//! scattered across individual check modules.
//!
//! Tables included:
//! - `BARE_PRIMITIVES_EXEMPT_PATHS` — crates and path prefixes whose public
//!   structs are allowed to use `usize` (test infrastructure and reference
//!   crates with justified justification).
//! - `STYLE_GUIDE_EXCEPTIONS` — per-function `long-block-exception` records for
//!   bodies that legitimately exceed the 60-line cap.
//! - `OWNERSHIP_PERMITS` — named patterns permitted by design with a brief
//!   justification string, referenced by the ownership-invariants check.

/// Crates and paths exempt from bare-primitive-type enforcement.
/// These are test infrastructure or have documented justification.
#[allow(dead_code)]
pub const BARE_PRIMITIVES_EXEMPT_PATHS: &[&str] = &[
    "crates/mem-link-profile/src/", // test infrastructure; mirrors reference-client
    "crates/reference-client/src/", // test infrastructure
    "/tests/",
    "/benches/",
];

/// Functions/types with explicit proc-macro exceptions for style-guide rules.
/// Format: "crate::path::function" -> "exception-type: reason"
#[allow(dead_code)]
pub const STYLE_GUIDE_EXCEPTIONS: &[(&str, &str)] = &[
    (
        "jacquard_pathway::routing_invariants::run",
        "long-block-exception: rule coordination workflow",
    ),
    // Add as discovered via code review
];

/// Ownership patterns that are "permitted by design" for documented reasons.
/// Maps pattern name to a brief justification.
#[allow(dead_code)]
pub const OWNERSHIP_PERMITS: &[(&str, &str)] = &[(
    "mem-link-profile-test-only",
    "test infrastructure crate; mirrors reference-client",
)];
