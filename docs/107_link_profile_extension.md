# LinkProfile Extension Point

This page documents LinkProfile as a first-class extension surface for network-type implementations, parallel to NodeProfile for device implementations.

## Overview

LinkProfile describes the static capabilities of a directed link (connection from one node to another). Just as NodeProfile is provided by device/OS extensions and implemented in-memory by `jacquard-mem-node-profile`, LinkProfile is provided by network-type extensions and implemented in-memory by `jacquard-mem-link-profile`.

The key insight: profiles describe what a resource *can do*, while runtime state (LinkState, NodeState) describes what it's currently *doing*.

## LinkProfile Design

LinkProfile lives alongside LinkState in the `Link` object:

```rust
pub struct Link {
    pub endpoint: LinkEndpoint,
    pub state: LinkState,
    pub profile: LinkProfile,  // Static capabilities per directed link
}
```

LinkProfile captures the static properties of a network connection:

```rust
pub struct LinkProfile {
    pub protocol: TransportProtocol,
    pub codec_stack: Vec<CodecId>,
    pub mtu_floor_bytes: ByteCount,
    pub latency_floor_ms: DurationMs,
    pub repair_capability: RepairCapability,
    pub partition_recovery_supported: bool,
}
```

These values do not change for the lifetime of the link. They represent what the hardware and protocols *support*, not what's currently observed. LinkState carries the observed conditions.

## Asymmetric Links

Jacquard models directed links with asymmetry. A link from A → B has different characteristics than B → A. Both directions are modeled as separate Link objects in the Configuration, each with its own LinkProfile and LinkState.

The `symmetry_permille` field in LinkState measures how much the reverse path resembles the forward path, allowing routing engines to reason about asymmetry without duplicating observations for both directions.

Example:

```rust
// Forward direction: A → B
let link_a_to_b = Link {
    endpoint: LinkEndpoint { /* B's address */ },
    state: LinkState {
        symmetry_permille: RatioPermille(800),  // B → A is about 80% symmetric
        // ... other state
    },
    profile: LinkProfile {
        protocol: TransportProtocol::WifiDirect,
        mtu_floor_bytes: ByteCount(1500),
        // ... other capabilities
    },
};

// Reverse direction: B → A
let link_b_to_a = Link {
    endpoint: LinkEndpoint { /* A's address */ },
    state: LinkState {
        symmetry_permille: RatioPermille(800),  // A → B is about 80% symmetric
        // ... other state
    },
    profile: LinkProfile {
        protocol: TransportProtocol::WifiDirect,
        mtu_floor_bytes: ByteCount(1500),
        // ... may differ from A → B profile
    },
};
```

## Extension Pattern

To implement LinkProfile for a new transport protocol:

1. **Define the LinkProfile struct** — capture the static capabilities that your protocol guarantees
2. **Implement observation conversion** — your transport integration observes current link health (LinkState) and feeds it alongside your LinkProfile definition
3. **Reference jacquard-mem-link-profile** — this in-tree crate shows the canonical pattern for in-memory LinkProfile implementation

See [`crates/mem-link-profile`](../../crates/mem-link-profile) for a complete working example with:

- SimulatedLinkProfile builder for constructing profiles in tests
- LinkEndpoint and LinkState modeling
- SharedInMemoryNetwork for multi-runtime in-memory delivery
- InMemoryMeshTransport and InMemoryRetentionStore for transport and retention simulation
- InMemoryRuntimeEffects for deterministic effect handling

## Composition

LinkProfile implementations do not depend on routing engines. The `jacquard-mem-link-profile` crate has no imports from `jacquard-mesh`, `jacquard-router`, or engine-specific code. It only depends on `jacquard-core` for the shared types and `jacquard-traits` for the transport and retention boundaries.

`jacquard-reference-client` then composes these profile implementations with the router and mesh engine for end-to-end testing. This strict layering keeps profile contracts independent and lets teams test new transport protocols in isolation from routing complexity.

## Cross-reference

See [World Extensions](107_world_extensions.md) for how LinkProfile fits into the broader world extension surface, and [Pipeline and World Observations](105_pipeline_observations.md) for how LinkProfile and LinkState fit into the routing observation pipeline.
