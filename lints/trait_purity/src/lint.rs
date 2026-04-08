//! Lint pass for explicit purity/effect annotations on public traits.

use rustc_hir::{Item, ItemKind};
use rustc_errors::DiagDecorator;
use rustc_lint::{LateContext, LateLintPass, LintContext};

use crate::source_scan::source_has_trait_purity_marker;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Requires public trait definitions to carry an explicit Jacquard purity or
    /// effect annotation in source code.
    ///
    /// ### Why is this bad?
    ///
    /// Jacquard treats trait purity and side-effect boundaries as part of the
    /// contract. Unmarked public traits make those boundaries ambiguous and are
    /// easy to drift over time.
    ///
    /// ### Example
    ///
    /// ```rust
    /// pub trait RoutingEnginePlanner {
    ///     fn plan(&self);
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[purity(pure)]
    /// pub trait RoutingEnginePlanner {
    ///     fn plan(&self);
    /// }
    /// ```
    pub TRAIT_PURITY,
    Warn,
    "public traits should declare #[purity(...)] or #[effect_trait]",
    TraitPurity
}

struct TraitPurity;

impl<'tcx> LateLintPass<'tcx> for TraitPurity {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Trait(..)) {
            return;
        }

        if item.span.from_expansion() {
            return;
        }

        if !cx.tcx.visibility(item.owner_id.def_id).is_public() {
            return;
        }

        if is_internal_support_trait(cx, item) {
            return;
        }

        if source_has_trait_purity_marker(cx.sess().source_map(), item) {
            return;
        }

        cx.emit_span_lint(
            TRAIT_PURITY,
            item.span,
            DiagDecorator(|diag| {
                diag.primary_message("public trait is missing #[purity(...)] or #[effect_trait]");
            }),
        );
    }
}

fn is_internal_support_trait(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    matches!(
        cx.tcx.item_name(item.owner_id.def_id).as_str(),
        "Sealed" | "EffectDefinition" | "HandlerDefinition"
    )
}
