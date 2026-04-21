# Running Experiments

This guide covers two kinds of work. The first half shows how to invoke the in-tree experiment suites through the `tuning_matrix` binary, where to find the artifacts, how to regenerate the report, and what the current corpus says about each engine. The second half shows how to assemble a custom suite programmatically when the in-tree families do not cover a 3rd party's analytical question.

See [Experimental Methodology](307_experimental_methodology.md) for what an experiment is, why the tuning phase runs first, and which variables are independent versus dependent. See [Simulator Architecture](306_simulator_architecture.md) for the harness the suites sit on, [Reference Client](408_reference_client.md) for the host composition, and [Running Simulations](501_running_simulations.md) for the base scenario flow each experiment builds on.

## Running The In-Tree Suites

Run the smaller smoke sweep:

```bash
cargo run --bin tuning_matrix -- smoke
```

Smoke runs a single seed across every family and completes quickly enough for a pre-PR sanity pass. Use it to confirm the binary builds and the families still produce non-empty artifacts.

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

The report lands at `artifacts/analysis/{suite}/latest/router-tuning-report.pdf`.

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

### Field

The Field sweep produces non-zero activation success and route presence at the router-visible boundary. The tested Field settings cluster closely, so the results should be read as a viable range rather than one sharply preferred point. The continuity profile table is the better place to choose between lower-churn and broader-reselection behavior.

### Mercator

Mercator is now represented on both the route-visible and diffusion surfaces. In route-visible head-to-head runs, it behaves as a corridor-maintenance engine: strong in connected high-loss and corridor-continuity families, viable but visibly constrained by bridge-transition and stale-recovery windows. In diffusion runs, its custody fallback is bounded rather than flood-like; the current corpus shows it can keep the broadcast overload and large congestion-threshold families viable after the protected bridge budget tuning.

### Mixed Comparison

The comparison regimes inform regime suitability rather than global winners. Low-loss connected-only cases favor the distance-vector stacks. Concurrent mixed workloads favor Pathway. High-loss bridge and some field-favorable comparison cases remain hard failure regimes across every tested stack.

### Head-To-Head Engine Sets

The head-to-head corpus runs the same regimes under explicit single-engine stacks: `batman-bellman`, `batman-classic`, `babel`, `olsrv2`, `scatter`, `mercator`, `pathway`, `field`, or `pathway-batman-bellman`. This is separate from the mixed-engine comparison, which asks which engine a router selects when several are available. Head-to-head asks what happens when only one is available.

## Assembling A Custom Suite

A 3rd party who needs an experiment the in-tree corpus does not cover assembles a suite directly against the simulator library. The main tools are `ExperimentSuite`, `ExperimentParameterSet`, `RegimeDescriptor`, and the `run_tuning_suite` entry point. All live in `jacquard_simulator` and are re-exported at the crate root.

```rust
use jacquard_simulator::{
    ExperimentParameterSet, ExperimentSuite, RegimeDescriptor,
    JacquardSimulator, ReferenceClientAdapter, run_tuning_suite,
};
use jacquard_core::SimulationSeed;

let mut suite = ExperimentSuite::new("custom-pathway-budget");
for seed in [SimulationSeed(41), SimulationSeed(43), SimulationSeed(47)] {
    for budget in [2u32, 3, 4, 6] {
        suite.add_run(
            RegimeDescriptor::default(),
            ExperimentParameterSet::pathway(budget),
            seed,
        );
    }
}

let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
let artifacts = run_tuning_suite(&mut simulator, &suite, "artifacts/custom-pathway/")
    .expect("run custom suite");
```

The call returns an `ExperimentArtifacts` handle that exposes the `runs.jsonl` path, the aggregate and breakdown summaries, and any model-lane artifacts written during the run. The artifact directory follows the same `{suite}/{timestamp}/` convention as the in-tree suites, so the Python report pipeline can ingest them without modification.

When the default `aggregate_runs` reduction does not answer the question being asked, the artifact layout supports two follow-up paths. A Rust post-processor can reload `runs.jsonl` and apply a custom aggregation. A Python analysis can derive alternate CSV tables or plots from the same artifact set through the `analysis/` package entry points.

## Catalog Extensibility Limits

The in-tree family catalog is currently crate-private, which means a 3rd party cannot add a new family directly through the library API without modifying the simulator crate. Two workarounds apply. One: upstream the family into `jacquard-simulator` itself so the catalog exposes it through the standard `tuning_matrix` entry. Two: duplicate the suite-assembly flow in a dependent crate, assembling runs with public `ExperimentSuite` and `ExperimentParameterSet` APIs.

The canonical reference for programmatic suite composition is the `tuning_matrix` binary at `crates/simulator/src/bin/tuning_matrix.rs`. It demonstrates CLI parsing, seed selection, stage filtering, suite dispatch, and report generation. A 3rd party assembling a custom harness outside the simulator crate can mirror its structure.

## Diffusion Suites

Diffusion suites follow the same shape as tuning suites with a different type family. `DiffusionSuite` assembles runs, `DiffusionPolicyConfig` parameterizes individual runs, and `run_diffusion_suite` executes them. The artifacts written are `diffusion_runs.jsonl` plus the diffusion aggregate and boundary summaries.

The in-tree diffusion catalog is similarly crate-private. The same two workarounds apply: upstream new scenarios into the simulator, or duplicate the suite-assembly flow in a dependent crate.

## Review Guidance

When reviewing report output, prefer the PDF report and CSV tables over any single composite score. Use the transition table to distinguish robust settings from lucky averages. Use the boundary table to see where an engine stops being acceptable. Rerun the matrix after meaningful engine, router, or simulator changes before updating defaults.
