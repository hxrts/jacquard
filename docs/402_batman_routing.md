# BATMAN Routing

This page documents the in-tree `jacquard-batman` engine.

## Model

The [BATMAN](https://en.wikipedia.org/wiki/B.A.T.M.A.N.) engine is a proactive next-hop engine. It maintains originator
observations privately, ranks neighbors per originator using a BATMAN-private TQ-like score, and exposes only the best next hop toward each destination. It reports `RouteShapeVisibility::NextHopOnly`.

The router still owns canonical publication, leases, handles, and
materialized-route identity. BATMAN owns only proactive ranking state and the engine-private forwarding record bound during materialization.

## Observation Model

The engine requires only a BATMAN OGM-equivalent observation baseline via
`LinkRuntimeState`. That baseline maps to a coarse TQ-like reachability score. Richer Jacquard link observations are optional refinements:
`delivery_confidence_permille`, `symmetry_permille`,
`transfer_rate_bytes_per_sec`, and `stability_horizon_ms`. The engine does not require those richer signals to function.

## Candidate and Admission Rules

BATMAN emits advisory candidates from its best-next-hop table only for
destinations that advertise BATMAN support for the requested service kind. A destination that advertises only mesh support receives no BATMAN candidates. A destination that advertises both mesh and BATMAN support may participate in
either layer.

Admission and materialization remain thin. The router owns canonical route
identity. BATMAN validates the chosen next hop against current private state and binds the canonical handle to one engine-private forwarding record during materialization.

## Tick and Maintenance

`engine_tick` is the proactive maintenance hook. BATMAN uses it to refresh
originator observations, decay stale observations, recompute neighbor ranking, and refresh the best-next-hop table. The returned `RoutingTickHint` is advisory scheduling pressure only. The host and router own final cadence.

Route maintenance watches for next-hop degradation, better next-hop replacement opportunities, and loss of reachability for the destination.

## Mixed-Engine Composition

The reference client demonstrates registering BATMAN and mesh together in one router. The mixed-engine path uses a BATMAN-backed first hop where the
destination advertises BATMAN support, and a mesh-backed second hop where the next destination advertises only mesh support.

Each engine remains private and honest about its visibility model. The router owns multi-engine orchestration. Cross-layer message movement happens by composing separate canonical routes, not by having one engine claim ownership of both layers.

## Current Limits

This first in-tree BATMAN engine does not provide full explicit path visibility, aggregate end-to-end path disclosure, or raw BATMAN packet modeling in shared core. It also does not implement `batman-adv` layer-2 tunneling semantics, gateway support, bonding, or fragmentation.

## Follow-On Evaluation

BATMAN V throughput-style scoring is not justified yet. The existing
OGM-equivalent baseline plus optional richer observations is sufficient for now. Gateway, bonding, and fragmentation remain out of scope.

No BATMAN-local shared inspection contract is justified yet. Ranking-table views can stay crate-private until a second real consumer appears. No new shared router/engine contract is justified beyond the richer route visibility model and tick scheduling hints already landed.
