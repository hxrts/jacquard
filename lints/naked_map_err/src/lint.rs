//! Lint pass: `.map_err(|_| RouteError::Runtime(...))` with a discarded
//! error must use a `ResultExt` extension method instead.
//!
//! Detects method call expressions of the form
//! `<recv>.map_err(|_| RouteError::Runtime(...))` where the closure parameter
//! is a wildcard, indicating the original error is silently dropped. This
//! pattern makes failures harder to diagnose and violates the workspace
//! error-mapping convention.
//!
//! The lint requires callers to use one of the named `ResultExt` methods
//! instead: `storage_invalid()`, `choreography_failed()`,
//! `maintenance_failed()`, or `invalidated()`. These methods preserve the
//! original error context while naming the mapping intent explicitly.
//!
//! Accepts: any `.map_err` closure that binds and uses the error argument.
//! Rejects: `.map_err(|_| RouteError::Runtime(...))` with a wildcard arg.

use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Detects calls of the form `.map_err(|_| RouteError::Runtime(...))` where
    /// the original error value is discarded via a wildcard closure argument.
    ///
    /// ### Why
    ///
    /// Discarding the original error makes failures harder to debug. The
    /// Jacquard workspace provides `ResultExt` extension methods that name the
    /// mapping intent without discarding error context:
    ///
    /// - `StorageResultExt::storage_invalid()` — maps to `Invalidated`
    /// - `ChoreographyResultExt::choreography_failed()` — maps to `MaintenanceFailed`
    /// - `MaintenanceResultExt::maintenance_failed()` — maps to `MaintenanceFailed`
    /// - `InvalidatedResultExt::invalidated()` — maps to `Invalidated` (choreography)
    ///
    /// ### Example
    ///
    /// ```rust
    /// // Bad
    /// store_bytes(&key, &value)
    ///     .map_err(|_| RouteError::Runtime(RouteRuntimeError::Invalidated))?;
    ///
    /// // Good
    /// store_bytes(&key, &value).storage_invalid()?;
    /// ```
    pub NAKED_MAP_ERR_ROUTE_ERROR,
    Deny,
    ".map_err(|_| RouteError::Runtime(...)) discards the original error; use a ResultExt method",
    NakedMapErrRouteError
}

struct NakedMapErrRouteError;

impl<'tcx> LateLintPass<'tcx> for NakedMapErrRouteError {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Looking for: <recv>.map_err(<closure>)
        let ExprKind::MethodCall(method, _recv, args, _span) = &expr.kind else {
            return;
        };

        if method.ident.name.as_str() != "map_err" {
            return;
        }

        let [closure_arg] = args else {
            return;
        };

        // The argument must be a closure.
        let ExprKind::Closure(closure) = &closure_arg.kind else {
            return;
        };

        // The closure must have exactly one parameter.
        let closure_body = cx.tcx.hir_body(closure.body);
        let [param] = closure_body.params else {
            return;
        };

        // The parameter must be a wildcard (`_`).
        if !matches!(param.pat.kind, PatKind::Wild) {
            return;
        }

        // The closure body must be a call to RouteError::Runtime(...).
        let body_expr = &closure_body.value;
        if !is_route_error_runtime_call(body_expr) {
            return;
        }

        cx.emit_span_lint(
            NAKED_MAP_ERR_ROUTE_ERROR,
            closure_arg.span,
            DiagDecorator(|diag| {
                diag.primary_message(
                    ".map_err(|_| RouteError::Runtime(...)) discards the original error; \
                     use .storage_invalid(), .choreography_failed(), or .maintenance_failed() instead",
                );
            }),
        );
    }
}

/// Returns true if `expr` is `RouteError::Runtime(...)` — a path call whose
/// last segment is `Runtime` on a type named `RouteError`.
fn is_route_error_runtime_call(expr: &Expr<'_>) -> bool {
    let ExprKind::Call(func, _args) = &expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = &func.kind else {
        return false;
    };
    let path = match qpath {
        QPath::Resolved(_, path) => path,
        QPath::TypeRelative(_, segment) => {
            return segment.ident.name.as_str() == "Runtime";
        }
    };
    let segments = path.segments;
    // Accept `RouteError::Runtime` (2 segments) or just `Runtime` (1 segment
    // after type-resolution collapses the path).
    if let Some(last) = segments.last() {
        if last.ident.name.as_str() == "Runtime" {
            return true;
        }
    }
    false
}
