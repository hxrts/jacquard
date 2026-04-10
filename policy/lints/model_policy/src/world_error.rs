//! Lint pass for keeping world-extension errors separate from routing errors.
//!
//! Enforces that `crates/traits/src/world.rs` does not mention `RouteError`.
//! World extensions produce observations about the environment; they must not
//! import or reference routing-layer failure types. Mixing the two layers
//! creates a hidden downward dependency from the observation boundary into the
//! routing engine, which undermines the separation of concerns the traits crate
//! is designed to maintain.
//!
//! Accepts: files that contain no mention of `RouteError` in the world module.
//! Rejects: any occurrence of `RouteError` anywhere in `world.rs`.

use std::collections::BTreeSet;

use rustc_hir::Item;
use rustc_errors::DiagDecorator;
use rustc_lint::{LateContext, LateLintPass, LintContext};

use crate::source_scan::{line_number, source_file_contents};

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Rejects `RouteError` mentions in the world-extension boundary.
    ///
    /// ### Why is this bad?
    ///
    /// World extensions produce observations. Routing failures should not leak
    /// downward into that boundary.
    pub WORLD_EXTENSION_ERROR_PURITY,
    Warn,
    "world-extension boundaries should use WorldError, not RouteError",
}

pub(crate) struct WorldExtensionErrorPurity {
    seen_files: BTreeSet<String>,
}

rustc_session::impl_lint_pass!(WorldExtensionErrorPurity => [WORLD_EXTENSION_ERROR_PURITY]);

impl Default for WorldExtensionErrorPurity {
    fn default() -> Self {
        Self {
            seen_files: BTreeSet::new(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for WorldExtensionErrorPurity {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if item.span.from_expansion() {
            return;
        }

        let source_map = cx.sess().source_map();
        let Some((path, contents)) = source_file_contents(source_map, item) else {
            return;
        };
        let rel = path.to_string_lossy().replace('\\', "/");
        if !rel.ends_with("crates/traits/src/world.rs") || !self.seen_files.insert(rel.clone()) {
            return;
        }

        if !contents.contains("RouteError") {
            return;
        }

        let message = format!(
            "world-extension boundary mentions RouteError in {rel}:{}",
            line_number(source_map, item.span)
        );
        cx.emit_span_lint(
            WORLD_EXTENSION_ERROR_PURITY,
            item.span,
            DiagDecorator(|diag| {
                diag.primary_message(message.clone());
            }),
        );
    }
}
