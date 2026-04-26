# Running Experiments

This guide covers two kinds of work. The first half shows how to invoke the in-tree experiment suites through the `tuning_matrix` binary, where to find the artifacts, how to regenerate the report, and what the current corpus says about each engine. The second half shows how to assemble a custom suite programmatically when the in-tree families do not cover a 3rd party's analytical question.

See [Experimental Methodology](307_experimental_methodology.md) for what an experiment is, why the tuning phase runs first, and which variables are independent versus dependent. See [Simulator Architecture](306_simulator_architecture.md) for the harness the suites sit on, [Reference Client](407_reference_client.md) for the host composition, and [Running Simulations](501_running_simulations.md) for the base scenario flow each experiment builds on.

## Running The In-Tree Suites

Run the smaller smoke sweep:

```bash
cargo run --bin tuning_matrix -- smoke
```

Smoke runs a single seed across every family and completes quickly enough for a pre-PR sanity pass. Use it to confirm the binary builds and the families produce non-empty artifacts.

Run the full local sweep and generate the report:

```bash
cargo run --bin tuning_matrix -- local
```

Local runs the full maintained seed set across every family, including diffusion and head-to-head corpora. The run writes artifacts to `artifacts/analysis/local/{timestamp}/` and updates the `artifacts/analysis/local/latest` symlink. Expect a long-running job on modest hardware.

The matrix caps worker concurrency by default to avoid exhausting memory on developer machines. Override the cap explicitly when needed:

```bash
cargo run --bin tuning_matrix -- local --jobs 1
JACQUARD_TUNING_JOBS=2 cargo run --bin tuning_matrix -- local
```

Both the `--jobs` flag and the environment variable apply to the same concurrency pool.

Regenerate the report without rerunning the simulator:

```bash
nix develop --command python3 -m analysis.report artifacts/analysis/local/latest
```

The route-visible router report lands at
`artifacts/analysis/{suite}/latest/router-tuning-report.pdf`.

## Engine Takeaways From The Current Corpus

Each paragraph below summarizes what the present corpus says about one engine. Treat them as observations rather than normative recommendations. Rerun the matrix and reread the report after any meaningful engine, router, or simulator change before updating defaults.

### BATMAN Bellman

BATMAN Bellman separates most clearly in recoverable transition families. Route-presence plateaus alone are too flat, so the report also weighs stability accumulation, first-loss timing, and failure boundaries. The responsive range clusters around the short-window settings. `batman-bellman-1-1` leads the balanced default ranking, with `batman-bellman-2-1` and `batman-bellman-3-1` close behind. Asymmetric bridge breakdown regimes remain hard failures across the tested window range.

### BATMAN Classic

BATMAN Classic converges more slowly than BATMAN Bellman due to its echo-only bidirectionality and lack of bootstrap shortcut. The tested decay window settings cluster tightly. The recommendation reflects the spec-faithful model's need for larger windows to allow receive-window accumulation.

### Babel

Babel separates most clearly in the asymmetry-cost-penalty family, where the bidirectional ETX formula produces measurably different route selection. The partition-feasibility-recovery family shows the FD table's bounded infeasible-fallback window. Decay window settings do not yet separate sharply, suggesting the FD table and seqno refresh interval dominate convergence timing.

### Pathway

The Pathway matrix shows a clear minimum-budget boundary. Budget `1` remains the hard cliff in the maintained service-pressure families. Budgets at and above `2` form the viable floor. `pathway-4-zero` and `pathway-4-hop-lower-bound` lead the balanced default ranking.

The practical interpretation is that `2` is the minimum viable budget floor, `3` to `4` is the sensible default range, and larger budgets need a regime-specific justification.

### Mercator

Mercator is now represented on both the route-visible and diffusion surfaces. In route-visible head-to-head runs, it behaves as a corridor-maintenance engine: strong in connected high-loss and corridor-continuity families, viable but visibly constrained by bridge-transition and stale-recovery windows. In diffusion runs, its custody fallback is bounded rather than flood-like; the current corpus shows it can keep the broadcast overload and large congestion-threshold families viable after the protected bridge budget tuning.

### Mixed Comparison

The comparison regimes inform regime suitability rather than global winners. Low-loss connected-only cases favor the distance-vector stacks. Concurrent mixed workloads favor Pathway. High-loss bridge and corridor-continuity cases remain hard failure regimes across several tested stacks.

### Head-To-Head Engine Sets

The head-to-head corpus runs the same regimes under explicit single-engine stacks: `batman-bellman`, `batman-classic`, `babel`, `olsrv2`, `scatter`, `mercator`, `pathway`, or `pathway-batman-bellman`. This is separate from the mixed-engine comparison, which asks which engine a router selects when several are available. Head-to-head asks what happens when only one is available.

## Assembling A Custom Suite

A 3rd party who needs an experiment the in-tree corpus does not cover should
assemble it through the public runner facade. The maintained tuning corpus is
available under `jacquard_simulator::builtin_suites`, while custom suites use
`ExperimentSuiteSpec`, `RouteVisibleRunSpec`, `ExperimentRunner`, and
`ArtifactSink`.

```rust
use jacquard_core::SimulationSeed;
use jacquard_simulator::{
    ArtifactSink, ExperimentRunner, ExperimentSuiteSpec, RouteVisibleRunSpec,
};

let suite = ExperimentSuiteSpec::route_visible(
    "custom-connected-line",
    vec![RouteVisibleRunSpec::new(
        "custom-connected-line-seed-41",
        "custom-connected-line-family",
        "batman-bellman",
        SimulationSeed(41),
        scenario,
        environment,
    )],
);

let artifacts = ExperimentRunner::default()
    .run_route_visible_suite(
        &suite,
        &ArtifactSink::directory("artifacts/custom-connected-line"),
    )
    .expect("run custom suite");
```

The call returns a `RouteVisibleArtifacts` handle with an in-memory manifest and
per-run summaries. With a directory sink, it writes external_manifest.json and
external_runs.jsonl. With `ArtifactSink::disabled()`, it executes without
writing files and without invoking Python.

The custom facade is deliberately separate from the maintained report writer.
Use it for downstream experiments, minimal consumer tests, and extraction
preparation. Use the built-in tuning suites when the output must feed the
current router report without an adapter.

## Catalog Extensibility Limits

The in-tree family catalog is intentionally separate from external suite
assembly. Built-in local, smoke, staged, comparison, head-to-head, diffusion,
and model-lane suites live behind `builtin_suites` and remain the source of the
standard `tuning_matrix` corpus. External suites do not import those modules
unless they intentionally want the maintained Jacquard corpus.

There is no crates/simulator/EXTERNAL_API.md. The external usage contract is
documented here and in [Running Simulations](501_running_simulations.md), so a
downstream developer can work from the 500-series guides without chasing a
crate-local API note.

## Diffusion Suites

Diffusion suites use the same runner facade and the standard diffusion artifact
writer. A downstream crate builds a `DiffusionSuite` from
`CustomDiffusionRunSpec` values. Each run provides a
`CustomDiffusionScenarioSpec`, a `DiffusionPolicyConfig`, and an explicit seed.

```rust
use jacquard_simulator::{
    ArtifactSink, CustomDiffusionRunSpec, DiffusionSuite, ExperimentRunner,
};

let suite = DiffusionSuite::from_custom_runs(
    "custom-diffusion",
    vec![CustomDiffusionRunSpec {
        family_id: "external-diffusion-family".to_string(),
        seed: 41,
        policy,
        scenario,
    }],
)
.expect("valid diffusion suite");

let artifacts = ExperimentRunner::default()
    .run_diffusion_suite(&suite, &ArtifactSink::directory("artifacts/custom-diffusion"))
    .expect("run diffusion suite");
assert_eq!(artifacts.manifest.run_count, 1);
```

The writer emits diffusion_manifest.json, diffusion_runs.jsonl,
diffusion_aggregates.json, and diffusion_boundaries.json. The manifest
contains `schema_version: 1`. Existing `analysis/` plots depend on this
standard diffusion layout, so extraction work must preserve these files or
provide a compatibility writer.

External diffusion families whose id starts with `external-` use a deterministic
default contact model. Maintained Jacquard families continue to use their
family-specific contact probabilities.

## Artifact Compatibility

The route-visible report path consumes the standard in-tree files:
manifest.json, runs.jsonl, aggregates.json, breakdowns.json, optional
model_artifacts.jsonl, and the diffusion files listed above. Both
manifest.json and diffusion_manifest.json carry explicit schema versions.
Schema changes must be additive or accompanied by a compatibility writer before
the `analysis/` report can move.

The external route-visible facade writes external_manifest.json and
external_runs.jsonl; those files are intentionally not consumed by the
current `analysis/` report. They are for downstream harnesses, consumer tests,
and future extraction work.

The former paper-facing research artifacts now live in DualTide. Jacquard's
simulator keeps the route-visible and diffusion artifact files consumed by the
maintained `analysis/` report, including the multi-router comparison corpus.
Moving or pruning research code must not remove or rename artifacts required by
`analysis/`.

## Review Guidance

When reviewing report output, prefer the PDF report and CSV tables over any single composite score. Use the transition table to distinguish robust settings from lucky averages. Use the boundary table to see where an engine stops being acceptable. Rerun the matrix after meaningful engine, router, or simulator changes before updating defaults.
