# Time Model

Contour uses a typed deterministic time model.
Time is part of the routing proof surface.
Raw timestamps are not sufficient because they hide domain meaning.
Each time value must state what kind of time it represents.

Clock time is a local choice in Contour.
It is valid for local expiration, retry, cooldown, and scheduling decisions.
It is not distributed semantic truth by itself.
The routing core must not treat wall clock as proof of ordering, completion, or agreement.

## Goals

The time model has four goals.

1. Preserve determinism in the routing core.
2. Keep elapsed time separate from topology versioning.
3. Make timeout and retry policy explicit and bounded.
4. Support production, simulation, and tests through one typed interface.

## Time Domains

Contour uses four core time domains.

- `Tick` for local monotonic time
- `DurationMs` for local durations
- `OrderStamp` for deterministic ordering that must not depend on wall clock
- `RouteEpoch` for topology and reconfiguration versioning

These domains must not be collapsed into one raw integer type.
`Tick` and `DurationMs` describe local waiting and expiration.
`OrderStamp` describes deterministic order.
`RouteEpoch` describes a version boundary in routing state.

```rust
pub struct Tick(pub u64);
pub struct DurationMs(pub u32);
pub struct OrderStamp(pub u64);
pub struct RouteEpoch(pub u64);
```

This block defines the canonical core time types. `Tick` is monotonic local time. `DurationMs` is a bounded duration unit. `OrderStamp` is a deterministic ordering token. `RouteEpoch` is a topology version, not elapsed time.

## Local Wall Clock

Wall clock time is a local integration concern.
It may exist at process boundaries, logs, metrics, and host integrations.
It must not define distributed routing truth.

This rule has direct consequences for routing behavior.
Route expiry may use local `Tick`.
Replay windows may use local `Tick`.
Retry policy may use local `Tick` and `DurationMs`.
None of those facts prove that another node observed the same time or reached the same conclusion.

Remote observation of another device clock must not occur inside the routing core.
If one device needs another device's clock-related state, that observation should happen at the application layer through explicit state exchange.
The routing core may carry that state as payload.
It must not treat remote clock observation as a native routing-time primitive.

This rule keeps temporal authority explicit.
One node may report its own local time state.
Another node may consume that report as application data.
That does not convert the remote clock into shared protocol truth.

## Validity Windows

Contour should represent validity as an explicit window.
This keeps start and end points together.
It also makes route and descriptor lifetime rules easier to audit.

```rust
pub struct TimeWindow {
    pub start_tick: Tick,
    pub end_tick: Tick,
}
```

This block defines a closed time interval in local monotonic time. It should be used for descriptor validity, route validity, replay windows, and any other bounded temporal surface in the routing core.

The following objects should use `TimeWindow` or `Tick` directly:

- service descriptor validity
- route expiry
- custody retention windows
- replay windows
- retry deadlines

## Timeout And Backoff Policy

Timeout behavior is a local owner choice.
It is not protocol ordering.
It is not semantic completion evidence.
It is not proof that a route succeeded or failed globally.

Every retry loop and waiting policy must be bounded.
Contour should use one typed timeout policy across production, simulation, and tests.
Different environments may change the time source or policy values.
They must not change the semantic meaning of success and failure.

```rust
pub struct TimeoutPolicy {
    pub attempt_count_max: u32,
    pub initial_backoff_ms: DurationMs,
    pub backoff_multiplier_permille: u16,
    pub backoff_ms_max: DurationMs,
    pub overall_deadline_ms: DurationMs,
}
```

This block defines a bounded local waiting policy. `attempt_count_max` limits retries. `initial_backoff_ms` and `backoff_ms_max` bound waiting. `overall_deadline_ms` gives the owner one total budget instead of several unrelated timeout domains.

Timeout policy should be used for:

- local route establishment deadlines
- transport retry budgets
- bounded exponential backoff
- maintenance and repair retry windows

Timeout policy should not be used for:

- causal ordering
- route-family comparison semantics
- global completion evidence
- topology versioning

## Deterministic Ordering

Some routing choices require a deterministic tie break.
Wall clock is not an acceptable tie break.
Floating-point scoring is not acceptable either.
Contour should use `OrderStamp` or an explicit stable ordering rule.

Typical uses for deterministic ordering are:

- candidate tie breaks
- replay queue ordering
- stable sorting of equal-cost routes
- deterministic simulation output

If two route candidates are equal under the main comparison rule, the implementation should use one explicit secondary rule.
That rule should be stable across hosts.
Examples include `NodeId` order, route identifier order, or `OrderStamp`.

## Time Effects

Contour should access time through injected effects.
Application code and routing code should not call raw wall-clock APIs directly.
This keeps production, simulation, and tests on one semantic model.

```rust
pub trait TimeEffects {
    fn now_tick(&self) -> Tick;
}

pub trait OrderEffects {
    fn next_order_stamp(&mut self) -> OrderStamp;
}
```

This block defines the minimum time boundary for the routing core. `TimeEffects` provides monotonic local time. `OrderEffects` provides deterministic ordering tokens when an explicit stable order is needed.

## Domain Rules

The routing core should follow these rules:

- Use `Tick` for route expiry, maintenance scheduling, and replay checks.
- Use `DurationMs` for timeout budgets and backoff values.
- Use `OrderStamp` only for deterministic ordering, not for expiration.
- Use `RouteEpoch` only for topology and reconfiguration versioning.

The routing core should reject these patterns:

- using wall clock as protocol truth
- storing raw `u64` timestamps without a domain type
- using timeout expiry as proof of distributed failure
- mixing topology version and elapsed time in one field
- observing a remote device clock directly inside routing logic instead of receiving explicit application-layer state

## Testing And Simulation

Tests and simulation should use a controllable time source.
That source should produce `Tick` and `OrderStamp` values through the same traits used in production.
Only the source and policy values should change across environments.
The semantic contract should stay the same.

This rule matters for routing because replay windows, retries, and route expiry are all temporal surfaces.
If tests use a different semantic model than production, deterministic behavior will drift.
Contour should avoid that split from the start.

## Summary

Contour should use typed local monotonic time for routing.
Clock time is a local choice.
It is valid for local expiration and waiting.
It is not distributed semantic truth.

`Tick`, `DurationMs`, `OrderStamp`, `RouteEpoch`, `TimeWindow`, and `TimeoutPolicy` are the core model.
They keep the routing core deterministic, explicit, and testable.
They also give host systems enough structure to integrate Contour without forcing raw wall-clock semantics into the protocol layer.
