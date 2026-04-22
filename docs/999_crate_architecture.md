# Crate Architecture

This page describes the crate layout, the boundary rules, and the implementation policies that keep the workspace consistent.

## Boundary Rule

`core` defines what exists. `traits` defines what components are allowed to do.

`core` owns shared identifiers, data types, constants, error types, and the full model pipeline from world objects through observations, engine-neutral estimates, policy, and action. Derives, trivial constructors, and simple validation are allowed. Cross-crate behavioral interfaces belong in `traits`.

`traits` owns the cross-crate behavioral interfaces, grouped below by purpose. The layering subset is forward-looking. The shared shape is part of the stable design. In-tree coverage is contract-oriented rather than a mature production layering stack.

Shared transport vocabulary follows the same rule. `core` keeps a small observed-world transport schema in `TransportKind`, `EndpointLocator`, `LinkEndpoint`, `TransportDeliveryIntent`, `TransportDeliverySupport`, and `RouteDeliveryObjective` because those types appear in shared link, service, send-intent, and admission surfaces. Jacquard intentionally does not force those types fully opaque.

`EndpointLocator` keeps only the neutral locator families the shared model actually needs. Endpoint identity and delivery intent are separate: a `LinkEndpoint` names the carrier, while `TransportDeliveryIntent` says whether an admitted send is unicast, multicast, or broadcast. Transport-specific endpoint builders belong in transport-owned profile crates rather than in `core` or the transport-neutral mem profile crates.

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

The workspace contains repo-local policy tooling in `jacquard-toolkit-xtask` plus the routing crates `jacquard-core`, `jacquard-traits`, `jacquard-host-support`, `jacquard-cast-support`, `jacquard-macros`, `jacquard-pathway`, `jacquard-field`, `jacquard-mercator`, `jacquard-batman-bellman`, `jacquard-batman-classic`, `jacquard-babel`, `jacquard-olsrv2`, `jacquard-scatter`, `jacquard-router`, `jacquard-mem-node-profile`, `jacquard-mem-link-profile`, `jacquard-reference-client`, `jacquard-testkit`, and `jacquard-simulator`.

```
jacquard-core
    ↑             ↑
jacquard-traits jacquard-host-support jacquard-cast-support
    ↑             ↑                    ↑
jacquard-mem-node-profile
      │
jacquard-mem-link-profile
      │
jacquard-pathway ─────────┐
jacquard-field   ─────────┤
jacquard-mercator ────────┤
jacquard-batman-bellman ──┤
jacquard-batman-classic ──┤
jacquard-babel ───────────┼──→ jacquard-router ←── jacquard-reference-client
jacquard-olsrv2 ──────────┤         │                ↑
jacquard-scatter ─────────┘         └──→ jacquard-simulator

jacquard-testkit provides shared test support (used by simulator and reference-client tests)
jacquard-reference-client composes mem-* + router + in-tree engines
jacquard-simulator reuses reference-client composition rather than a simulator-only stack

jacquard-toolkit-xtask
```

Every crate depends on `jacquard-core`. Every crate except `jacquard-core` depends on `jacquard-traits` only when they need behavioral boundaries. `jacquard-host-support` depends only on `jacquard-core` plus proc-macro and serialization support because it owns reusable mailbox, ownership, endpoint-convenience, and host-side observational projector helpers, not runtime traits or router semantics. `jacquard-cast-support` depends only on `jacquard-core` plus serialization support because it owns bounded cast evidence helper shapes, not transport implementations or router semantics. `jacquard-router` depends on registered engines only through shared traits, not through pathway or BATMAN internals.

`jacquard-mem-node-profile` depends on `jacquard-core` and `jacquard-host-support` plus serialization support. `jacquard-mem-link-profile` depends on `jacquard-core`, `jacquard-traits`, and `jacquard-host-support` because it implements shared transport, retention, and effect traits while reusing the canonical raw-ingress mailbox. `jacquard-core` and `jacquard-traits` remain runtime-free.

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

The same rule applies inside engine crates. Candidate generation and scoring consume an explicit planner snapshot rather than hidden mutable state. Round and maintenance logic run through pure reducers over explicit runtime state plus normalized input when an engine supports those transitions. Checkpoints persist only durable protocol facts, while derived caches are rebuilt during recovery.

`jacquard-traits` also carries the shared engine-model contract that the simulator consumes. `RoutingEnginePlannerModel` standardizes planner execution over typed planner snapshots. `RoutingEngineRoundModel` and `RoutingEngineMaintenanceModel` standardize pure transition execution where an engine exposes those reducers.

`RoutingEngineRestoreModel` standardizes route-private runtime reconstruction from router-owned route records. `jacquard-simulator` depends on that trait family rather than maintaining a separate engine-specific integration API.

## Enforcement

Trait purity and routing invariants are enforced by the lint suite. The stable-toolchain check lane is split between the external toolkit runner and Jacquard's local `toolkit/xtask`. Nightly compiler-backed coverage lives in the external `toolkit` lint suite plus `toolkit/lints/model_policy` and `toolkit/lints/routing_invariants`. Public trait definitions in `jacquard-traits` also carry `#[purity(...)]` or `#[effect_trait]` annotations that the proc macros validate at compile time.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, route-event logging, transport send capability, host-owned transport drivers, time, and ordering all cross explicit shared boundaries in `traits`.

`jacquard-host-support` sits alongside that boundary, not inside it. Reusable host-side ingress mailboxes, unresolved and resolved peer bookkeeping, claim guards, transport-neutral endpoint conveniences, and host-side topology projectors live there so `core` stays data-only and `traits` stays contract-only. The router consumes explicit ingress and advances through synchronous rounds rather than polling transports ambiently. That is how native execution, tests, and simulation share one semantic model.

## Portability Profiles

The default workspace profile is a `std` profile. It supports native host builds, the simulator, the reference client, and in-memory profiles.

The wasm profile is a target compatibility check for selected `std` crates on `wasm32-unknown-unknown`. It does not prove that a crate is `no_std`. A crate can compile for wasm while still using the standard library surface available on that target.

The embedded profile is a `no_std` plus `alloc` profile for the deterministic model, route, cast, host-support, and Mercator path needed by MCU transport adapters such as jq-lora. Mercator is the in-tree engine path for this profile. That profile avoids direct thread, blocking wait, wall-clock, filesystem, and host I/O APIs. Platform behavior enters through explicit host or executor adapters.

`jacquard-cast-support` sits alongside profile and host-integration crates as deterministic evidence and delivery support. It is part of the portable `no_std` plus `alloc` profile. It normalizes unicast, multicast, and broadcast cast inputs into bounded, ordered helper records, then can derive route-neutral delivery support from those records and an explicit delivery objective. Profiles can map that support into route-visible `TransportDeliverySupport` without flattening multicast or broadcast into fake unicast links. It leaves transport send/receive, endpoint authoring, retry scheduling, custody storage, and route publication to their owning crates.

Cast objective admission stays above engines. Profiles expose delivery support, router-owned compatibility helpers compare that support with `RouteDeliveryObjective`, and host effects receive the admitted `TransportDeliveryIntent`. Broadcast objectives name an explicit `BroadcastDomainId` plus receiver coverage requirements; there is no implicit default broadcast domain. Engines continue to produce generic route candidates and must not depend on `jacquard-cast-support` to understand multicast, broadcast, BLE fanout, or endpoint materialization.

The cast surface is integration vocabulary and helper plumbing, not complete multicast or broadcast route activation in the built-in engines. The in-tree engines materialize ordinary routes. A host or downstream profile can use the delivery-support and admission helpers to avoid lying about fanout transports while full cast route activation remains a router/profile integration concern.

The effect traits are narrower than the higher-level component traits. They model runtime capabilities, not whole subsystems. `RoutingEngine`, `Router`, and `RetentionStore` are larger behavioral contracts and should not be forced through the effect layer.

Recovery follows the same ownership split. The router persists canonical `MaterializedRoute` records. Engines restore route-private runtime through router-managed hooks. When the current topology is needed to rebuild a derived forwarding view, the router provides that topology during recovery rather than forcing the engine to persist the derived view itself.

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
| `jacquard-macros` | Annotation-site validation and syntax-local code generation for effect, handler, and purity attributes. No runtime state. |
| `jacquard-host-support` | Generic host-side ingress mailboxes, peer identity bookkeeping, claim ownership helpers, transport-neutral endpoint conveniences, and host-side observational read models. No route truth, no transport-specific protocol logic, no router actions, no time/order stamping. |
| `jacquard-cast-support` | Deterministic bounded unicast, multicast, and broadcast evidence helper records plus route-neutral delivery support shaping. No transport implementation, endpoint constructors, retry scheduling, route truth, router actions, or time/order stamping. |
| `jacquard-pathway` | Pathway-private forwarding state, topology caches, repair state, retention state, engine-local committee scoring, and the private choreography guest runtime plus its protocol checkpoints. |
| `jacquard-field` | Field-private posterior state, mean-field compression, regime/posture control state, Telltale-backed frozen-snapshot search, bounded runtime-round diagnostics, continuation scoring, and any field-private choreography runtime used only for observational summary exchange. |
| `jacquard-mercator` | Mercator-private evidence graph, corridor planner, stale-safe repair state, weakest-flow accounting, broker-pressure accounting, bounded custody records, and route-visible diagnostics. |
| `jacquard-batman-bellman` | BATMAN Bellman-private originator observations, gossip-merged topology, Bellman-Ford path computation, TQ enrichment, next-hop ranking tables, and active next-hop forwarding records. |
| `jacquard-batman-classic` | BATMAN Classic-private OGM-carried TQ state, receive windows, echo-based bidirectionality tables, learned advertisement state, next-hop ranking tables, and active next-hop forwarding records. |
| `jacquard-babel` | Babel-private route table, feasibility-distance state, additive-metric scoring, seqno management, and active next-hop forwarding records. |
| `jacquard-olsrv2` | OLSRv2-private HELLO state, symmetric-neighbor and two-hop reachability tables, deterministic MPR state, TC topology tuples, shortest-path derivation, and active next-hop forwarding records. |
| `jacquard-scatter` | Scatter-private retained messages, peer observations, per-route progress, replication and handoff state, and deterministic regime, budget, and transport policy thresholds. |
| `jacquard-router` | Canonical route identity, materialization inputs, leases, handle issuance, top-level route-health publication, delivery objective compatibility, and multi-engine orchestration state. |
| `jacquard-mem-node-profile` | In-memory node capability and node-state modeling only. No routing semantics. |
| `jacquard-mem-link-profile` | In-memory link capability, carrier, retention, route-visible delivery-support fixtures, and runtime-effect adapter state only. No canonical routing truth. |
| `jacquard-reference-client` | Narrow host-side bridge composition of profile implementations, bridge-owned drivers, router, and one or more in-tree engine instances for tests and examples. Observational with respect to canonical route truth, but owner of ingress queueing and round advancement in the reference harness. |
| `jacquard-testkit` | Shared test fixtures and scenario helpers consumed by the simulator and reference-client test suites. No canonical route truth. |
| `jacquard-simulator` | Replay artifacts, scenario traces, post-run analysis, and model-lane orchestration over engine-owned planner, reducer, and restore surfaces. No canonical route truth during a live run. |

A host-owned policy engine above the router may own cross-engine migration policy and substrate selection.

## Extensibility

`core::Configuration` is the shared graph-shaped world object. Engine-specific structure such as topology exports, peer novelty, bridge estimates, planning caches, and forwarding tables belongs in the engine crate behind its trait boundary rather than in `core`.

The extension surface is split across [Core Types](201_core_types.md), [Routing Engines](303_routing_engines.md), and [Pathway Routing](404_pathway_routing.md).

For first-party pathway specifically, Telltale stays an internal implementation substrate. Shared crates remain runtime-free. The future router may drive pathway through shared planning, tick, maintenance, and checkpoint orchestration. It must not depend on pathway-private choreography payloads, protocol session keys, or guest-runtime internals.

For first-party field, the proof and ownership boundary is even stricter. Field-private choreography may supply only observational evidence into the deterministic local controller. It must not publish canonical route truth or leak field-private session semantics into shared router surfaces.
