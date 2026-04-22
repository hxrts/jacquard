# Simulator Architecture

`jacquard-simulator` is the deterministic scenario and experiment harness for Jacquard. It sits above the shared routing boundaries and reuses the real reference-client host composition. It does not own canonical route truth. Canonical ownership remains on the router and engine crates.

The four core types are `JacquardScenario`, `ScriptedEnvironmentModel`, `JacquardSimulator`, and `JacquardReplayArtifact`. See [Running Simulations](501_running_simulations.md) for the step-by-step walkthrough a library consumer follows.

## Internal Lanes

The simulator is a dual-lane harness. The full-stack lane drives the reference-client bridge and the real router and runtime composition. The model lane runs explicit planner snapshots, pure round reducers, pure maintenance reducers, and checkpoint fixtures without a host bridge.

The model lane does not replace the full-stack lane. It offers a cheaper path for deterministic planner and transition checks against engine-owned pure surfaces.

Execution has three modes. `full-stack` runs the maintained comparative families. `model` runs explicit fixture-driven planner, reducer, and restore checks. `equivalence` runs a model fixture and a full-stack replay for the same case and asserts that the visible decision matches.

## Engine Selection Per Host

The simulator selects engines per host through `EngineLane`. Single-engine variants cover `Pathway`, `BatmanBellman`, `BatmanClassic`, `Babel`, `OlsrV2`, `Scatter`, `Field`, and `Mercator`. Mixed-engine variants include `PathwayAndBatmanBellman`, `PathwayAndBabel`, `PathwayAndOlsrV2`, `PathwayAndField`, `BabelAndBatmanBellman`, `OlsrV2AndBatmanBellman`, `FieldAndBatmanBellman`, and `AllEngines`.

All engines share one host bridge per node. The bridge owns ingress draining and `Tick` stamping. Engines keep private runtime state below the shared routing boundary.

## Reused Surfaces

The simulator reuses existing Jacquard composition surfaces. It does not maintain a simulator-only stack.

`jacquard-reference-client` provides host bridge ownership and round advancement. `jacquard-host-support` provides queueing and host support primitives. `jacquard-mem-link-profile` provides in-memory transport composition. `jacquard-mem-node-profile` provides node profile authoring.

## Environment Model

`ScriptedEnvironmentModel` schedules environment changes as `EnvironmentHook` values keyed to specific ticks. Applied hooks appear in each `JacquardRoundArtifact` for replay and inspection.

- `ReplaceTopology` swaps the full network configuration at a given tick.
- `MediumDegradation` adjusts delivery confidence and loss on a link between two nodes.
- `AsymmetricDegradation` adjusts forward and reverse confidence and loss independently on a directed link.
- `Partition` removes reachability between two nodes.
- `CascadePartition` removes multiple directed links simultaneously.
- `MobilityRelink` replaces one link with another to model node movement.
- `IntrinsicLimit` adjusts connection count or hold capacity constraints on a node.

## Replay Artifacts

`JacquardSimulator::run_scenario` returns a `JacquardReplayArtifact` and a `JacquardSimulationStats`. The artifact captures the complete observable state of the run.

- environment traces and applied hooks per round
- ingress-batch boundaries and host-round outcomes
- `RouteEvent` and `RouteEventStamped` outputs
- `DriverStatusEvent` records for dropped ingress
- deterministic checkpoints with host snapshots
- failure summaries for diagnostic inspection

Checkpoints carry `InMemoryRuntimeEffects` snapshots per host. These snapshots are needed to rebuild the bridge and recover checkpointed route state across all engines. Pathway is the only lane that exposes Telltale-native replay references. Checkpoint resume works across all engines.

## Model-Lane Artifacts

Model-lane runs use their own fixture outputs instead of host-round replay artifacts. They record explicit planner snapshots, candidate counts, reducer summaries, restore outputs, and equivalence results in `model_artifacts.jsonl`. This makes equivalence checks against full-stack runs possible without introducing a simulator-only engine stack.

That file is additive. The maintained full-stack artifact contract remains the full-stack run log plus the aggregate and breakdown JSON outputs, plus the diffusion artifact set for deferred-delivery analysis. The report pipeline does not score `model_artifacts.jsonl`. It uses it for model-lane inspection and equivalence debugging only.

The model-lane selectors are `babel-model-smoke`, `babel-equivalence-smoke`, `batman-bellman-model-smoke`, `batman-classic-model-smoke`, `olsrv2-model-smoke`, `field-model-smoke`, `pathway-model-smoke`, and `scatter-model-smoke` in the `tuning_matrix` binary. They exercise engine-owned planner seeds, planner snapshots, reducer state, and restore inputs through the shared model-trait family.
