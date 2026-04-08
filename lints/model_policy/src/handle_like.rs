//! Lint pass for explicit `#[must_use_handle]` on handle-like shared model types.

use rustc_hir::{Item, ItemKind};
use rustc_errors::DiagDecorator;
use rustc_lint::{LateContext, LateLintPass, LintContext};

use crate::source_scan::source_has_attribute;

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Requires handle-like public shared-model types to carry an explicit
    /// `#[must_use_handle]` annotation.
    ///
    /// ### Why is this bad?
    ///
    /// Handle, lease, and handoff types represent strong routing ownership or
    /// transfer semantics. Making `must_use` behavior implicit inside
    /// `#[public_model]` hides policy in a generic data-model macro.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[public_model]
    /// pub struct RouteHandle { /* ... */ }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[must_use_handle]
    /// #[public_model]
    /// pub struct RouteHandle { /* ... */ }
    /// ```
    pub HANDLE_LIKE_MUST_USE,
    Warn,
    "handle-like public model types should declare #[must_use_handle]",
}

pub(crate) struct HandleLikeMustUse;

rustc_session::impl_lint_pass!(HandleLikeMustUse => [HANDLE_LIKE_MUST_USE]);

impl<'tcx> LateLintPass<'tcx> for HandleLikeMustUse {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Struct(..) | ItemKind::Enum(..)) {
            return;
        }

        if item.span.from_expansion() {
            return;
        }

        if !cx.tcx.visibility(item.owner_id.def_id).is_public() {
            return;
        }

        if !is_handle_like_name(cx, item) {
            return;
        }

        if !source_has_attribute(cx.sess().source_map(), item, "public_model") {
            return;
        }

        if source_has_attribute(cx.sess().source_map(), item, "must_use_handle") {
            return;
        }

        cx.emit_span_lint(
            HANDLE_LIKE_MUST_USE,
            item.span,
            DiagDecorator(|diag| {
                diag.primary_message(
                    "handle-like public model type is missing #[must_use_handle]",
                );
            }),
        );
    }
}

fn is_handle_like_name(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    let name = cx.tcx.item_name(item.owner_id.def_id);
    let name = name.as_str();
    name.ends_with("Handle") || name.ends_with("Lease") || name.ends_with("Handoff")
}
