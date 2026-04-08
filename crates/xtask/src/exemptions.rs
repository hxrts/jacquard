//! Centralized permitted style-guide exemptions (mock infrastructure, tests,
//! documented exceptions).

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
        "jacquard_mesh::routing_invariants::run",
        "long-block-exception: rule coordination workflow",
    ),
    // Add as discovered via code review
];

/// Ownership patterns that are "permitted by design" for documented reasons.
/// Maps pattern name to documentation file/section.
#[allow(dead_code)]
pub const OWNERSHIP_PERMITS: &[(&str, &str)] = &[(
    "mem-link-profile-test-only",
    "work/_impl.md#mem-link-profile-test-infrastructure",
)];
