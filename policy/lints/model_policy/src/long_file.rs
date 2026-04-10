//! Lint pass: source files must stay within a fixed line budget.
//!
//! Any file longer than `MAX_LONG_FILE_LINES` source lines errors at
//! lint time. The expected fix is to split the file into two or more
//! coherent files that separate concerns. An exception marker exists
//! as a last resort for cases where the file is genuinely one
//! cohesive unit that splitting would obscure. To use it, add a
//! `// long-file-exception: <reason>` comment within the first 20
//! lines. The reason text must be non-empty and must explain why
//! splitting would harm cohesion.

use std::collections::BTreeSet;

use rustc_errors::DiagDecorator;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, LintContext};

use crate::source_scan::source_file_contents;

/// Upper bound for a source file, in lines.
pub const MAX_LONG_FILE_LINES: usize = 1000;

/// How many top-of-file lines the exception marker may appear in.
const EXCEPTION_SCAN_LINES: usize = 20;

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Rejects source files longer than 1000 lines.
    ///
    /// ### Why is this bad?
    ///
    /// Long files bundle too many concerns, hurt navigation, and make
    /// code review harder. Jacquard's module layout favors small,
    /// focused files that can be understood on their own.
    ///
    /// ### How to fix
    ///
    /// Split the file into two or more coherent files that separate
    /// concerns. Each new file should own one cohesive responsibility
    /// (one type and its inherent impls, one trait implementation,
    /// one phase of a pipeline, one category of helpers) so that a
    /// reader can hold each file's purpose in mind on its own.
    ///
    /// ### Exception
    ///
    /// The exception marker is a last resort. Only use it when the
    /// file is genuinely one cohesive unit that splitting would
    /// obscure, such as a canonical wire-format table or a generated
    /// data block. If you reach for the exception, the reason must
    /// explain why splitting would harm cohesion rather than just
    /// noting that the file is long.
    ///
    /// ```rust
    /// // long-file-exception: this file contains the canonical wire
    /// // format tables for BLE profile descriptors. Splitting it
    /// // would separate fields that must stay together to satisfy
    /// // the spec layout.
    /// ```
    pub LONG_FILE,
    Deny,
    "source files must stay within 1000 lines and should be split into coherent smaller files",
}

pub(crate) struct LongFile {
    seen_files: BTreeSet<String>,
}

rustc_session::impl_lint_pass!(LongFile => [LONG_FILE]);

impl Default for LongFile {
    fn default() -> Self {
        Self {
            seen_files: BTreeSet::new(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for LongFile {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if item.span.from_expansion() {
            return;
        }

        let source_map = cx.sess().source_map();
        let Some((path, contents)) = source_file_contents(source_map, item) else {
            return;
        };
        let rel = path.to_string_lossy().replace('\\', "/");

        // Only fire once per file even though check_item runs for every
        // top-level item the file contains.
        if !self.seen_files.insert(rel.clone()) {
            return;
        }

        // xtask routing-invariant fixtures deliberately violate other
        // policy rules as regression inputs and should not be subject
        // to ordinary workspace style enforcement.
        if rel.contains("/xtask/fixtures/") {
            return;
        }

        let line_count = contents.lines().count();
        if line_count <= MAX_LONG_FILE_LINES {
            return;
        }

        if has_long_file_exception(&contents) {
            return;
        }

        let message = format!(
            "source file is {line_count} lines; limit is {MAX_LONG_FILE_LINES}. \
             Split the file into two or more coherent files that separate concerns. \
             Only as a last resort, when the file is genuinely one cohesive unit that splitting would obscure, \
             add a `// long-file-exception: <reason>` marker within the first {EXCEPTION_SCAN_LINES} lines \
             explaining why splitting would harm cohesion."
        );
        cx.emit_span_lint(
            LONG_FILE,
            item.span,
            DiagDecorator(|diag| {
                diag.primary_message(message.clone());
            }),
        );
    }
}

// Scans the first `EXCEPTION_SCAN_LINES` lines looking for
// `// long-file-exception: <reason>`. Blank lines, `//!` crate-level
// doc comments, `///` doc comments, and attributes are all transparent
// so the marker can sit above or below crate docs without interfering.
// A non-empty reason is required.
fn has_long_file_exception(contents: &str) -> bool {
    for line in contents.lines().take(EXCEPTION_SCAN_LINES) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("// long-file-exception:") {
            return !rest.trim().is_empty();
        }
    }
    false
}
