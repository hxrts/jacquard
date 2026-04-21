# Mercator Routing Engine

`jacquard-mercator` is Jacquard's hybrid corridor routing engine for disrupted mesh networks. It combines bounded evidence, corridor search, stale-safe repair, weakest-flow fairness, and bounded custody posture inside one deterministic engine.

Mercator targets regimes where a connected route may exist for only part of the run. It does not replace router-owned canonical publication. It supplies one router-facing candidate or one custody posture from engine-private evidence.

See [Routing Engines](303_routing_engines.md) for the shared trait contract. See [Router Control Plane](304_router_control_plane.md) for canonical route ownership.

## Router Boundary

Mercator implements the same `RoutingEngine` and `RouterManagedEngine` surfaces as the other in-tree engines. Candidate production reads an explicit topology observation and an engine-owned evidence graph. Materialization happens only after router admission and router identity allocation.

The engine does not own transport streams. It does not assign `Tick`. Host bridges still drain ingress, attach time, and call router rounds. Mercator keeps private state below the shared boundary and exposes only bounded diagnostics and router analysis snapshots.

Its published route shape is `CorridorEnvelope`. The internal corridor may contain one primary realization plus bounded alternates. The router sees a single route candidate for a single objective.

## State Model

The core state is a bounded evidence graph. It records link support, reverse-link support, route support, broker pressure, service support, custody opportunities, objective accounting, and disruption markers.

Every record uses typed time and ordering. The implementation uses `Tick`, `DurationMs`, `OrderStamp`, and `RouteEpoch`. Scores are integer ranked. Pruning is deterministic and uses score first, then canonical identity.

The engine caps neighbors, brokers, service evidence, alternates, and custody opportunities. Evidence expires by policy and can be withdrawn immediately when a disruption epoch invalidates it.

## Candidate Publication

Mercator searches for a corridor instead of one brittle path. The planner expands from local evidence, uses maintained topology evidence when available, and reserves search effort for underserved objectives.

A selected corridor yields one router-facing `RouteCandidate`. Alternates remain private. This lets the runtime repair small topology changes without publishing multiple canonical routes for the same objective.

Admission stays conservative. A candidate must satisfy the objective, survive freshness checks, clear reachability confidence thresholds, and respect broker pressure limits.

## Stale Safety

Mercator tracks route support through `Fresh`, `Suspect`, `Repairing`, `Withdrawn`, and `CustodyOnly` states. These states are engine-private, but their aggregate effects appear in diagnostics and simulator summaries.

A disruption epoch invalidates dependent support immediately. Repair can reuse only corridor alternates whose support survives the current epoch. If no support survives, Mercator withdraws connected route support instead of continuing to publish a stale route as usable.

The stale metrics count active but unusable route support after disruption. Pre-disruption losses are not charged as post-disruption stale persistence.

## Custody Posture

When a connected route is not supportable, Mercator can enter bounded custody posture. Custody mode does not publish a connected route. It retains payload evidence only through the shared retention boundary and forwards only to carriers with strict deterministic improvement.

The custody policy uses copy budgets, protected bridge budget, same-cluster suppression, low-gain suppression, energy pressure, and leakage risk. These controls keep disconnected delivery bounded by construction.

This posture is closest to [Scatter Routing](405_scatter_routing.md), but Mercator uses it as a fallback beneath a route-visible corridor engine. Scatter remains the opaque deferred-delivery baseline.

## Diagnostics

Mercator reports selected-result rounds, no-candidate attempts, inadmissible attempts, support withdrawal, stale persistence, repair attempts, recovery rounds, objective service, broker concentration, broker switching, and custody pressure.

Custody diagnostics include retained records, reproduction count, copy budget use, protected bridge use, transmission count, storage bytes, energy units, leakage risk, and suppression counts.

The simulator consumes these diagnostics in route-visible summaries, routing-fitness families, diffusion families, report tables, and recommendation scoring.

## Simulator Usage

The reference client exposes `ClientBuilder::mercator` for a single-engine client. `ClientBuilder::all_engines` also registers Mercator with the maintained mixed-engine set.

The simulator exposes a `mercator` engine lane and includes Mercator in local route-visible, head-to-head, routing-fitness, large-population, and diffusion suites. The `tuning_matrix` binary writes Mercator rows into aggregate, breakdown, comparison, routing-fitness, and diffusion artifacts where those families apply.

See [Simulator Architecture](306_simulator_architecture.md) for host bridge ownership during runs. See [Experimental Methodology](307_experimental_methodology.md) for how maintained suites use fixed engine operating points.

## Comparisons

Mercator shares explicit search goals with [Pathway Routing](404_pathway_routing.md). It differs by retaining a bounded corridor and by treating stale repair and weakest-flow service as first-class diagnostics.

Mercator shares corridor publication shape with [Field Routing](406_field_routing.md). It differs by using a smaller bounded evidence graph rather than a continuously updated field model and Telltale-backed search substrate.

Mercator shares custody pressure concerns with [Scatter Routing](405_scatter_routing.md). It differs by remaining route-visible whenever connected corridor support exists.

See [Crate Architecture](999_crate_architecture.md) for the dependency and ownership rules that keep these engine-private mechanisms out of shared route truth.
