# Jacquard Routing: Tuning and Analysis

## Executive Summary

### Executive Summary Intro

This report studies Jacquard routing behavior across three engines, `batman-bellman`, `pathway`, and `field`, using a common simulator corpus and a shared analysis pipeline.

The goal is not only to pick default parameter settings. It is also to understand where each engine works cleanly, where it begins to degrade, what kinds of failures appear first, and how the engines compare when they are placed under the same network regimes.

That matters because routing quality is regime-dependent. A setting that looks strong in an easy connected network may break down under asymmetry, bridge loss, candidate pressure, or uncertainty. The report is therefore designed to connect tuning choices to concrete failure boundaries rather than treating routing as a single scalar benchmark.

The document is organized in two parts. Part I focuses on tuning: recommended configurations, transition behavior, failure boundaries, and the simulator assumptions that shape those results. Part II focuses on analysis: engine-specific behavior for `batman-bellman`, `pathway`, and `field`, followed by mixed-engine and head-to-head comparisons across maintained regimes.

The emphasis throughout is explanatory rather than purely prescriptive. The recommendations are still present, but they are framed by the measured transition surfaces, breakdown points, and comparative regimes that justify them.

## Part I. Tuning

### Recommendation Tables

#### Recommendation Overview

@table recommendation

This table condenses the highest-ranked configurations for each engine family.

Column guide: Score is the composite ranking value for this corpus; Activation is the share of runs that installed a route; Route Presence is the average fraction of rounds with a live route; Max Stress is the highest sustained stress level survived before the first maintained breakdown.

It is a scan-friendly companion to the narrative recommendation sections and should be read together with the transition and boundary tables rather than on its own.

#### Transition Behavior

@table transition-metrics

This table shows how the leading configurations behave over time, not only how they score in aggregate. It is the main place to see whether a recommendation is stable across seeds or only looks good on average.

Column guide: Route Mean is average route presence across runs; Route Stddev is the run-to-run spread of that presence; First Mat. is the first round in which a route appears; First Loss is the first round in which a live route disappears; Recovery is the first later round in which routing returns after a loss; Churn counts route changes or handoffs.

A `-` in this table means the event was not observed in the underlying runs. For example, some configurations never lost a route, never recovered after a loss, or never materialized a route at all.

#### Failure Boundaries

@table boundary-summary

This table makes the breakdown edges explicit by showing how much sustained stress each leading configuration survives before the first maintained failure family appears. It links the recommendation back to a concrete failure boundary instead of leaving the reader with a score only.

Column guide: Max Stress is the highest sustained stress level a configuration clears; First Failed Family is the first regime family that breaks it; Fail Stress is the stress level at that first maintained failure; Reason is the dominant recorded failure mode for that boundary.

A `-` in this table means no maintained failure boundary was observed in this corpus for that configuration, so there is no first failed family, fail stress, or dominant failure reason to report.

### Setup And Method

#### Simulation Setup

Each experiment run uses the Jacquard simulator to play out a fixed network scenario from a known random seed. Fixed seeds keep the runs repeatable, so the same input produces the same result.

A scenario defines the network layout, which routing engines are available on each node, which routing requests are active, and how many rounds the simulator should run.

The simulator can also apply planned changes during a run. For example, it can cut a link, restore a link, degrade one direction of a link more than the other, move a connection to a different neighbor, or impose local resource limits on a node.

The report does not score hidden internal state directly. It scores things a reviewer can observe from the replay output, such as whether a route appears, when it is first lost, whether it later recovers, how often it changes, and what kind of failure was recorded.

#### Matrix Design

The tuning matrix changes one small set of conditions at a time so the effect of each routing setting is easier to understand.

Across the full corpus, the matrix varies network density, message loss, interference, directional asymmetry, topology change, local node pressure, and routing objective type.

The report focuses on boundary cases as well as easy cases. The goal is not only to find settings that work when the network is healthy, but also to see where behavior changes sharply.

For BATMAN Bellman, the main sweep changes the decay-window settings. For Pathway and Field, the main sweep changes per-objective search budget and heuristic mode.

The analysis does not stop at one composite score. It also tracks transition metrics such as first materialization, first loss, recovery timing, churn, and run-to-run spread so the recommendations can distinguish robust settings from lucky averages.

The recommendations are meant to be good default settings for this modeled corpus, not single winners from one lucky scenario.

#### Regime Assumptions

The scenarios are stylized. They are designed to represent common kinds of mesh-network conditions, not to reproduce one exact real-world deployment.

Names like sparse line, medium ring, bridge cluster, and high fanout describe the shape of the network first. Loss and interference settings then make communication easier or harder within that shape.

Some families are intentionally placed near a break point. In those families, a small change in a routing parameter can change whether a route survives, flaps, or fails.

Because of that, the recommendations are most trustworthy for deployments that resemble these modeled families. They become less certain as a deployment moves farther away from those assumptions.

#### Regime Characterization

Topology regimes:

- `sparse line`: nodes mostly depend on a single chain of relays, so there are few alternate ways around a break.
- `medium ring`: nodes have looped connectivity, which allows one route to fail while another route may still exist.
- `medium mesh` and `dense mesh`: several neighbors can often reach the same destination, so contention and search choice matter more than raw reachability.
- `bridge cluster`: two better-connected groups are joined by one narrow bridge, so a single weak link can dominate the whole routing outcome.
- `high fanout`: one node sees many candidate neighbors or service continuations, which is useful for stress-testing search budget.

Condition regimes:

- `low`, `moderate`, and `high loss` describe how often messages are dropped on links.
- `interference` and `contention` describe how crowded the local medium is, which makes delivery less reliable even when links still exist.
- `asymmetry` means one direction of a link is worse than the other, which is especially important for next-hop protocols that depend on bidirectional behavior.
- `churn`, `relink`, `partition`, and `recovery` describe changing topology over time rather than static topology.
- `intrinsic node pressure` means the node itself becomes a bottleneck because it can hold fewer connections or less queued data.

Workload regimes:

- `connected-only` means the request only makes sense if an actual connected route exists.
- `repairable-connected` means the route may be temporarily disrupted but should recover if the network shape improves.
- `service` means the engine is choosing among candidate service locations rather than only driving to one fixed node.
- `concurrent mixed` means multiple route requests of different kinds are active at the same time, so the engines are competing under shared pressure.

#### BATMAN Bellman Algorithm

BATMAN Bellman is the simpler routing engine in this study. It tries to keep track of a good next hop toward a destination instead of planning a full end-to-end path. In practice, that means it works best when local neighbor information is enough to make a good forwarding choice. The settings tuned here control how quickly old information expires and how quickly the engine refreshes its view of the network. The most important questions for BATMAN Bellman are therefore: does it stay stable under loss, does it hold onto stale routes too long, and how quickly does it recover after a link change.

#### Pathway Algorithm

Pathway is the more search-heavy engine in this study. Instead of picking only a next hop, it explores candidate continuations and tries to choose a good full routing decision for the requested destination or service. The main tuning question is how much search budget it needs before it reliably finds good candidates. Too little budget can miss viable options; too much budget may add cost without improving results. The simulator therefore stresses Pathway in scenarios with competing service candidates, churn, and bridge failures.

#### Field Algorithm

Field is the corridor-based engine in this study. It does not only try to keep one exact next hop or one exact path. Instead, it maintains a continuously updated field model, searches over frozen snapshots of that model, and publishes one corridor-style routing claim while allowing the concrete realization to move inside that corridor as conditions change.

Field also has an explicit bootstrap phase. In bootstrap, the engine is allowed to publish a weaker corridor claim when the evidence is coherent but not yet strong enough for steady admission. That bootstrap route can then hold, upgrade to steady state, or withdraw if the corridor collapses.

The main tuning questions are therefore: how much search budget Field needs before it finds stable continuations, how often it has to reconfigure or shift continuation inside a corridor when the network moves near a boundary, and how often bootstrap routes successfully upgrade instead of withdrawing.

#### Analytical Approach

The report is organized around three practical questions: where an engine works comfortably, where it begins to degrade, and where it stops being acceptable.

The BATMAN Bellman families emphasize next-hop maintenance under loss, asymmetry, and relink pressure. The Pathway families emphasize service selection and explicit search under candidate pressure. The Field families emphasize corridor continuity, search reconfiguration, and continuation shifts under uncertain or changing evidence.

The plots put the tuned parameter directly on the x-axis. That makes it easier to see whether a result is a real trend or only an average over unrelated cases.

The transition metrics table complements the plots by showing time-oriented behavior for the leading settings: how quickly a route appears, how variable route presence is across seeds, when routes are first lost, and whether recovery occurs.

The boundary summary table captures the first maintained failure edge for each leading setting. That makes the recommendation easier to justify than a score alone because it shows which family actually causes the breakdown.

The comparison section is separate on purpose. Its job is to show when one engine is a better fit for a kind of workload, not to collapse everything into one global winner.

#### Recommendation Logic

The recommendation score is only a guide for ranking configurations.

It rewards settings that activate routes reliably, keep routes present for more of the run, tolerate harder stress levels, and, for BATMAN Bellman, maintain stronger stability totals.

It penalizes settings that cause frequent route churn, maintenance failures, lost reachability, or long degraded periods.

The report also publishes profile-specific recommendations so the same corpus can support different operational priorities. Conservative profiles weight stability and failure avoidance more heavily, while service-heavy or aggressive profiles tolerate more risk in exchange for broader coverage or search performance.

For Field, the scoring also has to be read alongside the bootstrap metrics, because a configuration that activates often but withdraws bootstrap corridors too quickly is not actually a strong default.

When several nearby settings score about the same, the report prefers the middle of the acceptable range rather than the most aggressive edge setting.

#### Profile Recommendation Logic

These profile recommendations provide alternative defaults for operators who care more about stability, stress tolerance, or service-heavy workloads than about one balanced default.

Each profile reuses the same simulator corpus but changes the ranking weights so the table answers a different operational question without changing the underlying evidence.

They are meant to show robust centers of acceptable behavior, not to overfit one narrow regime.

#### Profile Recommendation Logic Empty

No profile-specific recommendations are available for this artifact set.

#### Profile Recommendations

@table profile-recommendations

Column guide: Profile is the ranking policy being applied; Score is the profile-weighted composite value; Activation is the share of runs that installed a route; Route is average route presence; Max Stress is the highest sustained stress level survived under that profile.

## Part II. Analysis

### Figure Context

#### BATMAN Bellman Transition Analysis

This part of the BATMAN Bellman analysis asks how the protocol behaves near transition pressure rather than in easy steady-state cases.

The main questions are whether a decay-window choice preserves stable next-hop behavior as asymmetry and relink pressure increase, and whether that same choice delays route loss once the regime begins to break down.

The two BATMAN Bellman plots should therefore be read as one analytical pair: the first shows where stability accumulates across the transition families, and the second shows when those same settings first lose a route.

#### Figure 1

@figure batman_bellman_transition_stability

This plot uses the swept BATMAN Bellman axis directly: stale-after ticks on the x-axis, with transition-family lines showing accumulated stability. Small point annotations mark the paired refresh setting for each configuration.

#### Figure 2

@figure batman_bellman_transition_loss

This plot shows when routes are first lost under the same transition families. It is the clearest view of whether a shorter or longer decay window helps near relink and asymmetric bridge boundaries.

#### Pathway Budget Figures Intro

These Pathway figures show the budget question from two angles. The first asks how much route presence extra budget buys; the second asks where activation collapses outright. Read together, they explain why the report prefers the low stable floor rather than the largest budget.

#### Figure 3

@figure pathway_budget_route_presence

This plot focuses on the Pathway pressure families rather than all families at once. It shows where extra budget buys additional route presence under high-fanout and bridge-pressure service selection.

#### Figure 4

@figure pathway_budget_activation

This plot shows whether low budgets fail immediately and where heuristic choice changes the activation floor. It is the clearest view of minimum viable Pathway search breadth in this tuning corpus.

#### Field Corridor Figures Intro

These Field figures are the main diagnostic pair. The first shows how much route-visible continuity Field can maintain across the corridor-oriented families. The second shows how much search and continuation churn accompanies that behavior. Together they distinguish a healthy corridor default from a merely active but unstable bootstrap regime.

#### Figure 5

@figure field_budget_route_presence

This plot shows how Field's route-visible success changes with budget across the corridor-oriented families. It is the main budget-floor view for Field.

#### Figure 6

@figure field_budget_reconfiguration

This plot combines continuation shifts and search reconfiguration rounds into one Field-native reconfiguration load signal. Lower values indicate less corridor churn under pressure.

#### Figure 7

@figure comparison_dominant_engine

This comparison plot shows which engine dominates in the maintained mixed-engine comparison families. It is the clearest regime split between BATMAN-Bellman-favored and Pathway-favored workloads in this tuning corpus.

#### Figure 8

@figure head_to_head_route_presence

This figure compares explicit engine sets over the same regime families. It is the clearest direct comparison between `batman-bellman`, `pathway`, `field`, and the combined `pathway-batman-bellman` stack.

### Comparison And Head-To-Head

#### Mixed-Engine Regime Split

@table comparison-summary

This table restates the dominant engine per maintained comparison family.

It makes the regime split easier to scan than the bar chart alone.

Column guide: Dominant Engine is the best-performing engine in that family; Activation is the share of runs that installed a route; Route Presence is the average fraction of rounds with a live route; Stress is the sustained stress level for that family.

#### Head-To-Head Results

@table head-to-head-summary

This table compares explicit engine sets on the same regime families: `batman-bellman`, `pathway`, `field`, and `pathway-batman-bellman`.

It should be read as a direct stack-to-stack comparison rather than as the all-engines router outcome.

Column guide: Engine Set is the only routing stack enabled in that regime; Activation is the share of runs that installed a route; Route is average route presence; Dominant is the engine that contributed the winning route inside that stack; Stress is the sustained stress level for the regime.

A `-` in this table means the underlying run set never produced a route-visible winner or comparable event for that cell, so there is no measured value to summarize.

#### Head-To-Head Regimes

The head-to-head regimes are a compact direct-comparison subset of the larger simulator corpus.

- `connected-low-loss`: an easy connected route where all engines should be able to establish some route and the main question is efficiency under light pressure.
- `connected-high-loss`: a repairable connected route over a lossy bridge where the stack has to keep routing alive under heavy delivery pressure.
- `bridge-transition`: a bridge that degrades, partitions, and later restores, which exposes recovery and replacement behavior.
- `partial-observability-bridge`: a bridge case seeded with Field bootstrap summaries so corridor-style routing can compete under incomplete evidence.
- `corridor-continuity-uncertainty`: a bridge case with intermittent degradation and restoration designed to reward corridor continuity under uncertainty.
- `concurrent-mixed`: several active objectives on the same host set, which tests how each stack behaves when the workload is mixed rather than single-purpose.

#### Head-To-Head Findings Intro

The head-to-head matrix runs the same regime families under four explicit engine sets: `batman-bellman`, `pathway`, `field`, and `pathway-batman-bellman`.

These rows answer a different question from the all-engines comparison corpus. They show what each stack does when it is the only available routing surface for that host set.

#### Head-To-Head Findings Empty

No head-to-head summary is available for this artifact set.

### Data-Driven Templates

#### Pressure Findings Batman Plateau

BATMAN Bellman shows a broad plateau in easy regimes, so this report measures transition families on stability accumulation and loss timing.

#### Pressure Findings Batman Separation

BATMAN Bellman separates mainly in the transition families, where relink pressure and asymmetric bridge degradation expose different stability and loss timings.

#### Pressure Findings Pathway Cliff

Pathway query budget 1 fails immediately, and the high-fanout and bridge-pressure families test that budget plateau under stronger candidate competition.

#### Pressure Findings Field Plateau

Field currently shows a flat low route-presence plateau across the swept budgets: route presence={route_present} permille, bootstrap activation={bootstrap_activation} permille, and bootstrap upgrade={bootstrap_upgrade} permille at the low-budget point.

#### Engine Section Empty Field

No measured Field recommendation is available for this artifact set.

The current simulator matrix does extract Field replay, search, reconfiguration, and bootstrap signals, including corridor support, selected-result presence, bootstrap activation or upgrade behavior, continuation shifts, and protocol or route-bound reconfiguration counts.

But those signals still do not close the boundary to a stable bootstrap-to-steady route-visible default in the maintained Field families, so the report does not publish a measured Field default from this corpus.

#### Engine Section Empty Generic

No {engine_family} recommendation is available for this artifact set.

#### Engine Section Recommended

Recommended configuration: `{config_id}` (score={score}, activation={activation} permille, route presence={route_presence} permille, max sustained stress={max_stress}).

#### Engine Section Batman Bellman Plateau

The BATMAN Bellman transition families remain mostly flat on accumulated stability, which suggests a plateau rather than one narrow best setting in those cases.

#### Engine Section Batman Bellman Best

The BATMAN Bellman transition families separate most clearly at `{config_id}`, which yields stability-total {stability_total} and route presence {route_presence} permille.

#### Engine Section Batman Bellman Closing

Severe asymmetric bridge loss remains a breakdown regime across the tested BATMAN Bellman window range.

#### Engine Section Pathway Cliff

Pathway budget 1 is the clear cliff edge: activation={activation} permille.

#### Engine Section Pathway Floor

Budgets at and above `{config_id}` form the stable floor, and the high-fanout family is the clearest place where additional budget matters.

#### Engine Section Field Best

Field separates most clearly where corridor continuity and reconfiguration cost both matter. `{config_id}` keeps route presence at {route_presence} permille while holding continuation shifts to {continuation_shifts}.

#### Engine Section Field Bootstrap

Its corridor-continuity profile in the maintained corpus is bootstrap activation {activation} permille, hold {hold} permille, narrow {narrow} permille, upgrade {upgrade} permille, withdrawal {withdrawal} permille, degraded-steady occupancy {degraded} permille, service carry-forward {service} permille, and asymmetric shift success {shift} permille. The dominant commitment resolution is `{commitment}`, the dominant last recovery outcome is `{outcome}`, the dominant continuity band is `{band}`, the dominant continuity transition is `{transition}`, the dominant last decision is `{decision}`, and the dominant blocker is `{blocker}`.

#### Engine Section Field Tied

All swept Field configurations are effectively tied in this corpus. That means the current limit is not mainly a budget or heuristic choice inside the tested range; it is the weak bootstrap-to-steady continuity boundary itself.

#### Engine Section Field Replay

Field is no longer replay-only in this corpus: the maintained families now produce real router-visible activation and route presence, and the bootstrap phase is directly visible in replay and recovery surfaces.

#### Engine Section Field Families

The asymmetric-envelope and bridge anti-entropy families are the clearest places to see whether Field can keep one corridor alive while moving its realization inside that corridor, while the partial-observability and bootstrap-upgrade families are the clearest places to see whether bootstrap routes upgrade or withdraw.

#### Engine Section Field Diagnosis

The expanded corpus therefore changes the diagnosis. Earlier results left open the possibility that Field was mostly underexercised; the current results show that underexercise was only part of the story, because bootstrap still stalls even in families designed to favor corridor continuity.

#### Recommendation Rationale Empty Field

No Field recommendation rationale is published because the current corpus does not produce a stable bootstrap-to-steady route-visible Field default.

The report still includes Field because the analysis stack is present and informative: the simulator can observe corridor support, bootstrap activation or upgrade, search reconfiguration, continuation shifts, and checkpoint or recovery lineage.

The blocker is specifically the gap between replay-visible Field evidence and a stable route-visible bootstrap-to-steady materialization path. Until that closes, Field tuning should be read as diagnostic instrumentation rather than default selection.

#### Recommendation Rationale Empty Generic

No {engine_family} recommendation rationale is available.

#### Recommendation Rationale Primary

Primary recommendation: `{config_id}` with mean score {score}.

It combines activation {activation} permille, route presence {route_presence} permille, and max sustained stress {max_stress}.

#### Recommendation Rationale Runner Up

The next closest configuration is `{config_id}` with a score gap of {score_gap}.

#### Recommendation Rationale Small Gap

That small gap means the result should be read as an acceptable range rather than a single brittle optimum.

#### Recommendation Rationale Large Gap

That larger gap means this tuning corpus is finding a real preferred point, not only a shallow plateau.

#### Recommendation Rationale Batman Bellman 1

The BATMAN Bellman recommendation is driven mainly by how well each setting behaves in the recoverable transition families, not by easy-regime route presence alone.

#### Recommendation Rationale Batman Bellman 2

The severe asymmetric bridge regime fails across the entire tested BATMAN Bellman window range, so the recommendation should be read as guidance for recoverable pressure, not impossible bridges.

#### Recommendation Rationale Pathway 1

The main justification is the hard low-budget cliff: `pathway-1-zero` fails in the pressure families, while higher budgets plateau quickly.

#### Recommendation Rationale Pathway 2

The recommendation therefore chooses the lowest stable floor or a nearby central value, rather than spending more search budget after the curve flattens.

#### Recommendation Rationale Field 1

The Field recommendation is driven by corridor continuity, bootstrap upgrade behavior, and reconfiguration cost together. A setting that keeps route presence high but thrashes continuation shifts, route-bound reconfiguration, or bootstrap withdrawal is not treated as a good default.

#### Recommendation Rationale Field 2

The current measured continuity profile for `{config_id}` is bootstrap activation {activation} permille, hold {hold} permille, narrow {narrow} permille, upgrade {upgrade} permille, withdrawal {withdrawal} permille, degraded-steady occupancy {degraded} permille, service carry-forward {service} permille, and asymmetric shift success {shift} permille. The dominant commitment resolution is `{commitment}`, the dominant last recovery outcome is `{outcome}`, the dominant continuity band is `{band}`, the dominant continuity transition is `{transition}`, the dominant last decision is `{decision}`, and the dominant blocker is `{blocker}`.

#### Recommendation Rationale Field 3

The recommendation therefore acts more as a representative baseline than a sharply tuned optimum. In this artifact set, the Field configurations are essentially tied, so the recommendation is not evidence that one tested budget solved the core problem.

#### Recommendation Rationale Field 4

This is a measured but still narrow recommendation: the current Field default activates bootstrap routes in part of the corpus, yet upgrades are almost absent and route presence stays low even in the anti-entropy and bootstrap-upgrade families. That points to a real implementation bottleneck, not only a missing regime.

#### Limitations And Next Steps

These recommendations are only as good as the simulated regime corpus.

A flat curve can mean either genuine robustness or that the sweep has not landed exactly on the most informative failure boundary.

The BATMAN Bellman corpus exposes recoverable transition differences, but the asymmetry-plus-bridge families remain hard failures rather than nuanced separation regions.

The Pathway corpus clearly identifies the minimum viable budget floor, but it also shows a wide plateau above that floor.

The Field corpus now reaches the route-visible boundary with an explicit bootstrap phase, but its route-presence ceiling is still low and its first maintained breakdown arrives earlier than for the leading BATMAN Bellman and Pathway defaults.

The bridge anti-entropy and bootstrap-upgrade families make the corpus more favorable to Field than a simple bridge-failure matrix, so the report can distinguish between underexercise and real weakness more cleanly.

The current verdict is mixed: earlier matrices were underexercising Field, but the expanded corpus still shows Field underperforming at bootstrap-to-steady continuity. Field therefore needs implementation work more than additional budget sweeps inside the current parameter range.

The recommendations should therefore be treated as measured defaults for this tuning corpus, not as universal claims about every future deployment.
