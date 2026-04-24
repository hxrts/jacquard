# Running Simulations

This guide is for Rust developers who depend on `jacquard-simulator` as a library and want to script their own deterministic routing scenarios. We assume developers may use the simulator for three main purposes: running an existing preset to observe engine behavior, authoring a custom scenario to probe a specific condition, and driving an experiment suite to sweep parameters across a scenario family. This guide covers the first two, plus the shared tools for inspecting and asserting on replay results.

See [Simulator Architecture](306_simulator_architecture.md) for the architecture this guide sits on top of, [Reference Client](407_reference_client.md) for the host composition the default adapter uses, [Running Experiments](502_running_experiments.md) for the parameter sweep flow, and [Crate Architecture](999_crate_architecture.md) for the ownership and boundary rules.

## Adding the Dependency

`jacquard-simulator` tracks the workspace version `0.8.0`. Add it alongside the core and trait crates a consumer typically imports types from.

```toml
[dependencies]
jacquard-simulator = "0.8.0"
jacquard-core = "0.8.0"
jacquard-traits = "0.8.0"
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

`presets::pathway_line` returns a three-node line topology, three Pathway hosts, and a scripted environment that applies degradation, intrinsic limits, partition, and mobility relink hooks across seven rounds. `run_scenario` advances the bridge round by round and returns a full replay artifact plus a compact stats record. `ReducedReplayView::from_replay` projects the replay into the analysis-facing surface used by assertions and post-run tooling.

The preset set also includes single-engine lines for every in-tree engine, mixed-engine variants such as `all_engines_line` and `mixed_line`, regression fixtures, and composition fixtures. See `crates/simulator/src/presets/` for the full index.

## Building a Custom Scenario

A scenario takes four pieces. It needs an initial topology observation, an ordered host roster, an ordered list of bound objectives, and a round limit. Topology comes from the mem profile crates. The other three are scenario-level constructs.

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

`JacquardReplayArtifact` carries the full per-round record. It holds the scenario, the scripted environment, ordered round artifacts, route events, driver status events, failure summaries, and optional checkpoints. Each round artifact exposes the topology snapshot for that round, the applied environment hooks, and one host round artifact per host.

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

For analysis work, convert the full replay into the reduced surface through `ReducedReplayView::from_replay(&replay)`. To trade detail for throughput at capture time, call `run_scenario_with_capture` with `SimulationCaptureLevel::FullReplay`, `ReducedReplay`, or `SummaryOnly`. Summary-only runs produce `JacquardSimulationStats` without materializing per-round artifacts.

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

## Swapping the Host Adapter

The default `ReferenceClientAdapter` wires the reference client host into the simulator. Implement `JacquardHostAdapter` when the scenario needs a different host composition, for example a host that carries a custom transport or a different engine set than the reference client exposes.

```rust
use jacquard_core::NodeId;
use jacquard_reference_client::ReferenceClient;
use jacquard_simulator::{JacquardHostAdapter, JacquardScenario, SimulationError};
use std::collections::BTreeMap;

struct MyAdapter;

impl JacquardHostAdapter for MyAdapter {
    fn build_hosts(
        &self,
        scenario: &JacquardScenario,
    ) -> Result<BTreeMap<NodeId, ReferenceClient>, SimulationError> {
        todo!()
    }
}

let mut simulator = JacquardSimulator::new(MyAdapter);
```

Pass the custom adapter to `JacquardSimulator::new` instead of `ReferenceClientAdapter`. The rest of the scenario flow is identical. The canonical in-tree host wiring examples live under `crates/reference-client/tests/`.

## Going Further

For parameter sweeps across scenario families, see [Running Experiments](502_running_experiments.md). That guide covers both the in-tree `tuning_matrix` binary and custom suite assembly.

For composing the reference client outside the simulator harness, use `crates/reference-client/tests/` as the canonical wiring reference and [Profile Implementations](305_profile_reference.md) for profile vocabulary.

For the boundary rules the simulator works within, see [Crate Architecture](999_crate_architecture.md). For the internal dual-lane architecture, see [Simulator Architecture](306_simulator_architecture.md).
