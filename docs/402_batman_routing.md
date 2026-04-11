# Batman Routing

`jacquard-batman` is Jacquard's proactive next-hop reference routing engine. It implements a BATMAN-style originator-message model over the shared Jacquard world picture. The engine declares `RouteShapeVisibility::NextHopOnly`, meaning it honestly reports only which direct neighbor to use toward a destination rather than claiming knowledge of the full path. Its contract identifier is `BATMAN_ENGINE_ID`, fixed as the 16-byte string `jacquard.batman.`.

Batman is transport-neutral. It operates alongside other engines on a shared multi-engine router without assuming a specific link layer. The router retains canonical route publication, handle issuance, and lease management. Batman owns proactive originator observations, neighbor ranking, and best-next-hop state within its own crate boundary.

## Shared Inputs

Batman consumes the shared world model from `jacquard-core` without reshaping it. The engine reads `Observation<Configuration>`, `Node`, `Link`, and `ServiceDescriptor` values that are already merged by the host before each tick. It does not maintain a second advertisement schema on top of the shared configuration surface.

Destination eligibility is checked against `ServiceDescriptor` before batman produces a candidate. A destination node must declare support for the batman engine through its shared service surface before the engine will emit a `RouteCandidate` toward it. This check happens in the planner, not during gossip. See [Pathway Routing](401_pathway_routing.md) for a description of the shared planning contract that both engines implement.

## Gossip Protocol

Each node builds a local `OriginatorAdvertisement` from its current topology observation. The advertisement carries the originator's `NodeId`, a monotonically increasing sequence number, and one `AdvertisedLink` entry per directly reachable neighbor. Each link entry includes the destination node, transport kind, runtime state, and optional delivery confidence.

Advertisements are framed before transmission. `encode_advertisement` prepends the eight-byte magic prefix `JQBATMAN` and then appends the bincode-serialized advertisement body.

```text
[JQBATMAN (8 bytes)] [bincode-serialized OriginatorAdvertisement]
```

The receiver calls `decode_advertisement`, which rejects payloads that do not start with the magic prefix and returns `None` for any deserialization failure. This framing lets batman payloads be identified and discarded without touching the deserialization layer when the prefix does not match.

`flood_gossip` runs each tick. It sends both the local node's freshly built advertisement and all currently stored learned advertisements to every direct neighbor endpoint. Received payloads enter `ingest_advertisement`, which drops any advertisement whose originator matches the local node and discards any advertisement whose sequence number is not strictly greater than the stored sequence for that originator.

`merge_advertisements` folds learned advertisements into a copy of the current topology observation. For each non-stale learned advertisement, it inserts synthesized `Link` entries for gossip-discovered edges that are not already present in the direct topology view. Only advertisements received within the staleness window contribute to the merged view used for shortest-path computation.

## Transmit Quality

TQ (transmit quality) is a permille scalar on the range 0–1000. It measures the estimated end-to-end delivery quality for a link or path segment. All TQ arithmetic uses fixed-width integers. No floating-point operations appear in the scoring path.

`derive_tq` computes TQ for a single link. It always starts from an OGM-equivalent baseline derived from `LinkRuntimeState`.

| `LinkRuntimeState` | Baseline TQ |
|---|---|
| `Active` | 900 |
| `Degraded` | 650 |
| `Suspended` | 250 |
| `Faulted` | 0 |

When richer link observations are present in `LinkState`, `derive_tq` includes them as additional terms in a running average. The optional enrichments are `delivery_confidence_permille`, `symmetry_permille`, `transfer_rate_bytes_per_sec` (normalized against a 128 kbps saturation ceiling), and `stability_horizon_ms` (normalized against a 4000 ms saturation ceiling). The baseline is always included. The final TQ is the integer average over all contributing terms.

Links with a derived TQ below 700 are classified as `RouteDegradation::Degraded`. Links at or above 700 carry `RouteDegradation::None`.

`tq_product` propagates quality across a two-hop path using the classical BATMAN rule.

```text
tq_product(left, right) = (left * right) / 1000
```

The left and right values are both permille scalars. The product is divided by 1000 to keep the result on the same 0–1000 scale. This means a path through two links each at 900 TQ yields a compound score of 810. Multi-hop paths accumulate `tq_product` applications in sequence, so quality degrades monotonically with hop count.

## Observation and Ranking

`refresh_private_state` runs at each tick after the topology is merged with learned advertisements. It walks every direct neighbor of the local node and runs a shortest-path computation from each neighbor to every reachable originator. For each originator reached through a given neighbor, it applies `derive_tq` to the direct link and then accumulates `tq_product` along the shortest path to produce a per-(originator, neighbor) TQ score. These scores populate the `OriginatorObservationTable`.

The engine retains prior observations that remain within the `DecayWindow`. The default `DecayWindow` marks observations stale after 8 ticks and triggers a refresh within 4 ticks. Observations outside the staleness window are dropped before ranking.

`NeighborRanking` is the ordered list of candidate next-hops for a single originator. Candidates are sorted by TQ descending, then by hop count ascending, then by `NodeId` ascending for determinism. The top entry from each ranking becomes the `BestNextHop` for that originator. `BestNextHop` carries the next-hop `NodeId`, TQ score, hop count, observed tick, transport kind, degradation status, and a derived `BackendRouteId`.

## Planning and Admission

`RoutingEnginePlanner::candidate_routes` iterates the `BestNextHop` table and emits at most one `RouteCandidate` per reachable destination. A candidate is only emitted when the destination's `ServiceDescriptor` declares batman engine support. The candidate carries the `BackendRouteId` derived from the current best-next-hop entry as an opaque backend reference.

`admit_route` validates an incoming candidate against the live `BestNextHop` table. It checks that the candidate's backend reference still matches the current best-next-hop entry for the destination. On success it returns a `RouteAdmission` with a `RouteWitness` derived from the current observation. A stale or superseded backend reference results in an inadmissible response rather than silent acceptance.

`check_candidate` delegates directly to `admit_route` and returns the corresponding `RouteAdmissionCheck`. The engine declares `decidable_admission: Supported` in its static capability envelope, so the router may call `check_candidate` without treating the result as advisory. Batman does not produce multi-hop or disjoint candidates. Route shape visibility is `NextHopOnly` for every admitted route.

## Route Lifecycle

### Materialization

`materialize_route` decodes the admitted backend reference and looks up the corresponding entry in the `BestNextHop` table. It records an `ActiveBatmanRoute` keyed by `RouteId` in the active routes table. Route health is derived directly from the TQ of the installed next-hop: `HealthScore` is set to the raw TQ value and `PenaltyPoints` is set to `1000 - tq`. The returned `RouteProgressContract` uses `Limit::Bounded(1)` for both productive and total progress steps. Batman does not synthesize route health before `engine_tick` has supplied a topology observation.

### Maintenance

`maintain_route` checks whether the installed route's next-hop has been superseded. It returns `ReplacementRequired` when the best-next-hop table now names a different neighbor for the destination. It returns `Failed(LostReachability)` when the destination has no entry in the best-next-hop table at all. When the current next-hop is still the best available it returns `Continued` and updates the route health from the current TQ. Batman does not implement suffix repair. Route replacement is the only reconfiguration path when the next-hop changes.

### Forwarding

`forward_payload_for_router` looks up the `ActiveBatmanRoute` for the given `RouteId` and resolves the next-hop's endpoint from the current topology observation. It sends the payload to that endpoint via `TransportSenderEffects`. Batman does not buffer payloads. There is no hold support and no deferred-delivery path. A payload sent to a route with no resolvable endpoint fails immediately.

## Capabilities and Boundaries

Batman declares a fixed static capability envelope through `BATMAN_CAPABILITIES`.

| Capability | Value |
|---|---|
| `max_protection` | `LinkProtected` |
| `max_connectivity` | `ConnectedOnly` |
| `repair_support` | `Unsupported` |
| `hold_support` | `Unsupported` |
| `decidable_admission` | `Supported` |
| `quantitative_bounds` | `ProductiveOnly` |
| `reconfiguration_support` | `ReplaceOnly` |
| `route_shape_visibility` | `NextHopOnly` |

Batman does not implement repair or hold. A route that loses its best next-hop is replaced rather than repaired. A route that loses reachability entirely is failed with `LostReachability`. The explicit capability declaration communicates these constraints to the router without requiring behavioral inference. The router and host can rely on these static values to decide which engine to prefer for a given route objective.
