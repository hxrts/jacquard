# Certified Temporal Kernel Transformation In Path-Free Distributed AI

## Abstract

Distributed AI deployments often begin from a common global model or decision
kernel, while each node holds a local projected instance shaped by hardware,
adapter state, quantization, cache state, or deployment profile. We study when
such a system can certify a warranted global kernel transformation before raw
data, full model deltas, or complete local state can be synchronized. The
limiting quantity is not static connectivity or raw traffic. It is effective
independent transformation evidence: the Fisher-volume / effective-rank mass of
accepted, non-duplicated, task-relevant evidence that reaches the certificate
surface before the decision window closes.

Active belief diffusion is the constructive mechanism. Agents exchange coded
evidence, which carries audited local loss or statistic contributions, and
bounded demand summaries, which steer scarce contact opportunities toward
certificate-relevant evidence without ever counting as evidence. The striking
regime is path-free: no instantaneous graph contains an end-to-end route from
evidence holders to eventual committers, yet time-respecting evidence flow can
cross the threshold for a unique certified descendant. The paper's AI-facing
instance is a finite-dimensional decomposable convex objective: the global
kernel is the canonical objective or sufficient statistic, local model
instances are projections, and a guarded update is released only when the
certificate warrants the global transformation and projected local updates
remain coherent.

The theorem surface proves safety, merge soundness, demand
non-evidentiality, effective-independence limits, guarded compatibility,
objective merge, optimizer-certificate soundness, and guarded convex-decision
soundness over finite certificates. The replay artifacts instantiate the
capacity boundary: path-free evidence can support guarded commitment before
full recovery, raw spread can fail to raise effective rank, and certificate
evidence can be dramatically smaller than raw data, full deltas, checkpoints,
or passive state synchronization in thin regimes. Exact `k`-of-`n` recovery is
the threshold case; least-squares regression and hinge-loss classification are
the certified convex AI instances.

## 1. Introduction

Modern AI systems increasingly start from a common global object but operate
through heterogeneous local instances. A fleet, hospital network, sensor field,
or group of agents may share a model kernel, policy head, objective template, or
decision statistic, while each node holds a projected local instance shaped by
hardware, privacy boundary, quantization, cache state, adapter surface, or
deployment profile. The question is not whether every local instance can become
byte-identical. The question is when the system has enough evidence to certify a
single warranted global transformation and preserve the coherence of all honest
local projections.

This paper studies that question under temporal contact. Nodes see local data.
Links are intermittent. The decision window may close before raw data, full
gradients, checkpoints, or local state can be centralized. In the most striking
case, no instantaneous communication graph contains an end-to-end route from
evidence holders to eventual committers. Nevertheless, evidence can travel along
time-respecting journeys. Static connectivity is not the right boundary;
temporal evidence capacity is.

The core claim is that certified adaptation is governed by effective
independent evidence. Consensus can serialize or refuse transformations, but it
cannot manufacture evidence. A guarded transformation becomes warranted only
when enough accepted, non-duplicated, task-relevant evidence has reached the
certificate surface. That threshold is also the communication boundary: below
it, moving more copies or flooding more state need not help; above it, moving
the certificate-relevant evidence is enough for the supported task class.

Classical erasure coding assumes that coded pieces already exist and that the
main question is how many pieces a receiver obtains. Temporal decentralized
error correction has an additional bottleneck: independent pieces must be
created, carried, and preserved through space-time contact under byte, storage,
and observability constraints. Raw spread is therefore only a proxy. The
load-bearing quantity in this paper is effective independence: the amount of
audited contribution diversity that survives duplicate lineage, repeated
carrier overlap, and low-diversity contact.

This matters for AI systems that cannot rely on central observation. Examples
include swarms, edge sensing, disaster response, contested networks, rural
sensing, privacy-constrained deployments, intermittently connected autonomy,
and federated edge adaptation. The AI object here is a certified global kernel
transformation. In the main instance, the kernel is a finite-dimensional convex
objective or sufficient statistic. Nodes contribute bounded local loss or
statistic terms; local projected instances fold their projected certificate;
and a guarded update is allowed only when an optimizer certificate and a
margin/uncertainty guard make the transformation stable. The claim is not
general distributed learning, universal privacy, or arbitrary consensus.
Supported AI-central instances include least-squares regression and hinge-loss
linear classification; the probabilistic classifier is a finite
likelihood-vector special case of the same audited accumulation discipline.

The proposed primitive is active belief diffusion. Agents exchange two bounded,
replay-visible objects. The first is coded evidence: audited contributions to a
decomposable convex objective or its mergeable finite-statistic special case.
The second is a bounded demand summary:
replay-visible information about what evidence would most improve current
belief quality. The symmetry matters operationally because both objects diffuse
through the network. The asymmetry matters semantically because only coded
evidence can change the objective or sufficient statistic. Demand can
prioritize forwarding, retention, custody, and recoding. It cannot validate
evidence. It cannot create contribution identity or alter objective or merge
semantics.

The object of interest is a monotone audited kernel that can be transformed
before raw recovery or full synchronization. Exact recovery is one threshold
instance inside this broader task class. The paper focuses on deterministic
finite-dimensional convex objectives whose local loss contributions can be
deduplicated by contribution identity. Demand summaries are part of the
communication primitive, but they do not create evidence or change what counts
as evidence.

The paper makes four contributions:

- It states the central limit as an effective-independence bottleneck: raw
  copies, raw transmissions, and raw reproduction do not by themselves certify
  enough independent evidence for reconstruction, guarded inference, or a
  warranted kernel transformation.
- It defines active belief diffusion as a two-object primitive for temporal
  decentralized inference. Coded evidence carries audited objective or
  statistic contributions. Bounded demand summaries carry replay-visible
  summaries of what would most improve current belief quality and effective
  independence per byte.
- It identifies a decomposable convex task interface that cleanly separates
  global-kernel accumulation, projected local update, and guarded decision from
  batch reduction after delivery. This lets the same mechanism cover threshold
  reconstruction, additive anomaly localization, least-squares regression,
  hinge-loss linear classification, and finite likelihood-vector
  classification.
- It reframes communication cost as an information-geometric operating
  boundary. The relevant comparison is not only active versus passive bytes. It
  is certificate evidence versus raw-data movement, full update/delta movement,
  checkpoint synchronization, and passive state flooding.
- It presents a proof-scoped and replay-backed evaluation. The evaluation shows
  path-free collective inference in the supported regime. It includes a
  reduced finite-trace theorem for folded receiver state and guarded
  commitment, plus assumption-labeled rows where propagated demand improves
  effective-rank proxy, quality per byte, and commitment lead time. Demand
  remains first-class in communication while staying non-evidential.

The paper is organized as follows. Section 2 positions the primitive against
nearby networking, coding, and active-learning lines. Section 3 gives a
concrete running example. Sections 4 and 5 define the mechanism and its formal
boundary. Sections 6 and 7 give the experimental design and main results.
Section 8 points to supplementary validation material, and Section 9 states
the remaining limitations.

The paper does not claim arbitrary machine learning inference over
intermittent networks. It claims that for finite-dimensional decomposable
convex ERM / convex energy minimization with monotone audited evidence
accumulation, useful collective belief can emerge before full information
transit. The proof-backed performance claims apply to the assumption-labeled
sparse-bridge and clustered regimes; the semi-realistic mobility trace is an
empirical generalization probe. The core replay claim remains path-free: the
decision window has no stable path and no central aggregator.

### 1.1 Claim Boundary

The following claim-boundary table fixes the scope used by the rest of the
paper. The support level is part of the claim: theorem-backed rows are finite
certificate theorems, theorem-plus-replay rows combine those theorems with
deterministic artifact rows, and replay-only rows are empirical results for the
checked regimes.

| Claim | Support | Boundary |
| --- | --- | --- |
| Certified kernel transformation | theorem-backed for finite certificates | The proof-facing object is a global kernel with projected local instances. The paper does not require byte-identical local models. |
| Projection preservation | theorem-backed for finite certificates | Honest local instances must remain compatible projections of the certified global descendant. Arbitrary deployment mutation outside the projection relation is not claimed. |
| Independence bottleneck | theorem-backed for finite certificates | Recovery and commitment depend on effective independence, not raw copies, raw transmissions, or raw reproduction alone. This is not a universal temporal-network capacity theorem. |
| Path-free inference | theorem-plus-replay | The checked traces have no instantaneous static source-to-receiver path during the core window, while time-respecting evidence journeys exist. |
| Communication boundary shift | theorem-plus-replay | Certificate evidence can be smaller than raw data, full deltas, checkpoints, or passive state synchronization in thin regimes. Universal communication savings are not claimed. |
| Convex ERM task class | theorem-backed for finite certificates | Supported tasks decompose into audited local loss contributions over a finite-dimensional convex objective with checkable optimizer and guard certificates. Arbitrary nonconvex learning and neural-network training are not claimed. |
| Direct statistic decoding | theorem-backed | Decisions are read from audited mergeable statistics or certified convex objectives for supported tasks. Arbitrary ML compactness is not claimed. |
| Commitment before full recovery | theorem-plus-replay | Positive lead time is claimed only where commitment and full-recovery events are both logged or where the finite stable-basin certificate applies. Right-censored runs stay separate. |
| Active demand usefulness | theorem-plus-replay | Demand usefulness is validated for matched active/passive runs and explicit clean-model assumptions. Demand optimality under arbitrary mobility is not claimed. |
| Demand non-evidentiality | theorem-backed | Demand can affect priority, custody, forwarding, and recoding opportunities, but cannot validate evidence, create contribution identity, alter duplicate accounting, or change commitment guards. |
| Multi-receiver compatibility | theorem-plus-replay | Compatibility means guarded local decisions enter the same basin from receiver-indexed histories. It is not consensus, common knowledge, or identical belief. |
| Near-critical useful control | theorem-plus-replay | The controller reports raw and useful reproduction pressure separately. Raw reproduction near one is never treated as sufficient for inference. |
| Recoding and aggregation frontier | theorem-plus-replay | Recoding soundness is theorem-backed; frontier improvement is claimed only where replay rows improve margin, uncertainty, accuracy, latency, or quality per byte. |
| Equal-byte baseline advantage | replay-backed | Advantages are reported under fixed payload-byte budgets against the implemented uncoded, epidemic, spray-and-wait, uncontrolled coded, passive controlled coded, and active modes. |
| Bounded stress safety | theorem-plus-replay | Stress claims cover the named bounded stresses and false-confidence counters only. Arbitrary adaptive adversary robustness is not claimed. |
| Deterministic reproducibility | check-backed | Reported results are deterministic replay artifacts with typed time/order, canonical ordering, and integer or fixed-denominator metrics. |

## 2. Related Work And Positioning

The closest literature is not a single field. It is a stack of adjacent systems
and AI literatures. Federated inference and collaborative DNN inference at the
edge distribute model execution across devices or infrastructure [5, 9]. Active
belief diffusion instead distributes recoverable evidence through an
intermittent contact field.

Coded computation and coded inference use redundancy to tolerate stragglers or
unavailable workers. Parity-model prediction serving is a representative
example [6, 7]. Data-availability systems such as ZODA also show that sampled
coded symbols can serve both checking and reconstruction [4]. Active belief
diffusion uses the analogous systems principle for temporal-network inference.
Evidence movement, validity records, and receiver updates should improve
reconstruction or decision quality, not merely report telemetry.

Multi-agent reinforcement learning and active sensing study what agents should
communicate or observe [8, 12]. Belief propagation and active inference provide
useful vocabulary for local belief updates [3, 11]. These lines usually assume
that a communication graph or coordination substrate is available enough to
carry the messages. Here the contact field is part of the problem.

The positive distinction is the combination of four constraints that nearby
systems usually separate: mergeable statistic contributions, replay-visible
non-evidential demand, guarded commitment from the merged statistic, and
path-free temporal contact under an explicit byte budget. Gossip aggregation
and sketching give compact merges without a budgeted demand channel. Interest
networking and active querying give request-shaped control without making the
request provably non-evidential for a sufficient statistic. Delay-tolerant
forwarding gives temporal delivery without direct statistic decoding. Active
belief diffusion is the point where those constraints are enforced together.

The kernel/projection distinction is closest in spirit to projection theorems
in session-typed systems: a global object has local views, and correctness
requires local evolution to preserve the global invariant. Here the global
object is a model or decision kernel, not a choreography; local model instances
may differ operationally, but a certified global transformation must project to
compatible local updates. This is also where the paper separates itself from a
pure consensus framing. Agreement can prevent incompatible commits, but it does
not say whether enough task-relevant evidence exists to warrant the committed
kernel transformation.

### 2.1 Interest, Query, And Demand Signals

NDN and CCN use interest packets to pull named content through a network.
Those interests are request-shaped and non-content-bearing, so they are the
closest networking analogue to demand summaries. The difference here is not
that a request exists. The difference is that demand is typed as
non-evidential, replay-visible control for an audited objective or sufficient
statistic, and the comparison budget must count the control channel rather
than treating it as free.

Push-pull DTN systems also exchange request or custody signals under
intermittent contact. They are concerned primarily with making useful content
arrive. Active belief diffusion instead asks whether accepted contributions
change an audited objective or statistic enough for a guarded decision before
full raw recovery. Active-learning query strategies similarly choose informative
examples or labels, but they usually assume an available query substrate and do
not prove that the query itself cannot validate, mint, or double-count
evidence.

This leaves the paper with a narrow claim. Demand is not novel as a message
shape. The claim is that bounded demand, audited evidence, contribution
identity, direct statistic or objective decoding, and deterministic replay can
be tied into a single temporal-inference accounting surface. In that surface,
the communication question changes from "how do we synchronize the object?" to
"how much independent evidence is needed to certify this transformation of the
object?"

The privacy and traffic-analysis literatures are also relevant, but the role
here is inverted through the paper's error-correction lens. Statistical
disclosure attacks, Bayesian traffic analysis, and MANET anonymity work show
that communication metadata can reveal relationships [1, 2, 10]. This paper
uses observer projections to ask the complementary question: when does a
projection lack enough effectively independent evidence to reconstruct the
protected statistic? The claim is non-reconstructability under a stated
projection and horizon, not blanket privacy or post-revocation forgetting.

## 3. Running Example

Consider clustered anomaly localization with multiple receivers. Each node sees
only a local noisy signal about which cluster is anomalous. A receiver does not
need every raw observation. It needs enough innovative statistic contribution to
separate its top competing hypotheses and to satisfy a guarded commitment rule.

The mechanics are easiest to see over four rounds. In round 1, receiver `r1`
has score vector `(A=42, B=39, C=18)`, so its lead over the nearest rival is
small. Receiver `r2` has `(A=31, B=33, C=32)`, so it is effectively undecided.
In round 2, `r1` emits a bounded demand summary that says "evidence separating
`A` from `B` is high value"; `r2` emits one for `B` versus `C`. These summaries
do not assert that `A`, `B`, or `C` is true. They only rank which innovative
contributions would most reduce current uncertainty. In round 3, an
intermediate carrier holding two valid coded contributions chooses the one that
best matches the current demand summary under the byte budget and custody rule.
In round 4, `r1` accepts one innovative contribution, its score vector becomes
`(A=49, B=40, C=19)`, and the margin guard now holds. It can commit even though
no receiver has reconstructed the full raw observation set and no stable path
ever existed during the decision window.

This example is the paper's central case in local form. The global kernel is
the canonical score/objective surface. Each receiver holds a projected local
instance of that kernel, shaped by its temporal history. When a certificate
crosses the guard, the local update is valid only because it remains compatible
with the certified global kernel descendant. The same discipline reduces to
exact `k`-of-`n` recovery when the sufficient statistic is set union and the
decision rule is a threshold.

## 4. Primitive

Active belief diffusion is defined first over a global kernel with projected
local instances. The proof-facing kernel may be a decomposable convex objective
or a mergeable finite statistic. Each node `i` holds a local projection
`pi_i(K_t)` and receives a projected certificate `pi_i(C)`. A certified global
transformation has the form `K_{t+1} = U(K_t, C)`, and local correctness means
that applying the projected update gives a local instance compatible with
`pi_i(K_{t+1})`.

For the convex objective instance, each valid contribution identity selects a
bounded local loss term. A receiver accumulates those terms monotonically into
a deterministic finite-dimensional objective:

```text
domain:           bounded fixed-point decision domain X
local loss:       contribution i -> l_i(x)
regularizer:      R(x)
objective:        F_I(x) = R(x) + sum_{i in I} l_i(x)
solver cert:      x_hat, lower bound, epsilon gap, deterministic tie break
decision:         d(x_hat) -> y
guard:            margin > optimizer gap + evidence uncertainty
```

Contribution identity prevents double counting: adding a new valid identity
extends `F_I`, while replaying the same identity leaves the objective
unchanged. The monotone object is evidence accumulation into the objective, not
an assertion that every quality metric improves after every contribution.

Projection preservation is the local-instance counterpart:

```text
global kernel:      K_t
local projection:   K_t -> pi_i(K_t)
global certificate: C
local certificate:  pi_i(C)
global update:      U(K_t, C) = K_{t+1}
local update:       U_i(pi_i(K_t), pi_i(C)) ~= pi_i(K_{t+1})
```

The replayed convex/objective tasks instantiate this structure with a canonical
objective or statistic as the global kernel and receiver-indexed folded states
as local projections. The paper does not require byte-identical local model
instances.

Compact mergeable statistics are the finite-algebra special case of this
interface:

```text
local encode:      x_i -> a_i in A
merge:             A x A -> A
identity:          e in A
global statistic:  A* = merge_i a_i
decision:          d(A*) -> y
quality:           q(A*) -> margin / uncertainty / score
```

The merge must be associative and, unless the task intentionally depends on
order, commutative. Recoding or aggregation is valid only when it preserves
contribution identity and the declared objective or merge semantics. The
theorem-backed task class includes exact threshold set union, additive score
landscapes, bounded likelihood-vector classification, bounded least-squares
regression, and hinge-loss linear classification. Other sketches or embeddings
belong in scope only if they instantiate the same finite convex/objective
certificate interface.

The three evidence-origin modes are:

1. single-source reconstruction, where one source encodes a payload into
   independent fragments and any valid quorum of size `k` reconstructs;
2. distributed evidence inference, where many agents emit coded evidence about
   their own local statistic contribution;
3. in-network recoding and aggregation, where intermediate agents combine
   evidence while preserving validity and contribution identity.

The active extension adds local demand:

```text
belief_r(t):       local statistic and quality summary at receiver r
demand_r(t):       bounded summary of evidence that would reduce uncertainty
value_r(e):        deterministic estimated value of evidence e for receiver r
policy(u, v, e):   local forwarding or recoding score when u meets v
```

Demand may encode missing contribution classes, competing hypotheses that still
need separation, desired cluster coverage, or anti-duplicate diversity. The
active loop is:

```text
belief landscape -> bounded demand
bounded demand + coded evidence -> priority / recoding / custody
accepted coded evidence -> merge -> belief update
```

The controller maintains three invariants throughout this loop:

1. demand may affect priority, custody, recoding, and allocation only;
2. demand may not validate evidence, create contribution identity, or alter
   merge semantics;
3. only innovative valid evidence may change the folded sufficient statistic.

Algorithm 1 then spells out the deterministic round loop.

```text
Algorithm 1: Active Belief Diffusion
Input: temporal contacts, byte/storage/lifetime caps, convex objective or finite-statistic certificate
State: local objective/statistic A_r, accepted contribution identities I_r, bounded demand d_r

for each deterministic round t:
  observe local signal x_i, if any
  encode x_i as an audited contribution a_i with contribution identity
  update local belief summary from A_r
  derive bounded demand d_r from uncertainty, margin, and missing classes
  for each contact (u, v) in canonical order:
    enumerate candidate evidence under byte and custody caps
    score forwarding, retention, or recoding using demand and duplicate risk
    transmit selected bounded evidence and demand messages
  for each received evidence object in canonical order:
    validate evidence and parent contribution identities without demand
    if contribution identity is valid and innovative:
      merge contribution into A_r
      update L_r and belief summary
  if evidence guard and margin guard hold:
    emit guarded commitment
```

The symmetry matters operationally and the asymmetry matters semantically.
Evidence and demand are both bounded exchange objects. Only valid coded
evidence, however, can change the accumulated objective or merged sufficient
statistic.

## 5. Model And Formal Boundary

Time is discrete and finite horizon. A temporal contact trace gives directed or
undirected contact opportunities over time. A time-respecting journey is a
sequence of contacts whose times are nondecreasing. A trace can be path-free:
no instantaneous graph contains an end-to-end source-to-committer path during
the decision window, even though time-respecting journeys exist.

The formal object is a global kernel `K`, a family of local projections
`pi_i(K)`, and a monotone certificate `C`. A guarded transformation is released
as `U(K, C)`. Honest local instances may differ, but the projection-preservation
obligation is:

```text
K' = U(K, C)
local_i' = U_i(pi_i(K), pi_i(C))
local_i' ~= pi_i(K')
```

Split brain means accepting incompatible global descendants from the same
kernel, or allowing honest local projections to drift outside the accepted
global descendant's projection relation.

Each evidence fragment has a target id, origin mode, fragment id, rank or
contribution id, byte size, and validity marker. Recoded evidence also carries
optional parent ids. For exact reconstruction, a target has `k <= n`. The
receiver rank `D_t` is the number of valid independent contributions received
by time `t`.
Exact reconstruction occurs when `D_t >= k`. Duplicates do not increase `D_t`.

### 5.1 Threat Model And Contribution Identity

The duplicate and recoding claims assume sender-bound contribution identity.
Each contribution carries an origin node id, decision-window epoch, local
nonce, and signature or equivalent authenticated binding over the contribution
payload and parent contribution-identity references. The replay validator treats a
contribution id as valid only inside its epoch and only when the origin binding
matches the advertised sender.

The adversary model is bounded. A fraction `f` of nodes may be malicious and
may withhold evidence, replay stale evidence, emit duplicate copies, or send
misleading demand summaries. The model does not allow an unbounded Sybil
adversary to mint unlimited fresh identities, and it does not claim robustness
against arbitrary adaptive compromise. Under this assumption, forged
contribution identities are outside the validity predicate, while malicious but
properly signed contributions remain part of the modeled stress surface.

Demand summaries have the same replay-window discipline, but they are never
inputs to the evidence-validity predicate. A signed demand message can change
priority or custody decisions; it cannot make an unsigned contribution valid,
create a new contribution id, or bypass duplicate accounting.

For aggregate inference, each valid contribution is an element of a
deterministic merge algebra or a decomposable convex objective:

```text
a_i in A
A_t = merge of accepted innovative contributions through time t
decision_t = d(A_t)
quality_t  = q(A_t)

l_i(x)     local convex loss for accepted contribution i
F_I(x)     R(x) + sum_{i in I} l_i(x)
x_t        certified epsilon minimizer of F_I
decision_t = d(x_t)
```

Diffusion cost is tracked with deterministic integer replay records. The
controller records active innovative forwarding opportunities `Y_t`,
finite-horizon cost `C_T`, raw reproduction pressure, and useful reproduction
pressure. Primary comparisons fix payload-byte budget. Recoded fragments carry
parent contribution ids. The receiver counts canonical contribution ids rather
than copies.

The formal contribution is intentionally scoped. The Lean results split into
unconditional safety, algebra, accounting, and reduced finite-trace facts, plus
finite-horizon performance lemmas whose assumptions are explicit in Table 1.
The performance lemmas do not derive arrival floors, margin conditions,
controller band entry, or stress bounds from arbitrary traces. The proof-backed
performance rows apply to the synthetic sparse-bridge and clustered
duplicate-heavy regimes. The semi-realistic mobility trace is empirical support
only.

Theorem 1, receiver arrival. If a finite-horizon trace delivers enough
distinct valid contributions to reach the required rank, exact threshold
reconstruction succeeds. Assumption: the arrival-floor condition holds over the
finite horizon. Conclusion: duplicates never increase receiver rank, and the
required rank is sufficient for exact reconstruction. Lean attribution:
`receiver_arrival_reconstruction_bound`.

Theorem 2, useful inference arrival. If enough task-relevant contribution mass
arrives before the window closes, the receiver can reach a useful decision
before full raw recovery. Assumption: the useful-arrival condition holds for
the mergeable statistic. Conclusion: task-level inference can succeed from
partial audited accumulation rather than complete raw collection. Lean
attribution: `useful_inference_arrival_bound`.

Theorem 3, guarded anomaly commitment. A margin guard plus an evidence guard
bounds false commitment in the anomaly-localization task. Assumption: the
finite-horizon margin model and bounded-update conditions hold. Conclusion:
guarded commitment is sound inside the theorem-backed regimes. Lean
attribution: `anomaly_margin_lower_tail_bound`,
`guarded_commitment_false_probability_bounded`.

Theorem 4, non-evidential demand safety. Propagated demand may change
allocation, but it cannot validate invalid evidence or inflate rank through
duplicates. Assumption: evidence identity and validity are checked only through
the accepted contribution identities, and bounded demand rows expose a deterministic
variance-deflection cap. Conclusion: demand stays operationally active while
remaining semantically non-evidential. Lean attribution:
`propagated_demand_cannot_validate_invalid_evidence`,
`propagated_demand_duplicate_non_inflation`,
`demand_induced_allocation_variance_deflection_bounded`.

Theorem 5, progress accounting. The controller-side accounting surface records
useful progress against duplicate, storage, and transmission pressure.
Assumption: the measured controller row stays inside the stated band and budget
conditions. Conclusion: the operating-region plots have a proof-side accounting
companion rather than being a pure heuristic. Lean attribution:
`inference_potential_drift_progress`.

Theorem 6, reduced finite-trace soundness. The proof-facing receiver state is
the fold of replay-visible evidence events, and guarded commitment decodes from
that folded audited statistic when the reduced trace is valid. Assumption: the
reduced finite-trace predicate holds. Conclusion: the formal model matches the
paper's finite-trace commitment story without claiming that every simulator
execution satisfies that predicate. Lean attribution:
`active_belief_trace_soundness`.

Theorem 7, active-demand value model. Under an explicit value-order model,
active demand is non-worse than passive selection on equal-budget normalized
quality per byte. Assumption: passive selected value is no greater than active
demand value, and active demand value is covered by useful arrivals.
Conclusion: the demand claim has a proof-scoped improvement statement without
asserting optimality under arbitrary mobility. Lean attribution:
`active_demand_policy_improves_under_value_model`.

Theorem 8, bounded-Sybil graceful degradation. Under the signed-identity
ceiling, forged contribution identifiers are rejected and the remaining quality
degradation is bounded by the modeled malicious-identity fraction. Assumption:
the malicious identity count, forged-attempt count, rejection count, and quality
degradation bound are exported as replay certificate fields. Conclusion: the
paper claims a bounded adversarial ceiling, not arbitrary Byzantine privacy or
unbounded Sybil robustness. Lean attribution:
`bounded_sybil_graceful_degradation`.

Theorem 9, replay-validator adequacy. Exported replay-validator metadata is
enough to recover the proof-facing theorem profile and finite-trace inputs used
in Table 1. Assumption: the validator emits the stated metadata fields.
Conclusion: the proof-to-report handoff is checked explicitly, but this is not
a proof of complete simulator correctness. Lean attribution:
`trace_validator_adequacy`.

Theorem 10, direct statistic decoding. A receiver can decode a finite statistic
directly once enough task-effective independent evidence arrives, and guarded
commitment is sound only when the guard is task-effective for that statistic.
Assumption: the task-effective guard and finite independent-evidence floor hold,
and the quality map factors through the mergeable statistic for the task.
Conclusion: direct statistic recovery does not require raw object recovery, but
raw copies alone do not certify task-effective evidence. Lean attribution:
`generic_direct_statistic_decoding`,
`direct_statistic_commitment_requires_task_effective_guard`,
`monoid_homomorphism_preserves_decision_quality_under_partial_accumulation`.

Theorem 11, trace-class effective-independence bottleneck. Effective task independence is
bounded by both raw copies and raw transmissions, and recovery probability is
bounded by the effective independent evidence available at the receiver.
Assumption: the finite-horizon evidence-independence model is the proof-facing
one, or the trace family satisfies the explicit Path A trace-class certificate.
Conclusion: raw reproduction above one is insufficient to prove useful
reproduction above one, while certified trace classes close the theorem gap for
the regimes that satisfy their finite contact assumptions. Lean attribution:
`effective_task_independence_bounded_by_raw_copies`,
`effective_task_independence_bounded_by_raw_transmissions`,
`recovery_probability_bounded_by_effective_independence`,
`raw_reproduction_above_one_does_not_imply_effective_reproduction_above_one`,
`trace_class_temporal_contact_implies_independence_limit`.

Theorem 12, temporal error-correction capacity certificate. Contact entropy,
temporal dispersion, generator-rank proxies, and temporal contact capacity give
finite certificates for when a projection lacks enough independent arrivals to
reconstruct the protected statistic. Assumption: the replay projection,
horizon, and raw-activity counters are the proof-facing counters. Conclusion:
observer ambiguity is tied to finite capacity certificates rather than a broad
privacy or stochastic-capacity claim. Lean attribution:
`contact_entropy_and_dispersion_bounded_by_raw_activity`,
`effective_rank_bounded_by_temporal_generator_rank`,
`reconstruction_bound_from_entropy_and_dispersion`,
`temporal_contact_capacity_bounded_by_independent_arrivals`.

Theorem 13, reliability/resource/ambiguity boundary. For the finite contact
model, reliability, resource limits, and observer ambiguity cannot all be
maximized at once, and matched networks can separate by entropy and effective
rank even when raw activity is comparable. Assumption: the matched finite
certificate inputs are fixed. Conclusion: the observer and coding claims have a
finite boundary certificate rather than an unqualified general-capacity result.
Lean attribution:
`reliability_resource_ambiguity_triangle_incompatibility`,
`matched_networks_separate_by_entropy_and_effective_rank`.

Theorem 14, convex ERM task-class soundness. For finite-dimensional
decomposable convex objectives with accepted contribution identities, the
receiver objective is the deterministic sum of accepted local losses plus a
regularizer; duplicates do not change that objective; valid new evidence
monotonically extends it; certified optimizer gaps and margin guards justify
guarded decisions; and demand remains outside objective semantics. Assumption:
the task supplies finite fixed-point convexity, optimizer, and guard
certificates. Conclusion: least-squares regression and hinge-loss linear
classification sit inside the theorem-backed AI task class, while arbitrary
nonconvex neural training does not. Lean attribution:
`convex_duplicate_accept_preserves_objective`,
`convex_objective_monotone_accumulation`,
`convex_erm_objective_convex`, `optimizer_certificate_sound`,
`guarded_convex_decision_stable`,
`convex_effective_evidence_connected_to_temporal_limit`,
`convex_demand_does_not_change_objective`,
`bounded_least_squares_regression_instantiates_convex_erm`,
`hinge_loss_classifier_instantiates_convex_erm`,
`convex_replay_metadata_adequacy`.

Projection-preservation corollary, certified local instances. For the supported
finite convex/objective kernels, a valid global certificate induces compatible
receiver-indexed local folded states: local update from a projected certificate
is observationally equivalent to projecting the certified global kernel
descendant. Assumption: the projection map preserves contribution identity,
merge semantics, and guard fields. Conclusion: local histories may differ, but
honest projected instances remain coherent with the same certified kernel
descendant. This is stated as a corollary of the merge, trace-soundness, and
convex-objective theorems rather than as a claim about arbitrary model
deployment machinery.

Communication-boundary corollary, certificate bytes versus synchronization
bytes. For supported finite certificates, the bytes needed to warrant a
guarded transformation are the bytes needed to carry enough accepted evidence
to cross the certificate threshold, not the bytes needed to move raw data, full
model deltas, checkpoints, or complete local state. Assumption: the task admits
the stated certificate and projection interface. Conclusion: communication
savings are regime-dependent and strongest in thin, high-redundancy,
path-free, or sparse-evidence regimes; universal savings are not claimed.

Table 1 summarizes that boundary.

{{EXHIBIT:table_01_theorem_assumptions}}

## 6. Experimental Design

The central experiment is multi-receiver anomaly localization over clustered
temporal contacts. The traces contain intermittent bridge contacts and no
instantaneous static source-to-receiver path during the core window. Successful
runs require time-respecting evidence journeys. Nodes produce local noisy
observations. Receivers maintain integer score landscapes. Demand summaries are
derived from uncertainty, competitor margins, and missing contribution classes.

| Item | Implemented setting |
| --- | --- |
| Seeds | 100 deterministic seeds, `41..140` |
| Regimes | sparse bridge, clustered duplicate-heavy, semi-realistic mobility-contact |
| Proof-backed regimes | sparse bridge and clustered duplicate-heavy |
| Empirical-only regime | semi-realistic mobility-contact |
| Scale | 256-node sparse bridge, 512-node clustered, 1000-node mobility-contact |
| Receivers per run | three receiver identities in flagship rows; receiver-count sweep covers 3, 10, 25, and 50 identities |
| Tasks | anomaly localization, bounded least-squares regression certificate, hinge-loss classifier certificate, Bayesian classifier, majority threshold, bounded histogram, set-union threshold |
| Compared modes | uncoded replication, passive controlled coded, full active belief, recoded aggregate |
| Headline budget | 4096 payload bytes |
| Threshold sweep | `k/n` reconstruction settings swept through `16/32`, `32/64`, and `64/128` in addition to smaller coded-fragment choices |
| Demand accounting | demand summaries are 48-byte bounded records; demand-byte counts and total-byte counts are logged alongside payload bytes |
| Control knobs | reproduction band, forwarding budget, and controller-value terms |
| Commitment guard | evidence guard plus margin guard |
| Stress variants | normal, duplicate pressure, mobility, malicious duplicate pressure, delayed demand |

Table 2. Experimental protocol surface. This table fixes the experimental contract for the paper: regimes, tasks, compared modes, byte budget, control surface, and stress variants.

The paper uses three primary metrics:

1. commitment lead time, the number of rounds by which a guarded commitment
   precedes full recovery;
2. quality per byte under a fixed payload budget;
3. false-commitment rate under modeled stress.

Secondary metrics include receiver agreement, collective uncertainty, duplicate
pressure, bytes at commitment, and reproduction pressure. The main budgeted
comparison fixes payload transmission to 4096 bytes.

The communication-boundary metric is bytes to certificate. It is compared
against four conceptual synchronization targets: moving the raw data needed for
central reduction, moving a full model update or delta, moving checkpoints, and
passively flooding local state until receivers converge. The current artifact
logs payload bytes, demand bytes, total bytes, and bytes at commitment; those
rows expose the certificate-side boundary. The stronger systems implication is
regime-dependent: when the certificate surface is low-dimensional relative to
the raw object or model state, the required communication can collapse to the
effective independent evidence needed for the certificate.

Effective independence is measured operationally as an effective-rank proxy:
the replay counts innovative contribution identities, discounts repeated
lineage and low-diversity carrier overlap, and records the remaining task-useful
rank available to the receiver. It is intentionally distinct from raw
transmissions, raw fragment count, and raw copy count. A raw reproduction
estimate above one can still fail if the new copies circulate through the same
lineage or contact bottleneck; the receiver needs independently useful
contributions, not just more copies.

The suite compares single-source `k`-of-`n` reconstruction, distributed anomaly
localization from mergeable local score contributions, and in-network recoding
or aggregation. The anomaly-localization suite includes a no-central-encoder
setting. No node owns the global input. Each node emits only local evidence.
The receiver is evaluated against a later oracle reducer that sees the full
observation set only after the fact.

Core baselines are uncoded replication, epidemic forwarding, spray-and-wait,
contact-frequency opportunism, passive controlled coded diffusion, and active
belief diffusion with propagated demand. The key active ablations remove demand
propagation, duplicate-risk scoring, bridge-value scoring, landscape-value
scoring, or reproduction control while preserving byte accounting.

Exhibit roadmap. Table 1 marks the proof boundary, and Table 2 fixes the
protocol surface. Figure 1 checks path-free recovery, Figure 2 shows belief
landscapes over time, Table 4 separates the three evidence-origin modes,
Figure 3 and Table 5 show the finite-statistic task surface, Table 6 reports headline
paired summaries, Figures 4 and 5 give the multi-receiver and demand-ablation
results, Figures 6 and 7 compare coding and recoding under fair cost, Figures
8 and 9 show control and stress boundaries, and Table 7 audits demand safety.
Figure 10 and the supplementary exhibits are fairness and reproducibility
checks rather than central claims. Figure 13 records the demand-byte-budget
sweep used to audit the control-channel cost. Figure 14 records the high-gap
receiver-demand heterogeneity sweep. Figure 15 records the adversarial-demand
steering stress. Figure 16 records the byzantine-fragment injection stress.
Figure 17 records the receiver-count compatibility sweep.
Table 8 and Figure 18 record the matched raw-spread independence bottleneck
check. Table 9 records the convex ERM certificate surface for the AI-central
task-class boundary.

Trace validation and large-regime replay hygiene are supporting checks rather
than headline claims. They establish that the evaluation is deterministic,
canonicalized, and reproducible before the substantive comparisons are read. A
supplementary trace-validation table records those rows explicitly in the
Supplementary Figures and Tables section at the end of this report PDF.

## 7. Results

The empirical story has six central claims. First, useful belief formation can
occur in windows with no static end-to-end path. Second, the mechanism is not
limited to threshold delivery. It operates on a larger convex/objective-task
surface, with compact mergeable statistics as special cases. Third, the
communication boundary is the certificate threshold, not full synchronization
of raw data, model deltas, checkpoints, or local state. Fourth, propagated
demand improves collective inference quality per byte on the same traces
replayed across active and passive variants. Fifth, matched raw-spread traces
can differ sharply in effective independence and outcome quality. Sixth, the
gains persist under fair-cost comparisons and remain visible under explicit
stress and baseline checks.

### 7.1 Belief Landscapes Sharpen In Path-Free Windows

In the path-validation traces, every recorded run has no instantaneous static
path in the core window and does have a time-respecting evidence journey. Under
that condition, median path-free success is 88.5% for active belief diffusion,
80.5% for passive controlled coded diffusion, 90.8% for recoded aggregation,
and 58.5% for uncoded replication. Figure 1 shows useful inference progressing
through temporal contact history rather than through a hidden stable path.

{{EXHIBIT:figure_01_path_free_recovery}}

Figure 2 shows what that progress looks like at a projected local instance. As
innovative evidence arrives, quality rises while uncertainty falls in the
expected direction. The relevant phenomenon is the formation of a usable local
belief landscape that remains compatible with a certified global kernel
descendant before the receiver has the full raw information object.

{{EXHIBIT:figure_02_landscape_focus}}

### 7.2 The Mechanism Is Larger Than Threshold Delivery

Table 4 separates the three evidence-origin modes at the task-object level.
The threshold case remains important because it is the cleanest sanity check,
but it is not the distinctive case. The distinctive claim appears in the
distributed-evidence and recoded-aggregation modes. There, fragments carry
statistic or objective contributions rather than opaque bytes for later
centralized reduction, and the local instance commits from the merged
certificate surface.

{{EXHIBIT:table_03_three_mode_comparison}}

Figure 3 shows the task-family outcome pattern. Quality-per-byte ordering stays
stable across anomaly, Bayesian classifier, majority, histogram, and set-union
tasks, and bytes at commitment remain mode-specific but task-stable. The
contribution is not that these tasks require separate mechanisms. It is that
one audited accumulation interface supports the same early-commitment
discipline across several qualitatively different finite-statistic tasks.

{{EXHIBIT:figure_03_task_algebra}}

Table 5 then states that shared finite-statistic interface directly. Each task
admits a compact local contribution, a merge rule over a sufficient statistic,
and a guarded commit rule that reads directly from that merged statistic. The
Bayesian classifier row is the learned probabilistic finite-statistic case:
local evidence is a bounded per-class log-likelihood vector, merging is vector
addition, and the decision reads the posterior arg-max and margin guard from
the accumulated statistic. Table 9 extends this task-class boundary to
AI-central convex objectives with explicit optimizer certificates.

{{EXHIBIT:table_04_task_family_interface}}

### 7.3 The Communication Boundary Is Certificate Evidence

Under a fixed 4096-byte budget, active belief diffusion reaches 88.7%
quality per byte in the multi-receiver anomaly-localization
setting. It also reaches 88.8% receiver agreement, 10.9% collective
uncertainty, commitment lead time 3 rounds, and 1934 bytes at commitment. The
corresponding passive controlled coded medians are 80.7%, 86.2%, 16.1%, 1
round, and 2074 bytes. Uncoded replication reaches 58.7%, 78.8%, 38.1%, 1
round, and 2508 bytes. These rows are best read as a certificate boundary:
commitment happens when enough independent evidence reaches the guard, not
when the system has synchronized all raw observations or all receiver state.

The active-versus-passive comparison is still informative. Active demand
improves quality per byte by about 10%, adds two rounds of commitment lead
time, and lowers median uncertainty by about 32% against passive controlled
coding. After adding demand bytes to the denominator, the total-cost row is
much narrower: active is 80.9% versus 80.5% for passive, with a paired
bootstrap interval that crosses zero. That is the honest cost-accounted
boundary for the implemented active/passive replay surface. The broader
systems claim is not that every active row beats every passive row by a large
margin. It is that supported tasks can stop at a certificate threshold that is
often far smaller than raw data movement, full model-delta movement,
checkpoint exchange, or passive state synchronization.

Table 6 reports the paired deterministic summaries behind these headline
claims. The unit of pairing is seed, regime, receiver, and task for receiver
runs. It is seed, regime, and task for demand-ablation rows. The table uses
paired median differences, paired-difference IQRs, and paired-bootstrap 95%
confidence intervals rather than p-values. It also includes demand-byte and
total-byte rows so the control channel is costed.

{{EXHIBIT:table_05_headline_statistics}}

Figure 4 compresses the multi-receiver story into the four outcome surfaces
that matter most: receiver agreement, belief divergence, quality per byte, and
commitment lead time. Across clustered, mobility, and sparse-bridge regimes,
active belief remains in the high-agreement, low-divergence region while also
retaining the best or near-best quality per byte and about two extra rounds of
lead time over passive controlled coding and uncoded replication. That is the
paper's clearest evidence that projected local instances can remain compatible
while the system moves certificate-relevant evidence rather than merely moving
more bytes.

{{EXHIBIT:figure_04_active_belief_grid}}

The matched ablation supports the same conclusion within the deterministic
replay package, and the paired-delta view now makes the effect easier to read.
Propagated demand reaches 62.5% quality per byte with 32.5%
uncertainty, 16 innovative arrivals, duplicate count 10, and 2141 bytes at
commitment. No-demand drops to 51.8% quality per byte, 43.0% uncertainty, 10
innovative arrivals, duplicate count 13, and 2566 bytes. Under stale demand,
quality per byte is 53.6%, uncertainty is 41.3%, and bytes at commitment are
2497.
Across the same seed, regime, and task groups, propagated demand gains 10.8
percentage points of quality per byte over no-demand with a
paired-difference IQR of 9.8 to 11.3 percentage points, and it saves 426 bytes
at commitment with a paired-difference IQR of 393 to 459 bytes. The
improvement comes from current propagated uncertainty summaries
changing allocation toward useful evidence. It does not come from any change in
evidence semantics. The formal theorem stack now covers three parts of that
claim: demand remains non-evidential, active demand is non-worse under an
explicit value-order model, and reduced replay traces fold to the audited
receiver statistic used by guarded commitment. The replay ablation supplies the
measured policy comparison. A supplementary heterogeneity sweep reports the
same comparison as receiver demand becomes less aligned; in that family the
active-vs-passive gap grows with demand asymmetry rather than staying a single
point estimate.

{{EXHIBIT:figure_05_active_vs_passive}}

The information-geometric interpretation is that the feasible operating region
moves down and left. Down means fewer bytes are needed to certify a supported
decision than to synchronize the larger object. Left means weaker instantaneous
connectivity can still suffice when time-respecting evidence journeys deliver
enough independent certificate mass. The boundary is not traffic volume alone;
it is the joint surface of byte budget, contact sparsity, effective rank,
adversarial duplicate pressure, and guard margin.

### 7.4 Coding And Recoding Beat Replication Under Fair Cost Accounting

At the 4096-byte comparison point, active coded diffusion reaches median
quality 91.0% with duplicate count 7. Passive controlled coded reaches 82.2%
with duplicate count 11. Uncoded replication reaches 58.9% with duplicate
count 25. Coded diffusion is better both in decision quality and in
duplicate pressure under the same payload budget.

Figure 6 shows the same result with interquartile spread bands over the budget
axis and direct labels at the 4096-byte comparison point, so the coding
advantage reads as measured variation rather than as a single schematic curve.

{{EXHIBIT:figure_06_coding_vs_replication}}

Recoding modestly improves the tradeoff further. In the receiver-run summaries,
recoded aggregation reaches 91.0% quality per byte, 89.6% receiver
agreement, 10.9% collective uncertainty, commitment lead time 3 rounds, and
1988 bytes at commitment. That strictly dominates passive controlled coding.
Against active belief, the gain is narrower: recoding buys about 2.3
percentage points of quality per byte at about 54 extra bytes at
commitment, with the same 3-round lead time. It still respects the same
contribution-identity discipline. Figure 7 now shows that regime-wise tradeoff
directly, with passive coded demoted to a dominated reference and with the
active-versus-recoded deltas annotated in each regime.

{{EXHIBIT:figure_07_recoding_tradeoff}}

### 7.5 Raw Spread Is Not Effective Independence

The decisive bottleneck is not whether many transmissions happened. It is
whether the certificate surface obtained enough independent useful evidence.
Table 8
separates the four quantities that are easy to conflate: raw transmissions, raw
fragment count, innovative contributions, and the effective-rank proxy. The
matched high-correlation and high-independence rows keep payload budget and raw
spread comparable, but the high-independence rows produce a materially larger
effective-rank proxy and better quality and recovery.

{{EXHIBIT:table_07_independence_bottleneck}}

Figure 18 shows the same point visually. The high-correlation rows can have
nearly the same raw transmission count as the high-independence rows, but they
sit lower on effective rank, quality, and recovery. Active demand matters
because it steers scarce contacts toward independently useful contributions
under the same budget, not because it merely increases traffic.

{{EXHIBIT:figure_18_independence_bottleneck}}

Table 9 records the convex ERM certificate surface. The key rows are not
performance claims by themselves; they show that the AI-facing tasks expose the
finite certificate fields used by the theorem surface: accepted objective
terms, effective independent loss terms, solver gap, uncertainty bound,
decision margin, and guard status. This is the task-class bridge from compact
mergeable statistics to decomposable convex energy minimization.

{{EXHIBIT:table_08_convex_erm}}

### 7.6 Control And Robustness Boundaries Remain Visible

The coded mechanism is only useful if diffusion pressure stays bounded. Figure 8
makes the operating region explicit. The highlighted near-critical runs are the
ones that enter the target useful-reproduction band and obtain the best quality
gains without paying the duplicate costs seen in the supercritical runs. Raw
reproduction pressure is tracked separately because raw supercritical spread is
not proof of useful independent evidence. The controller exposes a visible
operating region rather than hiding cost inside unbounded diffusion. The
theorem-backed controller statement is conditional on the achieved useful band
and hard budget caps; it does not prove arbitrary controller convergence.

{{EXHIBIT:figure_08_phase_diagram}}

Figure 9 then gives the stress boundary. Median commitment accuracy is 95.5%
at severity 1, 88.0% at severity 2, and 80.5% at severity 3. Median
false-commitment rate rises from 1.45% to 2.25% to 3.05%. At severities 4 and
5, false commitment reaches 3.85% and 4.65%. The quality gains also flatten.
This is a useful robustness boundary. The method remains effective through
moderate modeled stress in the replayed stress surface. The degradation point
is explicit rather than hidden, and the claim is not arbitrary-adversary
robustness.

{{EXHIBIT:figure_09_robustness_boundary}}

The supplementary adversarial-demand-steering stress separates policy damage
from evidence damage. Biased demand summaries can reduce honest receiver
quality by steering scarce contact opportunities toward less useful evidence,
but they still do not change validity, contribution identity, or duplicate-rank
accounting. A second supplementary stress injects forged contribution
identifiers across malicious-node fractions. Forged ids are rejected by the
signed-identity predicate; degradation from properly signed malicious
contributions remains visible as a bounded stress outcome.

### 7.7 Demand Is First-Class In Communication But Not Evidential

The safety claim is architectural rather than purely statistical. Demand
summaries are replay-visible protocol objects that influence forwarding,
retention, and recoding decisions. They do not validate evidence, create
contribution identity, change merge semantics, publish route truth, or inflate
duplicate rank. Table 7 records both facts directly. Active variants carry
replay-visible demand summaries, their demand-byte counts are explicit, and all
forbidden evidential side effects stay at zero.

{{EXHIBIT:table_06_host_bridge_demand}}

This separation matters to the paper's AI framing. The system exchanges bounded
summaries of both learned information and remaining uncertainty. Only coded
evidence can change the sufficient statistic or convex objective, and only a
valid certificate can warrant the corresponding kernel transformation.

### 7.8 Supporting Fairness Checks

The strong-baseline comparison is a fairness check, not the conceptual center
of the paper. Its job is to show that the reported gains are not an artifact of
comparing only against obviously weak opportunistic policies. The paired-delta
view makes that easier to read: under the same byte budget, active belief stays
ahead of passive controlled coded diffusion, contact-frequency opportunism,
epidemic forwarding, spray-and-wait, random forwarding, and uncoded
replication.

{{EXHIBIT:figure_10_strong_baselines}}

Large-regime replay validation and observer non-reconstructability remain
supporting material in the Supplementary Figures and Tables section at the end
of this report PDF. They are useful for reproducibility and for connecting the
finite temporal error-correction certificates to observer projections: contact
entropy, dispersion, the generator-rank proxy, and temporal contact capacity
explain why an observer projection may lack enough independently useful
evidence to infer the protected statistic. They are not required to establish
the main path-free inference, active-demand, projection-compatibility,
communication-boundary, or fair-cost coding claims in the paper body.

## 8. Supplementary Materials

Table 3, Figure 11, Figure 12, Figure 13, Figure 14, Figure 15, Figure 16,
and Figure 17 are supplementary rather than
central. In this report build they appear in the Supplementary Figures and
Tables section after the main text. Table 3 records deterministic
trace-validation rows. Figure 11 records large-regime replay hygiene. Figure
12 records the observer non-reconstructability surface. Figure 13 records the
demand-byte-budget sweep at fixed payload budget. Figure 14 records the
high-gap receiver-demand heterogeneity sweep. Figure 15 records the
adversarial-demand-steering stress. Figure 16 records the byzantine-fragment
injection stress. Figure 17 records the compatibility sweep from 3 to 50
receiver identities. They support
reproducibility, scope boundary, and cost-accounting claims, not the main
path-free inference result.

## 9. Limitations

The paper covers finite-dimensional decomposable convex ERM / convex energy
minimization with monotone audited evidence accumulation, plus threshold and
compact mergeable-statistic special cases. It does not cover arbitrary machine
learning inference, nonconvex neural training, or open-ended generative
modeling. Safety, algebra, accounting, optimizer-certificate, guarded-decision,
and reduced finite-trace claims are deterministic theorems over the supported
proof-facing mechanism boundary. Performance claims that depend on arrival,
margin, controller-band, or stress assumptions are theorem-backed only in the
sparse-bridge and clustered duplicate-heavy regimes and stay empirical-only in
the semi-realistic mobility regime. The replay-validator theorem validates
narrow metadata handoff into the proof-facing trace surface; it is not a proof
of arbitrary simulator correctness. Observer non-reconstructability is bounded
by the stated projection, horizon, and evidence-independence model; it is not a
blanket privacy or deletion claim. The temporal-capacity and limit-triangle
results are finite certificate statements, not stochastic capacity theorems for
arbitrary temporal contact processes. The opportunistic baseline set is strong
enough to be informative, though not a complete survey of delay-tolerant
networking. The communication-collapse claim is also scoped: certificate bytes
can be dramatically smaller than synchronization bytes only when the task has a
low-dimensional audited certificate relative to the raw object or local model
state, and when the projection relation and guard are explicitly defined.

## 10. Conclusion

Temporal decentralized AI is limited by effective independent certificate
evidence, not raw spread. In networks with no stable path in the decision
window and no central aggregator, agents can still certify a single global
kernel descendant when contact generates enough independently useful objective
or statistic contributions. Active belief diffusion is the constructive
mechanism: coded evidence monotonically extends an audited convex objective or
mergeable sufficient statistic, while bounded demand summaries steer scarce
contact opportunities without becoming evidence themselves.

The resulting boundary is information-geometric. For finite-dimensional
decomposable convex ERM / convex energy minimization, with threshold
reconstruction and compact mergeable statistics as special cases, the system can
operate at the certificate surface instead of at the synchronization surface.
That means path-free guarded commitment, higher effective-rank proxy, compatible
projected local instances, and communication proportional to the evidence
needed to certify the transformation rather than to the full raw data, model
delta, checkpoint, or state object. The result does not solve arbitrary
learning, nonconvex neural training, arbitrary temporal capacity, blanket
privacy, deletion, or post-revocation secrecy; it gives a finite,
replay-backed independence limit and a mechanism that can operate inside it.

## References

1. Huseyin Can. "Anonymous Communications in Mobile Ad Hoc Networks." Master's Thesis IMM-Thesis-2006-91, Technical University of Denmark (DTU), 2006. https://www2.imm.dtu.dk/pubdb/edoc/imm4876.pdf
2. George Danezis. "Statistical Disclosure Attacks: Traffic Confirmation in Open Environments." IFIP TC11 International Conference on Information Security (SEC), 2003. https://www0.cs.ucl.ac.uk/staff/G.Danezis/papers/StatDisclosure.pdf
3. Stefano Ermon, Carla P. Gomes, Bart Selman. "Collaborative Multiagent Gaussian Inference in a Dynamic Environment Using Belief Propagation." International Conference on Autonomous Agents and Multi-Agent Systems (AAMAS), 2010. https://cs.stanford.edu/~ermon/papers/aamas2010_final.pdf
4. Alex Evans, Nicolas Mohnblatt, Guillermo Angeris. "ZODA: Zero-Overhead Data Availability." IACR Cryptology ePrint Archive 2025/034, December 2024. https://eprint.iacr.org/2025/034
5. Boyu Fan, Xiang Su, Sasu Tarkoma, Pan Hui. "Federated Inference: Towards Collaborative and Privacy-Preserving Inference over Edge Devices." Proceedings of the ACM SIGCOMM 2025 Posters and Demos, 2025. https://dl.acm.org/doi/10.1145/3744969.3748418
6. Divyansh Jhunjhunwala, Neharika Jali, Gauri Joshi, Shiqiang Wang. "Erasure Coded Neural Network Inference via Fisher Averaging." IEEE International Symposium on Information Theory (ISIT), 2024. https://arxiv.org/abs/2409.01420
7. Jack Kosaian, K. V. Rashmi, Shivaram Venkataraman. "Parity Models: Erasure-Coded Resilience for Prediction Serving Systems." ACM Symposium on Operating Systems Principles (SOSP), 2019. https://www.cs.cmu.edu/~rvinayak/papers/sosp2019parity-models.pdf
8. Ben McClusky. "Dynamic Graph Communication for Decentralised Multi-Agent Reinforcement Learning." arXiv:2501.00165, 2025. https://arxiv.org/abs/2501.00165
9. Weiqing Ren, Yuben Qu, Chao Dong, Yuqian Jing, Hao Sun, Qihui Wu, Song Guo. "A Survey on Collaborative DNN Inference for Edge Intelligence." Machine Intelligence Research, vol. 20, pp. 370-395, 2023. https://arxiv.org/abs/2207.07812
10. Carmela Troncoso, George Danezis. "The Bayesian Traffic Analysis of Mix Networks." ACM Conference on Computer and Communications Security (CCS), 2009. https://carmelatroncoso.com/papers/Troncoso-ccs09.pdf
11. Thijs W. van de Laar, Bert de Vries. "Simulating Active Inference Processes by Message Passing." Frontiers in Robotics and AI, 2019. https://doi.org/10.3389/frobt.2019.00020
12. Changxi Zhu, Mehdi Dastani, Shihan Wang. "A Survey of Multi-Agent Deep Reinforcement Learning with Communication." Autonomous Agents and Multi-Agent Systems, vol. 38, no. 1, 2024. https://link.springer.com/article/10.1007/s10458-023-09633-6
