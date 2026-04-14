# Babel Routing

`jacquard-babel` (engine ID `jacquard.babel..`) implements the Babel distance-vector routing protocol as described in RFC 8966. It uses bidirectional ETX link cost, additive path metric, and a feasibility distance table for loop-free route selection. This is the third distance-vector engine in Jacquard, alongside `jacquard-batman-bellman` and `jacquard-batman-classic`.

## Protocol Overview

Babel is a distance-vector routing protocol designed for wireless mesh networks. Each node originates route updates advertising itself as a destination with metric 0. Relay nodes add their local link cost before re-advertising the best route. Downstream nodes select the path with the lowest total metric.

Three properties distinguish Babel from the batman engines in Jacquard. First, link cost uses bidirectional ETX rather than forward-only TQ. Second, path metric is additive rather than multiplicative. Third, route selection is gated by a feasibility condition that provides loop freedom during transient topology changes.

## Shared Inputs

The Babel engine consumes `Observation<Configuration>` from the shared Jacquard world model. Destination eligibility is checked against `ServiceDescriptor` before the engine produces a candidate. A destination node must declare support for the engine's specific ID in its shared service surface before the engine emits a `RouteCandidate` toward it. See [Pathway Routing](401_pathway_routing.md) for the shared planning contract all engines implement.

## Update Structure

The Babel update carries four fields:

```text
BabelUpdate {
    destination: NodeId,
    router_id: NodeId,
    seqno: u16,
    metric: u16,
}
```

The originator sets `metric=0` and assigns a monotonically increasing `seqno`. Each relay node adds the local link cost to the metric before re-advertising. The `router_id` identifies the originator of the route entry. Updates are framed with the eight-byte magic prefix `JQBABEL.` and bincode-serialized.

No TTL field is present. Propagation depth is controlled by the decay window: stale entries are pruned when `observed_at_tick` exceeds `stale_after_ticks`. No hop-count bound is needed because only the selected route per destination is re-advertised, and infeasible routes are rejected by the feasibility condition.

## ETX Link Cost

Link cost uses the Expected Transmission Count formula:

```text
cost = 256 * 1_000_000 / (fwd_delivery_permille * rev_delivery_permille)
```

This captures bidirectional link quality. A perfectly symmetric active link (1000 permille in both directions) yields cost 256. An asymmetric link where the forward direction is good (980 permille) but the reverse is poor (300 permille) yields cost 871. The formula penalizes asymmetric links more heavily than BATMAN's forward-only TQ because poor reverse delivery means acknowledgments are lost, increasing true retransmission count.

If either direction is absent or faulted (delivery 0), cost equals `BABEL_INFINITY` (0xFFFF), making the route unusable. This replaces the echo-window bidirectionality gate used by batman-classic. No separate bidirectionality check is needed because asymmetry is encoded directly in the metric.

## Additive Metric

Path metric is the sum of link cost and the neighbor's reported metric:

```text
compound_metric = link_cost + neighbor_metric
```

If either input equals `BABEL_INFINITY`, the result is `BABEL_INFINITY`. Otherwise the sum saturates at `BABEL_INFINITY - 1` (0xFFFE). The metric scale runs from 0 (perfect local route) to 0xFFFF (unreachable). Values at or above 0xFFFF are treated as unreachable.

This additive model differs from BATMAN's multiplicative TQ product. A single bad hop in a multi-hop path raises the total metric by its full link cost. In BATMAN, the same bad hop would reduce the multiplicative product less dramatically relative to other hops. Babel therefore discriminates more strongly against paths with one weak link among otherwise strong links.

## Feasibility Distance Table

Every node maintains a per-destination feasibility distance `FD[D]` stored as a `(seqno, metric)` pair. A route entry for destination D is feasible if and only if:

```text
seqno_is_newer(entry.seqno, FD[D].seqno)
  OR (entry.seqno == FD[D].seqno AND entry.metric < FD[D].metric)
```

The `seqno_is_newer` function uses modular arithmetic over u16 as defined in RFC 8966 Section 3.5.1. A seqno is newer if the unsigned distance `(candidate - reference) mod 2^16` falls in the range `(0, 2^15)`.

When FD is absent for a destination (never selected, or all routes expired), any finite-metric route is feasible. The feasibility condition prevents routing loops during transient topology changes. It ensures that a node never selects a route whose metric has increased relative to its last feasibly selected route, unless a newer seqno proves that the originator has acknowledged the topology change.

### Admission vs Selection

Updates are always admitted to the route table. The feasibility condition gates selection only. This matches RFC 8966: a node stores all received route entries and uses the FC only when choosing which route to select.

### FD Update Rules

FD is updated to `(seqno, metric)` of the selected route only when the selection is feasible. Infeasible fallback selections leave FD unchanged. This preserves the loop-freedom guarantee: the FD ratchet never moves backward.

### Infeasible Fallback

When no feasible route exists for a destination, the engine selects the best infeasible route to preserve connectivity. This selection does not update FD. The periodic seqno increment (every 16 ticks) propagates a fresh seqno from the originator. When that update arrives, it satisfies the feasibility condition (newer seqno) and allows FD to be updated, ending the fallback period. This replaces the explicit SEQREQ mechanism from RFC 8966 with a bounded periodic refresh.

### FD Expiry

When all routes to a destination expire from the route table, FD for that destination is removed. The next route learned will be treated as if FD is absent (any finite metric is feasible).

## Sequence Number Management

The originator seqno is incremented every `SEQNO_REFRESH_INTERVAL_TICKS` (16 ticks). This periodic increment serves as the mechanism for resolving infeasible-fallback states across the network. The seqno uses u16 with modular arithmetic and wraps at 2^16.

No explicit seqno request mechanism is implemented. In the full RFC 8966 protocol, a node can send a SEQREQ to the originator asking it to bump its seqno immediately. In the Jacquard tick model, the periodic increment bounds the infeasible-fallback window to at most 16 ticks without requiring asynchronous request handling.

## Selected-Route Flooding

Each tick, the engine floods two types of updates to all direct neighbors. The first is the local node's originated update with the current seqno and metric 0. The second is a re-advertisement of the best selected route per destination. Non-selected routes are not re-broadcast.

This differs from batman-classic, which re-broadcasts all received OGMs. Babel's selected-route flooding reduces overhead and works in concert with the feasibility condition to provide loop freedom.

## Decay Window

`DecayWindow` governs route entry freshness. The default marks entries stale after 8 ticks and expects the next refresh within 4 ticks. Both parameters are configurable via `BabelEngine::with_decay_window`. Stale entries are pruned during each refresh pass before route selection.

The decay window is identical in shape to the one used by both batman engines. All three engines accept `with_decay_window` at construction for tuning.

## Quality Scoring

The engine converts Babel metric to a `RatioPermille` quality score using a linear mapping. Metric 0 maps to quality 1000 (perfect). Metric values at or above 1024 map to quality 0. Routes with metric at or above 512 are classified as degraded.

## Planning, Admission, and Lifecycle

Planning, admission, and route lifecycle follow the shared contract used by all Jacquard engines. The planner checks the destination's `ServiceDescriptor` for the Babel engine ID before emitting any candidate.

`candidate_routes` emits at most one `RouteCandidate` per reachable destination. `admit_route` validates the candidate's `BackendRouteId` against the current best next-hop table. A stale or superseded reference is inadmissible. `materialize_route` records an active route and derives health from the quality score.

`maintain_route` returns `ReplacementRequired` when the best next-hop has changed. It returns `Failed(LostReachability)` when the destination has no table entry. Route replacement is the only reconfiguration path. The engine does not implement suffix repair or hold.

## Capabilities

The Babel engine declares the same capability envelope as both batman engines:

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

## Comparison with Batman Engines

### vs Batman-Classic

Batman-classic is the correct comparison baseline for Babel. Both are pure distance-vector gossip protocols without local topology reconstruction. Babel addresses three gaps relative to batman-classic. Asymmetric-link handling: batman-classic's bidirectionality gate excludes poor-reverse paths entirely, while Babel's ETX cost encodes asymmetric quality as a finite metric. Loop freedom: batman-classic relies on sequence-number freshness, while Babel's feasibility condition provides a formal guarantee. Propagation: batman-classic re-broadcasts all received OGMs, while Babel forwards only the selected route.

### vs Batman-Bellman

Batman-bellman replaces the spec's distributed TQ propagation with a local Bellman-Ford computation over a gossip-merged topology graph. Comparing babel against batman-bellman conflates the Bellman-Ford and topology-enrichment changes with the DV-gossip differences, making performance attribution unreliable. For protocol-level comparison, use batman-classic.
