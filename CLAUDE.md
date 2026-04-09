# CLAUDE.md

Jacquard is a deterministic multi-engine routing system built around explicit router ownership, host bridges, and choreographic protocol support where an engine needs it. It uses Telltale for session types and choreography macros inside pathway, while the shared runtime model stays bridge-and-driver based rather than effect-stream based.

Jacquard is fully deterministic. No floating-point types, host-dependent ordering, or ambient randomness in routing or protocol state. Use typed time effects (`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`) rather than raw wall-clock APIs or ad hoc `u64` timestamp fields.

See [Crate Architecture](docs/999_crate_architecture.md) for the dependency graph, cross-crate invariants, ownership rules, purity model, and extension boundary.

## Development

Enter the dev shell with `nix develop` or direnv (`direnv allow`).

```
just check          # cargo check --workspace
just build          # cargo build --workspace
just test           # cargo test --workspace
just lint           # cargo clippy --workspace --all-targets -- -D warnings
just fmt            # nightly rustfmt via ./nix/nightly shell
just fmt-check      # nightly rustfmt --check
just book           # build mdbook docs (default recipe when running bare `just`)
just ci-dry-run     # run all CI checks locally (format, clippy, tests, xtask, dylint)
just install-hooks  # enable .githooks/pre-commit
cargo xtask check <name>   # run one policy/doc check directly
cargo xtask pre-commit     # run the staged-file pre-commit lane manually
```

Run a single test: `cargo test -p <crate> <test_name>`

## Crate rules

`core` defines what exists. `traits` defines what components are allowed to do. `core` must not grow behavioral traits. All cross-crate behavioral interfaces belong in `traits`. `core` and `traits` must remain runtime-free.

`adapter` exists for transport-neutral adapter support primitives only:
- bounded raw-ingress mailbox helpers
- unresolved/resolved peer bookkeeping
- in-flight claim ownership guards

`adapter` must not grow:
- world-model vocabulary that belongs in `core`
- capability or driver traits that belong in `traits`
- transport-specific protocol logic or endpoint constructors
- Jacquard time or ordering assignment

Transport ownership is split deliberately:

- `TransportSenderEffects` is the shared synchronous send capability.
- `TransportDriver` is the host-owned ingress and supervision surface.
- routers and engines must not own transport streams or assign `Tick`.
- routers consume explicit ingress through router-owned ingestion APIs and
  advance synchronously through `advance_round`.
- host bridges own ingress draining, batching, and time attachment.

`macros` owns syntax-local code generation and annotation-site validation. `lints/` owns nightly compiler-backed policy checks. `crates/xtask` owns the stable fast-path workspace checks used by `just`, CI, and the pre-commit hook. Do not hide broad policy in generic proc macros when the rule belongs in an explicit lint or xtask check.

Run individual policy checks with `cargo xtask check <name>`. Registered names:

- `checkpoint-namespacing` — storage keys inside pathway and router source trees must use the engine/pathway or router namespace prefix
- `adapter-boundary` — `jacquard-adapter` must stay transport-neutral, and adapter helper shapes must not leak back into `core` or `traits`
- `crate-boundary` — workspace dependency-graph invariants
- `docs-link-check` — broken, scratch-directory, or absolute-path links in markdown
- `docs-semantic-drift` — stale backtick references in markdown
- `dx-surface` — preset/client DX surface must use `NodePreset`/`LinkPreset` naming and `ClientBuilder`; public human-facing APIs must not exceed 4 positional parameters
- `engine-service-boundary` — `crates/pathway/src/lib.rs` must not export engine-private types
- `fail-closed-ordering` — delegates to `routing-invariants` for fail-closed mutation ordering
- `invariant-specs` — every `## Invariant:` section needs enforcement locus, failure mode, and verification hooks
- `pathway-choreography` — pathway choreography protocol coverage (pass `--validate` to execute the fixture pass)
- `pathway-async-boundary` — pathway may use async/Telltale session stepping only inside choreography modules, and pathway must not own transport drivers directly
- `no-scratch-refs-in-rust` — rust sources and comments must not reference the private scratch directory
- `no-usize-in-models` — bare `usize` fields rejected in `core`/`traits` model structs
- `ownership-invariants` — `core` and `traits` `lib.rs` must document the required ownership sections
- `proc-macro-scope` — every non-exempt Rust source file under a crate's `src/` tree must carry at least one Jacquard proc-macro annotation
- `proof-bearing-actions` — public methods returning high-consequence types must carry doc comments explaining proof semantics
- `result-must-use` — `fn method(...) -> Result<..>` trait methods under `crates/traits/src/` must carry `#[must_use]`
- `router-round-boundary` — explicit router ingress and `advance_round` vocabulary must remain in place, and poll-shaped router advancement is forbidden
- `routing-invariants` — routing-invariant rules (pass `--validate` to run fixture validation)
- `surface-classification` — traits whose name contains `Transport` must declare `connectivity surface` or `service surface`
- `test-boundaries` — unit vs integration test boundary rules
- `transport-authoring-boundary` — transport-neutral mem/reference crates must stay free of transport-specific endpoint authoring helpers
- `transport-ownership-boundary` — transport send capability and host-owned ingress supervision must stay split, and drivers must not stamp Jacquard time internally
- `reference-bridge-boundary` — reference-client may own drivers and direct transport attachment only inside its bridge/builders, and tests must advance bridges rather than routers directly
- `trait-purity` — public traits must be annotated with a `#[purity(..)]` mode

For nightly compiler-backed lint parity, use the nightly shell, run `install-dylint` once, and then run each dylint crate:

```bash
cargo dylint --path lints/trait_purity --all -- --all-targets
cargo dylint --path lints/model_policy --all -- --all-targets
cargo dylint --path lints/routing_invariants --all -- --all-targets
cargo dylint --path lints/trait_must_use --all -- --all-targets
cargo dylint --path lints/naked_map_err --all -- --all-targets
```

## Test layout

Unit tests co-locate with the module they cover. Higher-level tests go in `tests/` subdirectories by type (`integration/`, `regression/`, `property/`).

### Testing focus by crate

- `jacquard-core`: type invariants, canonical encoding, boundedness, deterministic ordering, content-addressing stability.
- `jacquard-traits`: compile-only surface checks, trait-object and generic-boundary tests.
- `jacquard-adapter`: transport-neutral ingress mailbox, peer-directory, and claim-guard helpers with explicit ownership semantics.
- `jacquard-pathway`: deterministic candidate production, admission/materialization, commitment tracking, forwarding, repair, topology-change, observation handling.
- `jacquard-router`: control-plane selection, ownership, capability enforcement, canonical handle issuance, lease expiry, explicit ingress, synchronous round advancement, fallback legality, adaptive-profile derivation.
- `jacquard-mem-node-profile`: deterministic node-profile and node-state builders with no routing-engine knowledge.
- `jacquard-mem-link-profile`: in-memory link-profile, carrier, retention, runtime-effect adapters, and host-owned transport driver surfaces with no routing semantics.
- `jacquard-reference-client`: host-side bridge composition of router + pathway/batman + in-memory profiles for end-to-end tests.
- `jacquard-xtask`: workspace policy checks, docs link/drift validation, and pre-commit entry point.

Transport-specific endpoint authoring belongs outside the transport-neutral mem profile crates and outside `jacquard-adapter`. `jacquard-core` owns the shared `TransportKind` / `EndpointLocator` schema; `jacquard-adapter` owns generic adapter-side support helpers; transport-owned profile crates own any BLE-, IP-, or other transport-specific endpoint helpers and defaults.

## Telltale dependency

Telltale crates are pinned from crates.io through the workspace `[workspace.dependencies]` table (`telltale`, `telltale-types`, `telltale-macros`, `telltale-runtime`, currently `11.3.0`). Individual crates import them via `{ workspace = true }`.

## long-block-exception

The `model_policy` dylint caps function bodies at 60 source lines (measured
from the opening brace to the closing brace, inclusive). Long bodies usually
mean a helper should be extracted. When a body legitimately needs to stay
long — e.g. a match statement that mirrors a shared enum one-to-one, or a
fixture constructor that assembles a full world sample — add an exception
marker directly above the `fn` signature:

```rust
// long-block-exception: <non-empty reason>
fn assemble_world() -> Configuration {
    // ... > 60 lines ...
}
```

Blank lines, doc comments, and `#[attr]` attributes between the marker and
the signature are allowed. The reason text must be non-empty so every
exception stays auditable in code review. Prefer splitting the function
first; reach for the exception marker only when extraction would obscure
the mapping.
