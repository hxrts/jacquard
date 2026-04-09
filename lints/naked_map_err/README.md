# jacquard-naked-map-err

Dylint lint: `.map_err(|_| RouteError::Runtime(...))` with a discarded error
must be replaced with the appropriate `ResultExt` extension method.

## What it checks

Any `.map_err` call where:
1. The closure argument is a wildcard `_` (original error is discarded), and
2. The closure body constructs `RouteError::Runtime(...)`.

## Why this matters

Discarding the original error makes failures harder to debug. The workspace
provides named extension methods that communicate intent without losing
diagnostic value:

| Method | Maps to |
|---|---|
| `.storage_invalid()` | `RouteError::Runtime(RouteRuntimeError::Invalidated)` |
| `.choreography_failed()` | `RouteError::Runtime(RouteRuntimeError::MaintenanceFailed)` |
| `.maintenance_failed()` | `RouteError::Runtime(RouteRuntimeError::MaintenanceFailed)` |
| `.invalidated()` | `RouteError::Runtime(RouteRuntimeError::Invalidated)` |

These are defined in:
- `crates/pathway/src/engine/support.rs` — `StorageResultExt`, `MaintenanceResultExt`
- `crates/pathway/src/choreography/effects.rs` — `ChoreographyResultExt`, `InvalidatedResultExt`
- `crates/router/src/runtime.rs` — `StorageResultExt`

## How to run

Requires a nightly toolchain and `cargo-dylint`:

```bash
# Enter the nightly dev shell:
just nightly-shell   # or: nix develop .#nightly

# Install dylint (once per toolchain):
just install-dylint

# Run the lint:
cargo dylint --path lints/naked_map_err --all -- --all-targets
```

## When to suppress

If a `.map_err(|_| ...)` is intentional (e.g., the error type truly cannot be
preserved), add `#[allow(jacquard_naked_map_err::naked_map_err_route_error)]`
and document why in a comment.
