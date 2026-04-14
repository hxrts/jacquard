# Batman Routing

Two BATMAN routing engines are provided. Each implements the proactive originator-message model over the shared Jacquard world picture.

- `jacquard-batman-bellman` (engine ID `jacquard.batmanb`) is the Jacquard-enhanced engine. It replaces the spec's distributed TQ propagation with a local Bellman-Ford computation over a gossip-merged topology graph. It enriches TQ with Jacquard link beliefs and includes a bootstrap shortcut for tick-1 route availability. This is the engine measured in the tuning corpus.

- `jacquard-batman-classic` (engine ID `jacquard.batmanc`) is a spec-faithful engine. It implements the BATMAN IV originator-message model without structural departures. TQ is carried in the OGM and updated by each re-broadcasting node. No candidate is emitted before receive-window data has accumulated.

Both engines declare `RouteShapeVisibility::NextHopOnly` and the same capability envelope. They are transport-neutral and operate alongside other engines on a shared multi-engine router. The router retains canonical route publication, handle issuance, and lease management. Batman owns proactive originator observations, neighbor ranking, and best-next-hop state within its own crate boundary.

---

## Shared Inputs

Both engines consume `Observation<Configuration>` from the shared Jacquard world model. Destination eligibility is checked against `ServiceDescriptor` before either engine produces a candidate. A destination node must declare support for the engine's specific ID in its shared service surface before the engine emits a `RouteCandidate` toward it. See [Pathway Routing](404_pathway_routing.md) for the shared planning contract both engines implement.

---

## Classic BATMAN (`jacquard.batmanc`)

### OGM Structure

The classic engine's originator advertisement carries only the fields required by the spec:

```text
OriginatorAdvertisement {
    originator: NodeId,
    sequence: u64,
    tq: RatioPermille,   // path quality from this node to originator; 1000 at source
    ttl: u8,             // hops remaining; decremented at each relay
}
```

No per-link state is included. Quality information travels as the `tq` scalar, which each re-broadcasting node updates before forwarding. Advertisements are framed with the eight-byte magic prefix `JQBATMNC` and bincode-serialized.

### Flooding and TTL

`flood_gossip` runs each tick. It sends the local originator OGM (`tq=1000`, `ttl=DEFAULT_OGM_TTL=50`) to every direct neighbor. It also sends a re-broadcast copy of each learned OGM whose TTL has not reached zero.

Before forwarding a learned OGM, the engine computes its path quality to the originator and encodes it in the outgoing advertisement:

```text
rebroadcast_tq = tq_product(link_state_tq_to_sender, received_tq)
rebroadcast_ttl = received_ttl - 1
```

OGMs with `ttl=0` are discarded and not forwarded. This bounds propagation to at most `DEFAULT_OGM_TTL` relay hops from the originator. Stale OGMs cannot circulate without bound in large meshes.

### TQ Propagation

TQ degrades multiplicatively as an OGM hops through the network. An originator X sends `tq=1000`. Each relay node B applies `tq_product(link_state_tq_to_sender, received_tq)` before re-broadcasting. When node A receives X's OGM via relay B, it reads B's reported path quality directly from the received TQ field:

```text
received_tq_via_B = link_B_to_prev × ... × link_Y_to_X × 1000 / 1000^n
```

A stores `received_ogm_info[X][B]` with the received TQ and a hop count derived from `DEFAULT_OGM_TTL - received_ttl + 1`. This data drives A's local routing decision for X without any local path computation.

This is the classic distributed implicit computation. Each node contributes its local link observation. The flood assembles an end-to-end quality estimate without any node building a full topology graph.

### Receive Window and Quality Scoring

A separate receive window is maintained per `(originator, via_neighbor)` pair. It counts unique sequence numbers received within the staleness window. The window occupancy permille is computed as:

```text
occupancy_permille = received_count / window_span × 1000
```

This receive quality is applied as a third factor in the local routing decision alongside `local_link_tq_to_B` and `received_tq_from_B`, combined via two nested `tq_product` calls. The receive quality is not encoded in the re-broadcast TQ. Downstream nodes see only the link-state-based path quality in re-broadcast advertisements.

### Echo-Only Bidirectionality

A neighbor B is confirmed bidirectional only when a local OGM has been received back via B. `bidirectional_neighbor_valid` checks the `bidirectional_receive_windows` table and returns `false` if no echo has been seen. There is no topology fallback. A neighbor for which no echo has been received does not contribute routing observations regardless of what the shared world model reports about the reverse link.

### No Bootstrap

If no receive-window data has accumulated for a `(originator, via_neighbor)` pair, `observation_tq` is zero and no routing observation is produced for that path. The engine produces no `RouteCandidate` on tick 1 for any multi-hop destination. Routes emerge as sequence windows fill. This matches the spec's behavior.

---

## Enhanced BATMAN (`jacquard.batmanb`)

### OGM Structure

The enhanced engine's originator advertisement carries full link state rather than a TQ scalar:

```text
OriginatorAdvertisement {
    originator: NodeId,
    sequence: u64,
    links: Vec<AdvertisedLink>,   // runtime_state, transport_kind, delivery_confidence
    // no tq field, no ttl field
}
```

This advertisement is sufficient to reconstruct a topology graph. It does not encode a pre-computed path quality. Advertisements are framed with magic `JQBATMAN`. The absence of TTL means OGMs are flooded verbatim to all neighbors every tick without hop-count bounds.

### Gossip Merging and Bellman-Ford

`merge_advertisements` folds learned advertisements into a copy of the current topology observation. It inserts synthesized `Link` entries for gossip-discovered edges not already present in the direct view. This produces a merged topology that may include nodes and links beyond the local one-hop view.

`refresh_private_state` then runs Bellman-Ford on this merged topology to compute `(path_tq, hop_count)` from each direct neighbor to every reachable originator. When a receive window exists for the `(originator, neighbor)` pair, the local routing decision uses three factors:

```text
steady_state_tq = tq_product(tq_product(local_link_tq, bellman_ford_path_tq), receive_quality)
```

When no receive window exists (bootstrap), the decision uses two factors:

```text
bootstrap_tq = tq_product(local_link_tq, bellman_ford_path_tq)
```

This substitutes a deterministic local computation for the spec's distributed OGM-propagated TQ. The computation is reproducible from the topology snapshot. The spec's TQ reflects whatever the neighborhood has recently observed.

### TQ Enrichment

`derive_tq` starts from the same `ogm_equivalent_tq(LinkRuntimeState)` baseline as the classic engine. When richer Jacquard link beliefs are present, it incorporates up to four additional terms in a running average.

| Enrichment | Normalization |
|---|---|
| `delivery_confidence_permille` | Direct permille value |
| `symmetry_permille` | Direct permille value |
| `transfer_rate_bytes_per_sec` | Normalized against 128 kbps saturation ceiling |
| `stability_horizon_ms` | Normalized against 4000 ms saturation ceiling |

The final TQ is the integer average over all contributing terms. With no beliefs present the result is identical to the classic engine's baseline. This enrichment has no equivalent in the BATMAN protocol.

### Topology Fallback for Bidirectionality

`bidirectional_neighbor_valid` first checks `bidirectional_receive_windows` as in the classic engine. If no echo window exists, it falls back to checking whether the shared topology contains a reverse link with usable state. This accelerates route availability on tick 1 before any echoes have been received. It admits routes the spec would withhold until echo confirmation.

### Bootstrap Shortcut

In `derive_originator_observations`, if no receive-window data exists for a specific `(originator, via_neighbor)` pair, the engine uses the Bellman-Ford path TQ directly as the combined TQ: `bootstrap_tq = tq_product(local_link_tq, path_tq)`. This is a two-factor formula. Once a receive window exists for that pair, the engine switches to the standard three-factor formula: `tq = tq_product(tq_product(local_link_tq, path_tq), receive_quality)`.

The bootstrap check is per-originator-per-neighbor. Receiving an OGM for one originator does not disable bootstrap for other originators that have not yet accumulated window data. On tick 1, before any OGMs have been received, the engine produces routing candidates from topology-derived path quality for all reachable destinations. The spec produces no candidates until receive-window data has accumulated.

---

## Shared Mechanisms

The following mechanisms are identical in both engines.

### OGM Receive Window

`OgmReceiveWindow` tracks received sequence numbers per `(originator, via_neighbor)` pair using a sliding window of size `stale_after_ticks`. Occupancy is computed as `received_count / window_span × 1000`. Sequences outside the staleness window are pruned. The window becomes empty once the last observed sequence ages out.

Sequence numbers are accepted strictly monotonically. Earlier or equal sequences from the same originator via the same neighbor are discarded.

### TQ Product

Both engines use the same compound quality formula:

```text
tq_product(left, right) = (left × right) / 1000
```

The result is on the same 0–1000 permille scale as the inputs. A path through two links each at 900 TQ yields 810. Multi-hop paths accumulate `tq_product` in sequence, producing monotonically decreasing quality with hop count. Links with a derived TQ below 700 are classified `RouteDegradation::Degraded`.

### Decay Window

`DecayWindow` governs observation freshness and refresh cadence. The default marks observations stale after 8 ticks and triggers a refresh within 4 ticks. Both engines accept `with_decay_window` at construction for tuning.

### Neighbor Ranking and BestNextHop

Candidates for each originator are ranked in this order:

1. `receive_quality` descending
2. `tq` descending
3. `is_bidirectional` descending
4. `observed_at_tick` descending
5. `hop_count` ascending
6. `via_neighbor` ascending (deterministic tie-break)

The top-ranked entry becomes `BestNextHop`. It carries the next-hop `NodeId`, TQ, receive quality, hop count, observation tick, topology epoch, transport kind, degradation status, bidirectionality flag, and a derived `BackendRouteId`.

---

## Planning, Admission, and Lifecycle

Planning, admission, and route lifecycle use identical logic in both engines. The planner checks the destination's `ServiceDescriptor` for the engine-specific ID before emitting any candidate.

`candidate_routes` emits at most one `RouteCandidate` per reachable destination. `admit_route` validates the candidate's `BackendRouteId` against the current `BestNextHop` table entry. A stale or superseded reference is inadmissible. `materialize_route` records an active route and derives health from TQ: `HealthScore = tq`, `PenaltyPoints = 1000 - tq`.

`maintain_route` returns `ReplacementRequired` when the best next-hop has changed. It returns `Failed(LostReachability)` when the destination has no table entry or when `is_bidirectional` is false. Route replacement is the only reconfiguration path. Neither engine implements suffix repair or hold.

---

## Capabilities

Both engines declare the same capability envelope:

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

---

## Spec Compliance

### Faithful Mechanisms (Classic)

| Mechanism | Implementation |
|---|---|
| OGM sequence-number freshness gating | Strictly monotonic: OGMs with equal or older sequence are discarded |
| Receive-window occupancy as route quality | `occupancy_permille = received / window_span` |
| TQ product formula | `(left × right) / 1000` |
| TQ propagated via OGM | `tq` field updated at each relay hop |
| TTL-bounded propagation | `DEFAULT_OGM_TTL=50`, decremented at each hop |
| Bidirectionality via echo | Echo window required. No topology fallback. |
| Proactive flood | OGMs sent to all direct neighbors each tick |
| Staleness window pruning | Sequences outside window are dropped |
| Next-hop-only route shape | `RouteShapeVisibility::NextHopOnly` |
| Single best next-hop per destination | Top-ranked entry only |

One minor deviation exists. The re-broadcast TQ uses `ogm_equivalent_tq(LinkRuntimeState)` as the local quality factor. The strict BATMAN IV spec uses receive-window occupancy as this factor. The local routing decision correctly applies receive-window quality as a third factor. The deviation therefore affects downstream quality estimates in OGMs rather than local route selection.

### Enhanced Engine Departures

| Mechanism | Change | Implication |
|---|---|---|
| TQ computation | Local Bellman-Ford on merged topology. No TQ field in OGM. | Path quality is deterministic and reproducible from the topology snapshot rather than reflecting recent neighborhood observation. The computation is closer to OLSR-style local SPF than DV gossip. |
| Link state in OGM | Full `AdvertisedLink` state per neighbor. No TQ scalar or TTL. | This is equivalent to distributing a topology database via gossip. It enables local path computation with no BATMAN protocol equivalent. |
| TTL | Absent. OGMs propagate without hop-count bounds. | OGMs circulate for as long as advertisements remain within the staleness window. The spec's propagation depth control is lost. |
| TQ enrichment | Delivery confidence, symmetry, transfer rate, stability averaged into TQ. | TQ reflects richer signal quality than packet counts alone. No BATMAN protocol equivalent. |
| Bidirectionality | Echo window with topology fallback. | Routes are available on tick 1. The engine admits paths the spec would withhold until echo confirmation. |
| Bootstrap | Per-originator-per-neighbor: path TQ used as combined TQ (two factors) when no window exists for that pair. | Routing candidates are produced on tick 1. Receiving OGMs for one destination does not disable bootstrap for others. The spec produces no candidates until receive-window data has accumulated. |
| Full topology reconstruction | `merge_advertisements` builds a complete adjacency graph. | The implementation behaves closer to a link-state protocol than a pure DV-gossip protocol. Path computation is centralized and explicit rather than implicit in the OGM flood. |

### Classic as Babel Comparison

The classic engine is the correct baseline when comparing against Babel (RFC 8966). Babel was designed to address specific weaknesses of classic DV-gossip protocols. These weaknesses are present in the spec-faithful implementation.

Babel addresses three gaps relative to classic BATMAN:

- **Asymmetric-link handling**: classic batman's bidirectionality gate excludes paths with poor incoming links entirely. Babel's feasibility condition handles asymmetric metrics without excluding those paths.
- **Loop freedom**: classic batman relies on sequence-number freshness for loop prevention. Babel's feasibility condition provides a provable loop-freedom guarantee during transient topology changes.
- **Triggered updates**: classic batman floods on a fixed tick schedule. Babel sends triggered updates immediately when a metric changes, reducing recovery latency.

Comparing classic batman against Babel measures what each mechanism contributes independently. Comparing the enhanced engine against Babel conflates the Bellman-Ford and topology-enrichment changes with the DV-gossip differences, making performance attribution unreliable.

### Enhanced as OLSRv2 Comparison

The enhanced engine is the correct baseline when comparing against OLSRv2 (`jacquard-olsrv2`). Both use local shortest-path computation over a topology database distributed by gossip. The primary structural difference is TC messages with MPR election versus OGM flooding.

The enhanced engine also exhibits the partition-recovery weakness that OLSR directly addresses. The receive window used as `receive_quality` is the same window used to gate bidirectionality. Both indicators require the full window span to recover when a partition clears, which delays route restoration compared to OLSR's explicit TC-flood response to topology changes.

That comparison therefore measures two distinct questions: the cost of MPR-suppressed flooding overhead versus the enhanced batman's simpler model, and whether OLSR's immediate topology-change response produces better recovery behavior under adverse conditions.
