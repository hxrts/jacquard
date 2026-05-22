# Walkthrough

Each section establishes the vocabulary the next section assumes. Some terms used in Jacquard mean something different from their use in other systems. Those terms are flagged where they appear.

## 1. What Jacquard Is

Jacquard is a deterministic routing system for ad hoc networks that are unstable, capacity-constrained, and potentially adversarial. Nodes churn, links degrade, identities may be weak, and there is often no reliable global authority. See [Introduction](101_introduction.md) for the longer framing.

Jacquard owns the routing contract: a stable interface above the concrete routing algorithm. A host composes one or several engines behind that contract. The in-tree engines are pathway, BATMAN, Babel, OLSRv2, scatter, and Mercator. Multiple engines can be live at once for different traffic without sharing canonical route truth.

Three commitments shape everything else. Determinism rules out floats, ambient randomness, and wall-clock time. Explicit ownership keeps each role's allowed surface small and named. Explicit certainty levels make every routing input a typed object that records what was seen, what is believed, and what is established.

## 2. Shared World Model

Jacquard's shared world model uses three types in increasing order of certainty: `Observation<T>`, `Estimate<T>`, and `Fact<T>`. An observation is a raw input plus source, auth, evidence class, and `Tick`. An estimate is a belief derived from observations and carries a confidence permille from 0 to 1000. A fact is established routing truth produced by admission or publication.

See [Core Types](201_core_types.md) for the full type definitions. The evidence class on each observation discriminates the kind of source. The enum lives at `crates/core/src/base/qualifiers.rs`.

```rust
pub enum RoutingEvidenceClass {
    DirectObservation,
    PeerClaim,
    AdmissionWitnessed,
}
```

Engines use this class to weight a routing input. A peer claim is information but weaker than a direct observation. An admission witness is the strongest class and is only produced by the router on a successful admission.

## 3. Typed Time

Every observation carries a tick. Typed time is therefore part of the shared world model, not a transport concern. See [Time Model](202_time.md) for the definitions of:

- `Tick(u64)`: local monotonic round counter used for expiry, replay, and scheduling
- `OrderStamp(u64)`: total ordering used for content-addressing and tiebreaks
- `RouteEpoch(u64)`: versions topology reconfigurations so objects from different epochs are not interchangeable
- `DurationMs(u32)`: bounded duration with no negatives and no overflow

Determinism is the reason for this layer. If routing borrowed from wall-clock or OS scheduling, the simulator could not replay a run. Bugs would not reproduce. Engine behavior could not be reasoned about at the contract level.

## 4. The Routing Pipeline

The system runs a seven-stage pipeline. The stages and their producer are summarized below.

```
observation → estimate → fact → candidate → admission → materialization → publication
```

Stages one through three live in `core`. Observations arrive at a node through a transport driver, the bridge stamps each one with a `Tick`, and they land in the shared world model. The world model derives estimates such as link quality and peer liveness. An estimate firms up or is witnessed by admission, at which point it becomes a fact.

Stages four through seven belong to engines and the router. An engine emits a `RouteCandidate` from current facts, then performs an admission check that satisfies the router's objective and profile constraints and returns a route-shaped proof. The engine then materializes the admitted route under a router-issued canonical identity. The router publishes the canonical route. The engine produces proofs. The router publishes truth.

## 5. Roles and Ownership

There are four named roles with explicit ownership boundaries. See [Router Control Plane](304_router_control_plane.md) for the full breakdown.

| Role | Owns | Does not own |
|---|---|---|
| Engine (`RoutingEngine` impl) | Candidate production, admission checks, engine-private state | Ingress, time, canonical identity |
| Router (`jacquard-router`) | Canonical handles, leases, publications, round advancement | Any routing algorithm |
| Bridge (`ReferenceClient` and host-specific equivalents) | Ingress draining, `Tick` stamping, transport-to-router ferrying | Routing decisions |
| Driver (`TransportDriver` impl) | Transport-specific ingress, supervision, capability | Anything above the transport boundary |

The router is a registrar. It grants identity, holds leases, and publishes what engines have proven. It does not know how to find routes.

The engine is a strategy. It consumes the shared world model and emits proof-bearing candidates. It does not know what time it is or where packets come from. This separation is what lets multiple routing algorithms coexist behind one contract.

## 6. The Seven Engines

Each engine produces candidates from the same shared world model. Engines differ mostly in the shape of route they publish.

| Engine | Family | Publishes |
|---|---|---|
| `pathway` | Explicit-path source routing | `ExplicitPath` |
| `batman-bellman` | Enhanced BATMAN with local Bellman-Ford over gossip-merged topology | `NextHopOnly` |
| `batman-classic` | Spec-faithful BATMAN IV with OGM flooding and echo bidirectionality | `NextHopOnly` |
| `babel` | RFC 8966 distance-vector with ETX and feasibility distance | `NextHopOnly` |
| `olsrv2` | Proactive link-state with deterministic MPR election and TC flooding | `NextHopOnly` |
| `scatter` | Bounded deferred-delivery diffusion | `Opaque` (a viability claim) |
| `mercator` | Hybrid corridor routing with stale-safe repair and bounded custody fallback | `CorridorEnvelope` |

Mercator is the only engine that publishes a `CorridorEnvelope`. The rest of this document explains why that shape is necessary for the regime Mercator addresses.

## 7. Mercator: Problem and Corridor Shape

Pathway assumes a path exists end-to-end at the time the packet is forwarded. BATMAN, Babel, and OLSRv2 maintain next-hop tables on a generally connected mesh and tolerate local churn. Scatter handles deeply disconnected delivery but publishes only an opaque viability claim with no route shape.

Mercator targets the regime in between. Connectivity to a destination might be good for one window and bad in the next. The engine publishes a useful description of how to reach the destination when connectivity is good and degrades through bounded states when connectivity is poor.

The published route shape is a corridor. The struct lives at `crates/mercator/src/corridor.rs`.

```rust
pub struct MercatorCorridor {
    pub objective: MercatorObjectiveKey,
    pub primary: MercatorRouteRealization,
    pub alternates: Vec<MercatorRouteRealization>,
    pub topology_epoch: RouteEpoch,
}
```

The router only sees a `CorridorEnvelope` worth of route shape. The alternates are engine-private and let Mercator perform fast repair without republishing a new canonical route every time the primary wobbles. The published object stays stable while the underlying topology churns.

## 8. Mercator: Support State and Custody

A single objective's relationship with the network degrades along a named ladder. The enum lives at `crates/mercator/src/evidence.rs`.

```rust
pub enum MercatorSupportState {
    Fresh,
    Suspect,
    Repairing,
    Withdrawn,
    CustodyOnly,
}
```

The states replace the implicit flapping that would otherwise occur when every blip triggers a withdraw and re-announce cycle. An objective slides smoothly from confirmed support, through degraded states, to a custody-only fallback in which no connected route is supportable.

The custody fallback is bounded. The relevant config struct lives at `crates/mercator/src/public_state.rs`.

```rust
pub struct MercatorOperationalBounds {
    pub custody_copy_budget_max: u32,
    pub custody_protected_bridge_budget: u32,
    pub custody_payload_bytes_max: u32,
    pub custody_low_gain_floor: u16,
    pub custody_energy_pressure_threshold: u16,
    pub custody_leakage_risk_threshold: u16,
}
```

The phrase `bounded custody posture` is a contraction of these budgets. Every custody behavior is gated by a named finite quantity: how many copies of a payload may exist, how many protected bridges may carry it, how large a payload qualifies, how much expected forwarding gain is required before storing, and how much energy or leakage pressure is tolerated. Classical DTN custody is unbounded eventual-delivery semantics. Mercator's custody is policy-bounded and deterministic.

## 9. Mercator: Evidence Accumulation

Mercator maintains an internal bounded evidence graph at `crates/mercator/src/evidence.rs`. The graph records link support, reverse-link support, route support, broker pressure, service support, and custody opportunities. Each record carries `Tick`, `DurationMs`, `OrderStamp`, and `RouteEpoch`. Pruning is deterministic and uses score first, then canonical identity as tiebreak.

The evidence graph is engine-private. The router does not see it. It is the engine's digest of the shared world model, organized for corridor planning and repair.

Per the engine description in [AGENTS.md](../AGENTS.md), the engine continues to evolve. The corridor shape, evidence model, and support-state names are stable surfaces. The planner is the area still under iteration.

## 10. Running a Demo

The simplest end-to-end test exercises the full bridge to router to engine to publication loop on a four-node topology for ten rounds. The relevant test is at `crates/reference-client/tests/e2e_pathway_shared_network.rs`. Run it with:

```bash
cargo test -p jacquard-reference-client --test e2e_pathway_shared_network -- --nocapture
```

Pathway is a useful starting point because it is the simplest engine to read top-to-bottom. The contract being exercised, including observations, candidate production, admission, materialization, and publication, is identical across engines. Once this loop is understood, the Mercator code reads against the same shape.

The simulator can run experiment matrices across all seven engines and emit a PDF report. Generate one with `just tuning-local` and open `artifacts/analysis/<suite>/latest/router-tuning-report.pdf`. The report contains per-engine recommendations, transition stability, failure boundaries, and diffusion coverage.

## 11. Glossary

The following definitions are compact restatements of terms used above.

| Term | Definition |
|---|---|
| admission | An engine's check that a candidate satisfies a router objective and profile constraints, returning a route-shaped proof. |
| bridge | Host glue. Owns ingress draining and `Tick` stamping. Reference implementation is `ReferenceClient`. |
| candidate | A `RouteCandidate`. An engine's advisory output before admission. |
| choreography | An engine-private protocol state machine. In pathway, expressed via Telltale session-type macros. |
| corridor | Mercator's published route shape. A primary realization plus engine-private alternates sharing a topology epoch. |
| custody | Store-carry-forward of a payload when no connected route is supportable. |
| driver | A `TransportDriver`. Host-owned, transport-specific ingress and supervision. |
| engine | A `RoutingEngine` implementation. Owns a strategy and engine-private state. |
| epoch | A `RouteEpoch`. Versions a topology reconfiguration. Objects from different epochs are not interchangeable. |
| estimate | An `Estimate<T>`. A belief derived from observations with a confidence permille. |
| evidence | A provenance discriminator on routing inputs. |
| evidence class | A `RoutingEvidenceClass`, one of `DirectObservation`, `PeerClaim`, or `AdmissionWitnessed`. |
| fact | A `Fact<T>`. Established routing truth. |
| gateway | An objective variant. A destination role distinct from a plain node or service. |
| materialization | An engine realizing an admitted route under router-issued canonical identity. |
| objective | What a route is for. A node, a service, or a gateway. |
| observation | An `Observation<T>`. A raw input plus provenance and a tick. |
| OrderStamp | A deterministic total ordering used for content-addressing and tiebreaks. |
| pathway | The first-party explicit-path engine. Uses Telltale for choreography. |
| publication | The router announcing a canonical route and its commitments. |
| RouteEpoch | See epoch. |
| router | `jacquard-router`. Owns canonical identity, leases, publications, and round advancement. |
| support state | Mercator's per-objective ladder of `Fresh`, `Suspect`, `Repairing`, `Withdrawn`, and `CustodyOnly`. |
| Tick | A local monotonic round counter. |
| viability claim | What scatter publishes. An opaque indication that an objective is reachable eventually. Carries no path or next hop. |
