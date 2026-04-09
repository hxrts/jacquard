//! Lint pass to prevent engine-private vocabulary leaking into shared crates.
//!
//! Detects public structs, enums, and type aliases in `jacquard-core` and
//! `jacquard-traits` whose names begin with engine-specific prefixes: `Pathway`,
//! `Mesh`, `Onion`, or `Field`. These prefixes identify vocabulary that belongs
//! to a particular routing engine and must not appear in the shared schema or
//! contract crates.
//!
//! The shared crates define engine-neutral types that all engines depend on.
//! Introducing engine-specific names there creates an upward coupling that
//! makes it impossible to swap or add engines without touching the shared layer.
//!
//! Accepts: shared-crate types with engine-neutral names.
//! Rejects: public types in `core/` or `traits/` whose names start with
//! `Pathway`, `Mesh`, `Onion`, or `Field`.

use rustc_hir::{Item, ItemKind};
use rustc_errors::DiagDecorator;
use rustc_lint::{LateContext, LateLintPass, LintContext};

rustc_session::declare_lint! {
    /// ### What it does
    ///
    /// Rejects engine-specific public type names in `jacquard-core` and
    /// `jacquard-traits`.
    ///
    /// ### Why is this bad?
    ///
    /// Shared crates define engine-neutral schema and contracts. Mesh-, onion-,
    /// and field-specific runtime vocabulary belongs in engine crates.
    pub SHARED_PRIVATE_BOUNDARY,
    Warn,
    "shared crates should not define engine-specific public vocabulary",
}

pub(crate) struct SharedPrivateBoundary;

rustc_session::impl_lint_pass!(SharedPrivateBoundary => [SHARED_PRIVATE_BOUNDARY]);

impl<'tcx> LateLintPass<'tcx> for SharedPrivateBoundary {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Struct(..) | ItemKind::Enum(..) | ItemKind::TyAlias(..)) {
            return;
        }

        if item.span.from_expansion() {
            return;
        }

        if !cx.tcx.visibility(item.owner_id.def_id).is_public() {
            return;
        }

        let source_map = cx.sess().source_map();
        let path = crate::source_scan::source_file_path(source_map, item);
        let rel = path.to_string_lossy();
        if !(rel.contains("/crates/core/src/") || rel.contains("/crates/traits/src/")) {
            return;
        }

        let name = cx.tcx.item_name(item.owner_id.def_id);
        let name = name.as_str();
        if !(name.starts_with("Pathway")
            || name.starts_with("Mesh")
            || name.starts_with("Onion")
            || name.starts_with("Field"))
        {
            return;
        }

        cx.emit_span_lint(
            SHARED_PRIVATE_BOUNDARY,
            item.span,
            DiagDecorator(|diag| {
                diag.primary_message("shared crate defines engine-specific public vocabulary");
            }),
        );
    }
}
