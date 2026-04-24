# Active Belief Diffusion With Coded Evidence

## Abstract

Agents can form useful beliefs without stable paths, central aggregation, or
full raw-data recovery. We show this for temporal networks where each node sees
only part of the world and the decision window may close before any node could
collect everything.

Active belief diffusion lets agents exchange two compact messages. Coded
evidence carries audited pieces of what an agent has learned. Demand summaries
carry what would most reduce uncertainty, but never count as evidence. Across
replayed path-free traces, receivers sharpen beliefs and reach guarded
commitments before full information transit. Propagated demand improves quality
per byte over passive controlled coding and uncoded replication, with earlier
commitment, lower uncertainty, and better receiver compatibility.

The result applies to compact mergeable inference tasks. Local observations
become deterministic statistic contributions, and the merged statistic yields a
decision, margin, or uncertainty summary. Exact `k`-of-`n` recovery is the
threshold case; additive score-vector anomaly localization is the main
AI-facing case.

## 1. Introduction

Many AI and distributed-sensing systems are built around an assumption that
eventually someone gets to see everything. Data can be centralized. The contact
pattern can eventually permit complete aggregation. Or a coordinator can reduce
the full observation set into a final answer. That assumption breaks in edge,
swarm, disaster, battlefield, rural, and privacy-constrained settings. Contacts
are intermittent. Links are capacity-constrained. The decision window may close
before any node could have gathered the full raw information object.

The motivating question is whether useful collective belief can form before
full information transit. Our answer is yes. The answer applies to a restricted
and explicit class of inference problems. The key move is to exchange compact
summaries of what agents have learned. Agents also exchange compact summaries
of what would most reduce their remaining uncertainty. Inference does not have
to wait until raw data arrives at one place.

This matters for AI systems that cannot rely on central observation. Examples
include swarms, edge sensing, disaster response, contested networks, rural
sensing, privacy-constrained deployments, and intermittently connected
autonomy. The AI object here is belief formation before aggregation from
partial audited statistics. The claim is not general distributed learning,
universal privacy, or consensus.

The proposed primitive is active belief diffusion. Agents exchange two bounded,
replay-visible objects. The first is coded evidence: audited contributions to a
mergeable sufficient statistic. The second is a bounded demand summary:
replay-visible information about what evidence would most improve current
belief quality. The symmetry matters operationally because both objects diffuse
through the network. The asymmetry matters semantically because only coded
evidence can change the sufficient statistic. Demand can prioritize forwarding,
retention, custody, and recoding. It cannot validate evidence. It cannot create
contribution identity or alter merge semantics.

The object of interest is a mergeable sufficient statistic that can be updated
before raw recovery. Exact recovery is one threshold instance inside this
broader task class. The paper focuses on compact deterministic merge algebras
with auditable contribution identity. Demand summaries are part of the
communication primitive, but they do not create evidence or change what counts
as evidence.

The paper makes three contributions:

- It defines active belief diffusion as a two-object primitive for temporal
  decentralized inference. Coded evidence carries audited statistic
  contributions. Bounded demand summaries carry replay-visible summaries of
  what would most improve current belief quality.
- It identifies a mergeable-task interface that cleanly separates direct
  statistic decoding from batch reduction after delivery. This lets the same
  mechanism cover threshold reconstruction, additive anomaly localization, and a
  small set of other compact tasks.
- It presents a proof-scoped and replay-backed evaluation. The evaluation shows
  path-free collective inference in the supported regime. It also shows that
  propagated demand improves byte-normalized quality and commitment lead time.
  Demand remains first-class in communication while staying non-evidential.

The scope is explicit. The paper does not claim arbitrary machine
learning inference over intermittent networks. It claims that for compact
mergeable tasks, useful collective belief can emerge before full information
transit. This remains true in decision windows with no stable path and no
central aggregator.

## 2. Related Work And Positioning

The closest literature is not a single field. It is a stack of adjacent systems
and AI literatures. Federated inference and collaborative DNN inference at the
edge distribute model execution across devices or infrastructure. Active belief
diffusion instead distributes recoverable evidence through an intermittent
contact field.

Coded computation and coded inference use redundancy to tolerate stragglers or
unavailable workers. Parity-model prediction serving is a representative
example. Data-availability systems such as ZODA also show that sampled coded
symbols can serve both checking and reconstruction. Active belief diffusion
uses the analogous systems principle for temporal-network inference. Evidence
movement, validity records, and receiver updates should improve reconstruction
or decision quality, not merely report telemetry.

Multi-agent reinforcement learning and active sensing study what agents should
communicate or observe. Belief propagation and active inference provide useful
vocabulary for local belief updates. These lines usually assume that a
communication graph or coordination substrate is available enough to carry the
messages. Here the contact field is part of the problem.

The positive distinction is bounded replay-visible inference over mergeable
sufficient statistics. Coded evidence carries audited statistic contributions.
Demand summaries are first-class messages about uncertainty, but they remain
non-evidential. This combination is the paper's contribution.

The privacy and traffic-analysis literatures are also relevant. Statistical
disclosure attacks, Bayesian traffic analysis, and MANET anonymity work show
that communication metadata can reveal relationships. This paper reports
observer ambiguity only as a proxy. It does not prove privacy. The Triangle of
Forgetting boundary is a useful guardrail: duplicate non-inflation and bounded
retention are not the same as post-revocation forgetting or temporal secrecy.

## 3. Running Example

Consider clustered anomaly localization with multiple receivers. Each node sees
only a local noisy signal about which cluster is anomalous. A receiver does not
need every raw observation. It needs enough innovative statistic contribution to
separate its top competing hypotheses and to satisfy a guarded commitment rule.

Suppose one receiver currently has a narrow lead for cluster A over cluster B,
while another is undecided between B and C. The first receiver emits a bounded
demand summary asking for evidence that separates A from B. The second asks for
evidence that separates B from C. Intermediate nodes do not learn global truth
from those summaries. They use them only to prioritize which valid coded
evidence to forward, retain, or recode under a fixed byte budget. When a
receiver accepts innovative coded evidence, it merges that contribution into
its local score vector. It updates its belief landscape. It may commit before
the full observation set could ever have been reconstructed at that receiver.

This example is the paper's central case. The same discipline reduces to exact
`k`-of-`n` recovery when the sufficient statistic is set union and the decision
rule is a threshold.

## 4. Primitive

Active belief diffusion is defined over a mergeable task interface:

```text
local encode:      x_i -> a_i in A
merge:             A x A -> A
identity:          e in A
global statistic:  A* = merge_i a_i
decision:          d(A*) -> y
quality:           q(A*) -> margin / uncertainty / score
```

The merge must be associative and, unless the task intentionally depends on
order, commutative. Contribution identity prevents double counting.
Recoding or aggregation is valid only when it preserves the contribution ledger
and merge semantics. The supported task class includes counts, votes,
histograms, and heavy hitters. It also includes sketches, additive score
vectors, bounded log-likelihood accumulators, linear-model scores,
random-feature embeddings, set union, and lattice-valued summaries.

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

Algorithm 1 spells out the deterministic round loop.

```text
Algorithm 1: Active Belief Diffusion
Input: temporal contacts, byte/storage/lifetime caps, mergeable task algebra
State: local statistic A_r, contribution ledger L_r, bounded demand d_r

for each deterministic round t:
  observe local signal x_i, if any
  encode x_i as an audited contribution a_i with contribution identity
  update local belief summary from A_r
  derive bounded demand d_r from uncertainty, margin, and missing classes
  for each contact (u, v) in canonical order:
    enumerate candidate evidence under byte and custody caps
    score forwarding, retention, or recoding using demand and duplicate risk
    demand may affect priority, custody, recoding, and allocation only
    demand may not validate evidence or create contribution identity
    demand may not alter merge semantics or the belief statistic
    transmit selected bounded evidence and demand messages
  for each received evidence object in canonical order:
    validate evidence and parent contribution ledger without demand
    if contribution identity is valid and innovative:
      merge contribution into A_r
      update L_r and belief summary
  if evidence guard and margin guard hold:
    emit guarded commitment
```

The symmetry matters operationally and the asymmetry matters semantically.
Evidence and demand are both bounded exchange objects. Only valid coded
evidence, however, can change the merged sufficient statistic.

## 5. Model And Formal Boundary

Time is discrete and finite horizon. A temporal contact trace gives directed or
undirected contact opportunities over time. A time-respecting journey is a
sequence of contacts whose times are nondecreasing.

Each evidence fragment has a target id, origin mode, fragment id, rank or
contribution id, byte size, and validity marker. Recoded evidence also carries
optional parent ids. For exact reconstruction, a target has `k <= n`. The
receiver rank `D_t` is the number of valid independent contributions received
by time `t`.
Exact reconstruction occurs when `D_t >= k`. Duplicates do not increase `D_t`.

For aggregate inference, each valid contribution is an element of a
deterministic merge algebra:

```text
a_i in A
A_t = merge of accepted innovative contributions through time t
decision_t = d(A_t)
quality_t  = q(A_t)
```

Diffusion cost is tracked with deterministic integer replay records. The
controller records active innovative forwarding opportunities `Y_t`,
finite-horizon cost `C_T`, and measured reproduction pressure `R_est`. Primary
comparisons fix payload-byte budget. Recoded fragments carry parent
contribution ids. The receiver counts canonical contribution ids rather than
copies.

The formal contribution is intentionally scoped. The proof-backed rows apply
to the synthetic sparse-bridge and clustered duplicate-heavy regimes. The
semi-realistic mobility trace is empirical support only.

Theorem 1, receiver arrival. The Lean theorem
`receiver_arrival_reconstruction_bound` states that a valid finite-horizon
arrival floor that reaches the required rank supports exact reconstruction in
the threshold case. In plain terms, distinct valid contributions matter.
Duplicates do not increase receiver rank.

Theorem 2, useful inference arrival. The Lean theorem
`useful_inference_arrival_bound` states that enough task-relevant contribution
mass can support a useful decision before full raw recovery. This is the formal
bridge from reconstruction to inference over a sufficient statistic.

Theorem 3, guarded anomaly commitment. The Lean theorems
`anomaly_margin_lower_tail_bound` and
`guarded_commitment_false_probability_bounded` state that a margin guard plus
an evidence guard bounds false commitment under the modeled finite-horizon
assumptions. This supports guarded commitment in the theorem-backed regimes.

Theorem 4, non-evidential demand safety. The Lean theorems
`propagated_demand_cannot_validate_invalid_evidence` and
`propagated_demand_duplicate_non_inflation` state that propagated demand cannot
validate invalid evidence or inflate receiver rank through duplicates. Demand
can influence allocation. It cannot change evidence identity, validity, or
merge semantics.

Theorem 5, progress accounting. The Lean theorem
`inference_potential_drift_progress` packages the controller-side accounting
surface. It records useful progress against duplicate, storage, and
transmission pressure. This is the proof-side companion to the operating-region
plot.

Table 13 summarizes that boundary.

{{EXHIBIT:figure_13_theorem_assumptions}}

## 6. Experimental Design

The central experiment is multi-receiver anomaly localization over clustered
temporal contacts. The traces contain intermittent bridge contacts and no
instantaneous static source-to-receiver path during the core window. Successful
runs require time-respecting evidence journeys. Nodes produce local noisy
observations. Receivers maintain integer score landscapes. Demand summaries are
derived from uncertainty, competitor margins, and missing contribution classes.

The protocol table makes the replay surface explicit.

| Item | Implemented setting |
| --- | --- |
| Seeds | 20 deterministic seeds, `41..60` |
| Core regimes | synthetic sparse bridge, synthetic clustered duplicate-heavy, semi-realistic mobility-contact |
| Theorem-backed regimes | synthetic sparse bridge and synthetic clustered duplicate-heavy |
| Empirical-only regime | semi-realistic mobility-contact |
| Receivers | three receiver identities per receiver-run artifact |
| Core tasks | anomaly localization, majority threshold, bounded histogram, set-union threshold |
| Evidence modes | uncoded replication, passive controlled coded, full active belief, recoded aggregate |
| Payload budget | 4096 bytes for headline comparisons |
| Phase-control surface | reproduction bands with `R_est`, forwarding budget, and `k/n` values `4/8` and `6/10` |
| Demand accounting | replay-visible demand counts in host/bridge demand rows |
| Commitment guard | evidence guard plus margin guard; reported as commitment lead time and false-commitment rate |
| Stress surface | normal, duplicate pressure, mobility, malicious duplicate pressure, delayed demand |
| Scale hygiene | 128-node sparse bridge, 256-node clustered, and 500-node mobility-contact replay rows |

The paper uses three primary metrics:

1. commitment lead time, the number of rounds by which a guarded commitment
   precedes full recovery;
2. quality per byte under a fixed payload budget;
3. false-commitment rate under modeled stress.

Secondary metrics include receiver agreement, collective uncertainty, duplicate
pressure, bytes at commitment, and reproduction pressure. The main budgeted
comparison fixes payload transmission to 4096 bytes.

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

Trace validation and large-regime replay hygiene are supporting checks rather
than headline claims. They establish that the evaluation is deterministic,
canonicalized, and reproducible before the substantive comparisons are read.
The trace-validation table is included here so the artifact hygiene is visible
before the substantive comparisons.

{{EXHIBIT:figure_15_trace_validation}}

## 7. Results

The empirical story has four central claims. First, useful belief formation can
occur in windows with no static end-to-end path. Second, the mechanism is not
limited to threshold delivery. It operates on a larger mergeable-task surface.
Third, propagated demand improves byte-normalized collective inference. Fourth,
the gains persist under fair-cost comparisons and remain visible under explicit
stress and baseline checks.

### 6.1 Belief Landscapes Sharpen In Path-Free Windows

In the path-validation traces, every recorded run has no instantaneous static
path in the core window and does have a time-respecting evidence journey. Under
that condition, median path-free success is 885 permille for active belief
diffusion, 805 for passive controlled coded diffusion, 908 for recoded
aggregation, and 585 for uncoded replication. The point of Figure 2 is
therefore not merely that delivery eventually happens. The point is that useful
inference progresses through temporal contact history rather than through a
hidden stable path.

{{EXHIBIT:figure_02_path_free_recovery}}

Figure 1 shows what that progress looks like at the receiver. As innovative
evidence arrives, quality rises while margin and uncertainty move in the
expected direction. The relevant phenomenon is the formation of a usable belief
landscape before the receiver has the full raw information object.

{{EXHIBIT:figure_01_landscape_focus}}

### 6.2 The Mechanism Is Larger Than Threshold Delivery

Table 3 separates the three evidence-origin modes at the task-object level.
The threshold case remains important because it is the cleanest sanity check,
but it is not the distinctive case. The distinctive claim appears in the
distributed-evidence and recoded-aggregation modes. There, fragments carry
statistic contributions rather than opaque bytes for later centralized
reduction.

{{EXHIBIT:figure_03_three_mode_comparison}}

Figure 5 extends the same discipline to a small task family beyond anomaly
localization. The contribution is not that every learning problem is mergeable.
It is that several useful compact tasks share the same direct statistic-decoding
surface and therefore admit the same transport and proof discipline.

{{EXHIBIT:figure_05_task_algebra}}

### 6.3 Propagated Demand Improves Byte-Normalized Collective Inference

Under a fixed 4096-byte budget, active belief diffusion reaches median quality
per byte 887 permille in the multi-receiver anomaly-localization setting. It
also reaches receiver agreement 888 permille, collective uncertainty 109
permille, commitment lead time 3 rounds, and bytes at commitment 1934. The
corresponding passive controlled coded medians are 807, 862, 161, 1, and 2074.
Uncoded replication reaches 587, 788, 381, 1, and 2508. Active demand improves
byte-normalized quality by about 10 percent over passive controlled coding and
about 51 percent over uncoded replication. It reduces median uncertainty by
about 32 percent and 71 percent respectively.

Figure 4 shows these gains as receiver-level distributions in direct units
across the clustered, mobility, and sparse-bridge regimes rather than as
normalized deltas from one baseline. That makes the collective-belief claim
visible at the level of actual receiver outcomes. It also makes the
commitment-lead-time story concrete. Active belief diffusion typically gives
about two extra rounds of lead time over passive controlled coding and uncoded
replication. It also reduces receiver-to-receiver divergence.

{{EXHIBIT:figure_04_active_belief_grid}}

The causal ablation supports the same conclusion. Propagated demand reaches
median quality per byte 621.5 permille with median uncertainty 328.5,
innovative arrivals 15, duplicate count 10, and bytes at commitment 2154.
No-demand drops to 517.5, 432.5, 10, 13, and 2570. Stale demand also degrades
to 535.5 quality per byte and 414.5 uncertainty. The improvement therefore
comes from current propagated uncertainty summaries changing allocation toward
useful evidence. It does not come from any change in evidence semantics.

{{EXHIBIT:figure_07_active_vs_passive}}

### 6.4 Coding And Recoding Beat Replication Under Fair Cost Accounting

At the 4096-byte comparison point, active coded diffusion reaches median
quality 926 permille with duplicate count 8. Passive controlled coded reaches
846 with duplicate count 11. Uncoded replication reaches 626 with duplicate
count 24. Coded diffusion is therefore better both in decision quality and in
duplicate pressure under the same payload budget.

Figure 8 shows the same result with interquartile spread bands over the budget
axis, so the coding advantage reads as measured variation rather than as a
single schematic curve.

{{EXHIBIT:figure_08_coding_vs_replication}}

Recoding modestly improves the frontier further. In the receiver-run summaries,
recoded aggregation reaches median quality per byte 910 permille, receiver
agreement 896 permille, collective uncertainty 109 permille, commitment lead
time 3 rounds, and bytes at commitment 1988. That slightly dominates passive
controlled coding. It is also competitive with, or better than, active belief
diffusion on the quality-byte frontier. It still respects the same
contribution-ledger discipline. Figure 9 shows this frontier by regime with
median points and interquartile spreads rather than with an overplotted cloud.

{{EXHIBIT:figure_09_recoding_frontier}}

### 6.5 Control And Robustness Boundaries Remain Visible

The coded mechanism is only useful if diffusion pressure stays bounded. Figure 6
makes the operating region explicit. The near-critical runs are the ones that
enter the target `R_est` band and obtain the best quality gains without paying
the duplicate and byte costs seen in the supercritical runs. The relevant
result is not one globally optimal setting. The controller exposes a visible
operating region rather than hiding cost inside unbounded diffusion.

{{EXHIBIT:figure_06_phase_diagram}}

Figure 10 then gives the stress boundary. Median commitment accuracy is 955
permille at severity 1, 880 at severity 2, and 805 at severity 3. Median
false-commitment rate rises from 14.5 to 22.5 to 30.5 permille. At severities
4 and 5, false commitment reaches 38.5 and 46.5 permille. The quality gains
also flatten. This is a useful robustness boundary. The method remains
effective through moderate modeled stress. The degradation point is explicit
rather than hidden.

{{EXHIBIT:figure_10_robustness_boundary}}

### 6.6 Demand Is First-Class In Communication But Not Evidential

The safety claim is architectural rather than purely statistical. Demand
summaries are replay-visible protocol objects that influence forwarding,
retention, and recoding decisions. They do not validate evidence, create
contribution identity, change merge semantics, publish route truth, or inflate
duplicate rank. Table 12 records both facts directly. Active variants carry
replay-visible demand summaries. All forbidden evidential side effects stay at
zero.

{{EXHIBIT:figure_12_host_bridge_demand}}

This separation matters to the paper's AI framing. The system exchanges bounded
summaries of both learned information and remaining uncertainty. Only coded
evidence can change the sufficient statistic.

### 6.7 Supporting Fairness Checks

The strong-baseline comparison is a fairness check, not the conceptual center
of the paper. Its job is to show that the reported gains are not an artifact of
comparing only against obviously weak opportunistic policies. In the baseline
summary, active belief diffusion stays ahead of passive controlled coded
diffusion, contact-frequency opportunism, epidemic forwarding, spray-and-wait,
random forwarding, and uncoded replication. The budget accounting is the same.

{{EXHIBIT:figure_16_strong_baselines}}

Large-regime replay validation and observer ambiguity remain supporting
material. They are useful for reproducibility and scope. They are not required
to establish the main path-free inference, active-demand,
multi-receiver compatibility, or fair-cost coding claims in the paper body.

## 7. Contributions

This paper makes three contributions:

1. a two-object primitive for temporal decentralized inference. Coded evidence
   and bounded demand summaries co-diffuse while remaining semantically
   distinct;
2. a mergeable-task interface that makes direct statistic decoding the primary
   object. Exact reconstruction is one threshold special case;
3. a proof-scoped and replay-backed empirical case. Path-free collective
   inference is possible in the supported regime. Demand improves allocation
   under equal byte budgets. The mechanism has explicit safety and stress
   boundaries.

## 8. Limitations

The paper covers compact mergeable sufficient statistics, not arbitrary machine
learning inference. Some claims are deterministic theorem-backed across the
whole supported mechanism boundary; the finite-horizon probabilistic claims are
theorem-backed only in the sparse-bridge and semi-realistic mobility regimes
and stay empirical-only in the clustered duplicate-heavy regime. Observer
ambiguity is a traffic-analysis proxy, not a formal privacy claim. The
opportunistic baseline set is strong enough to be informative. It is not a
complete survey of delay-tolerant networking.

## 9. Conclusion

In temporal networks with no stable path in the decision window and no central
aggregator, agents can still form useful collective beliefs. They do this by
exchanging two bounded objects. Coded evidence merges into an auditable
sufficient statistic. Demand summaries expose what evidence would most reduce
current uncertainty without becoming evidence themselves. For compact mergeable
tasks, that is enough to obtain earlier guarded commitment, better quality per
byte, lower uncertainty, and more compatible receiver-side beliefs from
different temporal histories. The core result is receiver-side commitment from
partial audited statistics. It is not route delivery followed by later
post-processing.
