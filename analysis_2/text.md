# Active Belief Diffusion With Coded Evidence

This file is the manuscript-facing paper text for the active-belief report. It
is extracted from `work/research_proposal.md` into `analysis_2` so the paper
package can stand apart from Jacquard routing documentation.

## Abstract

Active belief diffusion is a resource-bounded diffusion-coded inference
primitive for decentralized computation in temporal networks. Agents represent
payloads, local observations, or sufficient statistics as coded evidence.
Evidence fragments diffuse opportunistically under explicit byte, lifetime, and
storage caps. Receivers maintain local belief landscapes and exchange two
first-class bounded objects:

- mergeable coded evidence describing what an agent has learned,
- demand summaries describing what would most reduce uncertainty.

The objects are symmetric in communication status and asymmetric in semantics.
Both are bounded, replay-visible messages. Only coded evidence can validate a
contribution, create contribution identity, merge into the sufficient statistic,
or change the belief landscape. Demand can shape priority, custody, recoding,
and allocation, but it remains non-evidential.

The clean task class is mergeable inference: each local observation maps to a
deterministic sufficient statistic, valid statistics merge under an auditable
algebra, and the merged statistic directly yields a decision object, margin, or
uncertainty summary. Exact k-of-n reconstruction is the threshold case; additive
score-vector inference is the first AI-facing case.

The core claim is that useful inference can be performed by controlled diffusion
and reconstruction of mergeable coded evidence, even when no instantaneous
static end-to-end path exists during the core window. In the inference setting,
the full underlying information object need not transit. Enough independent
task-relevant sufficient statistics must reach or be recoded toward a receiver
to reduce uncertainty or cross a commitment threshold.

The active version strengthens the claim: multiple receivers can form compatible
local beliefs from different temporal evidence histories by exchanging bounded
evidence and bounded demand. Communication policy becomes part of inference:
demand and evidence co-diffuse, demand shapes priority and recoding
opportunities, and accepted evidence updates the belief statistic.

## Claim Boundary

The completed research claims active belief diffusion only for compact
mergeable inference tasks under explicit finite-horizon contact, resource, and
validity assumptions. Receiver-arrival reconstruction, useful-inference
arrival, anomaly-margin, guarded false-commitment, demand-soundness, and
potential-drift claims are theorem-backed only under the assumptions named in
the Lean theorem table and theorem-assumption artifacts.

The empirical claim is replay-based: active demand improves allocation and
belief formation for implemented mergeable tasks under fixed payload-byte
budgets and deterministic temporal traces. The work does not claim arbitrary ML
inference, consensus, common knowledge, formal privacy, optimal active policy,
production-network robustness, or robustness against arbitrary adaptive
adversaries. Observer ambiguity is included only as a measured proxy frontier.

## Problem

Many AI and distributed-sensing systems assume that data can eventually be
centralized, synchronized, or routed over stable end-to-end paths. Those
assumptions fail in edge, swarm, disaster, battlefield, rural, and
privacy-sensitive settings. Contacts are intermittent, nodes have partial
observations, links are capacity constrained, and centralized aggregation may be
unavailable or undesirable.

Traditional routing asks how to deliver messages through such networks. This
paper asks whether the network itself can participate in inference by
co-diffusing coded evidence and bounded demand, so agents exchange both what
they have learned and what they need to learn next.

The AI-facing problem is belief formation under intermittent contact. A useful
system should improve decision quality per byte and support task-level
commitment before the full observation set, raw payload, or global state has
arrived at one place. The stronger problem is collective belief formation:
many agents should reach compatible guarded decisions from different partial
histories without consensus, a stable server, or a synchronized observation set.

## Why This Is Interesting

The research contribution combines decentralized inference under partial
observability, erasure-coded or network-coded evidence accumulation, mergeable
sufficient statistics, distributed evidence generation without a central
encoder, active demand derived from local uncertainty, multi-receiver belief
compatibility without consensus, auditable in-network recoding, adaptive
diffusion pressure, temporal connectivity, deterministic replay, and
theorem-backed coordination where protocol choreography is needed.

The work should be evaluated as a decentralized-inference primitive, not as
another mesh-routing protocol.

## Primitive

Active belief diffusion is defined over a mergeable task interface, three
evidence-origin modes, and a symmetric exchange interface for evidence and
demand.

```text
local encode:      x_i -> a_i in A
merge:             A x A -> A
identity:          e in A
global statistic:  A* = merge_i a_i
decision:          d(A*) -> y
quality:           q(A*) -> margin / uncertainty / score
```

The merge must be associative, and it should be commutative unless the task has
intentional order. Contribution identity prevents double counting.
Recoding/aggregation is valid only when it preserves the contribution ledger
and merge semantics. This class includes counts, votes, histograms, heavy
hitters, sketches, additive score vectors, bounded log-likelihood
accumulators, linear-model scores, random-feature embeddings, set union, and
lattice-valued summaries.

The three evidence-origin modes are:

1. Single-source reconstruction, where a source encodes a payload into
   independent fragments and any valid quorum of size `k` reconstructs.
2. Distributed evidence inference, where many agents hold local observations
   and emit coded evidence about their own mergeable statistic contribution.
3. In-network recoding and aggregation, where intermediate agents combine or
   recode evidence while preserving validity and contribution identity.

The active exchange interface is narrow:

```text
belief_r(t):       local statistic and quality summary at receiver r
demand_r(t):       bounded summary of evidence that would reduce uncertainty
value_r(e):        deterministic estimated value of evidence e for receiver r
policy(u, v, e):   local forwarding or recoding score when u meets v
```

Demand may encode missing contribution classes, competing hypotheses needing
separation, desired cluster coverage, or anti-duplicate diversity. The active
loop is:

```text
belief landscape -> bounded demand
bounded demand + coded evidence -> priority / recoding / custody
accepted coded evidence -> merge -> belief update
```

This is the AI-facing mechanism: the population exchanges compact summaries of
both learned information and remaining uncertainty, while preserving the rule
that only audited evidence contributions change the statistic.

## Research Questions

The paper studies when coded diffusion reconstructs under temporal contact
rather than static path connectivity; whether partial independent evidence can
sharpen a hypothesis landscape and support commitment before full recovery;
which mergeable sufficient-statistic tasks allow direct statistic decoding; how
large the commitment lead time is; whether multiple receivers can reach
compatible guarded decisions without consensus; whether first-class bounded
demand improves uncertainty, agreement, lead time, and quality per byte; and
when demand-driven forwarding can remain soundly separated from evidence
validity.

The negative research questions are equally important: when does the primitive
stop helping because a task is not compactly mergeable, contribution identity
is missing, observer ambiguity costs too much latency, or the contact/resource
assumptions no longer match the theorem-backed boundary?

## Theoretical Model

Time is discrete:

```text
t = 0, 1, 2, ...
```

There is a finite set of agents `V`. A temporal contact trace gives directed or
undirected contact opportunities over time. A time-respecting journey is a
sequence of contacts whose times are nondecreasing.

Each evidence fragment has a target id, origin mode, fragment id, rank or
contribution id, byte size, validity marker, and optional parent ids for
recoded evidence. For exact reconstruction, a target has `k <= n` and receiver
rank `D_t`, the number of valid independent contributions received by time `t`.
Exact reconstruction occurs when `D_t >= k`; duplicates do not increase `D_t`.

For aggregate inference, each valid contribution is an element of a
deterministic merge algebra:

```text
a_i in A
A_t = merge of accepted innovative contributions through time t
decision_t = d(A_t)
quality_t  = q(A_t)
```

For anomaly localization, `A` is a bounded integer score vector over candidate
clusters, merge is vector addition, and `d(A_t)` is the top cluster when the
margin and evidence guard pass. This is where inference enters the encoding
semantics: fragments carry coded or recoded contributions to a task statistic,
not only opaque bytes that a later reducer interprets after delivery.

Diffusion cost is measured with integer replay artifacts. The controller tracks
active innovative forwarding opportunities `Y_t`, finite-horizon cost `C_T`,
and measured reproduction pressure `R_est`. The primary comparison fixes
payload-byte budget. Secondary comparisons may fix transmissions, forwarding
opportunities, or storage caps, but every artifact states which budget is fixed.

Recoded fragments carry parent contribution ids. The receiver counts canonical
contribution ids, not copies. Valid recoding cannot increase rank or useful
contribution count beyond the parent/local contribution ledger.

## Theorem Targets

The theory is compact and explicit about assumptions.

| Result | Lean theorem | Assumptions | Artifact rows |
| --- | --- | --- | --- |
| Finite-horizon diffusion control | `inference_potential_drift_progress` | controller progress credit, duplicate/storage/transmission pressure debit | `active_belief_theorem_assumptions.csv` |
| k-of-n receiver arrival | `receiver_arrival_reconstruction_bound` | finite horizon, success floor, dependence mode, arrived rank at least `k` | `active_belief_theorem_assumptions.csv`, `active_belief_exact_seed_summary.csv` |
| Useful inference arrival | `useful_inference_arrival_bound` | task-relevant contribution floor, quality threshold, finite horizon | `active_belief_theorem_assumptions.csv`, `active_belief_final_validation.csv` |
| Anomaly margin | `anomaly_margin_lower_tail_bound` | bounded updates, correct-cluster advantage, lower-tail failure bound | `active_belief_theorem_assumptions.csv` |
| Guarded false commitment | `guarded_commitment_false_probability_bounded` | margin threshold, evidence guard, false-commitment bound | `active_belief_exact_seed_summary.csv` |
| Mergeable-statistic soundness | task algebra lemmas | valid merge, recoding, contribution ledger, duplicate non-inflation | `active_belief_second_tasks.csv` |
| Host/bridge demand safety | `propagated_demand_cannot_validate_invalid_evidence`, `propagated_demand_duplicate_non_inflation` | replay-visible host/bridge demand record, valid bounded summary | `active_belief_host_bridge_demand.csv` |

Theorem-backed claims apply only when the finite-horizon contact,
update-bound, guard, and controller assumptions are marked satisfied. Rows
marked empirical-only are evidence for the implemented regime, not proof
instances.

## Experimental Plan

The central experiment is anomaly localization over clustered temporal
contacts: 100 or 500 agents, five or more clusters, intermittent bridge
contacts, no instantaneous static source-to-sink path during the core window,
time-respecting evidence journeys, local noisy observations, multiple receivers
with integer score landscapes, and bounded demand summaries derived from
uncertainty, competitor margins, and missing contribution classes.

The central success condition is stronger than delivery. The receiver's
landscape must sharpen, and preferably commit, before full observation recovery
or raw payload transit. For the active version, multiple receivers should
reduce uncertainty and converge toward compatible guarded decisions faster or
with fewer bytes than passive controlled coded diffusion.

The experiment suite progresses through:

1. single-source k-of-n message reconstruction,
2. distributed anomaly localization from mergeable local score contributions,
3. in-network recoding or aggregation ablation.

The distributed-evidence mode includes a no-central-encoder panel: no node owns
the global input, each node emits only local evidence, and the receiver is
compared against a later oracle reducer that sees the full observation set only
for evaluation.

Core comparisons include uncoded full-message replication, epidemic forwarding,
spray-and-wait, coded diffusion without reproduction control, passive
controlled coded diffusion, and active belief diffusion with bounded demand.
Core ablations remove demand propagation, duplicate-risk scoring, bridge-value
scoring, landscape-value scoring, reproduction control, or replace forwarding
with random selection under the same byte budget. Robustness stresses include
duplicate spam, selective withholding, biased observations, bridge-node loss,
and stale recoded evidence.

## Metrics

Primary metrics are recovery probability, decision accuracy, time to
reconstruction or commitment, receiver rank, top-hypothesis margin, scaled
uncertainty, quality per payload byte, commitment-before-full-recovery rate,
commitment lead time, bytes at commitment, receiver agreement, belief
divergence, collective uncertainty, demand satisfaction, demand-response lag,
evidence overlap, bytes transmitted, duplicate and innovative arrivals, peak
retained bytes, measured `R_est`, and potential value over time.

Secondary observer metrics report linkage inference accuracy, cluster or
source/receiver uncertainty, observer projection summaries, and cost at fixed
observer advantage. All metrics are deterministic: integer counts, fixed
denominators, canonical ordering, and typed Jacquard time/order values.

## Figure Plan

The proposal called for eleven figures. The final paper package keeps those
figures and adds five strong-validation figures for the host/bridge demand
surface, theorem assumptions, large-regime replay, trace validation, and
opportunistic baselines.

| Figure | Name | Proposal role | Source artifact |
| --- | --- | --- | --- |
| 1 | Landscape coming into focus | Belief landscape, rank, margin, uncertainty, bytes, duplicates, demand, and `R_est` over time | `coded_inference_experiment_a_landscape.csv` |
| 2 | Path-free recovery | Recovery or decision when no stable static path exists | `coded_inference_experiment_b_path_free_recovery.csv` |
| 3 | Three-mode comparison | Source-coded, distributed-evidence, and recoded/aggregated modes | `coded_inference_experiment_a2_evidence_modes.csv` |
| 4 | Active belief grid | Receiver agreement, uncertainty, margin, and commitment lead time | `active_belief_final_validation.csv` |
| 5 | Task algebra table | Direct statistic decoding across exact, anomaly, majority, and histogram tasks | `active_belief_second_tasks.csv` |
| 6 | Phase diagram | Quality, cost, duplicate pressure, and `R_est` across control regimes | `coded_inference_experiment_c_phase_diagram.csv` |
| 7 | Active versus passive | Demand-aware allocation versus passive controlled coded diffusion | `active_belief_final_validation.csv` |
| 8 | Coding versus replication | Quality or recovery at equal payload-byte budget | `coded_inference_experiment_d_coding_vs_replication.csv` |
| 9 | Recoding frontier | Forwarding-only versus in-network aggregation | `active_belief_final_validation.csv` |
| 10 | Robustness boundary | Stress regimes where the primitive holds or stops helping | `active_belief_exact_seed_summary.csv` |
| 11 | Observer ambiguity frontier | Observer proxy frontier, not a privacy theorem | `coded_inference_experiment_e_observer_frontier.csv` |
| 12 | Host/bridge demand safety | Demand is first-class but non-evidential across replay surfaces | `active_belief_host_bridge_demand.csv` |
| 13 | Theorem assumptions by regime | Proof-to-experiment assumption map | `active_belief_theorem_assumptions.csv` |
| 14 | Large-regime validation | Deterministic 500-node artifact sanity | `active_belief_large_regime.csv` |
| 15 | Trace validation | Canonical preprocessing and deterministic replay for traces | `active_belief_trace_validation.csv` |
| 16 | Opportunistic baseline comparison | Stronger deterministic forwarding baselines | `active_belief_strong_baselines.csv` |

## Evaluation Structure

The evaluation is claim-by-claim:

1. Direct statistic decoding works for compact mergeable tasks.
2. Demand improves allocation under equal payload-byte budget.
3. Demand remains non-evidential on simulator-local and host/bridge replay
   surfaces.
4. Multi-receiver guarded commitments are compatible without consensus.
5. Theorem assumptions are satisfied or explicitly labeled empirical.
6. Large-regime rows replay deterministically.
7. Semi-realistic mobility-contact traces are canonically preprocessed and
   replay checked.
8. Robustness boundaries show where the primitive stops helping.

## Contributions

The paper contributes:

- active belief diffusion as a two-object evidence/demand primitive for
  temporal decentralized inference,
- a mergeable-task interface that separates direct statistic decoding from
  routing plus post-processing,
- a non-evidential demand safety boundary,
- theorem-backed finite-horizon and merge-soundness claims with explicit
  assumption artifacts,
- deterministic replay experiments across reconstruction, anomaly
  localization, compact second tasks, recoding, active demand, robustness,
  observer proxies, and opportunistic baselines,
- a reproducible paper package with CSV rows, figures, captions, and sanity
  checks.

## Implementation Roles

Jacquard is the deterministic implementation substrate. It supplies typed time,
explicit router ownership, host bridges, replayable simulator artifacts, and
integer resource accounting.

Field is the experimental incubator only after corridor-routing machinery is
removed from the active research path. It is not the contribution name, and
Field routing is not part of this analysis package.

Telltale provides supporting proof vocabulary and choreographic protocol checks
where protocol structure matters. The contribution is not an MPST result.

## Non-Claims

The paper does not claim arbitrary ML inference, general distributed cognition,
consensus, common knowledge, privacy, optimal active policy, production
deployment readiness, universal delay-tolerant routing superiority, or
robustness against arbitrary adaptive adversaries. The observer-ambiguity
figures are proxy measurements. The large-regime rows are scale hygiene and
deterministic replay checks, not production deployment evidence.

## Positioning

The target audience is AI systems, decentralized inference, networked sensing,
and distributed systems researchers interested in how communication substrates
can carry useful inference state rather than only completed messages.

The short positioning line is:

> Agents in temporal networks can form useful beliefs by exchanging compact,
> mergeable summaries of both evidence and uncertainty.

The safety line is:

> Demand is first-class in communication, but non-evidential in semantics.

The technical line is:

> Direct statistic decoding, contribution identity, and deterministic replay
> separate active belief diffusion from ordinary routing followed by offline
> reduction.

## Submission Artifact Boundary

The reproducible active-belief report PDF is generated by
`just active-belief-report` under
`artifacts/analysis_2/latest/active-belief-report.pdf`. Venue-specific
typesetting, page limits, bibliography style, and camera-ready packaging are
submission work, not remaining research validation.

## Caption Audit Rules

Captions must name the fixed budget, seed set, regime set, trace family, and
theorem-assumption status when those quantities support the plotted claim.
Captions must not use privacy, consensus, optimal, general AI, adversarially
robust, or deployment-ready language unless the plotted artifact and theorem
table support that exact claim.

## Limitations

The strong paper covers compact mergeable sufficient statistics, not arbitrary
ML inference. Observer ambiguity remains an empirical proxy unless a separate
formal privacy definition is added. The contact-frequency baseline is
deterministic and useful for opportunistic-forwarding comparison, but it is not
a complete delay-tolerant-routing survey. Empirical rows outside theorem
assumptions should be read as implemented evidence, not proof instances.
