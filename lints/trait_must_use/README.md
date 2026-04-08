# jacquard-trait-must-use

Dylint lint: public trait methods returning meaningful values must carry `#[must_use]`.

## What it checks

For every `pub trait` item in the workspace, each method whose return type is
not `()` or `Result<(), _>` must have a `#[must_use]` or `#[must_use = "..."]`
attribute. Without it, callers can silently drop routing candidates, maintenance
results, and other evidence-bearing values.

## How to run

Requires a nightly toolchain and `cargo-dylint`:

```bash
# Enter the nightly dev shell:
just nightly-shell   # or: nix develop .#nightly

# Install dylint (once per toolchain):
just install-dylint

# Run the lint:
cargo dylint --path lints/trait_must_use --all -- --all-targets
```

## When to suppress

If a trait method intentionally returns a value that callers may ignore,
add `#[allow(clippy::must_use_candidate)]` and document why in a comment.
Do not suppress without explanation.
