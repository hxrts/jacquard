# Custom Transport

This guide walks through adding a custom link: the byte-carrying transport surface engines send through, plus the link-level profile that wraps it. It targets 3rd parties replacing the in-memory transport with something runtime-specific, for example a TCP, BLE, or LoRa carrier.

See [Profile Implementations](305_profile_reference.md) for the shared profile boundary. See [Reference Client](408_reference_client.md) for the host bridge composition the custom transport plugs into. See [Crate Architecture](999_crate_architecture.md) for the ownership rules the transport layer must respect.

## What A Transport Owns

A transport in Jacquard has two surfaces. Engines send payloads through `TransportSenderEffects` during a synchronous round. The host owns ingress and supervision through `TransportDriver`. The bridge attaches `Tick` to each ingress event before delivering it to the router.

Engines must not own async I/O directly. They must not poll the transport driver. They must not attach time. A 3rd party implementing a new transport replaces the two trait surfaces and hands the result to the bridge, which keeps those ownership boundaries.

Reuse `jacquard-adapter` for generic mailbox, peer-directory, or claim-ownership scaffolding when the transport needs those primitives. Do not introduce a pathway-specific or engine-specific transport trait. Keep the transport transport-neutral.

## Implementing TransportSenderEffects

`TransportSenderEffects` is the synchronous send capability the router hands to each engine. An implementation takes a `LinkEndpoint` and a payload, dispatches it through the runtime-specific carrier, and returns an error on failure.

```rust
use jacquard_core::LinkEndpoint;
use jacquard_traits::{TransportSenderEffects, TransportError};
use std::sync::mpsc::Sender;

pub struct ChannelSender {
    outbound: Sender<(LinkEndpoint, Vec<u8>)>,
}

impl TransportSenderEffects for ChannelSender {
    fn send_transport(
        &mut self,
        endpoint: &LinkEndpoint,
        payload: &[u8],
    ) -> Result<(), TransportError> {
        self.outbound
            .send((endpoint.clone(), payload.to_vec()))
            .map_err(|_| TransportError::Unavailable)
    }
}
```

The sender runs inside a deterministic round, so the implementation must complete quickly. If the underlying carrier needs async dispatch, buffer the outbound payload here and flush it on the driver side. Do not block the round on runtime I/O.

## Implementing TransportDriver

`TransportDriver` is the host-owned ingress surface. The bridge calls `drain_transport_ingress` once per round, attaches `Tick` to every returned event, and feeds the stamped events into the router as observations. The driver also owns lifecycle: `shutdown_transport_driver` releases runtime resources when the bridge is torn down.

```rust
use jacquard_traits::{TransportDriver, TransportError, TransportIngressEvent};
use std::sync::mpsc::Receiver;

pub struct ChannelDriver {
    inbound: Receiver<TransportIngressEvent>,
}

impl TransportDriver for ChannelDriver {
    fn drain_transport_ingress(
        &mut self,
    ) -> Result<Vec<TransportIngressEvent>, TransportError> {
        let mut batch = Vec::new();
        while let Ok(event) = self.inbound.try_recv() {
            batch.push(event);
        }
        Ok(batch)
    }
}
```

The driver returns a bounded batch per round, not a stream. Unbounded draining would let one round consume arbitrarily much wall-clock time. A real driver caps the drain to a per-round limit and defers remaining events to the next round.

The bridge attaches `Tick` to each event before delivery. The driver must not populate a tick field on its own. Handing the driver ownership of time is the primary failure mode for custom transports.

## Building A Link Profile

A link profile wraps the transport surfaces with the pieces the shared `Link`, `LinkEndpoint`, and `LinkState` vocabulary needs. `jacquard-mem-link-profile` is the canonical example. It provides `SimulatedLinkProfile`, `SharedInMemoryNetwork`, `InMemoryTransport`, `InMemoryRetentionStore`, and `InMemoryRuntimeEffects`.

```rust
use jacquard_core::{Link, LinkEndpoint};
use jacquard_traits::{RetentionStore, TransportDriver, TransportSenderEffects};

pub struct ChannelLinkProfile {
    sender: ChannelSender,
    driver: ChannelDriver,
    retention: ChannelRetentionStore,
}

impl ChannelLinkProfile {
    pub fn new(/* runtime-specific config */) -> Self {
        todo!()
    }

    pub fn sender(&mut self) -> &mut ChannelSender { &mut self.sender }
    pub fn driver(&mut self) -> &mut ChannelDriver { &mut self.driver }
    pub fn retention(&mut self) -> &mut ChannelRetentionStore { &mut self.retention }
}
```

The profile does not itself implement the effect traits. It bundles individual implementors and hands them to the bridge when the client is composed. A 3rd party composes the sender, driver, and retention store through whatever builder pattern fits the runtime. The reference client uses one per-engine sender, one shared driver, and one retention store per host.

For the retention store, either reuse `InMemoryRetentionStore` from `jacquard-mem-link-profile` or implement `RetentionStore` against persistent storage. Retention is independent of transport, so a custom transport paired with the in-memory retention store is a reasonable first pass.

## Shaping Cast Evidence

Custom transports own physical transport facts. A LoRa profile owns spreading factor, duty cycle, gateway behavior, and acknowledgement limits. A BLE profile owns scan windows and advertising behavior. A satellite profile owns contact schedules. These facts stay in the transport-owned profile crate.

Use `jacquard-cast-profile` when the profile needs to shape those facts into bounded unicast, multicast, or broadcast evidence. The helper crate sorts receiver sets deterministically, enforces explicit bounds, carries typed freshness and capacity fields, and keeps directional support separate from reverse confirmation. The helper does not implement a transport.

`jacquard-adapter` remains host plumbing. Use it for mailbox, peer-directory, endpoint convenience, and claim-ownership support. Do not put profile evidence logic there.

## Composing With A Host Bridge

The composed profile plugs into the reference client's `ClientBuilder` or into a custom host bridge. The default builder currently expects the in-memory network, so a non-default transport requires composing the router and engines directly or forking the builder. See [Reference Client](408_reference_client.md) for the composition the reference client exposes.

The minimum composition wires three things together. First, a router that owns canonical route publication. Second, one or more engines registered on that router, each holding a queue-backed `TransportSenderEffects` handle. Third, a host bridge that owns the `TransportDriver`, drains ingress, stamps `Tick`, and advances the router through synchronous rounds.

For end-to-end host composition patterns, see [Client Assembly](503_client_assembly.md) and [Crate Architecture](999_crate_architecture.md).
