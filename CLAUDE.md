# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Contour is an adaptive mesh routing system built on choreographic protocols. It uses Telltale (a sibling repo at `../telltale`) for session types, choreography macros, and the effect-based runtime.

Contour is fully deterministic. Core designs and implementations must avoid floating-point types, host-dependent ordering, and ambient randomness in routing or protocol state unless an explicit, deterministic abstraction says otherwise.

Contour should also use a typed deterministic time model in core code. Prefer injected time effects plus explicit types such as monotonic ticks, durations, and route epochs rather than raw wall-clock APIs or ad hoc `u64` timestamp fields.

## Development environment

The project uses a Nix flake for tooling. Enter the dev shell with `nix develop` or use direnv (`direnv allow`). All commands below assume you are inside the Nix shell.

## Commands

```
just check          # cargo check --workspace
just build          # cargo build --workspace
just test           # cargo test --workspace
just lint           # cargo clippy --workspace -- -D warnings
just fmt            # cargo fmt --all
just fmt-check      # cargo fmt --all -- --check
just book           # build mdbook docs (also the default recipe)
just serve          # live-reload doc server
just summary        # regenerate docs/SUMMARY.md from doc files
just install-hooks  # enable .githooks/pre-commit
```

Run a single test: `cargo test -p <crate> <test_name>`

## Architecture

Rust workspace with crates under `crates/`. Workspace-level `Cargo.toml` declares shared dependency versions. New crates go in `crates/<name>` and must be added to `workspace.members`.

### Telltale dependency

Three Telltale crates are imported as local path dependencies (`../telltale/rust/{types,macros,runtime}`). The workspace `[workspace.dependencies]` table pins them; individual crates re-export via `{ workspace = true }`. The sibling telltale repo must be checked out at `../telltale`.

## Git hooks

Pre-commit hook (`.githooks/pre-commit`) runs format and compile checks on staged Rust files. Install with `just install-hooks`. Bypass with `git commit --no-verify`.
