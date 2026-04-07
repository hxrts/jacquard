# Runtime Effects

This page describes the narrow runtime capability surface that Jacquard exposes to pure routing logic. See [World Extensions](106_world_extensions.md) for the layering overview, [Routing Engines](107_routing_engines.md) for the engine and policy contracts, and [Mesh Routing](109_mesh_routing.md) for the in-tree mesh implementation and its operational subcomponents.

## Effect Surface

Runtime effects are the lowest-level extensibility surface in Jacquard. They expose narrow runtime capabilities to pure routing logic. They are useful when a runtime or host needs to swap out how routing code gets time, storage, transport, or route-event logging services without changing the routing logic itself. Hashing is modeled separately as a pure deterministic boundary, not a runtime effect. None of these traits own route semantics, supervision, or canonical route state.

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

pub trait TransportEffects {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError>;

    fn poll_transport(&mut self) -> Result<Vec<TransportObservation>, TransportError>;
}

pub trait RoutingRuntimeEffects:
    TimeEffects + OrderEffects + StorageEffects + RouteEventLogEffects + TransportEffects
{}
```

Each effect trait covers one concern. `TimeEffects` provides monotonic local time. `OrderEffects` provides deterministic ordering tokens. `StorageEffects` provides byte-level key-value persistence. `RouteEventLogEffects` provides replay-visible route event recording. `TransportEffects` provides frame send and transport observation polling.

`RoutingRuntimeEffects` is the aggregate marker for runtimes that provide the current minimal effect set. A host runtime that implements all five constituent traits automatically satisfies the aggregate, so routing engines can require one bound rather than five.
