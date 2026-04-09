//! Lint passes for routing-invariant policy.

use std::collections::BTreeSet;

use regex::Regex;
use rustc_hir::Item;
use rustc_errors::DiagDecorator;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext, LintStore};
use rustc_session::Session;

use crate::source_scan::{first_line_matching, line_position, rel_path, source_file_contents};

dylint_linting::dylint_library!();

rustc_session::declare_lint! {
    pub PLANNER_CACHE_DEPENDENCE,
    Warn,
    "materialization and admission should not depend semantically on planner caches",
}

rustc_session::declare_lint! {
    pub FAIL_CLOSED_ORDERING,
    Warn,
    "routing runtime state should not mutate before validation, logging, or persistence",
}

rustc_session::declare_lint! {
    pub TICK_EPOCH_CONFLATION,
    Warn,
    "Tick and RouteEpoch should not be conflated by wrapper reconstruction",
}

rustc_session::declare_lint! {
    pub CHECKED_SCORE_ARITHMETIC,
    Warn,
    "bounded routing score arithmetic should use checked/saturating composition",
}

rustc_session::declare_lint! {
    pub TYPED_WRAPPER_ARITHMETIC,
    Warn,
    "typed time/version wrappers should not use unchecked raw-field addition",
}

rustc_session::declare_lint! {
    pub COMMITTEE_SWALLOW,
    Warn,
    "committee selector failures should not be silently erased",
}

rustc_session::declare_lint! {
    pub NULL_OBJECT_SELECTOR,
    Warn,
    "null-object selectors should not be wrapped in dead Option state",
}

rustc_session::declare_lint! {
    pub ROUTER_IDENTITY_MUTATION,
    Warn,
    "engine code should not mutate router-owned identity state",
}

rustc_session::declare_lint! {
    pub UNSCOPED_STORAGE_KEYS,
    Warn,
    "engine storage keys should be scoped by local engine identity",
}

rustc_session::declare_lint! {
    pub SYNTHETIC_FALLBACK,
    Warn,
    "routing code should fail closed rather than synthesizing authoritative state",
}

rustc_session::declare_lint! {
    pub NAMED_THRESHOLDS,
    Warn,
    "routing thresholds in production code should use named constants",
}

#[allow(unsafe_code)]
#[expect(clippy::no_mangle_with_rust_abi)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &Session, lint_store: &mut LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        PLANNER_CACHE_DEPENDENCE,
        FAIL_CLOSED_ORDERING,
        TICK_EPOCH_CONFLATION,
        CHECKED_SCORE_ARITHMETIC,
        TYPED_WRAPPER_ARITHMETIC,
        COMMITTEE_SWALLOW,
        NULL_OBJECT_SELECTOR,
        ROUTER_IDENTITY_MUTATION,
        UNSCOPED_STORAGE_KEYS,
        SYNTHETIC_FALLBACK,
        NAMED_THRESHOLDS,
    ]);
    lint_store.register_late_pass(|_| Box::new(PlannerCacheDependence::default()));
    lint_store.register_late_pass(|_| Box::new(FailClosedOrdering::default()));
    lint_store.register_late_pass(|_| Box::new(TickEpochConflation::default()));
    lint_store.register_late_pass(|_| Box::new(CheckedScoreArithmetic::default()));
    lint_store.register_late_pass(|_| Box::new(TypedWrapperArithmetic::default()));
    lint_store.register_late_pass(|_| Box::new(CommitteeSwallow::default()));
    lint_store.register_late_pass(|_| Box::new(NullObjectSelector::default()));
    lint_store.register_late_pass(|_| Box::new(RouterIdentityMutation::default()));
    lint_store.register_late_pass(|_| Box::new(UnscopedStorageKeys::default()));
    lint_store.register_late_pass(|_| Box::new(SyntheticFallback::default()));
    lint_store.register_late_pass(|_| Box::new(NamedThresholds::default()));
}

rustc_session::impl_lint_pass!(PlannerCacheDependence => [PLANNER_CACHE_DEPENDENCE]);
rustc_session::impl_lint_pass!(FailClosedOrdering => [FAIL_CLOSED_ORDERING]);
rustc_session::impl_lint_pass!(TickEpochConflation => [TICK_EPOCH_CONFLATION]);
rustc_session::impl_lint_pass!(CheckedScoreArithmetic => [CHECKED_SCORE_ARITHMETIC]);
rustc_session::impl_lint_pass!(TypedWrapperArithmetic => [TYPED_WRAPPER_ARITHMETIC]);
rustc_session::impl_lint_pass!(CommitteeSwallow => [COMMITTEE_SWALLOW]);
rustc_session::impl_lint_pass!(NullObjectSelector => [NULL_OBJECT_SELECTOR]);
rustc_session::impl_lint_pass!(RouterIdentityMutation => [ROUTER_IDENTITY_MUTATION]);
rustc_session::impl_lint_pass!(UnscopedStorageKeys => [UNSCOPED_STORAGE_KEYS]);
rustc_session::impl_lint_pass!(SyntheticFallback => [SYNTHETIC_FALLBACK]);
rustc_session::impl_lint_pass!(NamedThresholds => [NAMED_THRESHOLDS]);

#[derive(Default)]
struct PlannerCacheDependence {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct FailClosedOrdering {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct TickEpochConflation {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct CheckedScoreArithmetic {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct TypedWrapperArithmetic {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct CommitteeSwallow {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct NullObjectSelector {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct RouterIdentityMutation {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct UnscopedStorageKeys {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct SyntheticFallback {
    seen_files: BTreeSet<String>,
}

#[derive(Default)]
struct NamedThresholds {
    seen_files: BTreeSet<String>,
}

impl<'tcx> LateLintPass<'tcx> for PlannerCacheDependence {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("crates/pathway/src/engine/runtime") {
                return None;
            }
            let line = first_line_matching(contents, &Regex::new(r"find_cached_candidate_by_route_id\(").ok()?)?;
            Some((
                PLANNER_CACHE_DEPENDENCE,
                format!("{rel}:{line}: materialization depends on cache lookup helper"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for FailClosedOrdering {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("crates/pathway/src/engine/runtime") {
                return None;
            }
            let insert_line = line_position(contents, "self.active_routes.insert(");
            let record_line = line_position(contents, "self.record_event(RouteEvent::RouteMaterialized");
            if let (Some(insert_line), Some(record_line)) = (insert_line, record_line) {
                if insert_line < record_line {
                    return Some((
                        FAIL_CLOSED_ORDERING,
                        format!(
                            "{rel}:{insert_line}: active route table is mutated before RouteMaterialized is recorded"
                        ),
                    ));
                }
            }
            let apply_line = line_position(contents, "Self::apply_maintenance_trigger(");
            let checkpoint_line = line_position(contents, "self.store_checkpoint(&active_route_snapshot)");
            if let (Some(apply_line), Some(checkpoint_line)) = (apply_line, checkpoint_line) {
                if apply_line < checkpoint_line {
                    return Some((
                        FAIL_CLOSED_ORDERING,
                        format!(
                            "{rel}:{apply_line}: maintenance trigger mutates runtime state before checkpoint persistence"
                        ),
                    ));
                }
            }
            None
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for TickEpochConflation {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("/crates/") {
                return None;
            }
            let line = first_line_matching(
                contents,
                &Regex::new(r"RouteEpoch\([^)]*tick[^)]*\.0\)|Tick\([^)]*(epoch|current_epoch)[^)]*\.0\)")
                    .ok()?,
            )?;
            Some((
                TICK_EPOCH_CONFLATION,
                format!("{rel}:{line}: Tick and RouteEpoch are being conflated by wrapper re-construction"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for CheckedScoreArithmetic {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.ends_with("crates/pathway/src/engine/runtime/health.rs") {
                return None;
            }
            let re = Regex::new(
                r"quiet_pressure\s*\n?\s*\+\s*summary\.congestion_penalty_points\.0\.saturating_mul\(50\)",
            )
            .ok()?;
            if !re.is_match(contents) {
                return None;
            }
            let line = first_line_matching(
                contents,
                &Regex::new(r"quiet_pressure").ok()?,
            )?;
            Some((
                CHECKED_SCORE_ARITHMETIC,
                format!(
                    "{rel}:{line}: bounded routing score arithmetic uses plain + instead of saturating_add"
                ),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for TypedWrapperArithmetic {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("/crates/") {
                return None;
            }
            let line = first_line_matching(
                contents,
                &Regex::new(r"(Tick|RouteEpoch)\([^)]*\.0\s*\+\s*[A-Z_a-z0-9]+").ok()?,
            )?;
            Some((
                TYPED_WRAPPER_ARITHMETIC,
                format!(
                    "{rel}:{line}: typed time/version wrapper is reconstructed with unchecked raw-field addition"
                ),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for CommitteeSwallow {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("crates/pathway/src/engine/planner") {
                return None;
            }
            let line = first_line_matching(contents, &Regex::new(r"\.ok\(\)\.flatten\(\)").ok()?)?;
            Some((
                COMMITTEE_SWALLOW,
                format!("{rel}:{line}: committee selector error is being silently erased"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for NullObjectSelector {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.ends_with("crates/pathway/src/engine/mod.rs") {
                return None;
            }
            let line =
                first_line_matching(contents, &Regex::new(r"selector:\s+Option<Selector>").ok()?)?;
            Some((
                NULL_OBJECT_SELECTOR,
                format!("{rel}:{line}: null-object selector is wrapped in dead Option state"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for RouterIdentityMutation {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("/crates/pathway/src/") && !rel.contains("/crates/router/src/") {
                return None;
            }
            let line = first_line_matching(
                contents,
                &Regex::new(r"\.(lease|handle|route_id)\s*=").ok()?,
            )?;
            Some((
                ROUTER_IDENTITY_MUTATION,
                format!("{rel}:{line}: engine code appears to mutate router-owned identity state"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for UnscopedStorageKeys {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("/crates/pathway/src/") {
                return None;
            }
            let line = first_line_matching(contents, &Regex::new(r#"b"pathway/(topology-epoch|route/)"#).ok()?)?;
            Some((
                UNSCOPED_STORAGE_KEYS,
                format!("{rel}:{line}: storage key is not scoped by local engine identity"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for SyntheticFallback {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("/crates/pathway/src/") {
                return None;
            }
            let line = first_line_matching(
                contents,
                &Regex::new(
                    r"fallback_health_configuration|map_or_else\(\s*\|\|\s*self\.fallback_health_configuration",
                )
                .ok()?,
            )?;
            Some((
                SYNTHETIC_FALLBACK,
                format!("{rel}:{line}: synthetic authoritative-state fallback detected"),
            ))
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for NamedThresholds {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        scan_once(cx, item, &mut self.seen_files, |rel, contents| {
            if !rel.contains("/crates/pathway/src/") {
                return None;
            }
            let line = first_line_matching(contents, &Regex::new(r">\s*600\b").ok()?)?;
            Some((
                NAMED_THRESHOLDS,
                format!("{rel}:{line}: routing threshold literal should be a named constant"),
            ))
        });
    }
}

fn scan_once<'tcx, F>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    seen_files: &mut BTreeSet<String>,
    matcher: F,
) where
    F: Fn(&str, &str) -> Option<(&'static Lint, String)>,
{
    if item.span.from_expansion() {
        return;
    }
    let source_map = cx.sess().source_map();
    let Some((path, contents)) = source_file_contents(source_map, item) else {
        return;
    };
    let rel = rel_path(&path);
    if !seen_files.insert(rel.clone()) {
        return;
    }
    let Some((lint, message)) = matcher(&rel, &contents) else {
        return;
    };
    cx.emit_span_lint(
        lint,
        item.span,
        DiagDecorator(|diag| {
            diag.primary_message(message.clone());
        }),
    );
}
