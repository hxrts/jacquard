// Dylint lint: flags public traits missing #[purity(...)] or #[effect_trait].
// Companion to scripts/check/trait-purity.sh (grep-based) — this pass uses
// the compiler AST for deeper checks. Requires nightly + cargo-dylint.

#![feature(rustc_private)]
#![forbid(unsafe_code)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::source_map::SourceMap;

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
    /// pub trait RoutePlanner {
    ///     fn plan(&self);
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[purity(pure)]
    /// pub trait RoutePlanner {
    ///     fn plan(&self);
    /// }
    /// ```
    pub JACQUARD_TRAIT_PURITY,
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

        if matches!(
            item.ident.name.as_str(),
            "Sealed" | "EffectDefinition" | "HandlerDefinition"
        ) {
            return;
        }

        if source_has_trait_purity_marker(cx.sess().source_map(), item) {
            return;
        }

        cx.struct_span_lint(JACQUARD_TRAIT_PURITY, item.span, |diag| {
            diag.build("public trait is missing #[purity(...)] or #[effect_trait]")
                .emit();
        });
    }
}

fn source_has_trait_purity_marker(source_map: &SourceMap, item: &Item<'_>) -> bool {
    let file = source_map.lookup_source_file(item.span.lo());
    let Ok(contents) = std::fs::read_to_string(&file.name.prefer_remapped_unconditionally()) else {
        return false;
    };
    let line_index = source_map.lookup_char_pos(item.span.lo()).line.saturating_sub(1);
    let lines: Vec<&str> = contents.lines().collect();

    if line_index >= lines.len() {
        return false;
    }

    for line in lines[..line_index].iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("///") {
            continue;
        }

        return trimmed.starts_with("#[purity(") || trimmed == "#[effect_trait]";
    }

    false
}
