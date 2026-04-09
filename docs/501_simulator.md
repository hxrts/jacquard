# Simulator

`jacquard-simulator` is the deterministic scenario harness for Jacquard. It reuses the same core ownership model as the host bridge. The four core types are `JacquardScenario`, `ScriptedEnvironmentModel`, `JacquardSimulator`, and `JacquardReplayArtifact`.

- Hosts own transport drivers.
- The bridge stamps ingress with Jacquard `Tick`.
- The router advances through explicit synchronous rounds.
- Engines keep private runtime state below the shared routing boundary.

The simulator runs two lanes, selected per host through `EngineLane`. The `pathway` engine uses a Telltale-backed lane. Its runtime is choreography-driven internally. The `batman` engine uses a plain deterministic-round lane.

The `batman` engine is a proactive next-hop state machine. Mixed-engine scenarios use `EngineLane::PathwayAndBatman`. Both engines share one host bridge.

## Reused Surfaces

The simulator reuses existing Jacquard composition surfaces. It does not maintain a simulator-only stack.

- `jacquard-reference-client` for host bridge ownership and round advancement
- `jacquard-adapter` for queueing and adapter support primitives
- `jacquard-mem-link-profile` for in-memory transport composition
- `jacquard-mem-node-profile` and `reference-client::topology` for fixture topology authoring

## Environment Model

`ScriptedEnvironmentModel` schedules environment changes as `EnvironmentHook` values keyed to specific ticks. Five hook variants are available. Applied hooks appear in each `JacquardRoundArtifact` for replay and inspection.

- `ReplaceTopology` swaps the full network configuration at a given tick.
- `MediumDegradation` adjusts delivery confidence and loss on a link between two nodes.
- `Partition` removes reachability between two nodes.
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
