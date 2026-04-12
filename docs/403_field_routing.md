# Field Routing

`jacquard-field` is Jacquard's corridor-envelope routing engine. It does not
claim a full explicit path. Instead it maintains a continuously updated local
field model, freezes that model into deterministic search snapshots, runs
Telltale search privately, and publishes only conservative corridor-envelope
claims through the shared routing contract.

## Engine Shape

Field owns four private layers:

1. observer state
2. regime and posture control state
3. a bounded private summary-exchange choreography runtime
4. a Telltale-backed search substrate over frozen field snapshots

Those layers stay engine-private. The router still owns canonical route
identity, publication, and cross-engine selection.

## Continuously Updated Field Model

Field updates one destination-local model from three evidence classes:

- direct topology observations
- forwarded protocol summaries from neighbors
- reverse delivery feedback

The runtime ingests forwarded summaries and feedback explicitly on the engine
surface through `ingest_forward_summary`, `record_forward_summary`, and
`record_reverse_feedback`, stores them as pending evidence, and feeds them into
`refresh_destination_observers` on the next tick. Observer refresh is
fail-closed and explicit: the engine no longer hides protocol evidence behind
placeholder empty vectors.

That refresh updates:

- posterior belief
- progress belief
- corridor belief
- continuation frontier

The resulting frontier is the local admissible continuation surface that the
planner and runtime consume.

## Telltale Search

Field planning is search-backed.

For each routing objective, the planner:

1. resolves the objective into a native Telltale `SearchQuery`
2. freezes the current field model into one deterministic snapshot
3. runs exact Telltale search over that snapshot
4. uses the selected private witness only to choose a continuation
5. emits a shared `RouteCandidate` with `CorridorEnvelope` visibility

The public result shape stays corridor-only even when the private selected
result witness is a concrete node path. That split is deliberate: search is an
internal implementation substrate, not a new source of canonical route truth.

The current query split is explicit:

- exact node objectives resolve to `SearchQuery::single_goal`
- gateway and service objectives resolve to selected-result
  `SearchQuery::try_candidate_set` queries over frontier neighbors
- candidate-set queries are truncated by the field per-objective search budget
  before execution

The search record retained by the engine also captures snapshot transitions and
explicit reseeding decisions, so evidence changes within one shared route epoch
still show up as field-owned search reconfiguration rather than being silently
treated as the same run.

## Execution Policy

Field keeps truth semantics and execution policy separate.

- destination eligibility and selected-result meaning do not change with local
  posture or regime
- local posture and regime may change only the search execution profile

The current implementation defaults to canonical serial exact search and may
promote to threaded exact single-lane search on native targets when the engine
enters a congested regime or a risk-suppressed posture. Query meaning,
admissible destinations, and corridor-envelope publication stay unchanged.

## Runtime Surfaces

Field now exposes bounded private diagnostics for inspection and replay-oriented
tooling:

- the last planner search record, including query, effective search config,
  execution report, replay artifact, and snapshot reconfiguration data
- bounded protocol artifacts from the private choreography runtime
- bounded runtime round artifacts carrying blocked-receive state, host
  disposition, emitted-summary count, remaining step budget, execution-policy
  class, and one reduced observational route projection
- one route-commitment view per materialized route, with pending, lease-expiry,
  topology-supersession, evidence-withdrawal, and backend-unavailable outcomes

Those runtime round artifacts are intentionally observational. They expose only
reduced route shape and support hints. They do not promote the field runtime
into a second canonical route owner.

## Proof Boundary

The field proof stack remains intentionally narrower than the full Rust runtime.

Lean currently covers:

- the reduced local observer-controller model
- the reduced private protocol surface
- the reduced runtime-artifact adequacy bridge

Lean does not yet model the Rust Telltale search substrate directly. In
particular, the current proof stack does not yet prove the frozen-snapshot
search machine, its replay artifact, or its snapshot reconfiguration and
reseeding behavior end to end.

The most important assurance is ownership discipline:

- the deterministic local controller owns field semantics
- private protocol exports are observational-only
- runtime artifact reduction is observational-only
- canonical route truth remains router-owned

See:

- [Routing Engines](303_routing_engines.md)
- [Crate Architecture](999_crate_architecture.md)
- `verification/Field/Docs/Model.md`
- `verification/Field/Docs/Protocol.md`
- `verification/Field/Docs/Adequacy.md`
