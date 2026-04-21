# Custom Device

This guide walks through adding a custom node: the device-side profile that advertises capabilities, exposes node state, and emits service descriptors. It targets 3rd parties modeling a device that is not covered by the in-memory node profile, for example a physical BLE peripheral, a constrained IoT endpoint, or a heterogeneous mix of hosts with different service surfaces.

See [Profile Implementations](305_profile_reference.md) for the shared profile boundary. See [Custom Transport](505_custom_transport.md) for the companion link side. See [Reference Client](407_reference_client.md) for the host bridge composition the custom profile plugs into.

## What A Device Profile Owns

A device profile in Jacquard has three outputs. It emits a `NodeProfile` describing capability advertisement. It emits `NodeState` describing live state like relay budget and hold capacity. It emits one or more `ServiceDescriptor` values describing the services the node exposes.

A profile does not plan routes, issue canonical handles, or publish route truth. Those stay on the router and engines. The profile stays observational: it describes what the device is, not what routes it selects.

Reuse the core vocabulary unchanged. `Node`, `NodeProfile`, `NodeState`, `Link`, `LinkEndpoint`, `LinkState`, and `ServiceDescriptor` keep their shared shape end to end. A custom profile wraps builders around those shared objects rather than introducing a parallel schema.

## Identity And Endpoint

`NodeIdentity` pairs a `NodeId` with a `ControllerId`. The node id is the routing identifier other hosts use to reach this device. The controller id is the cryptographic actor that authenticates for that node.

```rust
use jacquard_core::{ByteCount, ControllerId, LinkEndpoint, NodeId, Tick, TransportKind};
use jacquard_mem_node_profile::{NodeIdentity, NodePresetOptions};

let identity = NodeIdentity::new(
    NodeId([1; 32]),
    ControllerId([1; 32]),
);

let endpoint = jacquard_adapter::opaque_endpoint(
    TransportKind::WifiAware,
    vec![1],
    ByteCount(256),
);

let options = NodePresetOptions::new(identity, endpoint, Tick(1));
```

`TransportKind` names the carrier the endpoint speaks. Custom transports add new variants when a runtime needs one not already in core. `opaque_endpoint` keeps the endpoint opaque at the shared boundary; transport-specific endpoint builders belong in transport-owned profile crates.

Observation timing is explicit. Every built object carries `observed_at_tick`, which is the bridge-stamped time at which the profile observed this state. A profile that loses track of observation timing cannot drive deterministic scenarios.

## Profile And Capabilities

A `NodePreset` wraps the options into a full `Node`. The in-tree `route_capable` helper registers a single routing engine. The `route_capable_for_engines` variant registers several at once.

```rust
use jacquard_mem_node_profile::NodePreset;
use jacquard_pathway::PATHWAY_ENGINE_ID;

let node = NodePreset::route_capable(options, &PATHWAY_ENGINE_ID).build();
```

Engine eligibility is encoded through the node's service descriptors. Only engines whose `RoutingEngineId` appears in the node's service surface are eligible to produce route candidates toward that node. A custom device that implements a custom engine tags the engine id in its service descriptor; see [Custom Engine](504_custom_engine.md).

For a device profile to advertise multiple services or multi-engine eligibility, compose the descriptors manually through `SimulatedServiceDescriptor` builders. `ServiceKind` distinguishes `Discover`, `Move`, `Hold`, and other service classes.

## Node State

`NodeState` carries live per-node state. This includes relay budget, hold capacity, and current resource pressure. The profile refreshes node state on each observation tick.

```rust
use jacquard_core::{ByteCount, Tick};
use jacquard_mem_node_profile::NodeStateSnapshot;

let state = NodeStateSnapshot::route_capable(Tick(1))
    .with_hold_capacity(ByteCount(4096))
    .build();
```

A device that conserves resources sets conservative budgets. A device with slack exposes higher budgets. Engines observing the state use it as input to admission decisions. An engine that sees `hold_capacity_available_bytes = 0` will not treat the node as hold-capable for deferred delivery.

State changes are observations, not mutations. The profile emits a fresh snapshot each tick the state is relevant. It does not mutate a previously emitted snapshot in place. This keeps the observation flow consistent with the shared `Observation<T>` surface described in [Core Types](201_core_types.md).

## Integration

A full device profile composes the pieces into an `Observation<Configuration>` that the host bridge ingests. The observation carries the node map, the link map, and the environment. Custom profiles typically build this by combining the node and link outputs with the environment observed at the same tick.

The composed observation plugs into `ClientBuilder` through the topology argument. Every constructor accepts `Observation<Configuration>`, so a custom profile interoperates with the reference client and the simulator without extra wiring. See [Client Assembly](503_client_assembly.md) for the composition flow.

For provenance qualifiers on the emitted objects, `FactSourceClass`, `OriginAuthenticationClass`, and `IdentityAssuranceClass` let the profile describe where each observation came from and how strongly authenticated it is. See [Core Types](201_core_types.md) for the provenance surfaces the profile populates.
