# Profile Implementations

This page documents the in-tree in-memory profile implementations and the thin client harness that composes them with the router and mesh engine.

## Purpose

Jacquard separates profile modeling from routing composition. `jacquard-mem-node-profile` models `NodeProfile`, `NodeState`, and service advertisement builders. `jacquard-mem-link-profile` models `LinkEndpoint`, `LinkState`, in-memory frame carriers, retention, and runtime-effect adapters. `jacquard-reference-client` composes these profile implementations with `jacquard-router` and `jacquard-mesh` for end-to-end tests.

This separation keeps profile crates observational. They model capabilities and state without planning routes, publishing canonical commitments, or owning canonical route truth.

## `jacquard-mem-node-profile`

Use this crate to build node capability and node-state inputs without importing routing-engine logic. Build a `SimulatedNodeProfile`, then build or evolve a `NodeStateSnapshot`, then assemble a `Node` for the shared `Configuration`. This crate is appropriate for unit tests that vary relay budget, hold capacity, or advertised services without constructing a full router or engine.

## `jacquard-mem-link-profile`

Use this crate to model link capabilities or drive an in-memory carrier in tests. It provides `SimulatedLinkProfile` for `Link` construction, `SharedInMemoryNetwork` for multi-runtime in-memory delivery, `InMemoryMeshTransport` for the shared `MeshTransport` boundary, and `InMemoryRetentionStore` plus `InMemoryRuntimeEffects` for deterministic tests.

This crate remains routing-engine-neutral. It carries frames and emits observations without minting route truth or interpreting routing policy.

## `jacquard-reference-client`

`jacquard-reference-client` is the integration harness. It composes a shared topology built from ordinary `Node` and `Link` values with one router instance, one mesh engine instance, and one in-memory transport attached to `SharedInMemoryNetwork`.

The reference end-to-end example is [`multi_device_mesh.rs`](../crates/reference-client/tests/multi_device_mesh.rs). That test demonstrates how to add a new client/runtime composition without bypassing the router-owned canonical path.

## Extension Guidance

To add a new device or profile implementation, implement node-side world inputs like `jacquard-mem-node-profile`. Implement link-side and carrier behavior like `jacquard-mem-link-profile`. Compose them through a host/client harness like `jacquard-reference-client`.

Keep the ownership boundary strict. Profile crates stay `Observed`. Routers stay the canonical `ActorOwned` route publisher. Routing engines own only route-private runtime state and typed evidence.
