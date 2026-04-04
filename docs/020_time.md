# Time Model

Contour uses a typed deterministic time model. It does not treat wall clock as distributed truth. The routing core works with local monotonic time, bounded durations, deterministic ordering tokens, and topology epochs.

See [Core Types](010_core_types.md) for the shared model objects that use these domains. See [Routing Observation Boundary](025_routing_observation_boundary.md) for the observation surfaces that carry many of the time-shaped fields. See [Routing Architecture](030_routing_architecture.md) for the runtime layers that consume them.

## Time Domains

`Tick` is local monotonic time. It is used for expiry, replay checks, scheduling, and publication timestamps. `DurationMs` is a bounded duration type for timeout and backoff policy. `OrderStamp` is a deterministic ordering token. `RouteEpoch` versions topology and reconfiguration state.

These domains are not interchangeable. `Tick` is not wall clock. `OrderStamp` is not an expiry. `RouteEpoch` is not elapsed time. Field names should carry their domain when needed, such as `*_tick`, `*_ms`, and `*_epoch`.

```rust
pub struct Tick(pub u64);
pub struct DurationMs(pub u32);
pub struct OrderStamp(pub u64);
pub struct RouteEpoch(pub u64);
```

Each type is a newtype over a fixed-width integer. They are distinct at the type level so the compiler rejects accidental mixing.

## Local Choice

Clock time is a local choice in Contour. It is valid for local waiting, retry, retention, and expiry decisions. It is not proof that another node observed the same event or reached the same conclusion.

Remote observation of another device clock must stay above the routing core. If a host needs to exchange time-related state, it should pass that state explicitly as application data. The routing core may carry the data, but it must not treat a remote clock as native routing truth.

## Runtime Boundary

Contour accesses time and deterministic ordering through abstract effects. `TimeEffects` provides `Tick`. `OrderEffects` provides `OrderStamp`. This keeps production, tests, and simulation on one semantic model even when their underlying runtimes differ.

`TimeWindow` and `TimeoutPolicy` are the main compound time objects in the model. `TimeWindow` is used for bounded validity. `TimeoutPolicy` is used for bounded retries and local waiting policy. Both stay in the deterministic time domain and avoid raw timestamp fields.

```rust
pub struct TimeoutPolicy {
    pub attempt_count_max: u32,
    pub initial_backoff_ms: DurationMs,
    pub backoff_multiplier_permille: RatioPermille,
    pub backoff_ms_max: DurationMs,
    pub overall_timeout_ms: DurationMs,
}
```

`TimeoutPolicy` governs all bounded retry and backoff behavior. The multiplier uses `RatioPermille` rather than a floating-point scale factor.
