//! Lint pass: public trait methods with non-unit return types must have
//! `#[must_use]`.

use rustc_errors::DiagDecorator;
use rustc_hir::{FnRetTy, Item, ItemKind, TraitItem, TraitItemKind, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::sym;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks that public trait methods whose return type is not `()` or
    /// `Result<(), _>` carry a `#[must_use]` or `#[must_use = "..."]`
    /// attribute.
    ///
    /// ### Why
    ///
    /// Without `#[must_use]`, callers can silently drop route candidates,
    /// maintenance results, or other values that represent work or evidence.
    /// Jacquard routing traits enforce this uniformly.
    ///
    /// ### Example
    ///
    /// ```rust
    /// pub trait RoutingEngine {
    ///     fn route_commitments(&self, ...) -> Vec<RouteCommitment>;
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// pub trait RoutingEngine {
    ///     #[must_use = "unread route commitments silently discard commitment state"]
    ///     fn route_commitments(&self, ...) -> Vec<RouteCommitment>;
    /// }
    /// ```
    pub TRAIT_METHOD_MUST_USE,
    Warn,
    "public trait methods returning meaningful values should carry #[must_use]",
    TraitMethodMustUse
}

struct TraitMethodMustUse;

impl<'tcx> LateLintPass<'tcx> for TraitMethodMustUse {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Trait(_, _, _, _, items) = item.kind else {
            return;
        };

        if item.span.from_expansion() {
            return;
        }

        if !cx.tcx.visibility(item.owner_id.def_id).is_public() {
            return;
        }

        for trait_item_ref in items {
            let trait_item: &TraitItem<'tcx> =
                cx.tcx.hir().trait_item(trait_item_ref.id);

            let TraitItemKind::Fn(sig, _) = &trait_item.kind else {
                continue;
            };

            // Skip methods whose return type is `()` (unit).
            if is_unit_return(sig.decl.output) {
                continue;
            }

            // Skip methods whose return type is `Result<(), _>`.
            if is_result_unit_return(sig.decl.output) {
                continue;
            }

            // Check for #[must_use] or #[must_use = "..."].
            if has_must_use_attr(&trait_item.attrs) {
                continue;
            }

            cx.emit_span_lint(
                TRAIT_METHOD_MUST_USE,
                trait_item.span,
                DiagDecorator(|diag| {
                    diag.primary_message(
                        "public trait method returns a meaningful value but lacks #[must_use]",
                    );
                }),
            );
        }
    }
}

fn is_unit_return(output: FnRetTy<'_>) -> bool {
    match output {
        FnRetTy::DefaultReturn(_) => true,
        FnRetTy::Return(ty) => matches!(ty.kind, TyKind::Tup(tys) if tys.is_empty()),
    }
}

fn is_result_unit_return(output: FnRetTy<'_>) -> bool {
    let FnRetTy::Return(ty) = output else {
        return false;
    };
    // Pattern: Result<(), _> — represented as Path with generic args where
    // first arg is the unit type.
    let TyKind::Path(rustc_hir::QPath::Resolved(_, path)) = ty.kind else {
        return false;
    };
    let Some(last_seg) = path.segments.last() else {
        return false;
    };
    if last_seg.ident.name.as_str() != "Result" {
        return false;
    }
    let Some(args) = last_seg.args else {
        return false;
    };
    let Some(first_arg) = args.args.first() else {
        return false;
    };
    let rustc_hir::GenericArg::Type(ok_ty) = first_arg else {
        return false;
    };
    matches!(ok_ty.kind, TyKind::Tup(tys) if tys.is_empty())
}

fn has_must_use_attr(attrs: &[rustc_ast::ast::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.has_name(sym::must_use))
}
