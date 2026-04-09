# Simulator

`jacquard-simulator` is the deterministic scenario harness for Jacquard. It reuses the same core ownership model as the host bridge.

- Hosts own transport drivers.
- The bridge stamps ingress with Jacquard `Tick`.
- The router advances through explicit synchronous rounds.
- Engines keep private runtime state below the shared routing boundary.

The simulator runs two lanes. The `pathway` engine uses a Telltale-backed lane. Its runtime is choreography-driven internally. The `batman` engine uses a plain deterministic-round lane.

The `batman` engine is a proactive next-hop state machine. Mixed-engine scenarios host both through one shared Jacquard harness. The top-level integration organizes around a pure `RoutingScenario`, a pure `RoutingEnvironmentModel`, a host adapter, one effectful `RoutingSimulator`, and one replay-visible artifact surface.

## Reused Surfaces

The simulator reuses existing Jacquard composition surfaces. It does not maintain a simulator-only stack.

- `jacquard-reference-client` for host bridge ownership and round advancement
- `jacquard-adapter` for queueing and adapter support primitives
- `jacquard-mem-link-profile` for in-memory transport composition
- `jacquard-mem-node-profile` and `reference-client::topology` for fixture topology authoring

## Replay Artifacts

Replay artifacts capture the observable state of each simulation run.

- environment traces and applied hooks
- ingress-batch boundaries, host-round outcomes, and next-round hints
- dropped-ingress driver status events
- shared `RouteEvent` and `RouteEventStamped` outputs
- deterministic checkpoints for replay and resume

For the `pathway` lane, checkpoints carry host snapshots. These snapshots are needed to rebuild the bridge and recover checkpointed route state. The Jacquard replay artifact includes an optional slot for Telltale-native replay references. Non-choreography engines do not expose Telltale-native internals to the simulation harness.

## Starter Path

1. Build a scenario with `jacquard_simulator::presets`.
2. Build the paired `ScriptedEnvironmentModel`.
3. Run it through `JacquardSimulator`.
4. Inspect the replay artifact through `RoutingReplayView`.
