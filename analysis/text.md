# Jacquard Router: Tuning and Analysis (Draft)

## Executive Summary

### Executive Summary Intro

This report studies Jacquard routing behavior across seven active routing engines using a common simulator corpus and analysis pipeline. The engines in scope are `batman-classic`, `batman-bellman`, `babel`, `olsrv2`, `scatter`, `mercator`, and `pathway`; historical Field corridor-routing artifacts are treated as legacy inputs rather than active comparison members. The goal is to understand where each active engine works well, where performance degrades, what kind of failures arise, and to compare engines under the same network conditions. Routing quality is regime-dependent: a setting that works well in an easy connected network may break down under asymmetry, bridge loss, candidate pressure, or uncertainty.

The report is organized in four parts. Part I covers tuning: recommended configurations, transition behavior, failure boundaries, and simulator assumptions. Part II covers engine-specific analysis and cross-engine comparisons. Part III calibrates diffusion-oriented engine profiles. Part IV evaluates these calibrated profiles under message-diffusion scenarios where node movement and intermittent contact opportunities carry messages, and end-to-end paths may not exist.

### Design Setting

The maintained corpus is designed for disrupted and mobility-driven mesh environments. In this setting, end-to-end paths are often absent. Connectivity appears through short contact windows, weak bridges, and repeated partial recovery rather than through one stable connected graph. Nodes are also resource-constrained, so routing quality depends on bounded state, bounded work, and disciplined use of transmissions and custody.

The route-visible matrix gives useful evidence for this setting because it stresses the conditions that determine whether a router-facing engine remains usable at all. The maintained families vary bridge pressure, asymmetry, loss, relink events, partitions, recovery, contention, and local node pressure. Those are the same forces that determine whether a proactive engine keeps a route, whether a search-driven engine finds one, and where each approach breaks down.

The diffusion track adds the second half of the picture. It models cases where movement is the transport mechanism and messages must persist across disconnection. Its mobility-driven contacts, bounded replication, energy and transmission accounting, storage utilization, and observer-leakage measures give insight into whether a deferred-delivery policy remains viable in the same population-level setting, not only in easy connected regimes. The coded-diffusion observer surface reports measured ambiguity rather than formal privacy: attacker advantage, uncertainty, and ambiguity-cost frontier rows are plot inputs for understanding the cost of hiding forwarding choices under explicit projections.

## Part I. Tuning

### Recommendation Tables

#### Recommendation Overview

@table recommendation

This table condenses the highest-ranked configurations for each engine family.

Column guide: Score is the composite ranking value; Activation is the share of runs that installed a route; Route Presence is the average fraction of rounds with a live route; Max Stress is the highest sustained stress level survived before the first maintained breakdown.

#### Recommendation Detail Note

Detailed transition, failure-boundary, and profile tables are collected in Appendix A so the main report can stay focused on the key recommendations and figures.

#### Transition Behavior

@table transition-metrics

This table shows how the leading configurations behave over time, not only how they score in aggregate.

Column guide: Route Mean and Route Stddev are average and spread of route presence across runs; First Mat. is the first round a route appears; First Loss is the first round a route disappears; Recovery is the first round routing returns after a loss; Churn counts route changes or handoffs. A `-` means the event was not observed.

#### Failure Boundaries

@table boundary-summary

This table shows how much sustained stress each leading configuration survives before the first maintained failure.

Column guide: Max Stress is the highest stress level cleared; First Failed Family is the regime that breaks it; Fail Stress is the stress level at that failure; Reason is the dominant failure mode. `not observed` means no failure was observed.

### Setup And Method

#### Simulation Setup

Each experiment run uses the Jacquard simulator to play out a fixed network scenario from a known random seed. A scenario defines the network layout, available routing engines, active routing requests, and round count. The simulator can apply planned changes during a run: cutting links, restoring links, degrading links asymmetrically, moving connections, or imposing local resource limits.

The report scores observable replay output: whether a route appears, when it is first lost, whether it recovers, how often it changes, and what failures are recorded.

The simulator now also records model-lane snapshot and reducer artifacts for validation, but the recommendation tables and figures in this report continue to score the maintained full-stack replay artifacts. The added model-lane output is there to check planner and reducer behavior, not to replace the router-visible outcome surface.

#### Matrix Design

The tuning matrix changes one small set of conditions at a time. Across the corpus, it varies network density, loss, interference, asymmetry, topology change, node pressure, and objective type. For `batman-classic`, `batman-bellman`, `babel`, and `olsrv2`, the main sweep changes decay-window settings. For `scatter`, the route-visible sweep compares the maintained `balanced`, `conservative`, and `degraded-network` profiles. For `mercator`, the route-visible sweep uses the fixed bounded-custody posture against the same maintained route-visible families. For `pathway`, the main sweep changes per-objective search budget and heuristic mode.

The recommendations are meant to be good defaults for this modeled corpus, not single winners from one scenario.

Methodology note:
- route-visible `Route` / `Active Route` report total-window route presence in comparison and head-to-head tables, so startup and repair gaps remain visible instead of being hidden by saturated active-window values.
- the head-to-head route-visible surface is a fixed representative-profile benchmark, while Part I recommendation tables are calibrated-best surfaces.
- mixed-stack `Selected-Round Leader` means the engine selected for the most active-route rounds inside one shared router policy, not the best standalone engine.
- generic family-by-family diffusion winners are still a representative weighted surface, but the current winner-sensitivity audit should be checked first: stable rows can be read with more confidence, while any unstable row remains provisional relative to the regime-aware diffusion tables.

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

BATMAN Bellman tracks a good next hop toward each destination using a local Bellman-Ford computation over a gossip-merged topology graph. It can bootstrap routes from topology data before OGMs arrive. The tuning parameters control how quickly old information expires and how quickly the engine refreshes its view. Like the other conventional proactive next-hop engines in this corpus, it retains routing control state such as learned advertisements and bidirectionality evidence, but it does not buffer payloads for deferred store-and-forward delivery.

#### BATMAN Classic Algorithm

BATMAN Classic is the spec-faithful BATMAN IV engine: TQ is carried in OGMs and updated multiplicatively at each relay hop, TTL bounds propagation depth, and bidirectionality requires echo-window confirmation. It produces no route candidates until receive-window data has accumulated and echo confirmation has been received. BATMAN Bellman can bootstrap routes from its local Bellman-Ford computation before OGMs arrive, but BATMAN Classic has no such shortcut. The analysis compares its behavior under identical regimes to isolate what the Bellman-Ford enhancement contributes. Its retained state is protocol state only, not deferred payload storage.

#### Babel Algorithm

Babel implements the RFC 8966 distance-vector protocol. Link cost uses bidirectional ETX rather than forward-only TQ, penalizing asymmetric links more heavily. Path metric is additive rather than multiplicative. Route selection is gated by a feasibility distance table that provides loop freedom during transient topology changes. The analysis targets asymmetric link regimes and partition recovery to surface these behavioral differences. As with the BATMAN variants, the cached state here is advertisement, feasibility, and metric state rather than payloads awaiting store-and-forward delivery.

#### OLSRv2 Algorithm

`olsrv2` is Jacquard's deterministic proactive link-state baseline. It learns one-hop and two-hop reachability from HELLO exchange, elects a stable MPR covering set, floods TC advertisements only when local topology changes or forwarded MPR-selected TC state is fresher, and runs shortest-path derivation over the learned topology tuples.

The Jacquard engine intentionally simplifies the full RFC surface: it keeps one deterministic link-state view, one MPR election policy, and next-hop-only route publication. The tuning sweep therefore focuses on the decay window that controls how long HELLO and TC evidence stays live and how quickly the engine requests another synchronous round. That retained state is topology and freshness metadata only; `olsrv2` is not a deferred-delivery payload buffer.

#### Scatter Algorithm

`scatter` is Jacquard's bounded deferred-delivery diffusion engine. It publishes an opaque, partition-tolerant route claim when the objective is supportable, then performs engine-private store-carry-forward movement with hard copy budgets, scope-relative carrier scoring, local regime selection, and contact-feasibility gates.

Unlike an acknowledgement-driven custody protocol, `scatter` does not assume authoritative transfer. It retains bounded payload custody locally, can split copy budget conservatively across better carriers, and may prefer a better carrier without claiming globally reliable handoff semantics. Its retained state is payload custody plus local diffusion metadata, not a topology graph or explicit path.

#### Mercator Algorithm

`mercator` combines connected corridor search, maintained repair evidence, weakest-flow reservation, broker-pressure accounting, and bounded custody posture. It publishes one connected corridor claim only when connected support exists. When support does not exist, custody remains engine-private and bounded through the shared retention boundary rather than becoming a synthetic connected route.

The route-visible experiments exercise Mercator as a fixed representative engine set. The diffusion experiments use its bounded custody profile to compare strict-improvement carrier selection, protected bridge budget, and suppression behavior against `scatter` and proactive baselines.

#### Pathway Algorithm

Pathway explores candidate continuations and chooses a full routing decision for the requested destination or service. The main tuning question is how much search budget it needs before it reliably finds good candidates. It is also one of the in-tree routing engines that supports bounded deferred delivery of payloads: when a route enters partition mode, payloads can be retained through the shared retention boundary and later replayed on recovery or handoff.

#### Legacy Field Algorithm

Field maintains a continuously updated field model, searches over frozen snapshots, and publishes one corridor-style routing claim while allowing the concrete realization to move inside that corridor. It has an explicit bootstrap phase where weaker corridor claims can be published when evidence is coherent but not yet strong enough for steady admission. The tuning surface is now split deliberately: search breadth still lives in `FieldSearchConfig`, while continuation, promotion, continuity, and evidence behavior sit behind a separate internal operational policy surface that the experiments can sweep more cleanly. Field does carry forward bounded routing and service evidence, but that carry-forward supports corridor and service continuity rather than acting as a general payload store-and-forward cache.

#### Recommendation Logic

The recommendation score rewards settings that activate routes reliably, maintain route presence, tolerate harder stress levels, and (for the distance-vector engines) maintain stability. It penalizes route churn, maintenance failures, lost reachability, and prolonged degradation.

Profile-specific recommendations allow different operational priorities. Conservative profiles weight stability and failure avoidance more heavily, while aggressive or service-heavy profiles tolerate more risk.

Historical Field corridor-routing calibration is not part of the active route-visible recommendation surface. Existing Field artifacts may still be parsed as legacy evidence, but new routing recommendations exclude Field rows.

When several nearby settings score about the same, the report prefers the middle of the acceptable range.

#### Tuning Reference Material

Appendix A contains the detailed transition, failure-boundary, and profile tables that support the main tuning recommendation.

#### Profile Recommendation Logic

These profile recommendations reuse the same simulator corpus but change the ranking weights so each table answers a different operational question.

#### Profile Recommendation Logic Empty

No profile-specific recommendations are available for this artifact set.

#### Profile Recommendations

@table profile-recommendations

Column guide: Profile is the ranking policy; Score is the profile-weighted composite value; Activation is share of runs that installed a route; Route is average route presence; Max Stress is highest stress level survived.

#### Legacy Field Continuity Profiles

@table field-profile-recommendations

This table treats Field lifecycle behavior as a tuning output in its own right.

Column guide: Profile names the continuity objective; Score is the profile-weighted value; Route is average route presence; Shifts is mean continuation-shift count; Carry is mean service carry-forward volume; Narrow is mean corridor-narrow count; Degraded is mean degraded-steady occupancy.

Interpretation guide: `field-stable-service` favors limited disruption; `field-low-churn` pushes harder against unnecessary movement; `field-broad-reselection` preserves more alternate branches and accepts more shifts; `field-conservative-publication` favors earlier narrowing and less corridor breadth.

#### Legacy Field Regime Calibration

@table field-routing-regime-calibration

This table calibrates `field` against regime-specific success criteria rather than only one flat route-visible recommendation score.

Column guide: Regime names the field-specific operating regime; Success Criteria states what the calibration is trying to optimize in that regime; Configuration is the best-scoring field setting for that regime; Route is mean route presence; Transition is the transition-health score; Shifts is mean continuation-shift count; Carry is mean service carry-forward volume; Stress is the highest stress envelope represented by the regime rows.

## Part II. Analysis

### Part II Reading Guide

Part II uses one repeated figure grammar across the engine sections:

- panels are scenario families
- the x-axis is the tuned control surface for that engine section
- solid lines with circle markers show the primary outcome view
- dashed lines with square markers show the cost, fragility, or startup view
- route-visible presence and activation are displayed as percentages

When several series share the same x category, the plot renderer applies a
small deterministic horizontal offset so coincident lines and markers remain
visible. The y-position and tooltip values remain the measured values.

This makes the paired figures easier to compare: the first figure in a section
shows how well the engine performs, and the second shows the cost, fragility,
or control-motion price associated with that behavior.

### Figure Context

#### BATMAN Classic Transition Analysis

These two plots form an analytical pair: the first shows where stability accumulates across the transition families, and the second shows when those same settings first lose a route. The classic engine converges more slowly than other BATMAN variants due to its echo-only bidirectionality requirement.

#### Figure 1

@figure batman_classic_transition_stability

BATMAN Classic stability across transition families. Higher values are better: they indicate more sustained route quality across the scenario. Flatter high lines indicate a decay setting that stays robust as transition stress changes.

#### Figure 2

@figure batman_classic_transition_loss

BATMAN Classic loss timing across transition families. Higher values mean the first route loss happens later, which is usually better. Sharp drops indicate settings that become brittle under the corresponding transition family.

#### BATMAN Bellman Transition Analysis

These two plots form an analytical pair: the first shows where stability accumulates across the transition families, and the second shows when those same settings first lose a route.

#### Figure 3

@figure batman_bellman_transition_stability

BATMAN Bellman stability across transition families. Higher values are better: they indicate more sustained route quality across the scenario. A broad plateau implies the stale-window setting is forgiving rather than narrowly tuned.

#### Figure 4

@figure batman_bellman_transition_loss

BATMAN Bellman loss timing across transition families. Higher values mean route loss is delayed further into the scenario, which is better. Early collapses indicate settings that cannot ride through the corresponding transition stress.

#### Babel Decay Analysis

The asymmetry-cost-penalty family is the primary differentiator: Babel's ETX formula penalizes poor reverse delivery more heavily than TQ, producing different route selection under identical topology. The partition-feasibility-recovery family shows the FD table's bounded infeasible-fallback window after partition clears.

#### Figure 5

@figure babel_decay_stability

Babel stability across decay families. Higher values are better: they indicate more sustained feasible routing across the family. If the curve stays high as stale ticks increase, Babel is not very sensitive to the decay setting there.

#### Figure 6

@figure babel_decay_loss

Babel loss timing across decay families. Higher values mean the first loss happens later, which is better. Downward bends show where stale-state retention starts to hurt feasibility recovery rather than help it.

#### OLSRv2 Decay Analysis

These plots answer the missing proactive link-state question directly: how quickly the full-topology engine stabilizes when links degrade, partitions clear, and relay roles shift. The topology-propagation and MPR-flooding families expose whether fresher HELLO and TC retention buys cleaner recovery or just unnecessary churn.

#### Figure 7

@figure olsrv2_decay_stability

OLSRv2 stability across topology and churn families. Higher values are better: they indicate more sustained route quality through churn and relink events. A high flat region suggests the control-state lifetime is long enough to cover one topology-change cycle without overfitting.

#### Figure 8

@figure olsrv2_decay_loss

OLSRv2 loss timing across topology and churn families. Higher values mean the first loss occurs later, which is better. Lower or falling values indicate churn windows where stale symmetric-link or TC state leaves the engine exposed.

#### Scatter Profile Figures Intro

These two figures put `scatter` on the same tuning-sweep footing as the other engines without pretending that route presence is the whole story. The first figure shows the route-visible tie, and the second shows the threshold-runtime behavior that now separates the maintained `balanced`, `conservative`, and `degraded-network` profiles.

#### Figure 9

@figure scatter_profile_route_presence

Scatter total-window route presence by maintained profile. Higher values are better because the route is available for more of the full scenario window, including startup and repair gaps. Each panel uses the same profile order so the scatter sweep can be compared directly across families.

#### Figure 10

@figure scatter_profile_runtime

Scatter threshold-runtime behavior by maintained profile. Higher values are not universally better: the point is to show which profiles spend rounds in handoff, constrained, bridging, or sparse regimes under the new threshold families. This is the informative Scatter tuning surface when route-visible outcomes tie.

#### Pathway Budget Figures Intro

These two figures show the budget question from two angles: how much route-visible outcome extra budget buys, and what startup or fragility cost remains.

#### Figure 11

@figure pathway_budget_route_presence

Pathway active route presence by search budget. Higher values are better: they indicate the route is present for more of the objective-active window. The y-axis is shown as a percentage so the budget sweep can be read directly against the other route-visible outcome figures in Part II.

#### Figure 12

@figure pathway_budget_activation

Pathway activation by search budget. Higher values are better: they indicate objectives activate successfully more often. The y-axis is shown as a percentage, and step changes reveal the budget threshold where Pathway moves from under-search to reliable activation.

#### Legacy Field Corridor Figures Intro

The first figure shows route-visible continuity across corridor-oriented families. The second shows the control-motion cost paid to preserve that continuity. Together they distinguish a healthy corridor default from an unstable bootstrap regime.

#### Figure 13

@figure field_budget_route_presence

Field active route presence by search budget. Higher values are better: they indicate the admitted corridor stays available for more of the active window. The y-axis is shown as a percentage so the continuity outcome can be compared directly against the other Part II route-visible figures.

#### Figure 14

@figure field_budget_reconfiguration

Field corridor reconfiguration by search budget. Lower values are generally better because they indicate less continuation churn and fewer search-driven reconfigurations. Rising lines mean the engine is paying more control-motion cost to maintain continuity.

#### Figure 15

@figure comparison_dominant_engine

Mixed-engine router arbitration by comparison regime. Bar color marks the engine the deterministic router selected most often in the mixed stack, while the in-bar label shows that leader's share of active-route rounds. This is an arbitration view, not a standalone performance comparison: values near 100% mean the router effectively stuck with one engine for that regime, while lower percentages mean arbitration was more split.

#### Figure 16

@figure head_to_head_route_presence

Head-to-head standalone capability by comparison regime. Darker cells are better: each cell shows one engine set's total-window route presence when run alone. This is the standalone capability view for the same regime families, so it exposes whether the leading tier is broad or whether a newer engine such as `mercator` is materially separated from the alternatives.

#### Figure 17

@figure head_to_head_timing_profile

Timing view for the same head-to-head families. The left panel shows who gets a route up first; the right panel shows who keeps that route longest before the first observed loss.

#### Figure 18

@figure recommended_engine_robustness

Robustness view for the current recommended defaults. This figure shows which engines combine high route presence with lower regime-to-regime spread, rather than only maximizing the mean.

#### Figure 19

@figure mixed_vs_standalone_divergence

Signed fitness gap between what the mixed router achieved and what the best standalone engine would have achieved for the same named family. Rightward bars mean the standalone engine scored higher, leftward bars mean the mixed router scored higher, and exact ties are labeled as ties after accounting for activation success, total-window route presence, materialization delay, route churn, and activation failures. This is the explicit bridge between the arbitration story in Figure 15 and the capability story in Figure 16.

### Comparison And Head-To-Head

#### Mixed-Engine Regime Split

@table comparison-summary

Selected-round leader per maintained comparison family. Column guide: Selected-Round Leader means the engine selected for the most active-route rounds in one shared mixed stack, not necessarily the best standalone performer. Activation is objective activation success, Active Route is total-window route presence, and Stress is the scenario stress score.

The mixed comparison surface is a single-router arbitration benchmark, not an oracle ensemble. The router gathers candidates across engines, publishes one canonical route per objective, and only reselects when maintenance or expiry requires it. Figure 15 therefore answers “which engine does the router actually use?” rather than “which engine is intrinsically best?” A mixed stack can legitimately underperform the best standalone engine in a family if the first durable admissible route comes from a weaker constituent engine.

#### Mixed-Engine Selected-Round Breakdown

@table comparison-engine-round-breakdown

Each row reports the best maintained mixed comparison config for that family. The per-engine columns show average selected-route rounds under one shared router policy, including the `mercator` column. A zero in that column means Mercator was present in the mixed stack for that simulated family but was not the router-selected publisher under the maintained arbitration policy.

Column guide: Family is the comparison regime. Leader is the selected-round leader. Active Route is total-window route presence. Handoffs is mean engine handoff count. The remaining columns show mean selected-route rounds per engine under the shared router policy.

#### Comparison Config Sensitivity Audit

@table comparison-config-sensitivity

Audit of whether the maintained configs separate each comparison family. `topline-and-selection` means both route outcomes and selected-engine behavior differ across configs. `selection-only` means route outcomes are identical but arbitration differs. `flat-control` means the family behaves identically under the maintained configs and should be read as a scenario/control row rather than a parameter-separation row.

Column guide: Surface is the comparison surface (`comparison` or `head-to-head`). Family is the regime. Class is the sensitivity classification. Configs is the number of configs observed. Topline Sigs and Selection Sigs are the counts of distinct outcome and arbitration signatures.

#### Head-To-Head Results

@table head-to-head-summary

Direct stack-to-stack comparison: `batman-classic`, `batman-bellman`, `babel`, `olsrv2`, `scatter`, `mercator`, `pathway`, and `pathway-batman-bellman`. Column guide: Engine Set is the only stack enabled; Activation is objective activation success; Active Route is total-window route presence; Selected Leader is the selected-round leader for that run and may be `tie`; Stress is the regime stress score.

#### Benchmark Profile Audit

@table benchmark-profile-audit

This appendix table separates the fixed representative benchmark configs used in the head-to-head surface from the calibrated-best profile recommendations in Part I. A `Match` row means the representative benchmark config and the current calibrated-best config happen to be the same.

Column guide: Engine Set is the evaluated stack; Representative is the surface kind; Benchmark Config is the fixed benchmark configuration; Calibrated Profile is the calibrated-best profile; Calibrated Config is the calibrated-best configuration; Match indicates whether the benchmark and calibrated configurations are the same.

#### Head-To-Head Regimes

The head-to-head regimes are:

- `connected-low-loss`: easy connected route where all engines should establish a route.
- `connected-high-loss`: repairable connected route over a lossy bridge.
- `bridge-transition`: bridge that degrades, partitions, and restores.
- `medium-bridge-repair`: moderate bridge degradation with a repair window rewarding durable recovery without needing a fully mixed workload.
- `partial-observability-bridge`: bridge case with incomplete evidence and asymmetric repair pressure.
- `corridor-continuity-uncertainty`: intermittent degradation and restoration rewarding corridor continuity.
- `concurrent-mixed`: multiple active objectives testing mixed-workload behavior.

#### Head-To-Head Findings Intro

These rows show what each stack does when it is the only available routing surface for that host set. This is a fixed representative-profile benchmark surface, not the calibrated-best profile surface from Part I.

#### Comparison Detail Note

The full mixed-engine and head-to-head tables are collected in Appendix B. The main body keeps the figures and takeaways.

#### Head-To-Head Takeaways

- `connected-low-loss` and `partial-observability-bridge` are broad tie regimes; in the latter, {partial_bridge_engine_sets} all reach {partial_bridge_route_presence} permille total-window route presence.
- The hard route-visible bridge families are the clearest separators: `connected-high-loss` is led by {connected_high_loss_engine_sets} at {connected_high_loss_route_presence} permille, while `bridge-transition` is shared by {bridge_transition_engine_sets} at {bridge_transition_route_presence} permille.
- `medium-bridge-repair` also stays broad at the top: {medium_bridge_repair_engine_sets} all reach {medium_bridge_repair_route_presence} permille.
- Mixed workloads still favor explicit search: `concurrent-mixed` is led by {concurrent_mixed_engine_sets} at {concurrent_mixed_route_presence} permille.
- `mercator` adds a corridor-and-repair point between pure explicit search and proactive next-hop routing: it reaches {mercator_connected_high_loss_route_presence} permille in `connected-high-loss`, {mercator_bridge_transition_route_presence} permille in `bridge-transition`, {mercator_corridor_uncertainty_route_presence} permille in `corridor-continuity-uncertainty`, and {mercator_concurrent_mixed_route_presence} permille in `concurrent-mixed`.
- `field` stays competitive in the hard bridge and corridor-continuity families: it reaches {field_connected_high_loss_route_presence} permille in `connected-high-loss`, {field_bridge_transition_route_presence} permille in `bridge-transition`, and {field_corridor_uncertainty_route_presence} permille in `corridor-continuity-uncertainty`.

#### Head-To-Head Findings Empty

No head-to-head summary is available for this artifact set.

### Part II Takeaways

- The routing comparison does not collapse to one universal winner. In the mixed-engine matrix, `connected-low-loss` is led by `{connected_low_loss_engine}`, `connected-high-loss` by `{connected_high_loss_engine}`, and `concurrent-mixed` by `{comparison_concurrent_mixed_engine}`.
- Among the maintained proactive next-hop defaults, `babel` and `olsrv2` are the strongest contrasting baselines: `{babel_config}` captures the asymmetry-sensitive distance-vector case, while `{olsrv2_config}` is the full-topology baseline when HELLO and TC propagation have time to pay off.
- Explicit search still matters when the workload is mixed rather than purely hop-by-hop. In the head-to-head matrix, `concurrent-mixed` is led by {concurrent_mixed_engine_sets} at {concurrent_mixed_route_presence} permille route presence.
- The mixed router can still leave performance on the table when early durable admissibility is not the same as standalone fitness: `connected-high-loss` settles on `{mixed_connected_high_loss_engine}` at {mixed_connected_high_loss_route_presence} permille while standalone {head_to_head_connected_high_loss_engines} {head_to_head_connected_high_loss_route_verb} {head_to_head_connected_high_loss_route_presence}. In `bridge-transition`, mixed arbitration now aligns with the standalone top tier at {mixed_bridge_transition_route_presence} permille.
- `mercator` now has enough route-visible coverage to read as an engine behavior rather than a placeholder: {mercator_connected_high_loss_route_presence} permille in `connected-high-loss`, {mercator_bridge_transition_route_presence} permille in `bridge-transition`, {mercator_corridor_uncertainty_route_presence} permille in `corridor-continuity-uncertainty`, and {mercator_concurrent_mixed_route_presence} permille in `concurrent-mixed`. Its role in this corpus is the corridor-maintenance middle ground: more route-visible than deferred diffusion, but explicitly measured against stale repair and broker-pressure limits.
- `field` is corridor-oriented rather than universal: it stays competitive in `connected-high-loss` at {field_connected_high_loss_route_presence} permille, in `bridge-transition` at {field_bridge_transition_route_presence} permille, and in `corridor-continuity-uncertainty` at {field_corridor_uncertainty_route_presence} permille. Its weaker evidence now appears more clearly in the large-population and shared-corridor multi-flow tails than in the single-corridor head-to-head family.

## Large-Population Findings

### Large-Population Introduction

These additions extend the maintained corpus beyond the small connected and single-bridge families so the report can ask three larger-network questions directly: control-plane scaling under fanout and diameter, multi-bottleneck fragility under overlapping repair pressure, and diffusion phase transitions in larger clustered populations.

The route-visible track adds moderate and high large-pop bands for a mixed-density core-periphery family and a multi-bottleneck repair family. The diffusion track adds sparse-threshold, congestion-threshold, and regional-shift continuity families at moderate and high bands.

### Large-Population Route Summary

@table large-population-route-summary

Compact route-visible large-population surface by topology class and engine set. Column guide: Topology is the analytical question family. Engine Set is the standalone routing stack. Small, Moderate, and High are total-window route-presence means for the maintained size bands. `dHigh` is the high-band minus small-band route-presence delta. High Loss is the mean first-loss round in the high-band family.

### Large-Population Diffusion Transitions

@table large-population-diffusion-transitions

Representative collapse, viable, and explosive profiles for each maintained large-population diffusion family. Column guide: Question is the diffusion question family. Size is the maintained population band. Collapse / Viable / Explosive show the representative configuration for that boundedness state when one was observed.

### Large-Population Figure Context

These figures separate route-visible scaling, route-visible fragility, and diffusion transition behavior across the maintained large-population corpus.

#### Figure 22

@figure large_population_route_scaling

Route-visible performance by size band for the maintained large-population families. Each panel fixes one topology class and traces how each standalone engine set moves from the small baseline into the moderate and high bands.
Read the slope more than the starting point: flatter lines mean the engine keeps its route-visible behavior as the population grows, while steep drops mean scaling pressure is exposing a control or search limit. An engine that stays high across all three bands is scaling cleanly in that topology class rather than winning only in the smallest case.

#### Figure 23

@figure large_population_route_fragility

Small-to-high route-presence drop for the maintained large-population route-visible classes. More negative bars mean stronger degradation as the graph grows or bottlenecks multiply. The inline loss label gives the mean first-loss round in the high band.
Use this as a degradation summary rather than a raw performance chart: bars near zero mean the engine preserved most of its small-band behavior, while large negative bars mean the larger graph is causing material fragility. Earlier high-band loss labels indicate that the engine is not only degrading more, but also failing sooner under the larger-population regime.

#### Figure 24

@figure large_population_diffusion_transitions

Representative bounded-state points for the maintained large-population diffusion families. Each panel shows delivery versus estimated reproduction for the best observed collapse, viable, and explosive representatives in that family, making the transition surface visible without scanning the full raw matrix.
Interpret the point positions by quadrant: the useful region is high delivery with bounded reproduction, while low-delivery collapse points and high-`R` explosive points show the two ways an engine can fail at scale. Engines that keep their viable representative well separated from both failure modes are handling larger-population diffusion pressure more cleanly.

### Large-Population Takeaways

- In the high large-pop route-visible bands, the core-periphery family is shared by {scaling_best_engines} at {scaling_high_route} permille, while the high multi-bottleneck family is shared by {bottleneck_best_engines} at {bottleneck_high_route} permille.
- The steepest diameter / fanout drop is `{diameter_sensitive_engine}` at {diameter_delta} permille from the small baseline to the high band, and the steepest multi-bottleneck drop is `{bottleneck_fragile_engine}` at {bottleneck_delta} permille.
- `field` is the clearest route-visible large-population loser: in the high core-periphery band it falls to {core_periphery_field_route} permille, and in the high multi-bottleneck band it reaches only {multi_bottleneck_field_route} permille. `scatter` no longer belongs in that loser bucket on the current route-visible surface, where it stays at {core_periphery_scatter_route} and {multi_bottleneck_scatter_route} permille in those same high bands.
- `mercator` is included in the large-population route-visible comparison as its maintained search-plus-maintenance peer: it reaches {core_periphery_mercator_route} permille in the high core-periphery band and {multi_bottleneck_mercator_route} permille in the high multi-bottleneck band.
- The Field large-population failures are not stale selected-search successes: the high core-periphery Field run has {core_periphery_field_selected_results} current selected-result round, {core_periphery_field_no_candidate} no-candidate reactivation attempts, and {core_periphery_field_inadmissible} inadmissible attempts; the high multi-bottleneck run has {multi_bottleneck_field_selected_results} current selected-result round, {multi_bottleneck_field_no_candidate} no-candidate attempts, and {multi_bottleneck_field_inadmissible} inadmissible attempts. The last active-route blocker still reports `{core_periphery_field_blocker}` / `{multi_bottleneck_field_blocker}`, but the post-loss state is now classified as no viable Field-evidence candidate after support withdrawal rather than a simulator activation gap.
- The combined `pathway-batman-bellman` stack no longer creates a high-band large-population advantage over plain `pathway`: both sit at {multi_bottleneck_pathway_batman_route} / {multi_bottleneck_pathway_route} permille on the current high multi-bottleneck route-visible surface. Its clearer benefit now appears in the multi-flow fairness surface, where maintenance support fills Pathway's shared-broker starvation tail.
- The sparse-threshold high band still shows viable `{sparse_viable}` against explosive `{sparse_explosive}`, the congestion-threshold moderate band separates viable `{congestion_viable}` from collapse `{congestion_collapse}`, the congestion-threshold high band is currently only `{congestion_high_states}`, and the regional-shift high band still spans `{regional_states}`.

## Routing-Fitness Remaining Questions

### Routing-Fitness Introduction

The earlier route-visible and large-population sections were enough to choose a design direction, but not enough to close the remaining fitness-for-purpose questions. Three gaps remained: where explicit search stops being sufficient by itself, what shared-broker contention does to the weakest flow rather than the mean, and whether delayed or asymmetric observations cause the router to cling to dead routes after the ground truth changes.

This section answers those questions directly. The crossover sweep reuses the maintained large-pop route-visible bands as a controlled search-burden versus maintenance-benefit surface. The multi-flow families force several simultaneous objectives through shared brokers. The stale-repair families inject host-specific lag windows so different regions act on delayed topology knowledge even though the underlying ground truth changes on one deterministic schedule.

Across these summaries, route churn is the report-primary cost metric and the route-observation count is retained as a control-activity proxy in the generated CSVs and figure hover data. That keeps the tables compact while preserving one explicit cost surface beyond route presence alone.

### Routing-Fitness Crossover Summary

@table routing_fitness_crossover_summary

Compact crossover view for the remaining route-visible design question. Column guide: Question is the analytical axis, Band is the maintained difficulty band, Engine Set is the standalone routing stack, Route is total-window route presence, Loss is the first-loss round, Churn is mean route churn, and Hop is the active-route hop-count proxy.

### Routing-Fitness Multi-Flow Summary

@table routing_fitness_multiflow_summary

Compact fairness view for the shared-broker families. Column guide: Min and Max are the weakest and strongest per-flow route-presence means, Spread is the gap between them, Starved is the mean count of objectives with zero route presence, Broker P/C/S reports broker participation percent, bottleneck concentration percent, and broker switch count as `participation/concentration/switches`. `no route` means there was no visible route to attribute, while `not next-hop` means the route was visible but the engine does not expose a next-hop route for broker attribution. Live is the mean number of rounds where multiple objectives are simultaneously live, and Churn is mean route churn.

### Routing-Fitness Stale Repair Summary

@table routing_fitness_stale_repair_summary

Compact stale-information repair view. Column guide: Persist is the mean bad-route persistence after the first disruptive topology change, Route is total-window route presence, Unrec. is mean unrecovered-after-loss count, Status distinguishes no-loss, recovered, ordinary unrecovered, pre-disruption-loss, and store-forward-unrecovered cases, Loss is the first-loss round, and Churn is mean route churn. `pre-disruption-loss` means the route was already lost before the stale-topology disruption, so it should not be counted as stale persistence caused by that disruption. Recovery-event success is still exported in the CSV, but it is not used as the headline because many stale scenarios never enter a loss/recovery event path.

### Routing-Fitness Figure Context

These figures isolate the last three decision questions directly: crossover under larger graph pressure, fairness under shared-broker contention, and stale-route persistence under delayed observations. They are meant to be read as envelope charts, not just winner charts.

#### Figure 25

@figure routing_fitness_crossover

Crossover view for the remaining route-visible design boundary. Each panel fixes one analytical question and moves from low to high difficulty. Lines show total-window route presence; first-loss, churn, and route-observation cost remain in the table and figure hover data because recovery events are not present in every crossover band.

#### Figure 26

@figure routing_fitness_multiflow

Multi-flow fairness under shared-broker contention. Each row spans the weakest-to-strongest per-flow route-presence results for one engine set in that family. Narrow spans with high left endpoints are good because they mean the weakest flow still gets service instead of the mean hiding a bad tail.
The broker detail labels summarize how much of the visible next-hop-attributable route activity still traverses tagged brokers and how concentrated that load becomes on the hottest broker. Engines that publish visible routes without a concrete next hop are labeled `not next-hop` instead of being folded into zero broker participation.

#### Figure 27

@figure routing_fitness_stale_repair

Bad-route persistence after delayed or asymmetric observations. Shorter bars are better because they mean the engine stops trusting stale routing state quickly after disruption. Blank persistence for `pre-disruption-loss` rows is intentional: those losses happened before the disruptive stale-topology event and are not evidence of post-disruption stale-route overcommit. The labels show total-window route presence, repair status, and unrecovered counts so the figure separates fast cleanup from cleanup that still leaves the objective unavailable.

### Routing-Fitness Takeaways

- In the high search-burden crossover band, {search_high_engines} lead at {search_high_route} permille total-window route presence.
- In the high maintenance-benefit crossover band, {maintenance_high_engines} lead at {maintenance_high_route} permille total-window route presence.
- Under shared-broker contention, `Shared corridor` is best handled by {shared_corridor_engines} at {shared_corridor_min_route} permille minimum per-flow route presence, while `Detour choice` is best handled by {detour_choice_engines} at {detour_choice_min_route} permille.
- The harshest fairness tail is currently `{worst_starvation_family}`, where `{worst_starvation_engine}` still records {worst_starvation_value} starved objectives on average.
- In the stale-repair surface, `Recovery window` is best handled by {stale_best_engines} at {stale_best_persistence} rounds of stale persistence and {stale_best_route} permille route presence, while the worst stale overcommit is `{worst_stale_family}` under `{worst_stale_engine}` at {worst_stale_persistence} rounds and only {worst_stale_route} permille route presence.
- Taken together, the new evidence says the candidate direction is {routing_fitness_envelope}.

## Part III. Diffusion Calibration

### Diffusion Calibration Introduction

Routing calibration and diffusion calibration are distinct objectives. Routing optimizes for activation, route presence, and recovery. Diffusion optimizes for eventual delivery, boundedness, latency, energy, and leakage.

Diffusion calibration uses integer delivery, coverage, boundedness, latency, resource, and leakage metrics. Field-specific diffusion posture tables are legacy report surfaces and are not emitted by the active routing-analysis report.

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

Best-performing engine set per maintained diffusion family. Column guide: Engine Set, Delivery (fraction of targets reached), Coverage (fraction of reachable nodes), Latency (mean delivery delay), State (boundedness classification), Leak (observer leakage for the selected winner), Max Leak (worst observed leakage and responsible configuration in the same family), Stress.

Here, `collapse` means the engine falls below the basic viability floor for delivery or coverage, while `explosive` means it preserves delivery only by driving reproduction, transmission, storage, or energy cost into an unbounded regime.

### Diffusion Calibration Boundaries

@table diffusion-engine-boundaries

Where each engine set stays viable and where it first collapses or becomes explosive. Column guide: Viable Families, First Collapse, First Explosive.

### Legacy Field Diffusion Regime Calibration

@table field-diffusion-regime-calibration

This table calibrates `field` on its own diffusion success surface instead of only asking whether the generic cross-engine score liked it.

Column guide: Regime names the diffusion posture regime; Success Criteria states what `field` is supposed to optimize there; Configuration is the accepted field diffusion profile for that regime, or an explicit no-acceptable-candidate marker if every field candidate still fails; Posture is the dominant field posture; State is the dominant boundedness class; Transition summarizes either the posture transition count or the first scarcity / congestion transition round; Delivery is mean delivery success; Tx is mean transmission count; Fit is the regime-specific field fitness score. The CSV export also includes protected-budget use, bridge-opportunity capture, cluster-seeding use, coverage-starvation counts, and deterministic suppression counts for the winning profile or best attempt.

### Diffusion Baseline Audit

@table diffusion-baseline-audit

These rows summarize the maintained non-field diffusion baselines. They are representative benchmark configs, not a calibrated-best sweep, so the generic winner tables should be read with that scope in mind.

Column guide: Config is the baseline configuration. Rep is the replication budget. TTL is the time-to-live in rounds. Forward is the forward probability. Bridge is the bridge bias. Delivery, Coverage, and Cluster are mean delivery, coverage, and cluster-coverage scores. State is the boundedness classification.

### Diffusion Winner Sensitivity

@table diffusion-weight-sensitivity

This table re-scores the generic Part IV family winners under delivery-heavy and boundedness-heavy weights. A `no` in Stable means the family-level winner is sensitive to generic weighting and should be read as provisional relative to the regime-specific tables.

Column guide: Family is the diffusion scenario. Balanced, Delivery-Heavy, and Boundedness-Heavy show the winning configuration under each weighting. Stable indicates whether the winner is consistent across all three weightings.

## Part IV. Diffusion Engine Comparison

### Diffusion Analysis Introduction

This part evaluates the calibrated profiles directly. The emphasis shifts from admissibility to how the engines differ when diffusion itself is the comparison surface.

The comparison surface here is regime-aware rather than purely family-by-family. Continuity, scarcity, congestion, privacy, and balanced regimes reward different behaviors, so the first summary table reports the best engine set per regime before the full family matrix.

The generic family-by-family winner table is a representative weighted surface, not a universal truth. Appendix C includes both the maintained non-field baseline audit and a winner-sensitivity table showing where delivery-heavy and boundedness-heavy generic weights keep or change the family winner.

### Diffusion Calibration Detail Note

Detailed diffusion calibration and boundary tables are collected in Appendix C so the main comparison can stay focused on regime winners and the figure-level differences.

### Diffusion Regime Comparison

@table diffusion-regime-engine-summary

Best-performing engine set per diffusion regime. Column guide: Regime, Engine Set, Delivery, Coverage, Cluster Cov. (target-cluster coverage), Tx, State, Score.

### Legacy Field Vs Best Alternative

@table field-vs-best-diffusion-alternative

Best field candidate per diffusion regime against the best non-field alternative under the same regime-aware comparison score. Column guide: Field is the best field attempt, OK reports whether that attempt cleared the field-specific acceptability gate, State / Alt State are boundedness modes, `dDel` / `dCov` / `dClus` are field-minus-alternative delivery, coverage, and target-cluster coverage deltas, `dTx` is transmission delta, and `dScore` is regime-score delta. Negative `dTx` is good for field.

### Diffusion Engine Comparison

@table diffusion-engine-comparison

Full maintained diffusion engine surface. Column guide: Family, Engine Set, Delivery, Coverage, Tx (transmission count), `R_est` (boundedness signal), State (boundedness classification). Use this table to inspect family-level exceptions after the regime summary above.

### Diffusion Figure Context

These two figures separate delivery success from resource boundedness across the most discriminating maintained diffusion families.

#### Figure 28

@figure diffusion_delivery_coverage

Diffusion delivery and coverage by scenario family. Longer bars are better because they indicate more successful delivery; the dot shows coverage, so a wider gap between bar and dot means delivery is lagging behind spread. Strong performers keep both high rather than trading one off against the other.

#### Figure 29

@figure diffusion_resource_boundedness

Diffusion transmission load and boundedness by scenario family. Lower transmission bars are better when delivery remains competitive because they indicate cheaper diffusion. The `R=` and bounded-state annotations show whether that load is staying inside the intended bounded operating regime or drifting toward over-spread.

### Diffusion Appendix Note

Appendix C contains the full diffusion family matrix and the field-versus-best-alternative regime table.

### Diffusion Takeaways

- The diffusion track is an engine comparison, but the regime summary is the right top-level view because continuity, scarcity, congestion, and privacy reward different tradeoffs.
- The regime winners are not `field`-universal: `{balanced_winner}` leads balanced, `{scarcity_winner}` leads scarcity, `{congestion_winner}` leads congestion, and {continuity_privacy_winners} {continuity_privacy_verb} continuity and privacy.
- `field` shows regime specialization without being universal: it is `{field_balanced_status}` in balanced with a regime-score delta of {field_balanced_score_delta}, {field_scarcity_phrase}, {field_privacy_phrase}, {field_continuity_phrase}, and still has `{field_congestion_status}` in congestion.
- One under-represented point is that the balanced regime currently prefers `{balanced_winner}`, which suggests the maintained corpus is rewarding bounded suppression-heavy behavior outside the explicit privacy regime.
- The harsher families are boundary finders: `bridge-drought` tests rare-opportunity carry, `energy-starved-relay` tests efficiency under scarcity, and `congestion-cascade` tests whether broad forwarding remains bounded without starving first-arrival cluster coverage.

### Legacy Field Diffusion Posture

The artifact set exposes posture-aware `field` diffusion behavior:

- In `bridge-drought`, `field` ends with dominant posture `{bridge_drought_posture}` after {bridge_drought_transitions} posture transitions, using {bridge_drought_protected_budget} protected budget units and converting {bridge_drought_bridge_use} of {bridge_drought_bridge_opportunities} protected bridge opportunities.
- In `energy-starved-relay`, `field` ends with dominant posture `{energy_starved_posture}`, first enters scarcity-conservative behavior at round {energy_starved_first_scarcity}, and suppresses {energy_starved_expensive_suppressions} expensive transport attempts.
- In `congestion-cascade`, `field` ends with dominant posture `{congestion_posture}`, first enters congestion control at round {congestion_first_transition}, seeds {congestion_cluster_seed_uses} first-arrival cluster transfers, records {congestion_cluster_starvation} cluster-coverage starvation events, and records {congestion_redundant_suppressions} redundant-forward suppressions plus {congestion_same_cluster_suppressions} same-cluster suppressions.

### Tuning Reference Tables Intro

These tables provide the detailed tuning reference material behind the main recommendation and analysis sections.

### Route-Visible Reference Tables Intro

These tables collect the exhaustive mixed-engine, mixed-engine selected-round breakdown, head-to-head route-visible results, and the remaining routing-fitness summary tables referenced by the main comparison sections.

### Diffusion Reference Tables Intro

These tables hold the exhaustive diffusion calibration and comparison material that supports the shorter regime-level discussion in the main body.

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

#### Legacy Pressure Findings Field Plateau

Field shows a broad viable plateau: route presence={route_present} permille, bootstrap activation={bootstrap_activation} permille, bootstrap upgrade={bootstrap_upgrade} permille at the low-budget point. Separation between configurations comes from lifecycle shape rather than raw route presence.

#### Legacy Engine Section Empty Field

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

#### Engine Section Scatter Best

Scatter separates most clearly in `{family_id}`, where the owner-side runtime surface records handoff {handoff}, constrained occupancy {constrained}, and bridging {bridging} while normalized route presence stays at {route_presence} permille.

#### Engine Section Scatter Closing

The key Scatter contrast is architectural rather than path-optimal: it is the opaque, partition-tolerant baseline that keeps payload custody local and bounded instead of searching a full path or publishing a corridor envelope.

#### Legacy Engine Section Field Best

Field separates where corridor continuity and reconfiguration cost both matter. `{config_id}` keeps route presence at {route_presence} permille while holding continuation shifts to {continuation_shifts}.

#### Legacy Engine Section Field Bootstrap

Corridor-continuity profile: bootstrap activation {activation} permille, hold {hold} permille, narrow {narrow} permille, upgrade {upgrade} permille, withdrawal {withdrawal} permille, degraded-steady occupancy {degraded} permille, service carry-forward {service} permille, asymmetric shift success {shift} permille. Dominant commitment resolution `{commitment}`, last recovery outcome `{outcome}`, continuity band `{band}`, continuity transition `{transition}`, last decision `{decision}`, blocker `{blocker}`.

#### Legacy Engine Section Field Tied

Route presence is close across the tested range. The service-oriented knobs separate configs in continuation-shift count, service carry-forward, and narrowing behavior.

#### Legacy Engine Section Field Replay

The maintained families produce router-visible activation and route presence, with the bootstrap phase directly visible in replay and recovery surfaces.

#### Legacy Engine Section Field Families

The asymmetric-envelope and bridge anti-entropy families test corridor continuity under realization movement. The partial-observability and bootstrap-upgrade families test bootstrap promotion and withdrawal. The service-overlap, freshness-inversion, and publication-pressure families test service-corridor continuity under broader publication, stronger freshness weighting, or earlier narrowing.

#### Legacy Engine Section Field Diagnosis

The service-corridor publication and materialization path is the key enabler for Field's route-visible behavior. The tuning question is which continuity style is preferable: narrower lower-churn publication or broader reselection with more carried-forward optionality.

#### Legacy Recommendation Rationale Empty Field

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

#### Recommendation Rationale Scatter 1

The Scatter recommendation is now driven by maintained runtime behavior as well as route presence. Measured runtime profile: handoff {handoff}, constrained occupancy {constrained}, bridging {bridging}.

#### Recommendation Rationale Scatter 2

The Scatter corpus is still mostly route-flat, so the useful differentiators are custody and regime signals rather than path-optimality. Retained-message peak {retained}, delivered-message peak {delivered}.

#### Legacy Recommendation Rationale Field 1

The Field recommendation is driven by corridor continuity, bootstrap upgrade behavior, and reconfiguration cost together. When route presence is effectively tied, the default choice should favor lower-churn corridor management rather than broader reselection.

#### Legacy Recommendation Rationale Field 2

Measured continuity profile for `{config_id}`: bootstrap activation {activation} permille, hold {hold} permille, narrow {narrow} permille, upgrade {upgrade} permille, withdrawal {withdrawal} permille, degraded-steady occupancy {degraded} permille, service carry-forward {service} permille, asymmetric shift success {shift} permille. Dominant commitment resolution `{commitment}`, last recovery outcome `{outcome}`, continuity band `{band}`, continuity transition `{transition}`, last decision `{decision}`, blocker `{blocker}`.

#### Legacy Recommendation Rationale Field 3

The Field configurations are close in top-line route presence. The continuity profile table is the better place to choose between lower-churn and broader-reselection behavior, and low-churn continuity should remain the default surface unless a regime explicitly needs broader reselection.

#### Legacy Recommendation Rationale Field 4

The corpus includes steady route-visible service continuity and bootstrap-aware corridor behavior. The service regimes make the Field knobs visible in lifecycle metrics even when route presence clusters closely.

#### Limitations And Next Steps

These recommendations are only as good as the simulated regime corpus. A flat curve can mean genuine robustness or that the sweep has not found the most informative failure boundary.

The BATMAN Bellman corpus exposes recoverable transition differences, but asymmetry-plus-bridge families remain hard failures. The BATMAN Classic corpus confirms slower convergence and tight clustering of decay window settings. The Babel corpus shows measurably different behavior under asymmetric conditions, with the FD table visible in partition recovery, but decay window settings do not yet separate sharply. The OLSRv2 corpus separates most clearly on topology propagation, MPR flooding stability, and asymmetric relink timing, but the maintained window sweep is still narrow enough that several settings remain tied on route visibility. The Pathway corpus identifies the minimum viable budget floor with a wide plateau above it.

The Field corpus reaches the route-visible boundary with an explicit bootstrap phase and working service-corridor path, but tested settings cluster tightly. The bridge anti-entropy and bootstrap-upgrade families allow the report to distinguish between underexercise and real weakness. The remaining limitation is that tested settings cluster closely, leaving room for more discriminating future regime design.

The remaining routing-fitness experiments narrow the route-visible decision to one explicit envelope rather than several open questions. Within the tested search-burden, shared-broker, and stale-observation bands, Mercator is now one of the strongest search-plus-maintenance directions, especially in the high maintenance-benefit crossover band. The remaining limitation is extrapolation beyond this maintained envelope, plus the stale-repair rows where the best measured cleanup still does not belong to Mercator.
