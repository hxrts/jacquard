# Router Control Plane

`jacquard-router` is a generic middleware layer that owns the canonical control plane above the routing-engine boundary. The router registers one or more routing engines, orchestrates cross-engine candidate selection, and publishes the selected engine result as canonical route truth. Routing engines plan, admit, and maintain route-private runtime state without touching canonical route identity or publication.

## Ownership

The router owns canonical route-handle issuance, canonical lease publication, canonical commitment publication, and router-owned tick cadence. The router also dispatches maintenance triggers to engines.

Routing engines remain the owners of route-private runtime state and proof-bearing evidence. Profile implementations and test harnesses remain observational with respect to canonical route truth.

## Cross-Engine Orchestration

The router coordinates multiple registered routing engines without depending on their internals. Engines are registered once and queried during activation and maintenance. Each engine returns candidates, evidence, and proofs through shared trait boundaries.

A policy engine computes the routing profile (protection class, connectivity posture, mode) from the current routing objective and local state. The router passes that profile to all registered engines. Engines return candidates ordered by cost and evidence. The router selects the best candidate, asks that engine to admit and materialize the route, and only then publishes canonical state.

The router remains oblivious to engine-specific scoring, topology models, or repair strategies. Engines remain oblivious to lease ownership, commitment publication, or multi-engine selection logic.

## Activation Flow

The control-plane path is:

```text
objective
  -> policy profile
  -> authoritative topology tick
  -> cross-engine candidate ordering
  -> selected-engine admission
  -> router-owned handle + lease
  -> engine materialization proof
  -> canonical publication
  -> router-published commitments
```

The engine does not mint the canonical handle, publish the canonical lease, or surface commitments as canonical truth. The router consumes `RouteMaterializationProof`, `RouteWitness`, `RouteMaintenanceResult`, and `RouteSemanticHandoff` to publish canonical state.

## Tick and Maintenance

The router drives `RoutingTickContext` into each registered engine and consumes `RoutingTickOutcome`. Engines may refresh private control state and summarize ingress. They may run family-private choreographies. Engines do not publish canonical truth directly during `engine_tick`.

When maintenance returns a typed engine result, the router decides whether that implies canonical mutation. `ReplacementRequired` triggers router-owned reselection and replacement. `HandedOff` triggers router-owned lease transfer. `LeaseExpired` or `Expired` removes the canonical route.

Continued or repaired states update the router-published commitment view without changing canonical identity.

`RoutingControlPlane` returns typed router outcomes instead of collapsing everything to `Result<(), E>`.

The router also owns the durable publication sequence for canonical state:

```text
typed engine evidence
  -> router checkpoint update
  -> router-stamped route event
  -> in-memory canonical publication
```

Mesh may still checkpoint route-private runtime payloads, but canonical route publication and canonical route-event emission now happen in the router.

## Configuration and State Updates

The router exposes `RoutingMiddleware` for hosts to update observable topology and policy inputs without triggering route activation or maintenance. Hosts replace topology when new observations arrive. Hosts replace policy inputs when local conditions (capacity, churn, health) change.

The router also exposes a recovery interface for checkpoint replay. Hosts call `recover_checkpointed_routes` after restart to restore the previous canonical route table and active materialized state.

## Discovery Boundary

Shared discovery and coarse capability selection stay on `ServiceDescriptor`. Mesh nodes advertise route-capable surfaces through shared service descriptors. The router and test harness consume those shared descriptors. Jacquard does not introduce one universal handshake object for `Discover`, `Activate`, `Repair`, or `Hold`.

If a future engine needs stronger bilateral terms, add service-specific negotiation objects on that concrete path only.

## Multi-Device Composition

A direct host/runtime composition harness exists outside the simulator. `jacquard-mem-link-profile` provides the shared in-memory carrier and effect adapters. `jacquard-reference-client` shows the minimum host/client wiring for a new device target. The end-to-end multi-device test exercises `reference-client`, `router`, `mesh`, and `mem-link-profile` across multiple runtimes.

This harness proves crate-boundary composition. It does not replace the simulator. The simulator remains the scenario/replay layer above these shared boundaries.

## Minimal Host Wiring

The reference example for a new deployment target is in `crates/reference-client/tests/multi_device_mesh.rs`.

1. build a shared `Observation<Configuration>` with ordinary `ServiceDescriptor` values
2. attach one shared `TransportEffects` implementation per device runtime
3. construct one mesh engine per device
4. wrap each engine in one router that owns canonical publication
5. submit typed router commands instead of minting route truth directly

The minimum composition surface for a new device includes world input, transport registration, router activation, and data-plane forwarding over admitted routes.
