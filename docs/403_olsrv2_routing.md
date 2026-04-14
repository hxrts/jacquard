# OLSRv2 Routing

`jacquard-olsrv2` (engine ID `jacquard.olsrv2.`) implements a deterministic OLSRv2-class proactive link-state engine. It preserves the core OLSRv2 shape: HELLO-driven symmetric-neighbor learning, deterministic MPR election, TC-style topology flooding, and shortest-path next-hop derivation over the learned topology database.

The crate is not a wire-compatible RFC 7181 daemon clone. It is a Jacquard engine that consumes `Observation<Configuration>`, advances only during router-owned synchronous rounds, and publishes only next-hop route candidates through the shared engine traits.

## Engine Overview

The engine owns five pieces of runtime state:

1. one-hop neighbor state learned from HELLO exchange
2. two-hop reachability learned from symmetric neighbors
3. local MPR and MPR-selector sets
4. topology tuples learned from TC advertisements
5. a derived shortest-path tree and best-next-hop table

The router and host own ingress draining, tick cadence, and time attachment. `jacquard-olsrv2` consumes explicit ingress through the shared runtime hook and returns router-visible `NextHopOnly` candidates.

## Jacquard-Specific Simplifications

Jacquard keeps the OLSRv2 surface deterministic and auditable:

- one deterministic decay window controls HELLO and TC freshness
- one deterministic MPR election policy is used for all nodes
- link cost is integer-only and derived from shared link observations
- all sets and maps use canonical ordering with no ambient randomness
- no async protocol loop, no host-driver ownership, and no external RFC interoperability layer
- route publication remains router-owned

The result is an OLSRv2-class baseline for comparative routing work rather than a feature-complete NHDP implementation.

## HELLO Semantics

Each round, the engine may originate one HELLO message carrying the local originator ID, a monotonically increasing local HELLO sequence number, the current symmetric-neighbor set, and the current local MPR set.

Inbound HELLO processing updates the one-hop neighbor table and the derived two-hop reachability map. A link is treated as symmetric only when the inbound HELLO confirms the local node inside the neighbor's symmetric-neighbor set. The shared topology observation constrains whether the underlying link is usable. HELLO state alone does not override a failed transport observation.

HELLO state expires when the engine-local hold window passes. Expiry uses `Tick`, not wall-clock time.

## MPR Election

MPR election is deterministic and local. The input surface is the currently symmetric one-hop neighbors, two-hop neighbors reachable through those one-hop neighbors, and the integer link metric derived from the shared observation model.

The algorithm chooses a minimal covering relay set for the known two-hop neighbors. Ties break first on lower metric cost, then on canonical node order. The elected set is exported only as engine-local control state plus the local HELLO advertisement. It is not promoted into shared `core` vocabulary.

## TC Flooding

TC advertisements carry the originator ID, a monotonically increasing local TC sequence number, and the advertised-neighbor set selected for flooding.

The engine originates a fresh TC when the advertised-neighbor surface changes or when the local topology state needs refresh. Inbound TC processing accepts only strictly fresher sequence numbers per originator, replaces older topology tuples for that originator, and expires stale tuples by the same tick-based hold window.

Forwarding is constrained by MPR-selector state. A node forwards only when the sender has selected it as an MPR and the TC sequence has not already been forwarded for that originator.

## Shortest-Path Computation

The topology database is a deterministic set of directed topology tuples. Shortest-path derivation runs over local symmetric edges, accepted TC tuples, and integer link cost derived from the shared link observation.

The shortest-path tree is recomputed when HELLO or TC ingestion changes the topology database. Best-next-hop derivation collapses the tree into one `NodeId` next hop per reachable destination. Only destinations that advertise support for `jacquard.olsrv2.` in the shared service surface are eligible for route candidates.

## Capability Envelope

The OLSRv2 engine declares the same conservative next-hop envelope used by the other proactive engines:

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

The engine keeps a full topology graph privately but does not claim explicit-path visibility at the shared contract boundary.

## Route Lifecycle And Maintenance

Planning and admission follow the standard Jacquard route lifecycle. `candidate_routes` emits next-hop candidates from the derived best-next-hop table. `check_candidate` validates the candidate against current engine-private topology state. `admit_route` binds the candidate to router-owned identity. `materialize_route` installs the active next-hop record.

Maintenance returns `Continued` while the selected next hop remains valid. It returns `ReplacementRequired` when the shortest-path table selects a new next hop. It returns `Failed(LostReachability)` when no route remains. There is no suffix repair or engine-owned hold mode. Route replacement is the only reconfiguration path.

## Comparison Role

`jacquard-olsrv2` is the in-tree full-topology proactive baseline. It answers a different question from the batman and Babel engines. `batman-classic` and `babel` are distance-vector gossip baselines. `batman-bellman` is a topology-enriched BATMAN variant. OLSRv2 is the proactive link-state baseline with explicit topology flooding.

OLSRv2 is the primary comparison point for measuring how much full topology knowledge buys over gossip-style next-hop routing.

## Related Pages

- [Routing Engines](303_routing_engines.md)
- [Batman Routing](401_batman_routing.md)
- [Babel Routing](402_babel_routing.md)
- [Crate Architecture](999_crate_architecture.md)
