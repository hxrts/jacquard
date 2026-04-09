# Field Engine

Target design note for a field-routing architecture under Jacquard's
shared engine contract.

The engine should assume hidden information in the style of BATMAN-like routing while still exposing a stronger `CorridorEnvelope` route shape.

---

## Problem Setup

Jacquard needs a routing engine that can:

- operate under partial and stale topology knowledge
- adapt to mobility, churn, service changes, and sustained demand shifts
- remain cheap enough for constrained devices
- converge toward stable behavior without requiring extensive offline
  simulation and calibration
- fit the shared `RoutingEnginePlanner` and `RoutingEngine` boundaries

The earlier field-router sketch was still too close to explicit-path routing.
It assumed the engine could often reduce local field state into concrete peer
or path choice in a way that looked structurally similar to Pathway.

That is the wrong default assumption for the environments this engine is meant
to handle.

In the intended operating regime:

- some peers are unknown
- some links are unknown
- some paths exist but are not locally visible
- some destination reachability is only inferred through propagated summaries
- the neighborhood may change faster than end-to-end path disclosure can stay
  honest

The engine therefore cannot claim explicit route truth in the Pathway sense.
But it also should not collapse all the way down to a pure `NextHopOnly`
engine if it can maintain a stable end-to-end corridor estimate from local
observations plus bounded forward-propagated evidence.

The design target is therefore:

- hidden-information routing
- local deterministic update rules
- distributed self-equilibration
- posture adaptation under changing regime
- honest `CorridorEnvelope` publication
- usefulness under extremely low-information feedback
- correctness under strongly asymmetric dataflow

---

## Design Criteria

### 1. Honest hidden-information model

The engine must distinguish:

- unknown from unreachable
- stale from current
- direct evidence from forward-propagated evidence
- aggregate corridor belief from explicit path knowledge

It must never publish an explicit path it does not actually know.

### 2. Correctness under low-information feedback

The base engine semantics must assume weak evidence.

The engine should remain coherent and useful under BATMAN-like minimal
feedback, including environments where the practical baseline is little more
than hello-style neighbor liveness and coarse reachability hints.

Richer evidence may:

- refine ranking
- narrow hop bands or posterior-support bands
- reduce uncertainty
- improve degradation classification

Richer evidence must not be required for the engine to remain semantically
honest.

### 3. Correctness under asymmetric dataflow

The base operating assumption is not only low evidence, but strongly
asymmetric evidence flow.

This engine is intended for mobile mesh regimes where:

- local direct observations are relatively available
- forward-propagated summaries may be available in bounded form
- reverse feedback is sparse, delayed, lossy, or absent

The engine must therefore remain coherent without assuming rich return-path
telemetry.

Reverse feedback may:

- narrow uncertainty
- improve degradation classification
- validate or invalidate corridor expectations faster
- improve alternate selection

Reverse feedback must not be a structural dependency of the semantic model.

### 4. Self-equilibration from local rules

The engine should converge through repeated local updates, not through:

- globally synchronized optimization
- hand-tuned per-device interaction models
- centralized calibration
- one-shot full-network simulation as a precondition for correctness

The system should be able to settle into stable local equilibria from bounded
local computation and bounded message exchange.

### 5. Regime adaptation

The engine must adapt when conditions shift materially, including:

- sustained bandwidth demand increase
- neighborhood density changes
- retention pressure increase
- churn or instability increase
- rising contention or congestion
- changing adversary pressure

Adaptation should occur by changing posture in response to regime and bounded
control variables,
not by requiring a completely different engine.

### 6. Deterministic and replayable updates

The engine must preserve Jacquard's determinism rules:

- integer-only or bounded discrete arithmetic
- explicit `Tick`, `OrderStamp`, and `RouteEpoch` handling
- deterministic tie-breaking
- no hidden ambient callbacks mutating routing truth
- synchronous engine advancement within a round

### 7. Cheapness

Ordinary operation should avoid full graph reconstruction and large path
ensembles.

The intended steady-state cost envelope is:

- `O(neighbor_count)` for most destination-summary refresh work
- `O(frontier_size)` for detailed candidate maintenance
- bounded per-destination state
- bounded memory for propagated summary retention

### 8. Honest route-shape claim

The engine should report `RouteShapeVisibility::CorridorEnvelope`.

That claim must have a precise contract.

The engine may claim:

- current primary continuation and bounded alternates
- conservative hop-count band
- conservative end-to-end viability or posterior-support estimate
- freshness window for that estimate
- degradation class
- regime and posture under which the claim is valid

The engine may not claim:

- stable intermediate-node set
- exact segment sequence
- suffix truth beyond immediate continuation
- exact downstream persistence guarantee

`CorridorEnvelope` therefore means corridor truth with bounded uncertainty, not
explicit path truth.

### 9. Shared-contract compatibility

The engine must fit Jacquard's current model:

- shared world observations in `Observation<Configuration>`
- shared routing objects such as `RouteCandidate`, `RouteAdmission`,
  `RouteWitness`, and `RouteMaintenanceResult`
- router-owned canonical identity and lease semantics
- host-owned async drivers and bridge-owned round progression

---

## Thesis

Jacquard should model field routing as a proactive corridor engine with shared
`CorridorEnvelope` visibility.

The engine is best understood as a local observer-controller over
destination-directed corridor viability under asymmetric information.

That means one governing invariant should drive the whole design:

- at every round, the engine publishes the strongest `CorridorEnvelope`
  justified by its current posterior, while choosing the routing posture that
  best responds to persistent prediction error under the inferred regime

Everything else in the design is in service of that invariant:

- the posterior is the observer state
- the corridor is the controlled routing object
- the mean field is the low-order neighborhood summary
- the regime is the inferred explanatory model for the local environment
- the posture is the policy stance chosen toward that regime
- the corridor envelope is the conservative shared projection of the posterior

This keeps the engine honest under hidden information while still providing a
stronger model than pure next-hop routing.

## Unified Model

The engine should be read as one pipeline, not a collection of separate ideas.

For each destination of interest:

1. collect direct evidence, forward-propagated evidence, and any reverse
   feedback that happens to exist
2. update a posterior over corridor viability and continuation quality
3. project that posterior into a private corridor belief envelope
4. compress neighborhood behavior into a low-order mean-field state
5. infer the most plausible operating regime from persistent residual error
6. choose a routing posture toward that inferred regime
7. score neighbor continuations under that posture
8. publish the strongest shared `CorridorEnvelope` justified by the posterior
9. accumulate residual error and repeat

This gives each major component one role:

- information theory defines the observer semantics
- mean field defines the low-order latent neighborhood state
- regime inference explains persistent residual structure
- posture selection is the control response
- maintenance is typed recovery when the corridor leaves its admissible
  envelope

## Mathematical Model

Mathematically, the cleanest interpretation is:

- the engine is a controlled partially observed switching system
- corridor publication is a conservative projection of belief

The engine should therefore be modeled around one latent state, one belief
state, one control choice, and one public projection.

### State Variables

For one destination of interest at round `t`, define a latent state

```text
x_t = (c_t, r_t, n_t)
```

where:

- `c_t`
  corridor state: viability, continuation quality, congestion posture,
  retention usefulness, and related destination-local corridor properties
- `r_t`
  regime state: sparse, congested, retention-favorable, unstable,
  adversarial, or other bounded operating regime class
- `n_t`
  neighborhood state: low-order local interaction structure summarized by the
  mean-field model

The engine does not observe `x_t` directly.

### Observation Model

At each round, the engine receives evidence

```text
o_t = (d_t, f_t, q_t)
```

where:

- `d_t`
  direct evidence
- `f_t`
  forward-propagated evidence
- `q_t`
  reverse feedback when available

The asymmetric dataflow model means `q_t` may be sparse or absent for long
intervals.

Conceptually:

```text
o_t ~ p(o_t | x_t)
```

with source weighting and reflection discounting built into the effective
observation model.

### Belief State

The engine maintains a posterior belief

```text
b_t(x) = P(x_t = x | o_1:t, u_1:t-1)
```

where `u_t` is the chosen routing posture at round `t`.

This belief state is the primary internal truth of the engine.

Everything else is derived from it:

- corridor belief is the marginal or projection over `c_t`
- regime belief is the marginal or projection over `r_t`
- uncertainty comes from entropy or dispersion of `b_t`
- posterior support comes from concentrated mass in `b_t`

### Mean-Field Compression

The mean field should be treated as a moment closure or low-order compression
of neighborhood interaction state:

```text
m_t = M(b_t, o_t)
```

where `M` is a bounded deterministic compression operator.

This makes the mean field mathematically subordinate to the belief model:

- it is not a competing state model
- it is a low-dimensional summary used by regime inference and control

### Regime Inference

The regime observer should infer regime from the belief state, mean field, and
persistent residual stream.

Conceptually:

```text
P(r_t | b_t, m_t, e_1:t)
```

where `e_t` is the prediction or innovation residual at round `t`.

Sequential evidence accumulation should be modeled with bounded residual
updates such as:

```text
z_{t+1} = clamp(z_t + residual_t)
```

or, in the likelihood view,

```text
L_{t+1} = clamp(L_t + loglikelihood_margin_t)
```

Regime change is then declared only when:

- the error residual or likelihood margin crosses the regime-change threshold
- dwell-time constraints are satisfied

This is the mathematical basis for error-correcting regime detection.

### Posture as Control

Posture is not another latent variable. It is the control choice.

Formally:

```text
u_t in U
```

where `U` is the finite set of routing postures.

The controller should choose posture by optimizing expected cost under the
current belief and regime estimate:

```text
u_t = argmin_u E_{b_t}[loss(x_t, u)] + switch_penalty(u, u_{t-1})
```

This gives posture a precise role:

- regime is inferred
- posture is chosen

### Dual-Variable Control State

The bounded control variables should be interpreted as dual variables or
constraint multipliers for persistent resource mismatch.

Conceptually:

```text
lambda_{t+1} = clamp(lambda_t + eta * error_t)
```

where `lambda_t` includes quantities like:

- congestion price
- relay price
- retention price
- risk price

and `error_t` measures persistent violation of local operating targets.

This makes the control layer mathematically coherent:

- it remembers persistent mismatch
- it biases future scoring and posture choice
- it remains bounded and deterministic

### Corridor Envelope as Conservative Projection

The shared `CorridorEnvelope` should be defined as a conservative projection of
the current posterior:

```text
E_t = Pi_alpha(b_t)
```

where `Pi_alpha` publishes only claims supported above the configured
conservative threshold or inside the admissible credibility envelope.

That means:

- the corridor envelope is not a separate hidden semantic object
- it is the public projection of private belief

### Observer-Controller Pipeline

The full closed-loop model is therefore:

```text
o_t -> b_t -> m_t -> r_t -> u_t -> E_t
```

where:

- `o_t`
  evidence
- `b_t`
  posterior belief
- `m_t`
  mean-field summary
- `r_t`
  inferred regime
- `u_t`
  chosen posture
- `E_t`
  published corridor envelope

This is the single mathematical backbone for the engine.

### Local Attractor View

Optional coordination should not be modeled as a separate authority object by
default. It should be modeled as a local attractor view derived from the same
observer-controller state.

Conceptually:

```text
a_t = A(b_t, m_t, r_t, u_t, lambda_t)
```

where:

- `a_t`
  local attractor view over destinations and continuations
- `b_t`
  posterior belief
- `m_t`
  mean-field summary
- `r_t`
  inferred regime
- `u_t`
  chosen posture
- `lambda_t`
  bounded control state

The attractor view is not a second routing ontology. It is a derived local
geometry that says:

- which destinations or continuations are locally attractive
- how strong that pull is
- how stable that pull is
- when competing pulls are close enough to require conservative behavior

The coherence story is therefore:

- each node maintains a local attractor view
- neighboring nodes run the same observer-controller model on overlapping
  evidence
- coherent forwarding behavior emerges from the overlap of those local
  attractor views

No explicit committee or globally agreed authority set is required for that
coherence.

### Unifying Objective

To keep the components harmonized, the design should behave as if it were
trying to reduce one composite objective:

```text
V_t =
  prediction_error
  + constraint_violation
  + uncertainty_penalty
  + switching_penalty
```

Intuitively:

- the observer reduces prediction error
- the controller reduces persistent constraint violation
- the publication rule penalizes unjustified certainty
- the switching rule penalizes unnecessary regime or posture churn

The document does not need a full proof of convergence, but this objective
gives every subsystem one coherent purpose.

### Compositional Compression Lens

The implementation should follow one additional mathematical rule:

- each layer exports only the smallest decision-sufficient summary needed by
  the next layer

This is the main way to control complexity without discarding the model.

The intended compressions are:

- posterior -> corridor belief envelope
  conservative projection for publication and runtime use
- posterior -> mean field
  low-order moment compression
- residual history -> regime state
  sequential test statistic rather than raw history
- destination universe -> active destination set
  relevance-pruned sparse support
- continuation universe -> frontier
  bounded top-k support
- candidate summaries -> transmitted summaries
  information-gain-gated fixed-size messages

This gives the implementation one compositional discipline:

- never pass rich raw state across layers if a bounded sufficient statistic
  will preserve the next decision

---

## Rationale

The design should not be read as "corridors plus information theory plus mean
field plus control theory." That framing makes the parts feel additive.

The intended reading is narrower:

- this is one observer-controller model
- information theory gives semantics to the observer
- mean field gives a low-order latent state for the observer and controller
- regime inference explains persistent residual structure
- posture selection is the controller's policy response

The following subsections justify why each of those pieces is needed inside
that one model.

### Why not explicit-path field routing

Explicit-path routing assumes path truth that this engine often will not have.
Forcing the field engine into that shape would either:

- make it dishonest about what it knows, or
- force heavy path-disclosure machinery that undermines the local adaptive
  design

Pathway already occupies the explicit-path part of the design space.

### Why not stop at BATMAN-style next hop

Pure `NextHopOnly` undersells what the engine can usefully maintain.

A field engine can often maintain a meaningful corridor posterior from:

- local observations
- bounded propagated destination summaries
- local relay and retention conditions
- persistent pressure signals

That is enough to publish a conservative envelope with:

- approximate hop-band information
- delivered connectivity posture
- aggregate posterior support and validity window
- degradation class

without claiming a concrete explicit path.

### Why bounded control variables matter

A pure local ranking rule is not enough if the observer-controller is expected
to self-equilibrate under changing demand.

If sustained demand rises, the engine needs an internal way to suppress
overused corridors and shift regime without a hand-tuned external controller.

The cleanest way to do that is with bounded local control variables that behave
like shadow prices:

- congestion price
- relay price
- retention price
- risk price

These prices are not global market variables. They are deterministic local
control signals that let the network re-balance through local responses.

### Why make the slow path explicitly mean-field plus control

The slow path should be more than "control variables change and the regime
switches."

If the field engine is meant to self-equilibrate, the observer-controller
needs two distinct slow-path mechanisms:

- a mean-field layer
  that summarizes the inferred regime through bounded order parameters
- a controller layer
  that updates local biases, penalties, and damping terms from persistent
  error signals

The mean-field layer explains how local behavior becomes coherent rather than
twitchy. The controller layer explains how the node regulates itself under
sustained demand, congestion, retention pressure, or instability.

This separation is stronger than a generic scoring heuristic:

- the controller carries slow control memory
- the mean field carries local equilibrium structure
- posture is derived from inferred regime and mean-field state under
  supervisory rules

That makes the slow path a real equilibrium mechanism rather than a loose
collection of ad hoc penalties.

### Why make the model explicitly information-theoretic

The engine should go one step further and interpret the observer as an
information-processing system.

That gives exact meanings to concepts that otherwise stay vague:

- uncertainty
- posterior support
- correction signal
- regime mismatch
- reflected or low-value evidence

The right model is a bounded discrete belief system over a small latent state,
updated by local observations and propagated summaries.

In that model:

- uncertainty is posterior entropy or dispersion
- posterior support is concentrated posterior mass on the leading corridor class
- correction strength comes from likelihood surprise or divergence between
  prediction and observation
- regime mismatch is accumulated evidence that the current regime model no
  longer explains observations well

This is stronger than saying the engine merely "smooths" or "scores" inputs.
It says the engine performs bounded information fusion and bounded
model-selection under deterministic resource constraints, then hands that
state to the controller.

### Why asymmetric dataflow must be explicit

This engine is not merely designed for "missing some data." It is designed for a forward-heavy, feedback-poor environment.

That means the baseline evidence model is:

- direct local observation
- bounded forward-propagated summary information
- little or no reliable reverse-path information

If the engine were designed around symmetric feedback and then degraded to
asymmetry, it would become brittle and overconfident in the environments it is
actually meant to serve.

The correct design stance is the opposite:

- asymmetric dataflow is the semantic baseline
- reverse feedback is an optional refinement channel
- absence of reverse feedback widens uncertainty rather than collapsing the
  model

---

## Complete Architecture

The architecture should be read as one state-transition system.

At every synchronous round, the engine:

1. updates posterior belief from evidence
2. projects that belief into a private corridor belief envelope
3. updates mean-field state from local neighborhood behavior
4. updates control state from persistent residual error
5. infers regime from the residual stream
6. chooses posture toward that regime
7. re-scores continuations and updates the active corridor
8. publishes the strongest shared corridor envelope justified by the posterior

The sections below describe different parts of that same transition system.

### 1. Semantic Contract

#### Engine Role

This engine is an external routing engine under the shared Jacquard contract.
It plugs into:

- `RoutingEnginePlanner`
- `RoutingEngine`
- shared router materialization and maintenance flow

It does not define a second top-level routing API.

#### Route Semantics

The engine's private semantic object is a corridor, not a path.

A corridor means:

- a destination-directed aggregate forwarding commitment
- a current primary continuation plus bounded alternates
- an estimated end-to-end envelope
- bounded uncertainty and freshness

The engine may know:

- current best egress
- plausible hop-count band
- posterior support that progress exists
- expected congestion or retention regime

The engine may not know:

- the full hop-by-hop path
- the full set of intermediate nodes
- whether all downstream choices remain unchanged

#### CorridorEnvelope Contract

A `CorridorEnvelope` claim means the engine can publish, for one destination and one current corridor selection:

- current primary continuation and bounded alternates
- conservative hop-count band
- conservative end-to-end viability or posterior-support estimate
- freshness window for that estimate
- degradation class
- regime and posture under which the claim is valid

A `CorridorEnvelope` claim explicitly does not mean the engine can publish:

- stable intermediate-node set
- exact segment sequence
- suffix truth beyond immediate continuation
- exact persistence guarantee of downstream choice

`CorridorEnvelope` is therefore a real semantic class:

- stronger than `NextHopOnly`
- weaker than `ExplicitPath`
- centered on corridor truth with bounded uncertainty

It is specifically the shared projection of the engine's private posterior and
not an independent object with its own hidden semantics.

#### Low-Information Baseline

The engine's base semantics must remain valid under extremely low-information
feedback.

The intended baseline is BATMAN-like:

- hello-style neighbor liveness
- coarse reachability signals
- sparse or absent downstream detail

The engine should still remain coherent in that low-information regime by widening
uncertainty, shortening validity windows, and suppressing strong claims.

Richer evidence may:

- refine ranking
- narrow hop bands or posterior-support bands
- improve degradation classification
- improve alternate selection

Richer evidence must not change the semantic contract. It only tightens the
same corridor envelope.

#### Asymmetric Dataflow Contract

The engine should explicitly assume asymmetric observability.

The semantic baseline is:

- local direct evidence
- forward-propagated bounded summaries
- weak, delayed, or absent reverse evidence

The contract-level rule is:

- reverse feedback may refine a claim, but the claim must remain
  coherent without it

The engine must not rely on symmetric end-to-end telemetry, frequent
acknowledgement traffic, or dense reverse-path summaries to make corridor
semantics meaningful.

#### Information-Theoretic Contract

The engine should interpret corridor estimation as a bounded belief over a
small latent state rather than as an unstructured support score.

That latent state should cover only what the engine can honestly model, such
as:

- corridor usability class
- congestion regime class
- retention usefulness class
- stability or risk class
- best continuation class

The contract-level rule is:

- public route claims are conservative projections of this private posterior
  belief

The engine must not publish stronger corridor-envelope claims than the posterior
support justifies under its bounded evidence model.

### 2. Private State and Update Algebra

#### Architectural Layers

1. Shared observation layer
   The engine consumes only shared observations:
   `Observation<Configuration>`, `Node`, `Link`, `Environment`,
   `ServiceDescriptor`, and `RoutingObjective`.
2. Engine-private inference layer
   The engine derives bounded private state from shared observations and
   retained prior state.
3. Planner and admission layer
   The planner converts private field state into shared route objects
   conservatively.
4. Runtime layer
   The runtime binds one admitted corridor to one engine-private forwarding
   record.
5. Maintenance layer
   Maintenance evaluates corridor validity, alternates, degradation, and
   replacement triggers.

No field-specific facts are promoted into `jacquard-core`.

#### Core Private State

The engine should separate slow-moving controller, mean-field, regime, and
posture
state from fast-moving destination-local forwarding estimates.

These are not peer concepts. They form one chain:

- posterior state estimates corridor viability
- mean-field state compresses neighborhood structure
- regime state explains persistent residuals
- posture state selects the control response

```rust
pub struct FieldEngineState {
    pub destinations: BTreeMap<DestinationId, DestinationFieldState>,
    pub mean_field: MeanFieldState,
    pub controller: ControlState,
    pub regime: RegimeObserverState,
    pub posture: PostureControllerState,
    pub last_tick_processed: Tick,
}

pub struct DestinationFieldState {
    pub destination: DestinationId,
    pub posterior: DestinationPosterior,
    pub progress_belief: ProgressBelief,
    pub corridor_belief: CorridorBeliefEnvelope,
    pub frontier: Vec<NeighborContinuation>,
    pub summary_freshness: SummaryFreshness,
    pub last_material_interest: Option<Tick>,
}

pub struct DestinationPosterior {
    pub usability_entropy: PenaltyPoints,
    pub top_corridor_mass: HealthScore,
    pub regime_belief: RegimeBeliefState,
    pub predicted_observation_class: ObservationClass,
}

pub struct ProgressBelief {
    pub progress_score: Belief<HealthScore>,
    pub uncertainty_penalty: Belief<PenaltyPoints>,
    pub posterior_support: Belief<HealthScore>,
}

pub struct CorridorBeliefEnvelope {
    pub expected_hop_band: HopBand,
    pub delivery_support: Belief<HealthScore>,
    pub congestion_penalty: Belief<PenaltyPoints>,
    pub retention_affinity: Belief<HealthScore>,
    pub validity_window: TimeWindow,
}

pub struct NeighborContinuation {
    pub neighbor_id: NodeId,
    pub net_value: Belief<HealthScore>,
    pub downstream_support: Belief<HealthScore>,
    pub expected_hop_band: HopBand,
    pub freshness: Tick,
}

pub struct MeanFieldState {
    pub relay_alignment: Belief<HealthScore>,
    pub congestion_alignment: Belief<HealthScore>,
    pub retention_alignment: Belief<HealthScore>,
    pub risk_alignment: Belief<HealthScore>,
    pub field_strength: Belief<HealthScore>,
}

pub struct ControlState {
    pub congestion_price: PenaltyPoints,
    pub relay_price: PenaltyPoints,
    pub retention_price: PenaltyPoints,
    pub risk_price: PenaltyPoints,
    pub congestion_error_integral: PenaltyPoints,
    pub retention_error_integral: PenaltyPoints,
    pub relay_error_integral: PenaltyPoints,
    pub churn_error_integral: PenaltyPoints,
}

pub struct RegimeObserverState {
    pub current: OperatingRegime,
    pub current_regime_score: HealthScore,
    pub regime_error_residual: PenaltyPoints,
    pub log_likelihood_margin: PenaltyPoints,
    pub regime_change_threshold: PenaltyPoints,
    pub regime_hysteresis_threshold: PenaltyPoints,
    pub dwell_until_tick: Tick,
}

pub struct PostureControllerState {
    pub current: RoutingPosture,
    pub stability_margin: HealthScore,
    pub convergence_score: HealthScore,
    pub posture_switch_threshold: PenaltyPoints,
    pub last_transition_tick: Tick,
}

pub enum OperatingRegime {
    Sparse,
    Congested,
    RetentionFavorable,
    Unstable,
    Adversarial,
}

pub enum RoutingPosture {
    Opportunistic,
    Structured,
    RetentionBiased,
    RiskSuppressed,
}
```

The important boundaries are:

- per-destination corridor state
- posterior belief state
- mean-field state
- controller state
- regime observer state
- posture controller state
- bounded frontier of neighbor continuations

#### Observed Inputs

The engine should build its state from shared observable facts such as:

- link delivery support
- link symmetry
- link loss or contention indicators
- link stability horizon
- node relay budget and hold capacity
- service advertisement validity
- coarse environment churn and contention regime
- any shared identity-assurance or authentication classes already present

It may also consume engine-private propagated destination summaries from
neighbors. Those summaries are evidence, not shared core truth.

Evidence classes should be treated distinctly:

- direct evidence
- forward-propagated evidence
- reverse feedback evidence

Reverse feedback is an optional high-value channel, not the semantic baseline.

#### Destination Interest Policy

The engine must define a deterministic policy for what counts as a destination
of interest.

Interest may be created by:

- active local-origin service demand
- recent local-origin traffic
- recent transit importance
- recent forward-propagated relevance from neighbors
- explicit administrative pinning

Interest should be retained only while a bounded relevance score remains above
the eviction threshold. That score should decay deterministically over time.

Interest must be evicted when:

- there is no recent local demand
- there is no recent transit relevance
- forward-propagated relevance has decayed below threshold
- the bounded destination budget is exceeded and lower-ranked destinations lose
  tie-breaks

This policy is engine-private, but it must exist explicitly so the destination
set stays practically bounded.

#### Propagated Summary Model

To support `CorridorEnvelope`, nodes exchange bounded destination summaries. These
are not explicit paths and do not require route-vector disclosure.

A propagated summary for destination `D` should carry only aggregate fields
such as:

- destination identifier
- summary freshness marker
- estimated hop-count band
- delivered support band
- congestion or pressure band
- retention support hint
- uncertainty class
- evidence contribution class

The base engine must function even when only a strict subset of those fields
is available. Missing fields widen uncertainty and reduce posterior support rather
than breaking the model.

In particular, the model must function when reverse-facing fields are absent
for long intervals.

#### Summary Algebra

The engine should define a small private algebra for deterministic summary
combination. At minimum, the spec should name these operators:

- `decay_summary(summary, now_tick)`
- `compose_summary_with_link(summary, direct_link)`
- `merge_neighbor_summaries(left, right)`
- `clamp_corridor_envelope(summary, regime, control_state)`
- `derive_degradation_class(summary, regime, control_state)`
- `discount_reflected_evidence(summary, local_origin_trace)`
- `project_posterior_to_claim(posterior)`

These operators should satisfy these rules:

- direct evidence has priority over forward-propagated evidence when they conflict
- freshness decay can only weaken a corridor envelope, never strengthen it
- uncertainty accumulation can only widen bands or reduce posterior support
- composition must be monotone with respect to degradation
- outputs must be clamped to bounded integer ranges
- lack of reverse evidence must never be treated as symmetric negative proof
  by default

The purpose of this algebra is to keep corridor estimation from becoming ad
hoc.

#### Information Fusion Rules

The summary algebra should be understood as an information-fusion layer, not
just a merge layer.

At minimum, it should obey these rules:

- direct evidence contributes more weight than forward-propagated evidence
- low-freshness evidence contributes less information than fresh evidence
- correlated or reflected evidence is discounted rather than counted twice
- reverse feedback, when present, may carry high correction value but low
  expected availability
- disagreement between prediction and observation increases uncertainty or
  widens the admissible band
- posterior concentration may tighten a corridor envelope, but only within the supported
  conservative envelope

The engine should therefore model three private quantities explicitly:

- posterior entropy
- prediction-versus-update divergence
- source-value discount for forward-propagated evidence

#### Loop and Anti-Reflection Semantics

The engine needs explicit loop semantics, not just vague loop-safe acceptance
language.

The minimum required invariants are:

- anti-reflection
  A node must never increase destination potential by accepting a reflected
  version of its own summary through a neighbor.
- monotone primary selection
  A primary continuation is admissible only if it improves destination
  potential under the engine's monotone ordering after uncertainty and
  freshness penalties are applied.
- no immediate backtracking
  A materialized corridor must not select a continuation that deterministically
  routes the next forwarding decision back to the local node under the same
  summary generation.

This does not claim full global loop freedom. It does require that local
summary processing cannot amplify self-originated optimism into admissible
corridor truth.

#### Two-Timescale Update Model

The engine should update on two coupled timescales.

Fast loop: destination progress-belief refresh

1. ingest latest shared topology observation
2. expire stale neighbor summaries
3. refresh direct-link and local-service signals
4. recompute per-destination neighbor continuation values
5. update destination progress belief and corridor belief envelope
6. refresh a bounded top-k frontier

This loop should be cheap and mostly destination-local.

Slow loop: control, regime inference, and posture adaptation

1. measure sustained congestion, relay pressure, retention pressure, and risk
2. update bounded controller state with deterministic feedback and decay
3. compute local mean-field order parameters from controller outputs and
   observed neighborhood state
4. run supervisory regime-change detection against the current regime
5. choose posture from the inferred regime, mean field, and controller state
6. apply hysteresis and dwell-time constraints before posture transition
7. re-score destination frontiers under the new posture

This is the self-equilibration loop.

#### Error-Correcting Observer Model

The fast path should behave like a bounded observer-corrector.

For each destination, the engine should:

1. predict the next corridor belief from prior state
2. derive the expected observation class from that prediction
3. ingest direct evidence and forward-propagated evidence
4. compute an innovation or surprise signal from the mismatch
5. update the posterior with bounded correction
6. project the corrected posterior into a conservative corridor envelope

The key invariant is:

- no correction step may strengthen a public corridor envelope beyond what the updated
  posterior support justifies

This observer is the primary state estimator for the whole engine. The mean
field, regime observer, and posture controller all consume its residuals or
its compressed outputs rather than defining competing truths.

The observer should be designed so that:

- direct evidence is the default correction channel
- forward-propagated evidence is the default downstream inference channel
- reverse feedback is opportunistic correction, not required closure of the
  loop

#### Mean Field, Regime, and Posture

The slow path has three distinct private layers.

Controller state is:

- continuously updated bounded control-memory
- derived from sustained error signals
- responsible for prices, penalties, damping terms, and bias shifts

Mean-field state is:

- a bounded set of local order parameters
- computed from observations plus controller outputs
- responsible for summarizing the current local equilibrium tendency

Regime is:

- an inferred environmental or operating condition
- estimated from observations, mean-field state, and controller residuals

Posture is:

- the engine's chosen stance toward the current regime
- selected from regime, mean-field state, and controller state
- protected by hysteresis, residual thresholds, and dwell-time constraints

Candidate scoring may depend on all three, but their roles differ:

- controller state regulates
- mean-field state summarizes
- regime describes conditions
- posture selects the active operating mode

These layers are sequential, not parallel:

- mean field summarizes what kind of neighborhood the observer currently sees
- regime explains that summary and the residual stream
- posture chooses how the controller should respond

#### Mean-Field Slow Path

The slow path should explicitly use a local mean-field model with
Ising-like intent, but without requiring floating-point simulation or global
equilibrium solves.

The engine should maintain bounded local order parameters such as:

- relay alignment
- congestion alignment
- retention alignment
- risk or instability alignment
- overall field strength

These are not route claims. They are engine-private slow variables describing
whether the local neighborhood currently supports structured forwarding,
retention-biased operation, or conservative suppression.

The key rule is:

- fast-path routing consumes corridor belief envelopes
- slow-path mean-field state shapes regime inference
- posture determines how the engine responds to that regime

#### Feedback Controller

The controller layer should be described explicitly as a deterministic
feedback controller rather than as "prices happen somehow."

It should regulate target quantities such as:

- forwarding load versus healthy relay capacity
- retention occupancy versus retention target
- contention versus congestion budget
- route churn versus stability target

The controller outputs bounded control variables such as:

- congestion price
- relay price
- retention price
- risk price
- damping or gain modifiers

Those control variables then bias the mean-field update and candidate scoring.

#### Regime Change Detection

Regime change detection should be stronger than raw thresholding.

The right model here is a deterministic regime observer over the mean-field
and controller state.

At minimum, the supervisor should:

1. measure filtered error signals and order-parameter residuals
2. compare them against the expected manifold of the current regime
3. accumulate a bounded regime error residual when the current regime no longer
   explains local conditions
4. declare regime change only when the residual crosses
   `regime_change_threshold` and dwell-time constraints are satisfied
5. reset or partially discharge the residual on confirmed regime transition
6. re-evaluate posture only after regime transition or a material
   regime-confidence shift

This is effectively a bounded change detector plus regime observer.
It is a better control-theory fit than one-shot regime thresholding.

If we want a concrete family to emulate, the right mental model is:

- low-pass filtered state observer
- bounded integral controller for persistent error
- CUSUM-like residual accumulation for change detection
- hysteretic posture controller for mode switching

All of that can still be implemented with deterministic bounded integer
arithmetic.

#### Information-Theoretic Regime Detection

The regime observer should be described as a sequential evidence test between
regime models.

Each regime defines an expected observation pattern and residual envelope.
The supervisor should:

1. evaluate how well the current observation stream fits the current regime
   model
2. compare that fit against one or more alternative regime models
3. accumulate bounded sequential evidence, including
   `regime_error_residual` and `log_likelihood_margin`, in favor of switching
   only when the current regime repeatedly explains observations poorly
4. require both `regime_change_threshold` crossing and dwell-time constraints
   before accepting a new regime
5. use a lower `regime_hysteresis_threshold` to remain in the new regime
   without chatter

The important semantics are:

- a regime is not only a scoring bias family
- a regime is also a predictive model for what the local residual stream
  should look like if that regime is appropriate

This turns regime change detection into bounded model selection rather than
simple threshold crossing.

Under asymmetric dataflow, regime detection should rely primarily on locally
observable residuals such as:

- contention growth
- congestion persistence
- relay burden
- retention occupancy
- route churn

It should not require rich downstream acknowledgement or return-path telemetry
to remain effective.

#### Regime Model

Suggested regime semantics:

- `Sparse`
  Weak connectivity, limited alternatives, low relay density.
- `Congested`
  Persistent contention, high relay burden, or queue pressure.
- `RetentionFavorable`
  Deferred-delivery or hold semantics are locally valuable.
- `Unstable`
  Churn, volatility, or topology drift dominate.
- `Adversarial`
  Risk suppression and conservative trust assumptions dominate.

Regime should change only when the accumulated evidence says the current
environmental model no longer explains observations well and the
error-correction residual has crossed the regime-change threshold.

#### Posture Model

Suggested posture semantics:

- `Opportunistic`
  Cheap forwarding, low coordination overhead, suitable toward sparse or
  weakly supported regimes.
- `Structured`
  Stronger corridor preference, more stable alternates, more willingness to
  preserve aggregate route quality in better-supported regimes.
- `RetentionBiased`
  Forwarding de-emphasized when congestion or partition pressure rises; hold-
  capable peers gain value.
- `RiskSuppressed`
  Aggressive behavior damped when the inferred regime is unstable or
  adversarial.

Posture should change only when:

- the regime has changed and a new posture is materially better toward the
  newly inferred regime, or
- the inferred regime has not changed but regime confidence has shifted enough
  to cross `posture_switch_threshold`, or
- the current posture is no longer valid under current control and evidence
  bounds

#### Local Control Law

Each destination-neighbor continuation should be evaluated by a deterministic
net-value rule:

```text
net_value(neighbor, destination)
  = destination_progress
  + relay_or_retention_bonus
  - link_cost
  - congestion_price
  - relay_price
  - retention_price
  - risk_price
  - uncertainty_penalty
```

The exact coefficients are engine-private, but the rules are not:

- all arithmetic is integer or bounded discrete
- every term has an explicit bounded range
- the same input sequence yields the same ordering
- unknown data increases uncertainty penalty rather than pretending to be zero
- persistent overload raises price terms over time

#### Self-Equilibration Mechanism

Self-equilibration comes from three coupled effects:

- feedback control
  When a node becomes overloaded or unstable, controller state integrates
  persistent error and updates bounded control variables.
- mean-field realignment
  As controller outputs shift, local order parameters move toward a different
  equilibrium regime.
- supervisory regime switching
  Regime changes require sustained residual evidence and avoid one-tick
  oscillation.

The correct target is stable local equilibrium under bounded-change
assumptions, not an unconditional proof of one globally optimal state.

#### Information-Theoretic Quantities

The engine should explicitly name the private quantities it uses to reason
about belief quality:

- entropy
  how uncertain the node is about corridor or regime state
- divergence
  how far the updated belief moved from the predicted belief
- likelihood margin
  how much better one regime model explains the current evidence stream than
  another
- source-value discount
  how much genuinely new information a forward-propagated summary contributes after
  freshness and reflection penalties

Under asymmetric dataflow, source-value discount should also account for the
fact that reverse evidence may be highly informative when present, but cannot
be assumed to arrive regularly.

These quantities remain engine-private and discretized. They are not promoted
into shared route objects directly.

### 3. Admission and Witness Contract

#### Candidate Formation

The planner should enumerate candidates from the destination frontier, not
from full path search.

Each candidate should be derived from:

- destination field state
- current posture
- best continuation or small alternate set
- corridor belief envelope

Candidate ordering should primarily reflect:

- net corridor value
- delivered connectivity posture
- expected hop band
- posterior concentration
- deterministic tie-breakers

The advisory `route_id` should be derived deterministically from the engine
identity, destination, and the stable corridor-selection key, not from an
explicit path.

#### Admission Contract

Admission should be conservative.

The engine may claim:

- aggregate corridor viability
- expected delivered connectivity posture
- bounded step estimates
- explicit degradation class when uncertainty is high

The engine may not claim:

- exact end-to-end path existence
- exact hop sequence
- stronger protection or reachability than the corridor evidence supports

For many candidates the right `ClaimStrength` is
`ConservativeUnderProfile`, not `ExactUnderAssumptions`.

`RouteAdmissionCheck` should reflect:

- route cost from corridor-envelope properties
- productive and total step bounds as bands conservatively collapsed into
  shared limits
- rejection when uncertainty, capacity, or protection requirements are not
  satisfied
- rejection when posterior support is too diffuse to justify the requested
  claim

#### Witness Contract

The witness should explain not only what was admitted, but how the engine was
allowed to admit it.

The witness should encode:

- requested versus delivered protection
- requested versus delivered connectivity
- admission profile and regime assumptions
- topology epoch
- degradation reason when the corridor is sparse, unstable, pressured, or
  uncertain
- direct-evidence contribution class
- forward-propagated-evidence contribution class
- reverse-feedback contribution class
- uncertainty level
- posterior concentration class
- freshness regime
- inferred regime
- routing posture

Typical degradation reasons remain:

- `SparseTopology`
- `LinkInstability`
- `CapacityPressure`
- `PartitionRisk`
- `PolicyPreference`

This keeps the witness diagnostic and epistemically honest.

#### Shared Publication Surface

The engine should publish shared route objects with:

- `route_shape_visibility = CorridorEnvelope`
- `hop_count_hint` as a conservative band or belief-derived estimate
- `valid_for` from the corridor freshness window
- `connectivity` from delivered posture
- `protocol_mix` from the transport and cooperative surface actually in play

The publication claim is:

"This engine maintains an end-to-end corridor envelope with bounded
posterior support under the stated evidence, inferred regime, and routing
posture."

It is not:

"This engine knows the concrete route path."

### 4. Maintenance Invariants

#### Materialization

Materialization binds the canonical route handle to one engine-private
corridor runtime record.

```rust
pub struct MaterializedFieldCorridor {
    pub destination: DestinationId,
    pub regime: OperatingRegime,
    pub posture: RoutingPosture,
    pub primary_neighbor: NodeId,
    pub alternates: Vec<NodeId>,
    pub corridor_belief: CorridorBeliefEnvelope,
    pub installed_at_tick: Tick,
    pub last_revalidated_at_tick: Tick,
}
```

This record is private to the engine. It does not contain:

- an explicit path
- hidden planner cache dependencies
- authority to mutate router-owned canonical route identity

#### Forwarding

Forwarding follows the materialized corridor's current primary neighbor.
If the primary becomes invalid, the engine may:

- switch to a ranked alternate through typed maintenance, or
- require replacement if posterior support collapses too far

The engine should not silently mutate the active route outside shared route
maintenance flow.

Absence of reverse feedback alone should not force invalidation unless the
current regime model specifically predicted such feedback within the corridor
freshness window.

#### Maintenance Triggers

Maintenance is driven by corridor invalidation rather than path-suffix repair.
Important triggers include:

- direct next-hop degradation
- corridor posterior-support collapse
- alternate becoming materially better
- posture invalidation
- sustained congestion pressure
- retention pressure forcing fallback
- destination summary expiry

#### Maintenance Invariants

Alternates are legal only if:

- they satisfy the same loop and anti-reflection rules as the primary
- their corridor envelope remains within the admitted uncertainty envelope or a
  newly revalidated envelope
- their posterior support and freshness remain above the alternate floor

Degradation is legal only if:

- the corridor remains usable under a weaker conservative claim
- the witness and runtime state can be updated without pretending stronger
  route truth than current evidence supports
- posterior uncertainty has widened without invalidating the corridor

Replacement is mandatory when:

- no legal primary or alternate continuation remains
- uncertainty rises above the admissible ceiling
- divergence from predicted state remains persistently outside the correctable
  envelope
- freshness expires past the fail-closed boundary
- posture or protection requirements are no longer satisfied

Typical outcomes are:

- keep current corridor
- replace with better corridor
- enter retention-biased fallback
- mark degraded and continue conservatively
- invalidate when no acceptable corridor remains

This is closer to BATMAN's best-next-hop maintenance than to Pathway's suffix
repair, but with stronger corridor-envelope semantics.

#### Retention

Retention is part of posture and corridor value, not a detached helper
subsystem.

When retention pressure or partition risk rises materially:

- `retention_price` rises
- posture controller state may move toward `RetentionBiased`
- hold-capable peers gain value
- maintenance may return typed hold fallback when forwarding is no longer the
  right posture

#### Local Attractor Coherence

The default coordination story for this engine should be local attractor
coherence, not committee formation.

Each node should derive a local attractor view from the same internal state
that drives corridor belief and posture choice:

- posterior belief
- mean-field state
- regime estimate
- posture choice
- control state

That attractor view should determine:

- which destinations are locally attractive
- which continuations are favored
- how strong or weak those preferences are
- when the node should behave conservatively because the local attractor
  landscape is ambiguous

The intended coherence property is:

- no node needs a globally agreed committee to behave coherently
- neighboring nodes become directionally aligned because their attractor views
  are coupled through overlapping evidence and propagated summaries

If some future regime requires stronger explicit coordination, that should be
treated as an optional additional protocol layer above the default attractor
coherence model, not as the foundation of the engine.

#### Sync/Async Boundary

This engine must follow Jacquard's host-bridged async architecture.

That means:

- async drivers stay host-owned
- propagated summary ingress is delivered explicitly before a round
- engine advancement is synchronous in `engine_tick` and route methods
- no background task inside the engine owns route truth
- outbound summary sends or other protocol traffic are flushed asynchronously
  by the host bridge after the round

#### Bounded Implementation Strategy

Implementation should treat the mathematical model as a bounded approximation,
not as permission to build an unbounded inference engine.

The core engineering rule is:

- every subsystem should consume compressed state, not raw state histories

The intended strategy is:

- factored state
  Do not carry the full joint state over destination, neighbor, regime,
  continuation, and evidence source. Carry marginals, moments, and a bounded
  continuation frontier instead.
- fixed-size destination records
  Every tracked destination should fit in a compact fixed-size state record
  with bounded posterior buckets, bounded residual accumulators, and bounded
  frontier capacity.
- top-k continuation support
  Keep only the strongest admissible continuation frontier per destination.
  Everything else is evicted or folded into aggregate uncertainty.
- sparse active destination set
  Track only destinations with current relevance under the destination-interest
  policy.
- fixed-size propagated summaries
  Summary messages should be width-bounded and quantized. They should not grow
  with observed topology.
- event-triggered slow path
  Regime inference and posture reevaluation should consume incremental
  sufficient statistics and run on slower cadence or innovation triggers, not
  on every observation for every destination.
- delta-updated mean-field state
  Mean-field moments should be updated from local changes, not recomputed from
  scratch over raw history.
- conservative projection everywhere
  Whenever rich local state must cross a layer boundary, project it to the
  smallest conservative representation that preserves the next decision.

This implementation discipline addresses the main risks directly:

- state explosion is controlled by factored state plus sparse support
- update cost is controlled by incremental sufficient statistics
- control instability is reduced by bounded monotone operators and slower
  regime/posture cadence
- message overhead is reduced by fixed-size information-gated summaries

#### Information Bottleneck Rule

Propagation should obey an information bottleneck discipline.

A summary should be transmitted only when:

- the destination is in the active set
- the summary changed materially
- the transmitted summary is likely to change a neighbor's decision or reduce
  its uncertainty enough to justify the bandwidth

This means:

- no broad raw gossip
- no per-destination propagation just because a destination exists
- no transmission of details that do not alter continuation ranking, regime
  inference, or envelope projection downstream

Reverse feedback should follow the same discipline. When present, it may be
high-value, but it should still be quantized and filtered through the same
decision-sufficiency rule.

#### Incremental Update Rule

The engine should explicitly separate fast-path incremental updates from
slow-path supervisory updates.

Fast path:

- direct evidence update
- posterior correction
- continuation frontier maintenance
- corridor belief-envelope refresh for active destinations

Slow path:

- control-state integration
- mean-field moment update
- regime residual accumulation
- posture reevaluation

The fast path should operate on cached bounded state.
The slow path should consume sufficient statistics produced by the fast path,
not raw evidence logs.

#### Determinism Rules

The engine must preserve these invariants:

- no floating-point arithmetic
- no wall-clock semantics as routing truth
- no hidden randomization
- no hidden ownership transfer
- no ambient path discovery callback mutating private route state
- no corridor-envelope publication stronger than current evidence

Unknown must remain unknown.

If evidence is insufficient, the engine should:

- reduce posterior support
- shorten validity windows
- widen hop bands conservatively
- degrade claims
- fail closed when needed

Under asymmetric dataflow, "insufficient evidence" should usually mean:

- rely more heavily on local direct evidence
- discount stale forward-propagated summaries
- treat absent reverse feedback as missing information rather than immediate
  contradiction
- shorten validity windows when the posture expected stronger confirmation

All belief updates, entropy buckets, divergence buckets, and likelihood
margins must remain implementable with deterministic bounded integer
arithmetic.

#### Complexity and Budget Rules

The engine should explicitly bound:

- number of tracked destinations
- number of retained neighbor summaries per destination
- frontier size per destination
- alternate count per materialized route
- history window for price updates
- posterior bucket count per destination
- regime residual state width
- propagated summary width

It should also preserve these implementation invariants:

- each destination record is fixed-size or tightly upper-bounded
- continuation maintenance is `O(k)` in frontier size, not `O(network_size)`
- slow-path updates consume sufficient statistics, not raw evidence histories
- propagated summaries remain fixed-width and information-gated
- inactive destinations are evictable without semantic ambiguity

Cheapness matters as much as elegance here. The engine is meant to be
credible on devices that are bandwidth-, CPU-, memory-, and
thermal-constrained.

### Summary

The right field-router design for Jacquard is not an explicit-path field
engine and not a pure next-hop BATMAN clone.

It is a proactive `CorridorEnvelope` engine with:

- hidden-information honesty
- low-information baseline semantics
- bounded propagated corridor summaries
- explicit summary algebra
- control-theoretic self-equilibration
- regime adaptation under sustained pressure
- router-owned canonical route identity
- synchronous deterministic advancement

Its core promise is:

local rules should be enough to produce stable adaptive corridor behavior
without requiring a fully disclosed topology or heavy pre-calibration.

## Phased Work Plan

Implementation note:

- the planned workspace location for the field engine is `crates/field`

### Phase 0: Contract Lock And Workspace Preparation

- [x] Confirm the final shared vocabulary in docs and code:
  `corridor`, `CorridorEnvelope`, `OperatingRegime`, `RoutingPosture`,
  `ControlState`, `ProgressBelief`, `CorridorBeliefEnvelope`.
- [x] Decide the implementation crate name and workspace location for the field
  engine.
- [x] Update any affected shared docs so the field engine terminology is
  consistent with the current design.
- [x] Audit existing `RouteShapeVisibility` usage and identify all places that
  must support the corridor-envelope semantics.
- [x] Gate: run `just ci-dry-run` and make sure it is clean.
- [x] Gate: make a git commit for Phase 0.

### Phase 1: Shared Surface And Visibility Plumbing

- [x] Add or rename the shared route-shape visibility variant to
  `CorridorEnvelope` in core.
- [x] Update shared serialization, ordering, tests, and any capability surfaces
  that reference route-shape visibility.
- [x] Update router tests, trait tests, and any in-tree engine fixtures that
  assume the previous visibility vocabulary.
- [x] Keep `ExplicitPath`, `CorridorEnvelope`, `NextHopOnly`, and `Opaque`
  semantics explicit in shared docs and tests.
- [x] Gate: run `just ci-dry-run` and make sure it is clean.
- [x] Gate: make a git commit for Phase 1.

### Phase 2: Field Crate Scaffolding

- [x] Create the field engine crate and wire it into the workspace.
- [x] Add engine identity, capability envelope, and crate-level docs.
- [x] Scaffold the public engine surface to implement the shared planner and
  runtime traits.
- [x] Add initial compile-only contract tests proving the crate advertises
  `CorridorEnvelope` visibility and compiles against the shared routing traits.
- [x] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 2.

### Phase 3: Core Private State And Bounded Data Model

- [ ] Implement the bounded private state structs:
  `DestinationPosterior`, `ProgressBelief`, `CorridorBeliefEnvelope`,
  `MeanFieldState`, `ControlState`, `RegimeObserverState`,
  `PostureControllerState`, bounded continuation frontier, and destination
  interest tracking.
- [ ] Enforce fixed-size or tightly upper-bounded destination records.
- [ ] Implement bounded buckets for entropy, posterior support, divergence, and
  residual accumulation.
- [ ] Add tests for clamping, boundedness, deterministic ordering, and eviction
  behavior.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 3.

### Phase 4: Evidence Intake, Summary Encoding, And Information Fusion

- [ ] Define the engine-private summary format for forward-propagated evidence.
- [ ] Implement fixed-width summary encoding and decoding.
- [ ] Implement evidence classification:
  direct evidence, forward-propagated evidence, and reverse feedback.
- [ ] Implement the summary algebra:
  `decay_summary`, `compose_summary_with_link`, `merge_neighbor_summaries`,
  `discount_reflected_evidence`, `clamp_corridor_envelope`,
  `derive_degradation_class`, and `project_posterior_to_claim`.
- [ ] Add tests for reflection discounting, asymmetric evidence handling,
  bounded summary width, and deterministic fusion results.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 4.

### Phase 5: Observer Layer And Corridor Envelope Projection

- [ ] Implement the fast-path observer-corrector update.
- [ ] Implement posterior prediction, innovation calculation, bounded posterior
  correction, and conservative corridor-envelope projection.
- [ ] Ensure public corridor-envelope claims never exceed posterior support.
- [ ] Add tests for low-information operation, absent reverse feedback,
  uncertainty widening, and conservative publication.
- [ ] Add tests showing richer evidence tightens envelopes without changing the
  semantic contract.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 5.

### Phase 6: Mean Field, Regime Inference, And Posture Control

- [ ] Implement mean-field compression from posterior and local evidence.
- [ ] Implement bounded control-state updates as dual-variable or
  constraint-multiplier updates.
- [ ] Implement regime observation from residual streams, including
  `regime_error_residual`, likelihood margin, threshold crossing, hysteresis,
  and dwell-time handling.
- [ ] Implement posture selection as a control choice over the inferred regime.
- [ ] Add tests for:
  stable regimes, regime transitions, posture switching, hysteresis, and
  non-oscillation under bounded jitter.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 6.

### Phase 7: Planner, Admission, Witness, And Materialization

- [ ] Implement candidate formation from the bounded continuation frontier.
- [ ] Implement route ordering, route-id derivation, and backend refs for the
  field engine.
- [ ] Implement conservative admission checks from corridor-envelope state.
- [ ] Implement witness generation carrying evidence contribution classes,
  inferred regime, routing posture, uncertainty, and degradation.
- [ ] Implement materialization into private corridor runtime records.
- [ ] Add tests for admissible versus inadmissible corridor envelopes,
  conservative claim strength, witness correctness, and materialization
  fail-closed behavior.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 7.

### Phase 8: Forwarding, Maintenance, And Attractor Coherence

- [ ] Implement active-corridor forwarding using the primary continuation and
  bounded alternates.
- [ ] Implement maintenance triggers:
  posterior-support collapse, stale summaries, alternate improvement,
  posture invalidation, congestion pressure, and retention fallback.
- [ ] Implement the local attractor view as a derived object of posterior,
  mean-field state, regime, posture, and control state.
- [ ] Use local attractor coherence as the default coordination model.
- [ ] Add tests for attractor coherence, alternate legality, maintenance
  degradation, replacement triggers, and fail-closed invalidation.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 8.

### Phase 9: Performance Bounding And Incremental Execution

- [ ] Implement sparse active destination tracking and deterministic eviction.
- [ ] Enforce top-k continuation frontiers and bounded alternate counts.
- [ ] Make fast-path updates incremental and cache-backed.
- [ ] Make slow-path updates consume sufficient statistics instead of raw
  history.
- [ ] Add explicit information-gain gating for forward summary transmission.
- [ ] Add tests or benchmarks for:
  bounded state growth, `O(k)` frontier maintenance, summary suppression, and
  incremental-update behavior under load.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 9.

### Phase 10: Router Integration And Mixed-Engine Composition

- [ ] Register the field engine with the router through the shared traits.
- [ ] Ensure mixed-engine routing works with Pathway, BATMAN, and the field
  engine without leaking engine-private semantics across the boundary.
- [ ] Add or expand integration tests showing the router can host all three
  engines together while preserving honest route-shape visibility and canonical
  ownership.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 10.

### Phase 11: Reference Client Expansion And End-To-End Multi-Router Tests

- [ ] Expand the reference client to compose all three routers:
  Pathway, BATMAN, and the field engine.
- [ ] Update the reference-client builder and topology helpers as needed:
  `crates/reference-client/src/lib.rs`,
  `crates/reference-client/src/clients.rs`,
  `crates/reference-client/src/topology.rs`,
  and any other supporting files.
- [ ] Expand end-to-end coverage in
  `crates/reference-client/tests/e2e_multi_layer_routing.rs` so the reference
  client can route with all three routers participating in the same scenario.
- [ ] Add scenarios where:
  one hop prefers BATMAN-like next-hop routing, another prefers field
  corridor-envelope routing, and another prefers explicit Pathway routing.
- [ ] Add scenarios covering low-information asymmetry, regime change, and
  posture change while mixed-engine routing remains stable.
- [ ] Gate: run `just ci-dry-run` and make sure it is clean.
- [ ] Gate: make a git commit for Phase 11.
