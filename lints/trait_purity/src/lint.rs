//! Lint pass for explicit purity/effect annotations on public traits.
//!
//! Requires every public trait definition to carry a Jacquard purity or effect
//! annotation immediately above the `trait` keyword in source. The accepted
//! markers are `#[purity(...)]` (e.g., `#[purity(pure)]`, `#[purity(read)]`,
//! `#[purity(write)]`) and `#[effect_trait]`.
//!
//! Purity and side-effect boundaries are part of the trait contract in
//! Jacquard. An unmarked public trait leaves those boundaries implicit, making
//! them easy to violate or drift without notice during refactoring. The
//! annotation is checked in source text rather than via HIR attributes because
//! the relevant proc-macro annotations may not be visible to the compiler at
//! the lint site.
//!
//! Accepts: public traits with `#[purity(...)]` or `#[effect_trait]` in source.
//! Rejects: public traits missing both annotations. Internal support traits
//! (`Sealed`, `EffectDefinition`, `HandlerDefinition`) are exempt.

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
