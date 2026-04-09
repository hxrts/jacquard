# Simulator

`jacquard-simulator` is the deterministic scenario harness for Jacquard. It
reuses the same core ownership model as the host bridge:

- hosts own transport drivers
- the bridge stamps ingress with Jacquard `Tick`
- the router advances through explicit synchronous rounds
- engines keep private runtime state below the shared routing boundary

The simulator is intentionally two-lane:

- `pathway` runs through a Telltale-backed lane because the engine runtime is
  choreography-driven internally
- `batman` runs through a plain deterministic-round lane because it is a
  proactive next-hop state machine rather than a choreography
- mixed-engine scenarios host both through one shared Jacquard harness

The crate models its top-level integration after Telltale's simulator harness:

- a pure `RoutingScenario`
- a pure `RoutingEnvironmentModel`
- a host adapter that builds concrete hosts
- one effectful `RoutingSimulator`
- one replay-visible artifact surface

## Reused Surfaces

The simulator should reuse existing Jacquard composition surfaces instead of
maintaining a simulator-only stack:

- `jacquard-reference-client` for host bridge ownership and round advancement
- `jacquard-adapter` for queueing and adapter support primitives
- `jacquard-mem-link-profile` for in-memory transport composition
- `jacquard-mem-node-profile` and `reference-client::topology` for fixture
  topology authoring

## Replay Artifacts

Replay artifacts should capture:

- environment traces and applied hooks
- ingress-batch boundaries, host-round outcomes, and next-round hints
- dropped-ingress driver status events
- shared `RouteEvent` and `RouteEventStamped` outputs
- deterministic checkpoints for replay/resume

For the current `pathway` lane, checkpoints also carry the host snapshots needed
to rebuild the bridge and recover checkpointed route state. The Jacquard replay
artifact reserves an optional slot for richer Telltale-native replay or
checkpoint references when the underlying lane can expose them. Non-choreography
engines must not be forced to expose Telltale-native internals to participate
in simulation.

## Starter Path

1. Build a scenario with `jacquard_simulator::presets`.
2. Build the paired `ScriptedEnvironmentModel`.
3. Run it through `JacquardSimulator`.
4. Inspect the replay artifact through `RoutingReplayView`.
