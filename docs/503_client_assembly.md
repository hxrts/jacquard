# Client Assembly

This guide covers using `jacquard-reference-client` as a library in a downstream application, and wrapping it in a small binary for standalone deployment. It also covers the two composition-time swaps a 3rd party is most likely to reach for: a custom `PolicyEngine` and a custom `CommitteeSelector`.

See [Reference Client](407_reference_client.md) for the implementation spec this guide builds on. See [Profile Implementations](305_profile_reference.md) for the profile boundary and [Crate Architecture](999_crate_architecture.md) for the workspace ownership rules.

## Adding The Dependency

`jacquard-reference-client` tracks the workspace version `0.6.0`. Add it alongside the core and trait crates a consumer typically imports types from.

```toml
[dependencies]
jacquard-reference-client = "0.6.0"
jacquard-core = "0.6.0"
jacquard-traits = "0.6.0"
```

The reference client re-exports node and link profile types from `jacquard-mem-node-profile` and `jacquard-mem-link-profile`, so most consumers do not need direct dependencies on those crates.

## Building A Client As A Library

`ClientBuilder` exposes one constructor per engine choice. Call the constructor for the lane needed, apply optional overrides through the `with_*` builder methods, then call `build` to produce a `ReferenceClient`.

```rust
use jacquard_core::{Observation, Configuration, NodeId, Tick};
use jacquard_reference_client::{ClientBuilder, SharedInMemoryNetwork};

let network = SharedInMemoryNetwork::default();
let client = ClientBuilder::pathway(
    NodeId([1; 32]),
    topology,
    network.clone(),
    Tick(0),
)
.with_queue_config(Default::default())
.build()
.expect("build pathway client");
```

The single-engine constructors are `pathway`, `batman_bellman`, `batman_classic`, `babel`, `olsrv2`, `field`, and `scatter`. The multi-engine constructor `all_engines` registers every in-tree engine on one client. Per-engine parameter overrides flow through `with_batman_bellman_decay_window`, `with_babel_decay_window`, `with_pathway_search_config`, `with_field_search_config`, and similar builders.

The built `ReferenceClient` bundles a router, the configured engines, and the host bridge. Bind it once with `.bind()`, then advance synchronous rounds. Every round drains ingress, runs engine and router logic, and flushes the outbound queue through the bridge-owned transport driver.

## Running A Client As A Binary

A minimal binary wraps the library. The `main` function constructs the builder, binds the bridge, and drives rounds on a cadence the application chooses.

```rust
use jacquard_core::Tick;
use jacquard_reference_client::{ClientBuilder, SharedInMemoryNetwork};
use std::thread::sleep;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let network = SharedInMemoryNetwork::default();
    let mut client = ClientBuilder::pathway(node_id, topology, network, Tick(0))
        .build()?;
    let mut bound = client.bind();
    loop {
        bound.advance_round()?;
        sleep(Duration::from_millis(100));
    }
}
```

The cadence is application policy. A standalone node drives rounds on a wall-clock cadence. A test harness drives rounds as fast as the scenario allows. A batch integration advances rounds whenever new ingress arrives.

The bridge owns transport ingress and `Tick` stamping. A binary wrapper must not call the transport driver directly or advance `Tick` inside engine code. External transports must implement the shared effect traits described in [Custom Transport](505_custom_transport.md). A binary that wires a non-default transport plugs it into the builder at construction time rather than patching it in later.

## Customizing The Routing Policy

`PolicyEngine` converts a `RoutingObjective` plus local inputs into `SelectedRoutingParameters`. The default reference client uses a neutral policy appropriate for the enabled engine set. A consumer overrides protection, connectivity, or mode decisions by implementing the trait and passing the instance during construction.

```rust
use jacquard_core::{RoutingObjective, RoutingPolicyInputs, SelectedRoutingParameters};
use jacquard_traits::PolicyEngine;

struct StrictProtectionPolicy;

impl PolicyEngine for StrictProtectionPolicy {
    fn compute_profile(
        &self,
        objective: &RoutingObjective,
        inputs: &RoutingPolicyInputs,
    ) -> SelectedRoutingParameters {
        // derive a profile that raises the protection floor
        todo!()
    }
}
```

The custom policy replaces the default during composition. The routing pipeline consults it once per route activation, so the trait stays stateless from the router's perspective. Policies that need per-activation state carry that state inside the trait implementor.

## Swapping The Committee Selector

`CommitteeSelector` chooses the committee for an objective that requires coordinated membership. Pathway is the only current consumer through `CommitteeCoordinatedEngine`. A custom selector returns `Option<CommitteeSelection>` or `None` when no committee applies.

```rust
use jacquard_core::{CommitteeSelection, Observation, RoutingObjective, SelectedRoutingParameters};
use jacquard_traits::CommitteeSelector;

struct LocalityBiasedSelector;

impl CommitteeSelector for LocalityBiasedSelector {
    type TopologyView = jacquard_core::Configuration;

    fn select_committee(
        &self,
        objective: &RoutingObjective,
        profile: &SelectedRoutingParameters,
        topology: &Observation<Self::TopologyView>,
    ) -> Result<Option<CommitteeSelection>, jacquard_core::RouteError> {
        todo!()
    }
}
```

The selector attaches to the pathway engine through the composition API. For the pathway-specific committee coordination contract, see [Pathway Routing](404_pathway_routing.md). Engines that do not use committee coordination ignore the selector.

## Going Further

For building a custom engine that plugs into this same `ClientBuilder`, see [Custom Engine](504_custom_engine.md). For replacing the default in-memory transport, see [Custom Transport](505_custom_transport.md). For authoring a new device profile, see [Custom Device](506_custom_device.md). For a capstone example that threads all three together, see [Bringing It Together](507_bringing_it_together.md).
