# Simulator Developer Guide

This guide is for Rust developers who want to depend on `jacquard-simulator` as a library and script their own deterministic routing scenarios. It walks through dependency setup, the minimal run path, custom scenario construction, environment hook scripting, engine selection per host, replay inspection, assertions, and checkpoint resume.

See [Simulator](501_simulator.md) for the harness at a glance, [Routing Tuning](502_tuning.md) for the maintained analysis corpus driven by the `tuning_matrix` binary, and [Crate Architecture](999_crate_architecture.md) for the workspace ownership and boundary rules that constrain what the simulator exposes.

## Adding the Dependency

`jacquard-simulator` tracks the workspace version `0.6.0`. Add it alongside the core and trait crates a consumer typically imports types from.

```toml
[dependencies]
jacquard-simulator = "0.6.0"
jacquard-core = "0.6.0"
jacquard-traits = "0.6.0"
```

Building topology and profile observations also requires `jacquard-mem-node-profile` and `jacquard-mem-link-profile`. These crates provide the `NodePreset`, `NodeIdentity`, and `LinkPreset` builders documented in [Profile Implementations](305_profile_reference.md).

## Running a Preset Scenario

The fastest path to a running simulation is to pair a preset from `jacquard_simulator::presets` with the default `ReferenceClientAdapter`.

```rust
use jacquard_simulator::{
    presets, JacquardSimulator, ReducedReplayView,
    ReferenceClientAdapter, ScenarioAssertions,
};
use jacquard_traits::RoutingSimulator;

let (scenario, environment) = presets::pathway_line();
let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
let (replay, stats) = simulator
    .run_scenario(&scenario, &environment)
    .expect("run pathway scenario");
let reduced = ReducedReplayView::from_replay(&replay);
assert!(stats.executed_round_count > 0);
```

`presets::pathway_line` returns a three-node line topology, three Pathway hosts, and a scripted environment that applies degradation, intrinsic limits, partition, and mobility relink hooks across seven rounds. `JacquardSimulator::run_scenario` advances the bridge round by round and returns a full `JacquardReplayArtifact` plus a compact `JacquardSimulationStats`. `ReducedReplayView::from_replay` projects the replay into the analysis-facing surface used by assertions and post-run tooling.

The preset set also includes single-engine lines for every in-tree engine, mixed-engine variants such as `all_engines_line` and `mixed_line`, regression fixtures, and composition fixtures. See `crates/simulator/src/presets/` for the full index.

## Building a Custom Scenario

A scenario takes four pieces. It needs an initial topology observation, an ordered host roster, an ordered list of bound objectives, and a round limit. Topology comes from `jacquard-mem-node-profile` and `jacquard-mem-link-profile` builders documented in [Profile Implementations](305_profile_reference.md). The other three are scenario-level constructs.

```rust
use jacquard_core::{NodeId, OperatingMode, SimulationSeed};
use jacquard_simulator::{BoundObjective, HostSpec, JacquardScenario};

let scenario = JacquardScenario::new(
    "custom-pair",
    SimulationSeed(42),
    OperatingMode::FieldPartitionTolerant,
    topology,
    vec![
        HostSpec::pathway(NodeId([1; 32])),
        HostSpec::pathway(NodeId([2; 32])),
    ],
    vec![BoundObjective::new(NodeId([1; 32]), move_objective(NodeId([2; 32])))],
    8,
)
.with_checkpoint_interval(2);
```

`JacquardScenario::new` consumes the positional arguments above. The `with_checkpoint_interval` builder enables deterministic mid-run checkpoints used by `resume_replay` and by cross-run replay fidelity checks. The `move_objective` helper in the snippet is application code that populates a `RoutingObjective` struct. See `crates/simulator/src/presets/common.rs` for the `connected_objective` and `service_objective` patterns the in-tree presets use.

Additional builders cover less common shapes. `with_initial_configuration` and `with_round_limit` override their positional counterparts after construction. `with_topology_lags` models asymmetric observation delays per host. `with_broker_nodes` marks a subset of nodes as explicit brokers.

## Scheduling Environment Changes

`ScriptedEnvironmentModel::new` accepts a list of `ScheduledEnvironmentHook` values, each pairing a `Tick` with an `EnvironmentHook`. Hooks fire deterministically at their scheduled tick and are recorded into the replay as `AppliedEnvironmentHook` artifacts.

```rust
use jacquard_core::{NodeId, RatioPermille, Tick};
use jacquard_simulator::{EnvironmentHook, ScheduledEnvironmentHook,
    ScriptedEnvironmentModel};

let environment = ScriptedEnvironmentModel::new(vec![
    ScheduledEnvironmentHook::new(Tick(3), EnvironmentHook::MediumDegradation {
        left: NodeId([1; 32]),
        right: NodeId([2; 32]),
        confidence: RatioPermille(800),
        loss: RatioPermille(150),
    }),
    ScheduledEnvironmentHook::new(Tick(5), EnvironmentHook::Partition {
        left: NodeId([1; 32]),
        right: NodeId([2; 32]),
    }),
]);
```

The `EnvironmentHook` variants cover the bulk of practical perturbations. `ReplaceTopology` swaps the full configuration at a given tick. `MediumDegradation` and `AsymmetricDegradation` lower link quality either symmetrically or per direction.

`Partition` and `CascadePartition` remove one or several directed links. `MobilityRelink` simulates node movement by redirecting a link onto a new peer. `IntrinsicLimit` enforces per-node connection and hold-capacity ceilings.

## Selecting Engines Per Host

Each `HostSpec` carries an `EngineLane` that names the engine or engine set that host runs. The scenario constructor accepts a mixed roster. The reference client adapter composes a router plus the selected engine crates per host.

```rust
use jacquard_simulator::HostSpec;

let roster = vec![
    HostSpec::pathway(owner),
    HostSpec::pathway_and_batman_bellman(relay),
    HostSpec::batman_bellman(edge),
];
```

Each constructor returns a `HostSpec` with a sensible default overrides bundle. Single-engine constructors include `pathway`, `field`, `batman_bellman`, `batman_classic`, `babel`, `olsrv2`, and `scatter`. Multi-engine constructors include the pairwise composites plus `all_engines`. Per-host knobs apply through `.with_profile`, `.with_policy_inputs`, `.with_batman_bellman_decay_window`, and similar builders.

## Inspecting Replay Artifacts

`JacquardReplayArtifact` carries the full per-round record. It holds the scenario, the scripted environment, ordered round artifacts, route events, driver status events, failure summaries, and optional checkpoints. Each `JacquardRoundArtifact` exposes the topology snapshot for that round, the applied environment hooks, and one `HostRoundArtifact` per host.

```rust
for round in &replay.rounds {
    for host in &round.host_rounds {
        for route in &host.active_routes {
            println!(
                "{:?} -> {:?} via {:?}",
                host.local_node_id, route.destination, route.next_hop_node_id,
            );
        }
    }
}
```

For analysis work, convert the full replay into the reduced surface through `ReducedReplayView::from_replay(&replay)`. To trade detail for throughput at capture time, call `JacquardSimulator::run_scenario_with_capture` with `SimulationCaptureLevel::FullReplay`, `ReducedReplay`, or `SummaryOnly`. Summary-only runs still produce `JacquardSimulationStats` without materializing per-round artifacts.

## Asserting Expectations

`ScenarioAssertions` is a builder that turns a reduced replay into a pass or fail outcome. Each `.expect_*` method accumulates one rule. The `.evaluate(&reduced)` call runs them all.

```rust
use jacquard_core::DestinationId;
use jacquard_pathway::PATHWAY_ENGINE_ID;
use jacquard_simulator::ScenarioAssertions;

ScenarioAssertions::new()
    .expect_route_materialized(owner, DestinationId::Node(target))
    .expect_engine_selected(owner, DestinationId::Node(target), &PATHWAY_ENGINE_ID)
    .expect_distinct_engine_count(1)
    .evaluate(&reduced)
    .expect("pathway assertions");
```

The available rules are `expect_route_materialized`, `expect_route_absent`, `expect_engine_selected`, `expect_distinct_engine_count`, and `expect_recovery_within_rounds`. Failures surface as an `AssertionFailure` carrying a structured detail string, ready to bubble through `Result` chains or to drive integration-test diagnostics.

## Resuming From A Checkpoint

Enabling checkpoints on the scenario through `with_checkpoint_interval(n)` tells the simulator to snapshot host state every `n` rounds. Given a completed replay, `resume_replay` reconstructs the bridge at the last checkpoint and advances again through the remaining rounds.

```rust
let mut simulator = JacquardSimulator::new(ReferenceClientAdapter);
let (replay, _) = simulator.run_scenario(&scenario, &environment)?;
let (resumed, _) = simulator.resume_replay(&replay)?;
assert_eq!(
    replay.rounds.last().map(|r| &r.topology),
    resumed.rounds.last().map(|r| &r.topology),
);
```

Resume is the primary determinism check. Identical replay tails across the original and resumed runs confirm that nothing in the composed engine state leaked ambient time or host-dependent ordering. The `crates/simulator/tests/phase0_determinism.rs` suite uses this pattern across every preset.

## Going Further

Custom host adapters implement the `JacquardHostAdapter` trait and replace `ReferenceClientAdapter` when the harness needs something other than the reference client composition. Most 3rd-party consumers will not need this seam. Reach for it when testing against a host that already exists outside the standard bridge.

For parameter sweeps, the `tuning_matrix` binary at `crates/simulator/src/bin/tuning_matrix.rs` is the canonical reference. It composes experiment and diffusion suites, runs them in parallel, and writes artifacts to `artifacts/analysis/{suite}/{timestamp}/` for the Python report pipeline described in [Routing Tuning](502_tuning.md).

For the boundary rules that the simulator works within, see [Crate Architecture](999_crate_architecture.md). For a deeper discussion of the simulator internals, see `crates/simulator/ARCHITECTURE.md` in the repository.
