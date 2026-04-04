# Introduction

Contour is an adaptive mesh routing system built on choreographic protocols. It defines a stable routing abstraction and provides a first-party mesh implementation that supports transport-mixed forwarding over BLE, Wi-Fi, and future radio transports. The system is designed to operate on constrained mobile devices where topology changes frequently and network partitions are routine.

Contour depends on [Telltale](https://github.com/hxrts/telltale) for session types, choreography macros, and an effect-based distributed runtime. Protocol interactions in the mesh layer are defined as choreographies, projected into local roles, and embedded behind trait boundaries.

## Core Separation

The routing abstraction separates four concerns that must not be conflated.

Connectivity surfaces describe how a peer or service can be reached right now. A BLE GATT endpoint, a Wi-Fi LAN address, and a QUIC relay are all connectivity surfaces. They change as radios come and go.

Service surfaces describe what a peer is willing to do. The five service families are `Discover`, `Establish`, `Move`, `Repair`, and `Hold`. A node may offer some or all of these depending on its role and capacity.

Route families define which routing semantics are in use. Contour implements one first-party family, `Mesh`, and exposes a plugin boundary for external families. Each family owns its internal route construction, maintenance, and data-plane objects.

Local adaptive policy governs route selection at runtime. Policy state is never shared as network truth. It remains local to the selecting node.

## Mesh Family

`Mesh` is the baseline routing family. It uses explicit source-routed paths over a local topology graph. Route structure is visible, which buys repairability and transport mixing at the cost of exposing path shape.

Mesh supports five operational modes through the shared service families. Discovery propagates neighbor advertisements and route exports. Establishment admits and installs concrete paths. Move forwards typed frames hop by hop. Repair patches, shortens, or extends degraded routes in place. Hold provides custody and deferred delivery during partitions.

When local repair cannot preserve route viability, the mesh family returns a replacement request to the top-level router. The top-level router then decides whether to reselect from available candidates.

## Determinism

Contour is a deterministic system. There are no floating-point types in stored state, protocol objects, or routing policy. There is no host-dependent ordering in route selection. All ranking and policy surfaces use integers, enums, or fixed-width byte strings.

Fractional quantities use explicit integer scales. `RatioPermille` represents values from 0 to 1000. `PriorityPoints`, `HealthScore`, and `PenaltyPoints` carry implementation-scaled integer weights.

The system also enforces explicit upper bounds for candidate sets, hop counts, queues, and payload sizes. There are no stored or protocol-facing `usize` fields.

## Time Model

Contour uses a typed deterministic time system rather than raw wall-clock APIs.

`Tick` represents local monotonic time. `DurationMs` represents local durations and timeout budgets. `OrderStamp` provides deterministic ordering that does not depend on wall clock. `RouteEpoch` versions topology and reconfiguration state independently of elapsed time.

These types govern descriptor validity windows, route expiry, replay windows, retry and backoff policy, maintenance scheduling, and local timeout ownership. Wall clock may exist at process boundaries and in logs, but the routing core operates on deterministic local time domains.

## Ownership Model

Contour uses four ownership levels to prevent multiple layers from accidentally owning the same truth.

`Pure` covers descriptor validation, route summary comparison, witness validation, and scoring functions. Pure code must not own route caches, live connections, or background maintenance.

`MoveOwned` covers installed route leases, route handoff between owners, route teardown tokens, and any transfer where stale handles must become invalid. If an active route changes owner, the transfer is modeled as a move.

`ActorOwned` covers topology caches, provider health smoothing, adaptive controller state, the route-family registry, the installed-route table, and mesh runtime loops. Local selection state and adaptive profile state are runtime-local actor-owned data that must not be published as shared descriptors.

`Observed` covers diagnostics, metrics export, UI and simulation views, and debug snapshots. Observed code may read route summaries and witnesses but may not invent canonical route state.

## Extensibility

The routing abstraction supports external route families through the `RouteFamilyPlugin` trait boundary. An external family registers with the top-level router, declares its capability envelope, and participates in common adaptive selection. Contour core does not inspect family-private route internals.

The top-level router owns cross-family candidate comparison, fallback legality, and route replacement policy. Each family plugin owns its internal route construction, maintenance, and teardown. This separation lets external families integrate without forcing their internal formats or path semantics into Contour core.

## Choreography Direction

The mesh implementation follows a choreography-first approach.

1. Neighborhood discovery and route export form the discovery plane.
2. Route admission and install create the active route object.
3. Steady-state forwarding carries ordinary traffic.
4. Bounded route repair and reconfiguration preserve connectivity under churn.
5. Custody and deferred delivery provide typed partition fallback.

These protocol objects are defined as Telltale choreographies, projected into per-role state machines, and embedded behind the mesh family boundary.
