# Crate Architecture

This page describes the crate layout, the boundary rules, and the implementation policies that keep the workspace consistent.

## Boundary Rule

`core` defines what exists. `traits` defines what components are allowed to do.

`core` owns shared identifiers, data types, constants, error types, and the full model pipeline from world objects through observations, engine-neutral estimates, policy, and action. Derives, trivial constructors, and simple validation are allowed. Cross-crate behavioral interfaces belong in `traits`.

`traits` owns the cross-crate behavioral interfaces, grouped below by purpose. The layering subset is forward-looking. The shared shape is part of the stable design, but in-tree coverage is still contract-oriented rather than a mature production layering stack.

Shared transport vocabulary follows the same rule. `core` keeps a small,
observed-world transport schema in `TransportKind`, `EndpointLocator`, and
`LinkEndpoint` because those types appear in shared `Link`,
`ServiceDescriptor`, and `TransportObservation` facts. Jacquard intentionally
does not force those types fully opaque today; `EndpointLocator` keeps only the
neutral locator families the shared model actually needs, while transport-
specific endpoint builders belong in transport-owned profile crates rather than
in `core` or the transport-neutral mem profile crates.

| Category | Traits |
|---|---|
| Routing contract | `RoutingEnginePlanner`, `RoutingEngine`, `Router`, `RoutingControlPlane`, `RoutingDataPlane`, `PolicyEngine` |
| Local coordination | `CommitteeSelector`, `CommitteeCoordinatedEngine` |
| Layering | `SubstratePlanner`, `SubstrateRuntime`, `LayeredRoutingEnginePlanner`, `LayeredRoutingEngine`, `LayeringPolicyEngine` |
| Runtime effects | `TimeEffects`, `OrderEffects`, `StorageEffects`, `RouteEventLogEffects`, `TransportSenderEffects` |
| Host-owned drivers | `TransportDriver` |
| Hashing and content | `Hashing`, `ContentAddressable`, `TemplateAddressable` |
| Simulator | `RoutingScenario`, `RoutingEnvironmentModel`, `RoutingSimulator`, `RoutingReplayView` |

## Dependency Graph

The workspace today contains repo-local policy tooling in `jacquard-toolkit-xtask` plus the routing crates `jacquard-core`, `jacquard-traits`, `jacquard-adapter`, `jacquard-macros`, `jacquard-pathway`, `jacquard-field`, `jacquard-batman`, `jacquard-router`, `jacquard-mem-node-profile`, `jacquard-mem-link-profile`, `jacquard-field-client`, `jacquard-reference-client`, and `jacquard-simulator`.

```
jacquard-core
    ↑          ↑
jacquard-traits jacquard-adapter
    ↑             ↑
jacquard-mem-node-profile
      │
jacquard-mem-link-profile
      │
jacquard-pathway ─┐
jacquard-field   ─┼──→ jacquard-router ←── jacquard-reference-client
jacquard-batman  ─┘         │                ↑
      │                     ├──→ jacquard-field-client
      └────────────────────→ jacquard-simulator
                             └── composes mem-* + router + in-tree engines

jacquard-toolkit-xtask
```

Every crate depends on `jacquard-core`. Every crate except `jacquard-core` depends on `jacquard-traits` only when they need behavioral boundaries. `jacquard-adapter` depends only on `jacquard-core` plus proc-macro/serialization support because it owns reusable mailbox, ownership, endpoint-convenience, and host-side observational projector helpers, not runtime traits or router semantics. `jacquard-router` depends on registered engines only through shared traits, not through pathway or BATMAN internals. `jacquard-mem-node-profile` depends on `jacquard-core` and `jacquard-adapter` plus serialization support. `jacquard-mem-link-profile` depends on `jacquard-core`, `jacquard-traits`, and `jacquard-adapter` because it implements shared transport, retention, and effect traits while reusing the canonical raw-ingress mailbox. `jacquard-core` and `jacquard-traits` remain runtime-free.

## Crate Layout

Inside `core`, files are grouped into three areas. `base/` holds cross-cutting primitives: identity, time, qualifiers, constants, and errors. `model/` holds the world-to-action pipeline: world objects, observations, estimation, policy, and action. `routing/` holds route lifecycle and runtime coordination objects.

`core` defines result shapes, not policies. It exposes coordination objects like `CommitteeSelection`, layering objects like `SubstrateLease`, and route lifecycle objects like `RouteHandle`, but it does not encode engine-local scoring, committee algorithms, leader requirements, layering decisions, or a parallel authority system above those route objects. Authority flows through the route contracts themselves: admitted routes, witnesses, proofs, leases, and explicit lifecycle transitions.

## Purity And Side Effects

Jacquard treats purity and side effects as part of the trait contract.

- `Pure` traits must be deterministic with respect to their inputs. They should not perform I/O, read ambient time, allocate order stamps, or mutate hidden state that changes outputs.
- `Read-only` traits may inspect owned state or snapshots, but they must not mutate canonical routing truth or perform runtime effects.
- `Effectful` traits may perform I/O or mutate owned runtime state, but only through an explicit boundary with a narrow purpose.

Signature design follows the same split. Use `&self` for pure and read-only methods. Use `&mut self` only when the method has explicit state mutation or side effects. Do not mix pure planning and effectful runtime mutation in one trait unless the split is impossible and documented.

That is why Jacquard separates `RoutingEnginePlanner` from `RoutingEngine`, `SubstratePlanner` from `SubstrateRuntime`, and `LayeredRoutingEnginePlanner` from `LayeredRoutingEngine`. Engine-specific read-only seams such as pathway topology access stay in the owning engine crate rather than leaking into `jacquard-traits`. The shared round lifecycle follows the same rule: router-owned cadence and explicit ingress live at the contract layer, while engine-specific control loops and control-state contents stay inside the owning engine crate.

## Enforcement

Trait purity and routing invariants are enforced by the lint suite. The stable-toolchain check lane is split between the external toolkit runner and Jacquard's local `toolkit/xtask`, while nightly compiler-backed coverage lives in the external `toolkit` lint suite plus `toolkit/lints/model_policy` and `toolkit/lints/routing_invariants`. Public trait definitions in `jacquard-traits` also carry `#[purity(...)]` or `#[effect_trait]` annotations that the proc macros validate at compile time.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, route-event logging, transport send capability, host-owned transport drivers, time, and ordering all cross explicit shared boundaries in `traits`. `jacquard-adapter` sits alongside that boundary, not inside it: reusable adapter-side ingress mailboxes, unresolved/resolved peer bookkeeping, claim guards, transport-neutral endpoint conveniences, and host-side topology projectors live there so `core` stays data-only and `traits` stays contract-only. The router consumes explicit ingress and advances through synchronous rounds rather than polling adapters ambiently. That is how native execution, tests, and simulation share one semantic model.

The effect traits are narrower than the higher-level component traits. They model runtime capabilities, not whole subsystems. `RoutingEngine`, `Router`, and `RetentionStore` are larger behavioral contracts and should not be forced through the effect layer.

First-party pathway keeps one additional internal layer above those shared effects: pathway-private choreography effect interfaces generated from Telltale protocols. Those generated interfaces are not promoted into `jacquard-traits`. Concrete host/runtime adapters implement the shared effect traits, and `jacquard-pathway` interprets its private choreography requests in terms of those stable shared boundaries.

Within `jacquard-pathway` itself, the async envelope is narrower still. Telltale session futures are driven to completion only inside choreography modules. The engine/runtime layer owns a bounded explicit ingress queue, consumes it during one synchronous round, and exposes a pathway round-progress snapshot for host-facing inspection. It does not own transport drivers, ambient async callbacks, or executor-shaped advancement.

## Invariants

- No crate may use floating-point types in routing logic, routing state, routing policy, or simulator verdicts.
- No crate may treat wall-clock time as distributed semantic truth.
- `Tick` is time and `RouteEpoch` is configuration versioning. Crates must not convert between them by rewrapping the inner integer.
- Canonical ordering must flow through shared ordering types. Crates must not invent crate-local tie-break schemes.
- Canonical hashing and content IDs must flow through the shared hash and content-addressing boundaries.
- Transport may observe links and carry bytes, but it must not invent route truth, publish canonical route health, or mutate materialized-route ownership.
- GPS, absolute location, clique grids, and singleton leaders are not shared routing truth. Spatial hints stay engine-private above the shared observation boundary.
- Multiple routing engines may coexist in one host runtime. Generic mixed-engine canonical route ownership is not a base-layer assumption.

## Ownership

Each crate owns a narrow slice of runtime state.

| Crate | Owns |
|---|---|
| `jacquard-core` | Shared vocabulary. No live state. |
| `jacquard-traits` | Compile-time boundaries. No runtime state. |
| `jacquard-adapter` | Generic adapter-side ingress mailboxes, peer identity bookkeeping, claim ownership helpers, transport-neutral endpoint conveniences, and host-side observational read models. No route truth, no transport-specific protocol logic, no router actions, no time/order stamping. |
| `jacquard-pathway` | Pathway-private forwarding state, topology caches, repair state, retention state, engine-local committee scoring, and the private choreography guest runtime plus its protocol checkpoints. |
| `jacquard-field` | Field-private posterior state, mean-field compression, regime/posture control state, Telltale-backed frozen-snapshot search, bounded runtime-round diagnostics, continuation scoring, and any field-private choreography runtime used only for observational summary exchange. |
| `jacquard-batman` | BATMAN-private originator observations, next-hop ranking tables, TQ derivation, and active next-hop forwarding records. |
| `jacquard-router` | Canonical route identity, materialization inputs, leases, handle issuance, top-level route-health publication, and multi-engine orchestration state. |
| `jacquard-mem-node-profile` | In-memory node capability and node-state modeling only. No routing semantics. |
| `jacquard-mem-link-profile` | In-memory link capability, carrier, retention, and runtime-effect adapter state only. No canonical routing truth. |
| `jacquard-reference-client` | Narrow host-side bridge composition of profile implementations, bridge-owned drivers, router, and one or more in-tree engine instances for tests and examples. Observational with respect to canonical route truth, but owner of ingress queueing and round advancement in the reference harness. |
| `jacquard-simulator` (future) | Replay artifacts, scenario traces, post-run analysis. No canonical route truth during a live run. |

A host-owned policy engine above the router may own cross-engine migration policy and substrate selection.

## Extensibility

`core::Configuration` is the shared graph-shaped world object. Engine-specific structure such as topology exports, peer novelty, bridge estimates, planning caches, and forwarding tables belongs in the engine crate behind its trait boundary rather than in `core`.

The extension surface is split across [Core Types](201_core_types.md), [Routing Engines](303_routing_engines.md), and [Pathway Routing](401_pathway_routing.md).

For first-party pathway specifically, Telltale stays an internal implementation substrate. Shared crates remain runtime-free. The future router may drive pathway through shared planning, tick, maintenance, and checkpoint orchestration, but it must not depend on pathway-private choreography payloads, protocol session keys, or guest-runtime internals.

For first-party field, the proof and ownership boundary is even stricter:
field-private choreography may supply only observational evidence into the
deterministic local controller. It must not publish canonical route truth or
leak field-private session semantics into shared router surfaces.
