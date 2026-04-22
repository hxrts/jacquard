# Mercator Routing Engine

`jacquard-mercator` is Jacquard's corridor routing engine for disrupted mesh networks, where connectivity is real but intermittent. It keeps a bounded view of recent network evidence, searches for a route with backup options, repairs around stale paths when it can, and falls back to limited carry-and-forward behavior when no connected route is safe.

Mercator is for regimes where a connected route may be valid for only part of a run. It does not replace router-owned publication. It produces one router-facing route candidate when connected support is good enough, and one bounded custody posture when connected support is not.

See [Routing Engines](303_routing_engines.md) for the shared trait contract and [Router Control Plane](304_router_control_plane.md) for canonical route ownership.

## Router Boundary

Mercator implements the same `RoutingEngine` and `RouterManagedEngine` surfaces as the other in-tree engines. Candidate production reads explicit topology observations plus Mercator's private evidence graph. Materialization happens only after router admission and router identity allocation.

The engine does not own transport streams, drain ingress, or assign `Tick`. Host bridges attach time and advance router rounds. Mercator stays below that boundary: it maintains evidence, searches for a corridor, and exposes bounded diagnostics and router analysis snapshots.

Its published route shape is `CorridorEnvelope`. Internally, a corridor contains one primary realization plus bounded alternates. Externally, the router sees one candidate for one destination, service, or gateway objective.

## Evidence Model

Mercator's core state is a bounded evidence graph. It records link support, reverse-link support, route support, broker pressure, service support, custody opportunities, objective accounting, and disruption markers. Each record type carries a distinct role:

- link and reverse-link support describe whether a path is usable in both directions where that matters
- route support records which candidate routes still have evidence behind them
- broker pressure tracks whether a bridge-like node is becoming too central
- service support maps service objectives to providers
- custody opportunities describe possible store-carry-forward handoffs

Every record uses Jacquard's typed time and ordering: `Tick`, `DurationMs`, `OrderStamp`, and `RouteEpoch`. Scores are integer-ranked. Pruning is deterministic and uses score first, then canonical identity.

The graph is bounded by configuration. Mercator caps neighbors, brokers, service evidence, alternates, and custody opportunities. Evidence expires by policy and can be withdrawn immediately when a disruption epoch invalidates it.

## Candidate Publication

Mercator searches for a corridor instead of a single brittle path. The planner expands from local evidence, uses maintained topology evidence when available, and reserves search effort for underserved objectives so one high-demand objective does not monopolize planning.

A selected corridor yields one router-facing `RouteCandidate`. Alternates remain private. This lets Mercator repair small topology changes without publishing multiple canonical routes for the same objective.

Admission stays conservative. A candidate must satisfy the objective, pass freshness checks, clear reachability confidence thresholds, and respect broker-pressure limits.

## Stale Safety

Mercator treats staleness as a normal operating condition, not an exceptional failure. Route support moves through private states: `Fresh`, `Suspect`, `Repairing`, `Withdrawn`, and `CustodyOnly`.

A disruption epoch invalidates dependent support immediately. Repair can reuse only corridor alternates whose support survives the current epoch. If no support survives, Mercator withdraws connected route support instead of continuing to publish an obsolete route as usable.

The stale metrics count active but unusable route support after disruption. Pre-disruption losses are not charged as post-disruption stale persistence.

## Custody Fallback

When a connected route is not supportable, Mercator can enter bounded custody posture. Custody mode does not publish a connected route. It retains payload evidence only through the shared retention boundary and forwards only to carriers with strict deterministic improvement.

The custody policy uses copy budgets, protected bridge budget, same-cluster suppression, low-gain suppression, energy pressure, and leakage-risk checks. These controls keep disconnected delivery bounded by construction rather than relying on best-effort flooding.

This posture is closest to [Scatter Routing](405_scatter_routing.md), but Mercator uses it as a fallback beneath a route-visible corridor engine. Scatter remains the opaque deferred-delivery baseline.

## Diagnostics

Mercator reports selected-result rounds, no-candidate attempts, inadmissible attempts, support withdrawals, stale persistence, repair attempts, recovery rounds, objective service, broker concentration, broker switching, and custody pressure.

Custody diagnostics include retained records, reproduction count, copy-budget use, protected-bridge use, transmission count, storage bytes, energy units, leakage risk, and suppression counts.

The simulator consumes these diagnostics in route-visible summaries, routing-fitness families, diffusion families, report tables, and recommendation scoring. In those reports, route-visible rows measure connected-route publication, while diffusion rows measure the bounded custody behavior.

## Simulator Usage

The reference client exposes `ClientBuilder::mercator` for a single-engine client. `ClientBuilder::all_engines` also registers Mercator with the maintained mixed-engine set.

The simulator exposes a `mercator` engine lane and includes Mercator in local route-visible, head-to-head, routing-fitness, large-population, and diffusion suites. The `tuning_matrix` binary writes Mercator rows into aggregate, breakdown, comparison, routing-fitness, and diffusion artifacts where those families apply.

See [Simulator Architecture](306_simulator_architecture.md) for host bridge ownership during runs and [Experimental Methodology](307_experimental_methodology.md) for how maintained suites use fixed engine operating points.

## Comparisons

Mercator shares explicit search goals with [Pathway Routing](404_pathway_routing.md). It differs by retaining bounded alternates and treating stale repair and weakest-flow service as first-class diagnostics.

Mercator shares corridor publication shape with [Field Routing](406_field_routing.md). It differs by using a smaller bounded evidence graph rather than a continuously updated field model and Telltale-backed search substrate.

Mercator shares custody pressure concerns with [Scatter Routing](405_scatter_routing.md). It differs by remaining route-visible whenever connected corridor support exists.

See [Crate Architecture](999_crate_architecture.md) for the dependency and ownership rules that keep these engine-private mechanisms out of shared route truth.
