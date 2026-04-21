# Reference Client

`jacquard-reference-client` is the host-bridge composition that wires the router, engines, transport driver, and profile crates into a single runnable host. It is the canonical example of how a Jacquard client is assembled. Integration tests and the simulator use it as the default host. Downstream consumers can use it as a library with stock components or as a starting point for their own composition.

See [Profile Implementations](305_profile_reference.md) for the profile-boundary spec. See [Client Assembly](503_client_assembly.md) for a library-consumer walkthrough.

## What The Reference Client Provides

`ClientBuilder` is the wiring entry point. It attaches one bridge-owned `InMemoryTransport` driver to a `SharedInMemoryNetwork`, constructs queue-backed sender capabilities for each enabled engine, registers the engine set on a fresh `MultiEngineRouter`, and returns a `ReferenceClient` host bridge.

The builder accepts any combination of in-tree engines: pathway, batman-bellman, batman-classic, babel, olsrv2, field, and scatter. The `EngineKind` enum names the selectable engines. Multiple clients built against the same network share one deterministic carrier. Each client still advances routing state through its own explicit bridge rounds.

The bridge surface exposes `HostBridge` and `BoundHostBridge` for binding and round advancement. Round outcomes flow through `BridgeRoundReport` and `BridgeWaitState`, with `BridgeQueueConfig` controlling ingress queueing behavior. Together these let a caller drive synchronous rounds, inspect per-round outcomes, and observe the bridge's waiting behavior.

## Ownership Boundaries

The bridge owns three responsibilities the consumer must not bypass. It owns the transport driver, so engines never hold onto async I/O directly. It owns ingress draining, which converts raw transport input into the shared observation surface. It owns `Tick` stamping, which attaches Jacquard logical time at the ingress boundary.

Engines retain their private runtime state under the shared router contract. The router owns canonical route truth, handle issuance, and lease management. The reference client wires these pieces together but does not mutate canonical truth on their behalf.

Profile types flow through unchanged. `NodeProfile`, `NodeState`, `ServiceDescriptor`, `Link`, `LinkEndpoint`, and `LinkState` keep their shared-model shape end to end. The reference client only composes them into a runnable bridge.

## Reference Tests

The reference client's test suite is the canonical living example of host composition. The tests at `crates/reference-client/tests/` exercise client builder options, pathway-on-shared-network flows, batman-pathway handoff, olsrv2 handoff, and shared scenarios from the testkit. They serve as executable documentation for how the builder and bridge fit together.

Shared scenario helpers live at `crates/testkit/src/reference_client_scenarios.rs`. A 3rd party composing their own scenarios can mirror the pattern there rather than reimplementing the builder plumbing.
