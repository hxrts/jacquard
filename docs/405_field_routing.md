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

The current Rust implementation now makes the operational layer more explicit:

- `policy.rs` centralizes calibrated regime, posture, continuity, promotion,
  and evidence thresholds as one deterministic `FieldPolicy` surface
- `operational.rs` derives a reduced `FieldOperationalView` with support,
  retention, entropy, and freshness bands for decision code
- those operational surfaces remain runtime-private and do not become
  posterior truth or canonical route truth

## Continuously Updated Field Model

Field updates one destination-local model from three evidence classes:

- direct topology observations
- forwarded protocol summaries from neighbors
- reverse delivery feedback

The runtime ingests forwarded summaries and feedback explicitly on the engine
surface through `ingest_forward_summary`, `record_forward_summary`, and
`record_reverse_feedback`, stores them as pending evidence, and feeds them into
`refresh_destination_observers` on the next tick. Observer refresh is
fail-closed and explicit: protocol evidence enters the observer path only
through those engine-owned evidence buffers.

That refresh updates:

- posterior belief
- progress belief
- corridor belief
- continuation frontier

The resulting frontier is the local admissible continuation surface that the
planner and runtime consume.

## Regime Detection

Field runs a local control-plane pass on each engine tick before planning. That
pass compresses destination-local state plus topology observations into one
bounded mean-field summary and one bounded price vector.

The regime detector scores five operating regimes:

- `Sparse`
- `Congested`
- `RetentionFavorable`
- `Unstable`
- `Adversarial`

Those scores are derived from the current combination of:

- congestion pressure
- relay pressure
- retention pressure
- churn pressure
- risk pressure
- mean-field alignment and field-strength signals
- control prices accumulated by the bounded PI loop

The active regime is not replaced immediately on every score change. Field uses
residual accumulation, a change threshold, a hysteresis threshold, and a
post-transition dwell window to prevent one-tick oscillation. A regime change
happens only when a different regime stays strong enough for long enough to
clear that bounded switching logic.

## Telltale Search

Field planning is search-backed.

For each routing objective, the planner:

1. resolves the objective into a native Telltale `SearchQuery`
2. freezes the current field model into one deterministic snapshot
3. runs exact Telltale search over that snapshot
4. derives one selected private continuation from the selected-result witness
5. emits a shared `RouteCandidate` with `CorridorEnvelope` visibility

The public result shape stays corridor-only even when the private selected
result witness is a concrete node path. That split is deliberate: search is an
internal implementation substrate, not a new source of canonical route truth.
Field may consider multiple admissible continuations internally, but that
plurality stays private. One routing objective still yields one field-selected
private result and one planner-visible corridor claim.

The query split is:

- exact node objectives resolve to `SearchQuery::single_goal`
- gateway and service objectives resolve to selected-result
  `SearchQuery::try_candidate_set` queries over frontier neighbors
- candidate-set queries are truncated by the field per-objective search budget
  before execution

The search record retained by the engine also captures snapshot transitions and
explicit reseeding decisions, so evidence changes within one shared route epoch
still show up as field-owned search reconfiguration rather than being silently
treated as the same run.

The search/publication boundary is explicit:

- the selected private result stays inside the search record
- continuation choice is reduced to one selected runtime realization
- the published route summary remains one corridor-envelope claim
- backend token and active-route state keep the richer private realization
  detail needed for runtime maintenance and forwarding

## Experimental Surface

Field now separates two different tuning surfaces:

- `FieldSearchConfig` remains the search-substrate surface:
  - scheduler profile
  - batch-width / effort invariants
  - heuristic mode
  - query budget
  - reseeding policy
- `FieldPolicy` is the operational surface:
  - regime detection and dwell
  - posture switching
  - continuity and bootstrap floors
  - promotion / hold / narrow / withdraw gates
  - evidence aging, carry-forward, publication, and replay thresholds

The intended maintained experiment knobs are profile-level and few:

- regime sensitivity
- posture conservatism
- continuity softness
- promotion strictness
- evidence freshness / corroboration weight

Those profile-level variables expand into the lower-level policy families
internally. The point of the split is to keep the experiment surface legible
without turning the runtime into an unbounded configuration matrix.

## Execution Policy

Field keeps truth semantics and execution policy separate.

- destination eligibility and selected-result meaning do not change with local
  posture or regime
- local posture and regime may change only the search execution profile

## Posture Control

Posture is the field engine's local execution stance. It determines how the
engine reacts to the currently detected regime when it ranks continuations,
publishes corridor claims, and chooses a search execution profile.

Field chooses among four postures:

- `Opportunistic`
- `Structured`
- `RetentionBiased`
- `RiskSuppressed`

The posture controller scores all four against the current regime, mean-field
state, and control prices, then selects the highest-scoring posture subject to
its own hysteresis. The primary posture mapping is:

- sparse regime -> `Opportunistic`
- congested regime -> `Structured`
- retention-favorable regime -> `RetentionBiased`
- unstable or adversarial regime -> `RiskSuppressed`

As with regimes, posture changes are damped. Field keeps a posture switch
threshold and a short dwell window after each transition. That prevents one
tick of changed evidence from causing immediate flapping. When the regime is
very strong, the controller can move more quickly back to that regime's primary
posture, but posture still remains an execution choice rather than a truth
owner.

Field defaults to canonical serial exact search and may
promote to threaded exact single-lane search on native targets when the engine
enters a congested regime or a risk-suppressed posture. Query meaning,
admissible destinations, and corridor-envelope publication stay unchanged.

## Corridor Realization

The public field route is a corridor claim, not a single next-hop commitment.

Field therefore keeps two private runtime notions separate:

- one selected runtime realization inside the corridor
- one bounded continuation envelope of admissible neighbor realizations

That means runtime can change its concrete send target inside the installed
corridor envelope without forcing immediate route replacement. Replacement is
required only when the best available continuation leaves the installed
continuation envelope, the corridor support is withdrawn, or policy state makes
the installed route inadmissible.

## Route Lineage

The field route lineage is:

1. local field evidence updates observer state and the continuation frontier
2. field search selects one private result inside the frozen snapshot
3. the selected private result yields one selected runtime realization
4. planner publication emits one corridor-envelope candidate
5. router admission/materialization turns that candidate into an installed route
6. runtime forwarding and maintenance continue to operate inside the installed
   continuation envelope

That lineage is intentionally asymmetric:

- field owns the private evidence, search, and runtime-realization layers
- router owns candidate comparison, canonical publication, and installed-route
  truth
- field quality/comparison objects remain reference-only unless one theorem or
  router rule explicitly promotes them into router-owned truth

## Runtime Surfaces

Field exposes bounded private diagnostics for inspection and replay-oriented
tooling:

- the last planner search record, including query, effective search config,
  execution report, replay artifact, and snapshot reconfiguration data
- a versioned `FieldReplaySnapshot` surface that packages search, protocol,
  runtime, and commitment views without requiring access to hidden engine
  internals
- a reduced `reduced_runtime_search_replay()` extraction from
  `FieldReplaySnapshot` that exposes the proof-facing search/runtime bundle
  without re-reading private engine state
- a reduced `reduced_protocol_replay()` extraction from `FieldReplaySnapshot`
  that exposes the proof-facing protocol artifact and protocol-reconfiguration
  bundle without re-reading private engine state
- a versioned `FieldExportedReplayBundle` surface derived from the reduced
  replay helpers, with stable JSON packaging for debugging and regression
  fixtures
- a reduced `FieldLeanReplayFixture` derived from that exported replay bundle
  so proof-facing fixture vocabulary tracks Rust replay structure directly
- bounded protocol artifacts from the private choreography runtime
- bounded runtime round artifacts carrying blocked-receive state, host
  disposition, emitted-summary count, remaining step budget, execution-policy
  class, destination class, search-snapshot linkage metadata, bootstrap class,
  and one reduced observational route projection
- one route-commitment view per materialized route, with pending, lease-expiry,
  topology-supersession, evidence-withdrawal, and backend-unavailable outcomes
- route-scoped recovery state carrying checkpoint, continuation-shift, and
  bootstrap activation/upgrade/withdrawal counters

Private protocol flows such as summary dissemination, anti-entropy, retention
replay, and explicit coordination remain bounded operational surfaces. They
affect field semantics only when they yield engine-owned evidence that is later
ingested through the forward-summary or reverse-feedback paths. Otherwise they
remain observational runtime behavior rather than semantic route truth.

Route-scoped explicit-coordination sessions are also the current field
reconfiguration surface. When a live route shifts its concrete realization
inside the already-admitted continuation envelope, field reconfigures the
route-scoped protocol session instead of forcing full route replacement.
Owner-transfer, checkpoint/restore, and continuation-shift steps are retained
as replay-visible protocol reconfiguration markers.

Field route publication also has an explicit bootstrap phase. A bootstrap route
is a weaker corridor claim that is allowed when the evidence is coherent but
not yet strong enough for steady admission. Promotion out of bootstrap is not
just a second support threshold. The runtime evaluates five observable gates:

- support growth relative to the installed bootstrap corridor
- uncertainty reduction
- anti-entropy confirmation from recent coherent summary publication
- continuation coherence inside the installed corridor envelope
- freshness of the leading continuation

Between `Steady` and `Bootstrap`, runtime now also keeps one explicit
degraded-steady continuity band. A degraded-steady route is still a conservative
steady corridor claim at the publication boundary, but the runtime has started
preserving narrowed corridor structure, asymmetric continuation shifts, and
anti-entropy carry-forward more aggressively because the corridor is no longer
comfortably steady.

Runtime and replay surfaces then distinguish five bootstrap transitions:

- activation
- hold
- narrowing when the corridor is still conservative but must contract before it
  can strengthen
- upgrade to steady state
- withdrawal when the corridor collapses

Replay also distinguishes continuity-band movement itself:

- entering degraded-steady before bootstrap collapse
- recovering from degraded-steady back to steady
- downgrading from degraded-steady into bootstrap when continuity can no longer
  be preserved

When promotion does not occur, replay also records the dominant blocker:

- weak support trend
- unresolved uncertainty

Service destinations also use a bounded service-retention carry-forward path.
When a coherent service corridor has just been published, the observer/runtime
path can synthesize a short-lived forwarded-evidence reinforcement window so
service fanout families do not lose continuity after one missing forwarded
round. That carry-forward is bounded and replay-visible; it preserves coherent
service summaries, but it does not invent a route when no corridor evidence
remains.
- missing anti-entropy confirmation
- broken continuation coherence
- stale leading evidence

The participant-set boundary is explicit:

- owner and generation movement are supported
- route-scoped checkpoint/restore is supported
- continuation-shift reconfiguration inside one admitted corridor is supported
- participant-set change is not supported

Those runtime round artifacts are intentionally observational. They expose only
reduced route shape, reduced search linkage, and support hints. They do not
expose the selected witness, the full continuation envelope, or hidden protocol
session state. They do not promote the field runtime into a second canonical
route owner.

The replay surfaces also carry an explicit surface-class split:

- search replay is observational
- protocol replay packaging is observational, while
  `reduced_protocol_replay()` is the maintained proof-facing protocol replay
  reduction
- runtime replay is reduced
- commitment replay is observational
- exported replay is reduced, versioned, and tooling-oriented rather than
  authoritative

## Proof Boundary

The field proof stack is intentionally narrower than the richer Rust runtime,
and that reduction is deliberate.

Lean covers:

- the reduced local observer-controller model
- the reduced private protocol boundary, including fixed-participant closure,
  fragment-trace alignment, receive-refinement witnesses, and explicit
  observational-only reconfiguration semantics
- the reduced field search boundary, including query-family mapping, snapshot
  identity, selected-result shape, execution-policy vocabulary, and
  reconfiguration metadata
- the reduced runtime and runtime-search adequacy boundary, including
  trace/evidence extraction, runtime-state refinement, runtime-artifact search
  linkage, search projection, reduced protocol replay projection, and reduced
  canonical-route refinement
- replay-derived fixture vocabulary mirrored in
  `verification/Field/Adequacy/ReplayFixtures.lean`

Lean does not own router truth, private choreography internals, or full replay
packaging semantics. Those richer Rust surfaces remain observational or
out-of-scope unless an explicit reduction theorem promotes part of them.

The most important assurance is ownership discipline:

- the deterministic local controller owns field semantics
- private protocol exports are observational-only
- runtime artifact reduction is observational-only
- canonical route truth remains router-owned

Router-owned truth can still be richer than support-only ranking. The current
verification tree also carries a stronger support-then-hop-then-stable router
selector and the matching system-level selector lift. Field does not publish
extra planner-visible candidates to satisfy that richer objective. It still
publishes one corridor candidate per objective and leaves richer canonical
choice to the router/system layer.

The current broader resilience story is likewise router/system-owned rather
than field-private-search-owned. The maintained proof stack includes bounded
dropout and bounded non-participation stability packs under the reduced
reliable-immediate regime. Those results say how router-owned canonical support
stabilizes once the selected winner survives the stated fault budget; they do
not turn field-private replay or protocol reconfiguration into new owners of
canonical route truth.

See:

- [Routing Engines](303_routing_engines.md)
- [Crate Architecture](999_crate_architecture.md)
- `verification/Field/Docs/Model.md`
- `verification/Field/Docs/Protocol.md`
- `verification/Field/Docs/Adequacy.md`
