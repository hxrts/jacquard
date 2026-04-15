# Simulator

`jacquard-simulator` is the deterministic scenario harness for Jacquard. It reuses the same core ownership model as the host bridge. The four core types are `JacquardScenario`, `ScriptedEnvironmentModel`, `JacquardSimulator`, and `JacquardReplayArtifact`.

Hosts own transport drivers. The bridge stamps ingress with Jacquard `Tick`. The router advances through explicit synchronous rounds. Engines keep private runtime state below the shared routing boundary.

The simulator selects engines per host through `EngineLane`. Available lanes include single-engine variants (`Pathway`, `BatmanBellman`, `BatmanClassic`, `Babel`, `OlsrV2`, `Scatter`, `Field`) and mixed-engine variants (`PathwayAndBatmanBellman`, `PathwayAndBabel`, `PathwayAndOlsrV2`, `PathwayAndField`, `BabelAndBatmanBellman`, `OlsrV2AndBatmanBellman`, `FieldAndBatmanBellman`, `AllEngines`). All engines share one host bridge per node.

The simulator also owns the maintained tuning and diffusion harnesses. The `tuning_matrix` binary runs scenario sweeps, writes deterministic artifacts under `artifacts/analysis/`, and automatically generates the analysis report. The tuning methodology and current recommendations live in [Routing Tuning](502_tuning.md).

## Reused Surfaces

The simulator reuses existing Jacquard composition surfaces. It does not maintain a simulator-only stack.

`jacquard-reference-client` provides host bridge ownership and round advancement. `jacquard-adapter` provides queueing and adapter support primitives. `jacquard-mem-link-profile` provides in-memory transport composition. `jacquard-mem-node-profile` and `reference-client::topology` provide fixture topology authoring.

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

`JacquardSimulator::run_scenario()` returns a `JacquardReplayArtifact` and a `JacquardSimulationStats`. The artifact captures the complete observable state of the run.

- environment traces and applied hooks per round
- ingress-batch boundaries and host-round outcomes
- `RouteEvent` and `RouteEventStamped` outputs
- `DriverStatusEvent` records for dropped ingress
- deterministic checkpoints with host snapshots
- failure summaries for diagnostic inspection

For the `pathway` lane, checkpoints carry `InMemoryRuntimeEffects` snapshots per host. These snapshots are needed to rebuild the bridge and recover checkpointed route state. Simulations can be resumed from the last checkpoint using `JacquardSimulator::resume_replay()`. Non-choreography engines do not expose Telltale-native internals to the simulation harness.

## Starter Path

1. Build a `JacquardScenario` and `ScriptedEnvironmentModel` with `jacquard_simulator::presets`.
2. Pass them to `JacquardSimulator::run_scenario()`.
3. Inspect the returned `JacquardReplayArtifact` for round, event, and checkpoint data.
4. For matrix sweeps, run `cargo run --bin tuning_matrix -- local` and review the generated report at `artifacts/analysis/local/latest/router-tuning-report.pdf`.
