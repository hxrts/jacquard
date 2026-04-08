# Router Control Plane

`jacquard-router` is the canonical control-plane owner above the routing-engine boundary. Engines plan, admit, and maintain route-private runtime state. The router turns that typed engine evidence into canonical route truth.

## Ownership

The router is `ActorOwned` for:

- canonical route-handle issuance
- canonical lease publication and lease transfer
- canonical commitment publication
- router-owned tick cadence and maintenance dispatch

Mesh remains the owner of route-private runtime state and proof-bearing engine evidence. Mock transports and mock devices remain observational with respect to canonical route truth.

## Activation Flow

The control-plane path is:

```text
objective
  -> policy profile
  -> authoritative topology tick
  -> candidate ordering
  -> engine admission
  -> router-owned handle + lease
  -> engine materialization proof
  -> canonical publication
  -> router-published commitments
```

This split is deliberate:

- the engine does not mint the canonical handle
- the engine does not publish the canonical lease
- the engine does not surface commitments directly as canonical truth
- the router consumes `RouteMaterializationProof`, `RouteWitness`, `RouteMaintenanceResult`, and `RouteSemanticHandoff` to publish canonical state

## Maintenance And Tick

The router drives `RoutingTickContext` into the engine and consumes `RoutingTickOutcome`. The engine may refresh private control state, summarize ingress, and run mesh-private choreographies, but it does not publish canonical truth directly during `engine_tick`.

When maintenance returns a typed engine result, the router decides whether that implies canonical mutation:

- `ReplacementRequired` triggers router-owned reselection and replacement
- `HandedOff` triggers router-owned lease transfer
- `LeaseExpired` or `Expired` removes the canonical route
- continued or repaired states update the router-published commitment view without changing canonical identity

That is why `RoutingControlPlane` now returns typed router outcomes instead of collapsing everything to `Result<(), E>`.

The router also owns the durable publication sequence for canonical state:

```text
typed engine evidence
  -> router checkpoint update
  -> router-stamped route event
  -> in-memory canonical publication
```

Mesh may still checkpoint route-private runtime payloads, but canonical route publication and canonical route-event emission now happen in the router.

## Discovery Boundary

Shared discovery and coarse capability selection stay on `ServiceDescriptor`.

- mesh nodes advertise route-capable surfaces through shared service descriptors
- the router and mock-device harness consume those shared descriptors
- Jacquard does not introduce one universal handshake object for `Discover`, `Activate`, `Repair`, or `Hold`

If a future engine needs stronger bilateral terms, those should be added as service-specific negotiation objects on that concrete path only.

## Multi-Device Composition

Phase 3 keeps one direct host/runtime composition harness outside the simulator:

- `jacquard-mock-transport` provides the shared in-memory carrier and effect adapters
- `jacquard-mock-device` shows the minimum host/device wiring for a new device target
- the end-to-end multi-device test exercises `mock-device -> router -> mesh -> mock-transport` across multiple runtimes

This harness exists to prove crate-boundary composition, not to replace the simulator. The simulator remains the scenario/replay layer above these shared boundaries.

## Minimal Host Wiring

The Phase 3 harness in [`crates/mock-device/tests/multi_device_mesh.rs`](../crates/mock-device/tests/multi_device_mesh.rs) is the reference example for a new deployment target:

1. build a shared `Observation<Configuration>` with ordinary `ServiceDescriptor` values
2. attach one `MeshTransport` implementation per device runtime
3. construct one mesh engine per device
4. wrap each engine in one router that owns canonical publication
5. let the device wrapper submit typed router commands instead of minting route truth directly

That is the intended minimum composition surface for a new device: world input, transport registration, router activation, and data-plane forwarding over admitted routes.
