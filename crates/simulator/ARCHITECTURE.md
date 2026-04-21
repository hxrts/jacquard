# `jacquard-simulator` Architecture

`jacquard-simulator` is the deterministic scenario, replay, and experiment harness for Jacquard.

It sits above the shared routing boundaries and reuses the real reference-client host composition. It does not own canonical route truth. The router and engine crates still own live routing semantics; the simulator owns scenario description, execution orchestration, and post-run artifacts.

This document assumes the runtime-state partitioning design in [work/runtime_state_partitioning.md](../../work/runtime_state_partitioning.md): engines keep protocol-private state, but candidate generation and transition logic move toward explicit planner snapshots and pure reducers that the simulator can drive directly.

For workspace-wide boundary rules, see [docs/999_crate_architecture.md](../../docs/999_crate_architecture.md). For the broader simulator architecture and workflow, see [docs/306_simulator_architecture.md](../../docs/306_simulator_architecture.md) and [docs/501_running_simulations.md](../../docs/501_running_simulations.md).

## Main Structure

The crate has four main areas:

- **Route-visible full-stack simulation**
  - scenario module
  - environment module
  - harness modules
  - replay module
  - reduced replay module
- **Route-visible experiment suites**
  - [`src/experiments/`](./src/experiments)
- **Diffusion simulation**
  - [`src/diffusion/`](./src/diffusion)
- **Model-lane fixtures**
  - model-lane fixture module

Supporting helpers live in the topology, assertions, presets, and utility modules.

## Architectural Shape

The simulator is intentionally a **dual-lane harness**:

- **Full-stack lane**
  - builds real `ReferenceClient` hosts
  - advances bridges synchronously round by round
  - captures full replay, reduced replay, or summary-only artifacts
  - remains authoritative for bridge cadence, ingress ordering, checkpoint I/O, and end-to-end route behavior

- **Model lane**
  - runs pure planner, reducer, and restore fixtures
  - uses explicit planner snapshots, normalized inputs, and typed transition results
  - exists to validate the same engine-owned decision logic without host-runtime wiring

The model lane is not a second implementation of routing logic. It is a harness for engine-owned pure surfaces.

## Core Flow

### Route-visible full-stack

1. Build a `JacquardScenario`.
2. Pair it with a `ScriptedEnvironmentModel`.
3. Build hosts through `ReferenceClientAdapter`.
4. Advance the environment once per round.
5. Share one immutable topology snapshot with all hosts for that round.
6. Advance each host bridge synchronously.
7. Collect route events, host artifacts, failures, and optional checkpoints.
8. Emit full replay, reduced replay, or summary-only output.

### Route-visible experiments

1. Assemble an `ExperimentSuite`.
2. Expand maintained family descriptors and parameter sets into run specs.
3. Execute runs in parallel.
4. Sort results back into deterministic suite order.
5. Reduce runs into stable summaries and write artifacts.

### Diffusion

1. Build a `DiffusionSuite`.
2. Materialize deterministic scenario and policy combinations.
3. Run the diffusion runtime state machine.
4. Aggregate summaries and boundary surfaces.
5. Write stable report-facing artifacts.

## Ownership And Separation Of Concerns

These boundaries should hold:

- the scenario module describes what to run, not how to run it
- the environment module owns deterministic world evolution, not routing policy
- the harness modules own route-visible execution, not family catalog design
- the experiment modules own maintained route-visible analytical families and result reduction
- the diffusion modules own diffusion-only schema, runtime, posture, scoring, and summaries
- the model-lane module owns pure-lane fixture vocabulary only
- Engine-private protocol state stays in the engine crates, not in the simulator.

The runtime-state partitioning assumption is:

- planner snapshot: smallest read-only route-choice summary
- runtime state: full mutable protocol state
- checkpoint state: durable subset of runtime state
- derived caches: rebuildable indexes and memoized tables

The simulator may drive these surfaces, compare model-lane and full-stack behavior, and record their outputs, but it must not become the owner of engine-private protocol state.

## Key Invariants

### Determinism

- No floating-point types in simulator verdicts, summaries, or reducers.
- No ambient randomness. Random-looking behavior must come from explicit seeds and deterministic machinery.
- No host-dependent ordering. Persisted and compared outputs must use stable ordering.
- Parallel execution may reduce wall-clock time, but it must not change results or artifact order.

### Time And Topology

- `Tick` is the simulator time boundary.
- Environment hooks fire at explicit ticks.
- Links are modeled as directed edges keyed by `(from, to)`.
- Topology epoch changes reflect environment mutation, not arbitrary host-side effects.

### Execution Ownership

- Route-visible full-stack simulation must reuse the real reference-client bridge composition.
- The simulator must not invent simulator-only route ownership, transport ownership, or route-health semantics.
- The model lane may execute pure planner and reducer logic directly, but only through engine-owned explicit inputs and outputs.

### Replay And Summary Semantics

- Full replay is the debugging and replay-fidelity artifact.
- Reduced replay is the analysis-facing route/environment surface.
- Summary-only execution exists for suite throughput and must not change behavior.
- Summary reducers should derive from replay-visible facts, not hidden simulator bookkeeping.
- `model_artifacts.jsonl` is an additive model-lane artifact. It does not replace
  the maintained full-stack run log plus aggregate and breakdown JSON outputs
  that the report pipeline already consumes.

## Interface Contracts

### `JacquardScenario`

Must provide:

- initial topology observation
- deterministic seed
- ordered host roster
- ordered objectives
- round limit and scenario metadata

Host order is meaningful and must be preserved through host build planning.

### `ScriptedEnvironmentModel`

Must remain a deterministic mapping from `(configuration, tick)` to:

- next topology observation
- applied hook artifacts

It may mutate directed links asymmetrically.

### `JacquardHostAdapter`

Is the host materialization seam. It should:

- build runnable hosts from a `JacquardScenario`
- preserve deterministic construction
- remain a thin build boundary rather than a second source of planner or reducer logic

### `JacquardSimulationHarness`

Owns route-visible full-stack execution. It should preserve:

- one environment advancement per round
- one shared topology snapshot per round
- one synchronous host-bridge advancement per host per round
- capture-level differences in bookkeeping only, not semantics

### Replay Types

Replay types are analysis-facing contracts. Their meaning should remain stable:

- `JacquardReplayArtifact` for replay-grade inspection
- `ReducedReplayView` for analysis and experiment reduction

### Experiment And Diffusion APIs

These APIs own maintained analytical surfaces. Their identifiers and summary fields should remain stable enough for the Python analysis and report pipeline.

### Model-Lane Fixtures

Model-lane fixtures should remain:

- pure
- generic over engine-private state
- free of host-runtime wiring
- suitable for equivalence checks against the full-stack lane

## Extension Guidance

Use the right extension point:

- add a **preset** for reusable small test/example scenarios
- add an **experiment family** for maintained route-visible report questions
- add a **diffusion family** for bounded-delivery questions
- add a **model-lane fixture** for pure planner/reducer/restore validation

Prefer:

- deterministic descriptors
- explicit analytical questions
- stable identifiers
- replay-derived summaries
- explicit planner snapshots and reducer inputs where available

Avoid:

- simulator-only routing semantics
- ambient time or host-dependent behavior
- mixing report prose into scenario builders
- moving engine-private protocol state into the simulator

## Non-Negotiable Promises

- The crate remains deterministic.
- It remains above the shared routing boundaries.
- It continues to reuse the real reference-client composition for route-visible full-stack simulation.
- It keeps full-stack replay, diffusion analysis, and model-lane fixtures as separate concerns.
- It treats the model lane and full-stack lane as complementary views over the same engine-owned decision logic.
