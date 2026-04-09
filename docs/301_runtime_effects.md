# Runtime Effects

This page describes the narrow runtime capability surface that Jacquard exposes to pure routing logic. See [World Extensions](302_world_extensions.md) for the layering overview, [Routing Engines](303_routing_engines.md) for the engine and policy contracts, and [Pathway Routing](401_pathway_routing.md) for the in-tree explicit-path implementation and its operational subcomponents.

## Effect Surface

Runtime effects are the lowest-level extensibility surface in Jacquard. They expose narrow runtime capabilities to pure routing logic. They are useful when a runtime or host needs to swap out how routing code gets time, storage, transport, or route-event logging services without changing the routing logic itself.

```rust
pub trait TimeEffects {
    #[must_use]
    fn now_tick(&self) -> Tick;
}

pub trait OrderEffects {
    #[must_use]
    fn next_order_stamp(&mut self) -> OrderStamp;
}

pub trait StorageEffects {
    fn load_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;

    fn store_bytes(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;

    fn remove_bytes(&mut self, key: &[u8]) -> Result<(), StorageError>;
}

pub trait RouteEventLogEffects {
    fn record_route_event(
        &mut self,
        event: RouteEventStamped,
    ) -> Result<(), RouteEventLogError>;
}

pub trait TransportSenderEffects {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError>;
}

pub trait RoutingRuntimeEffects:
    TimeEffects
    + OrderEffects
    + StorageEffects
    + RouteEventLogEffects
    + TransportSenderEffects
{}
```

Each effect trait covers one concern. `TimeEffects` provides monotonic local time. `OrderEffects` provides deterministic ordering tokens. `StorageEffects` provides byte-level key-value persistence. `RouteEventLogEffects` provides replay-visible route event recording. `TransportSenderEffects` provides endpoint-addressed payload send only. Host-owned ingress supervision lives on `TransportDriver`, outside the effect vocabulary.

## Why The Boundary Exists

The effect surface is what lets one routing engine compile against one set of traits and run unchanged across production, in-process tests, and the deterministic simulator. The simulator implements `TimeEffects` from a virtual clock, `TransportSenderEffects` from a scenario script, and `StorageEffects` from an in-memory map. The production runtime backs the same traits with real OS calls. Routing logic depends on the trait, not the implementation.

This is what keeps Jacquard testable end-to-end without forking the routing model. A test that wants to advance time, drop a frame, or simulate a storage failure does so by swapping the effect implementation, not by editing the routing engine. The same engine binary participates in deterministic replay because every nondeterministic input crosses one of these traits.

First-party pathway adds one private layer above these shared traits: Telltale-generated choreography effect interfaces used only inside `jacquard-pathway`. Those generated interfaces are not promoted into `jacquard-traits`. Instead, pathway interprets its private protocol requests onto the stable shared effect traits through a concrete host/runtime adapter.

The current in-tree composition crates keep that split intact:

- `jacquard-router` consumes shared effect traits to mint publication ids, build leases, and drive router-owned cadence
- `jacquard-router` also wraps those traits in one router-local sequencing adapter so checkpoint writes, route-event logging, and canonical publication stay in one fail-closed order
- `jacquard-mem-link-profile` implements the shared effect traits and in-memory carrier traits for tests and examples only
- `jacquard-reference-client` composes routers, engines, and profile implementations through one host bridge per runtime, but remains observational with respect to canonical route truth

In other words, Jacquard now has both sides of the runtime-adapter story:

- a native router-side adapter that interprets the shared effect traits as canonical publication sequencing
- a test-side in-memory adapter that implements those same shared traits for end-to-end composition and failure injection

## Determinism Contract

Each effect trait carries rules that runtimes must honor for routing logic to remain deterministic. `TimeEffects::now_tick` must be monotonic non-decreasing within a runtime instance, since replay and expiry checks depend on that ordering. The clock may pause or skip ahead under simulator control but must never move backward. `OrderEffects::next_order_stamp` must be strictly increasing and never repeat within a runtime instance, because stamps participate in canonical ordering and a duplicate would corrupt deterministic tie-breaking.

`StorageEffects` reads must reflect prior writes from the same runtime within the same logical session. Writes are not assumed to be visible to other runtimes or processes. `RouteEventLogEffects::record_route_event` is append-only and must commit before the control plane reports the next lifecycle transition as durable, so replay can reconstruct the same routing state. A runtime that persists state across restarts must also persist enough order-stamp state to keep stamps unique after recovery.

`TransportSenderEffects::send_transport` delivers a frame on a best-effort basis. Transport ingress is not polled through the effect surface anymore; host-owned `TransportDriver` implementations supervise ingress and hand observations to the bridge/router explicitly. Neither surface may invent or mutate canonical route truth.

This rule applies directly to the in-memory multi-device harness too. The shared in-memory transport may deliver `PayloadReceived` observations between attached endpoints, but it still does not choose routes, repair canonical state, or mint route handles. The reference-client bridge drains that ingress, attaches Jacquard time, and advances the router through explicit synchronous rounds. The router remains the semantic owner of canonical route truth even in tests.

## Aggregate And Constituent Bounds

`RoutingRuntimeEffects` is the aggregate marker for runtimes that provide the current minimal effect set. A host runtime that implements all five constituent traits automatically satisfies the aggregate, so routing engines can require one bound rather than five.

Code that needs the full runtime should bound on `RoutingRuntimeEffects`. Code that only touches one capability should bound on the narrower trait, which keeps test doubles small and signals intent in the function signature. A test that exercises planning under varied clock conditions can implement `TimeEffects` alone without supplying a transport or storage backend.

## What Is Not An Effect

Hashing stays a pure deterministic boundary through `Hashing` and `ContentAddressable` rather than an effect, since the same input must yield the same output across every runtime. Canonical route truth is published through route lifecycle objects, not transport observations or stored bytes. Supervision, engine lifetime, and route event interpretation belong to the host runtime above the effect surface.
