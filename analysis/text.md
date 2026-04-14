# Jacquard Routing: Tuning and Analysis

## Executive Summary

### Executive Summary Intro

This report studies Jacquard routing behavior across six engines, `batman-bellman`, `batman-classic`, `babel`, `olsrv2`, `pathway`, and `field`, using a common simulator corpus and a shared analysis pipeline.

The goal is to understand where each engine works cleanly, where it begins to degrade, what kinds of failures appear first, and compare engines under the same network regimes. Routing quality is regime-dependent: a setting that works well in an easy connected network may break down under asymmetry, bridge loss, candidate pressure, or uncertainty.

The document is organized in four parts. Part I covers tuning: recommended configurations, transition behavior, failure boundaries, and simulator assumptions. Part II covers engine-specific analysis and cross-engine comparisons. Part III calibrates diffusion-oriented engine profiles. Part IV evaluates these calibrated profiles under message-diffusion scenarios where mobility is the transport and end-to-end paths may not exist.

## Part I. Tuning

### Recommendation Tables

#### Recommendation Overview

@table recommendation

This table condenses the highest-ranked configurations for each engine family.

Column guide: Score is the composite ranking value; Activation is the share of runs that installed a route; Route Presence is the average fraction of rounds with a live route; Max Stress is the highest sustained stress level survived before the first maintained breakdown.

#### Transition Behavior

@table transition-metrics

This table shows how the leading configurations behave over time, not only how they score in aggregate.

Column guide: Route Mean and Route Stddev are average and spread of route presence across runs; First Mat. is the first round a route appears; First Loss is the first round a route disappears; Recovery is the first round routing returns after a loss; Churn counts route changes or handoffs. A `-` means the event was not observed.

#### Failure Boundaries

@table boundary-summary

This table shows how much sustained stress each leading configuration survives before the first maintained failure.

Column guide: Max Stress is the highest stress level cleared; First Failed Family is the regime that breaks it; Fail Stress is the stress level at that failure; Reason is the dominant failure mode. A `-` means no failure was observed.

### Setup And Method

#### Simulation Setup

Each experiment run uses the Jacquard simulator to play out a fixed network scenario from a known random seed. A scenario defines the network layout, available routing engines, active routing requests, and round count. The simulator can apply planned changes during a run: cutting links, restoring links, degrading links asymmetrically, moving connections, or imposing local resource limits.

The report scores observable replay output: whether a route appears, when it is first lost, whether it recovers, how often it changes, and what failures are recorded.

#### Matrix Design

The tuning matrix changes one small set of conditions at a time. Across the corpus, it varies network density, loss, interference, asymmetry, topology change, node pressure, and objective type. For `batman-bellman`, `batman-classic`, `babel`, and `olsrv2`, the main sweep changes decay-window settings. For `pathway` and `field`, the main sweep changes per-objective search budget and heuristic mode.

The recommendations are meant to be good defaults for this modeled corpus, not single winners from one scenario.

#### Regime Assumptions

The scenarios are stylized representations of common mesh-network conditions. Names like sparse line, medium ring, and bridge cluster describe the network shape. Loss and interference settings then make communication easier or harder within that shape. Some families are intentionally placed near break points where a small parameter change can determine whether a route survives.

#### Regime Characterization

Topology regimes:

- `sparse line`: nodes depend on a single chain of relays with few alternate paths.
- `medium ring`: looped connectivity allows one route to fail while another may survive.
- `medium mesh` and `dense mesh`: multiple neighbors can reach the same destination, so contention and search choice matter more.
- `bridge cluster`: two groups joined by one narrow bridge, so a single weak link dominates routing.
- `high fanout`: one node sees many candidate neighbors, stress-testing search budget.

Condition regimes:

- `low`, `moderate`, and `high loss` describe message drop rates.
- `interference` and `contention` describe medium crowding.
- `asymmetry` means one direction of a link is worse than the other.
- `churn`, `relink`, `partition`, and `recovery` describe topology changes over time.
- `intrinsic node pressure` means the node itself becomes a bottleneck.

Workload regimes:

- `connected-only` requires an actual connected route.
- `repairable-connected` allows temporary disruption with expected recovery.
- `service` means the engine chooses among candidate service locations.
- `concurrent mixed` means multiple route requests compete under shared pressure.

#### BATMAN Bellman Algorithm

BATMAN Bellman tracks a good next hop toward each destination using a local Bellman-Ford computation over a gossip-merged topology graph. It can bootstrap routes from topology data before OGMs arrive. The tuning parameters control how quickly old information expires and how quickly the engine refreshes its view.

#### BATMAN Classic Algorithm

BATMAN Classic is the spec-faithful BATMAN IV engine: TQ is carried in OGMs and updated multiplicatively at each relay hop, TTL bounds propagation depth, and bidirectionality requires echo-window confirmation. It produces no route candidates until receive-window data has accumulated and echo confirmation has been received. BATMAN Bellman can bootstrap routes from its local Bellman-Ford computation before OGMs arrive, but BATMAN Classic has no such shortcut. The analysis compares its behavior under identical regimes to isolate what the Bellman-Ford enhancement contributes.

#### Babel Algorithm

Babel implements the RFC 8966 distance-vector protocol. Link cost uses bidirectional ETX rather than forward-only TQ, penalizing asymmetric links more heavily. Path metric is additive rather than multiplicative. Route selection is gated by a feasibility distance table that provides loop freedom during transient topology changes. The analysis targets asymmetric link regimes and partition recovery to surface these behavioral differences.

#### OLSRv2 Algorithm

`olsrv2` is Jacquard's deterministic proactive link-state baseline. It learns one-hop and two-hop reachability from HELLO exchange, elects a stable MPR covering set, floods TC advertisements only when local topology changes or forwarded MPR-selected TC state is fresher, and runs shortest-path derivation over the learned topology tuples.

The Jacquard engine intentionally simplifies the full RFC surface: it keeps one deterministic link-state view, one MPR election policy, and next-hop-only route publication. The tuning sweep therefore focuses on the decay window that controls how long HELLO and TC evidence stays live and how quickly the engine requests another synchronous round.

#### Pathway Algorithm

Pathway explores candidate continuations and chooses a full routing decision for the requested destination or service. The main tuning question is how much search budget it needs before it reliably finds good candidates.

#### Field Algorithm

Field maintains a continuously updated field model, searches over frozen snapshots, and publishes one corridor-style routing claim while allowing the concrete realization to move inside that corridor. It has an explicit bootstrap phase where weaker corridor claims can be published when evidence is coherent but not yet strong enough for steady admission.

The main tuning questions are how much search budget Field needs, how often it reconfigures or shifts continuation, and how often bootstrap routes successfully upgrade.

#### Recommendation Logic

The recommendation score rewards settings that activate routes reliably, maintain route presence, tolerate harder stress levels, and (for the distance-vector engines) maintain stability. It penalizes route churn, maintenance failures, lost reachability, and prolonged degradation.

Profile-specific recommendations allow different operational priorities. Conservative profiles weight stability and failure avoidance more heavily, while aggressive or service-heavy profiles tolerate more risk.

`field` is now calibrated on two surfaces in Part I: the generic route-visible recommendation surface and a separate regime-specific surface that explicitly scores corridor continuity, bootstrap upgrade quality, service continuity, and transition health.

When several nearby settings score about the same, the report prefers the middle of the acceptable range.

#### Profile Recommendation Logic

These profile recommendations reuse the same simulator corpus but change the ranking weights so each table answers a different operational question.

#### Profile Recommendation Logic Empty

No profile-specific recommendations are available for this artifact set.

#### Profile Recommendations

@table profile-recommendations

Column guide: Profile is the ranking policy; Score is the profile-weighted composite value; Activation is share of runs that installed a route; Route is average route presence; Max Stress is highest stress level survived.

#### Field Continuity Profiles

@table field-profile-recommendations

This table treats Field lifecycle behavior as a tuning output in its own right.

Column guide: Profile names the continuity objective; Score is the profile-weighted value; Route is average route presence; Shifts is mean continuation-shift count; Carry is mean service carry-forward volume; Narrow is mean corridor-narrow count; Degraded is mean degraded-steady occupancy.

Interpretation guide: `field-stable-service` favors limited disruption; `field-low-churn` pushes harder against unnecessary movement; `field-broad-reselection` preserves more alternate branches and accepts more shifts; `field-conservative-publication` favors earlier narrowing and less corridor breadth.

#### Field Regime Calibration

@table field-routing-regime-calibration

This table calibrates `field` against regime-specific success criteria rather than only one flat route-visible recommendation score.

Column guide: Regime names the field-specific operating regime; Success Criteria states what the calibration is trying to optimize in that regime; Configuration is the best-scoring field setting for that regime; Route is mean route presence; Transition is the transition-health score; Shifts is mean continuation-shift count; Carry is mean service carry-forward volume; Stress is the highest stress envelope represented by the regime rows.

## Part II. Analysis

### Figure Context

#### BATMAN Bellman Transition Analysis

These two plots form an analytical pair: the first shows where stability accumulates across the transition families, and the second shows when those same settings first lose a route.

#### Figure 1

@figure batman_bellman_transition_stability

Stale-after ticks on the x-axis, transition-family lines showing accumulated stability. Point annotations mark the paired refresh setting.

#### Figure 2

@figure batman_bellman_transition_loss

When routes are first lost under the same transition families. The clearest view of whether a shorter or longer decay window helps near relink and asymmetric bridge boundaries.

#### BATMAN Classic Transition Analysis

These plots mirror the BATMAN Bellman pair for the spec-faithful engine. The classic engine converges more slowly due to its echo-only bidirectionality requirement.

#### Figure 3

@figure batman_classic_transition_stability

Same stale-after-ticks axis as Figure 1 but for the spec-faithful engine.

#### Figure 4

@figure batman_classic_transition_loss

When routes are first lost under the batman-classic transition families. Compare against Figure 2 to isolate the Bellman-Ford enhancement contribution.

#### Babel Decay Analysis

The asymmetry-cost-penalty family is the primary differentiator: Babel's ETX formula penalizes poor reverse delivery more heavily than TQ, producing different route selection under identical topology. The partition-feasibility-recovery family shows the FD table's bounded infeasible-fallback window after partition clears.

#### Figure 5

@figure babel_decay_stability

Accumulated stability across the three Babel decay families.

#### Figure 6

@figure babel_decay_loss

When routes are first lost under the Babel decay families. The partition-feasibility-recovery family shows the FD table's infeasible-fallback window.

#### OLSRv2 Decay Analysis

These plots answer the missing proactive link-state question directly: how quickly the full-topology engine stabilizes when links degrade, partitions clear, and relay roles shift. The topology-propagation and MPR-flooding families expose whether fresher HELLO and TC retention buys cleaner recovery or just unnecessary churn.

#### Figure 7

@figure olsrv2_decay_stability

Accumulated stability across the four maintained OLSRv2 topology and churn families.

#### Figure 8

@figure olsrv2_decay_loss

When routes are first lost under the same OLSRv2 families. Compare against the BATMAN and Babel figures to separate link-state freshness from distance-vector decay effects.

#### Pathway Budget Figures Intro

These two figures show the budget question from two angles: how much route presence extra budget buys, and where activation collapses outright.

#### Figure 9

@figure pathway_budget_route_presence

Route presence under the Pathway pressure families by search budget.

#### Figure 10

@figure pathway_budget_activation

Activation success by search budget. The clearest view of minimum viable Pathway search breadth.

#### Field Corridor Figures Intro

The first figure shows route-visible continuity across corridor-oriented families. The second shows search and continuation churn. Together they distinguish a healthy corridor default from an unstable bootstrap regime.

#### Figure 11

@figure field_budget_route_presence

Field route-visible success by budget across corridor-oriented families.

#### Figure 12

@figure field_budget_reconfiguration

Continuation shifts and search reconfiguration rounds combined into one reconfiguration load signal.

#### Figure 13

@figure comparison_dominant_engine

Which engine dominates in each maintained mixed-engine comparison family.

#### Figure 14

@figure head_to_head_route_presence

Direct comparison of explicit engine sets over the same regime families.

### Comparison And Head-To-Head

#### Mixed-Engine Regime Split

@table comparison-summary

Dominant engine per maintained comparison family. Column guide: Dominant Engine is the best performer; Activation, Route Presence, and Stress as above.

#### Head-To-Head Results

@table head-to-head-summary

Direct stack-to-stack comparison: `batman-bellman`, `batman-classic`, `babel`, `olsrv2`, `pathway`, `field`, and `pathway-batman-bellman`. Column guide: Engine Set is the only stack enabled; Activation, Route, Dominant, and Stress as above. A `-` means no route-visible winner was observed.

#### Head-To-Head Regimes

The head-to-head regimes are:

- `connected-low-loss`: easy connected route where all engines should establish a route.
- `connected-high-loss`: repairable connected route over a lossy bridge.
- `bridge-transition`: bridge that degrades, partitions, and restores.
- `partial-observability-bridge`: bridge case with Field bootstrap summaries for corridor-style routing under incomplete evidence.
- `corridor-continuity-uncertainty`: intermittent degradation and restoration rewarding corridor continuity.
- `concurrent-mixed`: multiple active objectives testing mixed-workload behavior.

#### Head-To-Head Findings Intro

These rows show what each stack does when it is the only available routing surface for that host set.

#### Head-To-Head Takeaways

- `connected-low-loss` is mostly a tie regime.
- Lossy and bridge-recovery cases remain the clearest separators: `connected-high-loss` is led by `{connected_high_loss_engine_set}` at {connected_high_loss_route_presence} permille, `bridge-transition` by `{bridge_transition_engine_set}` at {bridge_transition_route_presence} permille.
- Mixed workloads favor explicit search: `concurrent-mixed` is led by `{concurrent_mixed_engine_set}` at {concurrent_mixed_route_presence} permille.
- `field` is strongest when corridor continuity is the question: {corridor_uncertainty_route_presence} permille in `corridor-continuity-uncertainty`, but only {partial_bridge_route_presence} permille in `partial-observability-bridge`.

#### Head-To-Head Findings Empty

No head-to-head summary is available for this artifact set.

## Part III. Diffusion Calibration

### Diffusion Calibration Introduction

Routing calibration and diffusion calibration use different objectives and should not be merged. Routing optimizes for activation, route presence, and recovery. Diffusion optimizes for eventual delivery, boundedness, latency, energy, and leakage.

For `field`, diffusion calibration also now uses regime-specific success criteria: continuity families reward protected bridge-budget preservation and corridor persistence, scarcity families reward early conservative transition plus lower generic spread and expensive transport use, congestion families reward timely transition from cluster seeding into duplicate suppression without starving first-arrival cluster coverage, and privacy families reward lower observer leakage.

The maintained diffusion families are:

- `random-waypoint-sanity`: lightweight baseline with mixed movers.
- `partitioned-clusters`: separated clusters with rare bridger contacts.
- `disaster-broadcast`: urgent one-to-many over disrupted mobility.
- `sparse-long-delay`: sparse network with long delays and few long-range movers.
- `high-density-overload`: dense camp-like setting exposing overload.
- `mobility-shift`: clusters reconfiguring over time.
- `adversarial-observation`: clustered delivery with observer nodes exposing leakage.
- `bridge-drought`: prolonged low-contact bridge regime.
- `energy-starved-relay`: low-energy relay regime punishing over-forwarding.
- `congestion-cascade`: dense low-capacity broadcast regime.

### Diffusion Calibration Summary

@table diffusion-engine-summary

Best-performing engine set per maintained diffusion family. Column guide: Engine Set, Delivery (fraction of targets reached), Coverage (fraction of reachable nodes), Latency (mean delivery delay), State (boundedness classification), Stress.

### Diffusion Calibration Boundaries

@table diffusion-engine-boundaries

Where each engine set stays viable and where it first collapses or becomes explosive. Column guide: Viable Families, First Collapse, First Explosive.

### Field Diffusion Regime Calibration

@table field-diffusion-regime-calibration

This table calibrates `field` on its own diffusion success surface instead of only asking whether the generic cross-engine score liked it.

Column guide: Regime names the diffusion posture regime; Success Criteria states what `field` is supposed to optimize there; Configuration is the accepted field diffusion profile for that regime, or an explicit no-acceptable-candidate marker if every field candidate still fails; Posture is the dominant field posture; State is the dominant boundedness class; Transition summarizes either the posture transition count or the first scarcity / congestion transition round; Delivery is mean delivery success; Tx is mean transmission count; Fit is the regime-specific field fitness score. The CSV export also includes protected-budget use, bridge-opportunity capture, cluster-seeding use, coverage-starvation counts, and deterministic suppression counts for the winning profile or best attempt.

## Part IV. Diffusion Engine Comparison

### Diffusion Analysis Introduction

This part evaluates the calibrated profiles directly. The emphasis shifts from admissibility to how the engines differ when diffusion itself is the comparison surface.

The comparison surface here is regime-aware rather than purely family-by-family. Continuity, scarcity, congestion, privacy, and balanced regimes reward different behaviors, so the first summary table reports the best engine set per regime before the full family matrix.

### Diffusion Regime Comparison

@table diffusion-regime-engine-summary

Best-performing engine set per diffusion regime. Column guide: Regime, Engine Set, Delivery, Coverage, Cluster Cov. (target-cluster coverage), Tx, State, Score.

### Field Vs Best Alternative

@table field-vs-best-diffusion-alternative

Best field candidate per diffusion regime against the best non-field alternative under the same regime-aware comparison score. Column guide: Field is the best field attempt, OK reports whether that attempt cleared the field-specific acceptability gate, State / Alt State are boundedness modes, `dDel` / `dCov` / `dClus` are field-minus-alternative delivery, coverage, and target-cluster coverage deltas, `dTx` is transmission delta, and `dScore` is regime-score delta. Negative `dTx` is good for field.

### Diffusion Engine Comparison

@table diffusion-engine-comparison

Full maintained diffusion engine surface. Column guide: Family, Engine Set, Delivery, Coverage, Tx (transmission count), `R_est` (boundedness signal), State (boundedness classification). Use this table to inspect family-level exceptions after the regime summary above.

### Diffusion Figure Context

These two figures separate delivery success from resource boundedness across the most discriminating maintained diffusion families.

#### Figure 15

@figure diffusion_delivery_coverage

Delivery and coverage by engine set.

#### Figure 16

@figure diffusion_resource_boundedness

Transmission load and boundedness by engine set.

### Diffusion Takeaways

- The diffusion track is an engine comparison, but the regime summary is the right top-level view because continuity, scarcity, congestion, and privacy reward different tradeoffs.
- The conservative stacks are still strongest when scarce relays and bounded forwarding matter most.
- `field` now shows clearer regime specialization, but the calibration surface also fails closed when all field congestion candidates are still unacceptable.
- The harsher families are still boundary finders: `bridge-drought` tests rare-opportunity carry, `energy-starved-relay` tests efficiency under scarcity, and `congestion-cascade` tests whether broad forwarding remains bounded without starving first-arrival cluster coverage.

### Field Diffusion Posture

The artifact set exposes posture-aware `field` diffusion behavior:

- In `bridge-drought`, `field` ends with dominant posture `{bridge_drought_posture}` after {bridge_drought_transitions} posture transitions, using {bridge_drought_protected_budget} protected budget units and converting {bridge_drought_bridge_use} of {bridge_drought_bridge_opportunities} protected bridge opportunities.
- In `energy-starved-relay`, `field` ends with dominant posture `{energy_starved_posture}`, first enters scarcity-conservative behavior at round {energy_starved_first_scarcity}, and suppresses {energy_starved_expensive_suppressions} expensive transport attempts.
- In `congestion-cascade`, `field` ends with dominant posture `{congestion_posture}`, first enters congestion control at round {congestion_first_transition}, seeds {congestion_cluster_seed_uses} first-arrival cluster transfers, records {congestion_cluster_starvation} cluster-coverage starvation events, and records {congestion_redundant_suppressions} redundant-forward suppressions plus {congestion_same_cluster_suppressions} same-cluster suppressions.

### Data-Driven Templates

#### Pressure Findings Batman Plateau

BATMAN Bellman shows a broad plateau in easy regimes.

#### Pressure Findings Batman Separation

BATMAN Bellman separates in the transition families under relink pressure and asymmetric bridge degradation.

#### Pressure Findings Batman Classic Plateau

BATMAN Classic shows a flat stability profile across decay windows. The slower convergence means the decay window has less impact than in the Bellman-Ford variant.

#### Pressure Findings Batman Classic Separation

BATMAN Classic separates in the partition-recovery family where echo-only bidirectionality creates timing differences between configurations.

#### Pressure Findings Babel Plateau

Babel shows a broad plateau across decay windows. The feasibility distance table dominates convergence timing.

#### Pressure Findings Babel Separation

Babel separates in the asymmetry-cost-penalty family and the partition-feasibility-recovery family.

#### Pressure Findings Pathway Cliff

Pathway query budget 1 fails immediately. Higher budgets plateau quickly.

#### Pressure Findings Field Plateau

Field shows a broad viable plateau: route presence={route_present} permille, bootstrap activation={bootstrap_activation} permille, bootstrap upgrade={bootstrap_upgrade} permille at the low-budget point. Separation between configurations comes from lifecycle shape rather than raw route presence.

#### Engine Section Empty Field

No measured Field recommendation is available for this artifact set. The simulator extracts Field replay, search, reconfiguration, and bootstrap signals, but those signals do not close the boundary to a stable route-visible default.

#### Engine Section Empty Generic

No {engine_family} recommendation is available for this artifact set.

#### Engine Section Recommended

Recommended configuration: `{config_id}` (score={score}, activation={activation} permille, route presence={route_presence} permille, max sustained stress={max_stress}).

#### Engine Section Batman Bellman Plateau

The BATMAN Bellman transition families are flat on accumulated stability, suggesting a plateau rather than one narrow best setting.

#### Engine Section Batman Bellman Best

The BATMAN Bellman transition families separate most clearly at `{config_id}` (stability-total {stability_total}, route presence {route_presence} permille).

#### Engine Section Batman Bellman Closing

Severe asymmetric bridge loss remains a breakdown regime across the tested window range.

#### Engine Section Batman Classic Plateau

The BATMAN Classic transition families are flat on accumulated stability, consistent with the engine's slower convergence.

#### Engine Section Batman Classic Best

The BATMAN Classic transition families separate most clearly at `{config_id}` (stability-total {stability_total}, route presence {route_presence} permille).

#### Engine Section Batman Classic Closing

BATMAN Classic's echo-only bidirectionality makes it consistently slower to materialize routes than BATMAN Bellman.

#### Engine Section Babel Plateau

The Babel families show a flat stability profile. The feasibility distance table does not significantly differentiate decay window settings in recoverable regimes.

#### Engine Section Babel Best

The Babel families separate most clearly at `{config_id}` (stability-total {stability_total}, route presence {route_presence} permille).

#### Engine Section Babel Closing

The feasibility distance table bounds convergence after partition recovery. Routes with the same seqno as pre-partition are infeasible until the next seqno increment.

#### Engine Section OLSRv2 Plateau

The OLSRv2 families show a broad stable region. Once HELLO and TC state stay live long enough to cover one topology churn cycle, extra retention mostly changes recovery timing rather than route visibility.

#### Engine Section OLSRv2 Best

The OLSRv2 families separate most clearly at `{config_id}` (stability-total {stability_total}, route presence {route_presence} permille).

#### Engine Section OLSRv2 Closing

The remaining stress point is asymmetric relink timing: full-topology knowledge helps after relays settle, but stale symmetric-link evidence can still leave one churn window where the best next hop is temporarily absent.

#### Engine Section Pathway Cliff

Pathway budget 1 is the cliff edge: activation={activation} permille.

#### Engine Section Pathway Floor

Budgets at and above `{config_id}` form the stable floor.

#### Engine Section Field Best

Field separates where corridor continuity and reconfiguration cost both matter. `{config_id}` keeps route presence at {route_presence} permille while holding continuation shifts to {continuation_shifts}.

#### Engine Section Field Bootstrap

Corridor-continuity profile: bootstrap activation {activation} permille, hold {hold} permille, narrow {narrow} permille, upgrade {upgrade} permille, withdrawal {withdrawal} permille, degraded-steady occupancy {degraded} permille, service carry-forward {service} permille, asymmetric shift success {shift} permille. Dominant commitment resolution `{commitment}`, last recovery outcome `{outcome}`, continuity band `{band}`, continuity transition `{transition}`, last decision `{decision}`, blocker `{blocker}`.

#### Engine Section Field Tied

Route presence is close across the tested range. The service-oriented knobs separate configs in continuation-shift count, service carry-forward, and narrowing behavior.

#### Engine Section Field Replay

The maintained families produce router-visible activation and route presence, with the bootstrap phase directly visible in replay and recovery surfaces.

#### Engine Section Field Families

The asymmetric-envelope and bridge anti-entropy families test corridor continuity under realization movement. The partial-observability and bootstrap-upgrade families test bootstrap promotion and withdrawal. The service-overlap, freshness-inversion, and publication-pressure families test service-corridor continuity under broader publication, stronger freshness weighting, or earlier narrowing.

#### Engine Section Field Diagnosis

The service-corridor publication and materialization path is the key enabler for Field's route-visible behavior. The tuning question is which continuity style is preferable: narrower lower-churn publication or broader reselection with more carried-forward optionality.

#### Recommendation Rationale Empty Field

No Field recommendation rationale is published because the corpus does not produce a stable route-visible Field default. The analysis stack is present and informative, but Field tuning should be read as diagnostic instrumentation rather than default selection.

#### Recommendation Rationale Empty Generic

No {engine_family} recommendation rationale is available.

#### Recommendation Rationale Primary

Primary recommendation: `{config_id}` with mean score {score}. Activation {activation} permille, route presence {route_presence} permille, max sustained stress {max_stress}.

#### Recommendation Rationale Runner Up

Next closest: `{config_id}` with a score gap of {score_gap}.

#### Recommendation Rationale Small Gap

That small gap means the result should be read as an acceptable range rather than a single brittle optimum.

#### Recommendation Rationale Large Gap

That larger gap means this corpus is finding a real preferred point.

#### Recommendation Rationale Batman Bellman 1

The BATMAN Bellman recommendation is driven by the recoverable transition families, not by easy-regime route presence alone.

#### Recommendation Rationale Batman Bellman 2

The severe asymmetric bridge regime fails across the entire tested window range, so the recommendation applies to recoverable pressure, not impossible bridges.

#### Recommendation Rationale Batman Classic 1

The BATMAN Classic recommendation reflects the spec-faithful model's slower convergence. Larger decay windows are needed for echo-based bidirectionality and receive-window accumulation.

#### Recommendation Rationale Batman Classic 2

Comparing BATMAN Classic to BATMAN Bellman under the same regimes isolates the Bellman-Ford enhancement contribution. The difference is most visible in early-round materialization timing.

#### Recommendation Rationale Babel 1

The Babel recommendation balances decay window size against the feasibility distance table's convergence behavior.

#### Recommendation Rationale Babel 2

The asymmetry-cost-penalty family is the clearest differentiator. The bidirectional ETX formula penalizes asymmetric links more than TQ.

#### Recommendation Rationale OLSRv2 1

The OLSRv2 recommendation is driven by the topology-propagation and partition-recovery families. The selected decay window keeps HELLO and TC state live long enough to span one churn window without delaying recovery so long that stale next hops dominate.

#### Recommendation Rationale OLSRv2 2

The MPR-flooding and asymmetric-relink families are the main differentiators. If the window is too short, symmetric-link state expires before the topology settles; if it is too long, the link-state graph holds on to obsolete bridge evidence longer than needed.

#### Recommendation Rationale Pathway 1

The main justification is the hard low-budget cliff: `pathway-1-zero` fails in the pressure families while higher budgets plateau.

#### Recommendation Rationale Pathway 2

The recommendation chooses the lowest stable floor rather than spending more search budget after the curve flattens.

#### Recommendation Rationale Field 1

The Field recommendation is driven by corridor continuity, bootstrap upgrade behavior, and reconfiguration cost together. When route presence is effectively tied, the default choice should favor lower-churn corridor management rather than broader reselection.

#### Recommendation Rationale Field 2

Measured continuity profile for `{config_id}`: bootstrap activation {activation} permille, hold {hold} permille, narrow {narrow} permille, upgrade {upgrade} permille, withdrawal {withdrawal} permille, degraded-steady occupancy {degraded} permille, service carry-forward {service} permille, asymmetric shift success {shift} permille. Dominant commitment resolution `{commitment}`, last recovery outcome `{outcome}`, continuity band `{band}`, continuity transition `{transition}`, last decision `{decision}`, blocker `{blocker}`.

#### Recommendation Rationale Field 3

The Field configurations are close in top-line route presence. The continuity profile table is the better place to choose between lower-churn and broader-reselection behavior, and low-churn continuity should remain the default surface unless a regime explicitly needs broader reselection.

#### Recommendation Rationale Field 4

The corpus includes steady route-visible service continuity and bootstrap-aware corridor behavior. The service regimes make the Field knobs visible in lifecycle metrics even when route presence clusters closely.

#### Limitations And Next Steps

These recommendations are only as good as the simulated regime corpus. A flat curve can mean genuine robustness or that the sweep has not found the most informative failure boundary.

The BATMAN Bellman corpus exposes recoverable transition differences, but asymmetry-plus-bridge families remain hard failures. The BATMAN Classic corpus confirms slower convergence and tight clustering of decay window settings. The Babel corpus shows measurably different behavior under asymmetric conditions, with the FD table visible in partition recovery, but decay window settings do not yet separate sharply. The OLSRv2 corpus separates most clearly on topology propagation, MPR flooding stability, and asymmetric relink timing, but the maintained window sweep is still narrow enough that several settings remain tied on route visibility. The Pathway corpus identifies the minimum viable budget floor with a wide plateau above it.

The Field corpus reaches the route-visible boundary with an explicit bootstrap phase and working service-corridor path, but tested settings cluster tightly. The bridge anti-entropy and bootstrap-upgrade families allow the report to distinguish between underexercise and real weakness. The remaining limitation is that tested settings cluster closely, leaving room for more discriminating future regime design.
