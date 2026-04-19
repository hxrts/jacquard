# Router Control Plane

`jacquard-router` is a generic middleware layer that owns the canonical control plane above the routing-engine boundary. The router registers one or more routing engines, orchestrates cross-engine candidate selection, and publishes the selected engine result as canonical. Routing engines plan, admit, and maintain route-private runtime state without touching canonical route identity or publication.

This includes proactive engines. The router does not own proactive routing tables. It only owns canonical publication over the evidence those engines return.

## Ownership

The router owns canonical route-handle issuance, canonical lease publication, canonical commitment publication, explicit ingress queues, and router-owned round cadence. The router also dispatches maintenance triggers to engines.

Routing engines remain the owners of route-private runtime state and proof-bearing evidence. Profile implementations and test harnesses remain observational with respect to canonical route truth.

## Cross-Engine Orchestration

The router coordinates multiple registered routing engines while keeping engine internals encapsulated. Engines are registered once and queried during activation and maintenance. Each engine returns candidates, evidence, and proofs through shared trait boundaries.

A policy engine computes the routing profile (protection class, connectivity posture, mode) from the current routing objective and local state. The router passes that profile to all registered engines. Engines return candidates ordered by cost and evidence. The router selects the best candidate, asks that engine to admit and materialize the route, and only then publishes canonical state.

The router remains oblivious to engine-specific scoring, topology models, or repair strategies. Engines remain oblivious to lease ownership, commitment publication, or multi-engine selection logic.

## Activation Flow

The control-plane path is:

```text
objective
  -> policy profile
  -> authoritative topology observation
  -> explicit queued ingress
  -> synchronous router round
  -> cross-engine candidate ordering
  -> selected-engine admission
  -> router-owned handle + lease
  -> engine materialization proof
  -> canonical publication
  -> router-published commitments
```

The engine does not mint the canonical handle, publish the canonical lease, or surface commitments as canonical truth. The router consumes `RouteMaterializationProof`, `RouteWitness`, `RouteMaintenanceResult`, and `RouteSemanticHandoff` to publish canonical state.

## Route Lifecycle

The route lifecycle is owned by the control plane above the engine boundary.

1. A host activates a `RoutingObjective`.
2. The router computes policy and queries registered engines for candidates.
3. The selected engine admits and materializes under router-owned identity.
4. The router publishes canonical route state and commitments.
5. Later rounds drive maintenance, replacement, handoff, expiry, or teardown.

Engines report proof-bearing maintenance outcomes such as continued health, repair, handoff, replacement pressure, or expiry. The router decides whether that engine result implies canonical mutation.

## Tick and Maintenance

The router advances through synchronous rounds. Hosts feed topology, policy inputs, and transport observations into `RoutingMiddleware`, then call `advance_round` on the control plane. During that round the router drives `RoutingTickContext` into each registered engine and consumes `RoutingTickOutcome`.

Engines may refresh private control state and summarize previously ingested observations. They may run engine-private choreographies. Engines do not publish canonical truth directly during `engine_tick`.

`RoutingTickOutcome.next_tick_hint` lets proactive engines report scheduling pressure without taking ownership of cadence. The router or host may honor that hint, clamp it, or ignore it. The cadence decision remains router or host owned.

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

Pathway may checkpoint route-private runtime payloads. Canonical route publication and canonical route-event emission happen in the router.

## Configuration and State Updates

The router exposes `RoutingMiddleware` for hosts to update observable topology, policy inputs, and transport ingress without triggering route activation or maintenance. Hosts ingest topology observations when new world state arrives. Hosts ingest policy inputs when local conditions (capacity, churn, health) change. Hosts ingest transport observations explicitly instead of letting engines or routers poll transport adapters directly.

The router also exposes a recovery interface for checkpoint replay. Hosts call `recover_checkpointed_routes` after restart to restore the previous canonical route table and active materialized state.

## Discovery Boundary

Shared discovery and coarse capability selection stay on `ServiceDescriptor`. Pathway nodes advertise route-capable surfaces through shared service descriptors. The router and test harness consume those shared descriptors. Jacquard does not introduce one universal handshake object for `Discover`, `Activate`, `Repair`, or `Hold`.

If a future engine needs stronger bilateral terms, add service-specific negotiation objects on that concrete path only.

## Multi-Device Composition

A direct host/runtime composition harness exists outside the simulator. `jacquard-mem-link-profile` provides the shared in-memory carrier and effect adapters. `jacquard-reference-client` shows the minimum host bridge wiring for a new device target: one bridge-owned transport driver, one or more queue-backed transport senders handed to engines, explicit ingress stamping, and explicit synchronous router rounds. The end-to-end multi-device test exercises `reference-client`, `router`, `pathway`, `batman-bellman`, `batman-classic`, `babel`, and `mem-link-profile` across multiple runtimes.

This harness proves crate-boundary composition. It does not replace the simulator. The simulator remains the scenario/replay layer above these shared boundaries.

## Minimal Host Wiring

The reference examples for a new deployment target are the split
`reference-client` end-to-end tests in
[`crates/reference-client/tests/e2e_pathway_shared_network.rs`](../crates/reference-client/tests/e2e_pathway_shared_network.rs),
[`crates/reference-client/tests/e2e_batman_pathway_handoff.rs`](../crates/reference-client/tests/e2e_batman_pathway_handoff.rs),
[`crates/reference-client/tests/e2e_olsrv2_shared_network.rs`](../crates/reference-client/tests/e2e_olsrv2_shared_network.rs),
and
[`crates/reference-client/tests/e2e_olsrv2_pathway_handoff.rs`](../crates/reference-client/tests/e2e_olsrv2_pathway_handoff.rs),
backed by the shared scenarios in
[`crates/testkit/src/reference_client_scenarios.rs`](../crates/testkit/src/reference_client_scenarios.rs).

1. build a shared `Observation<Configuration>` with ordinary `ServiceDescriptor` values
2. attach one bridge-owned `TransportDriver` per device runtime
3. construct one or more engines per device over queue-backed `TransportSenderEffects`
4. wrap those engines in one router that owns canonical publication
5. bind one host bridge owner per runtime, ingest topology and transport updates explicitly there, and advance the router through synchronous bridge rounds instead of minting route truth directly

The minimum composition surface for a new device includes world input, bridge-owned transport registration, router activation, and data-plane forwarding over admitted routes.
