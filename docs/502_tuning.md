# Routing Tuning

`jacquard-simulator` includes a maintained tuning harness for BATMAN, Pathway,
and Field. The harness runs deterministic scenario matrices, sweeps maintained
public parameters, writes stable artifacts under `artifacts/tuning/`, and
generates CSV tables plus a PDF report with vector plots through the repo-local
Python, Polars, matplotlib, and ReportLab toolchain. It also includes a
dedicated head-to-head corpus that runs the same regimes under four explicit
engine sets: `batman-bellman`, `pathway`, `field`, and `pathway-batman-bellman`.

## Commands

Run the smaller smoke sweep:

```bash
just tuning-smoke
```

Run the full local sweep and generate the report:

```bash
just tuning-local
```

Regenerate the report for an existing artifact directory:

```bash
just tuning-report artifacts/tuning/local/<run-id>
```

## Matrix Structure

The maintained matrix varies replay-visible regime dimensions rather than only
one aggregate stress score:

- topology and density: `sparse line`, `medium ring`, `medium mesh`, `dense
  mesh`, `bridge cluster`, `high fanout`
- delivery pressure: low, moderate, and high loss
- medium pressure: interference and contention
- directional mismatch: none, mild, moderate, and severe asymmetry
- topology movement: relink, partition, recovery, and cascade partition
- local node pressure: connection-count and hold-capacity limits
- workload class: connected-only, repairable-connected, service, and concurrent
  mixed workloads

The harness writes:

- `runs.jsonl`: one run-level summary per scenario seed and parameter setting
- the generated aggregate summary file: grouped means and maintained field
  metrics
- the generated breakdown summary file: first sustained breakdown boundary per
  config
- `head_to_head_summary.csv`: explicit engine-set comparisons over shared
  regimes
- CSV tables for recommendations, transitions, boundaries, and profile variants
- vector plot assets plus a generated PDF report

## Measured Outputs

The report scores configurations with route-visible metrics and also publishes
transition and boundary tables:

- activation success
- route presence
- first materialization, first loss, and recovery timing
- route churn and engine handoffs
- stress boundary and first breakdown family
- Field-specific replay measures such as selected-result rounds, search
  reconfiguration rounds, protocol reconfiguration counts, continuation shifts,
  and checkpoint restore counts

The default recommendations are intended to be robust centers of acceptable
behavior for this maintained corpus, not one-off winners from a single easy
scenario.

## Current Guidance

For the latest full local artifact set, see:

- `artifacts/tuning/local/20260414-110123/report`
- the generated report in that directory

### BATMAN

The current BATMAN matrix is most informative in recoverable transition
families. Route-presence plateaus alone are too flat, so the maintained report
also looks at stability accumulation, first-loss timing, and failure
boundaries.

Current measured guidance:

- the responsive BATMAN range is still clustered around the short-window
  settings
- `batman-1-1` currently leads the balanced default ranking, with `batman-2-1`
  and `batman-3-1` close behind
- asymmetric bridge breakdown regimes remain hard failures across the tested
  window range, so the recommendation should be read as guidance for
  recoverable pressure rather than impossible bridges

### Pathway

The Pathway matrix still shows a clear minimum-budget boundary:

- budget `1` remains the hard cliff in the maintained service-pressure
  families
- budgets at and above `2` form the viable floor
- `pathway-6-hop-lower-bound` currently leads the balanced default ranking,
  with `pathway-2-hop-lower-bound` and `pathway-4-zero` effectively tied in
  the current corpus

The practical interpretation is unchanged:

- `2` is the minimum viable budget floor
- `3` to `4` remains the sensible default range
- larger budgets need a regime-specific justification, not just the hope that
  more search is always better

### Field

The simulator now includes dedicated Field families, Field replay extraction,
Field-specific CSV columns, and Field plot/report sections. The matrix is able
to observe:

- corridor route-support evolution in the replay surface
- degraded-steady continuity-band entry, recovery, and downgrade timing
- bootstrap activation, hold, narrowing, upgrade, and withdrawal behavior in
  recovery and runtime linkage
- dominant promotion decisions and dominant promotion blockers exported from the
  replay surface
- service-retention carry-forward and asymmetric continuation-shift success
- search and protocol replay metadata
- continuation-shift and reconfiguration counters
- field-favorable comparison regimes

The current simulator boundary now yields a measured route-visible Field
default. In the maintained local corpus, the Field sweep produces non-zero
activation success and route presence at the router-visible route boundary, and
the report currently recommends `field-10-hop-lower-bound` as the balanced
default for this corpus, but that result is effectively tied with the other
tested Field settings.

What this means operationally:

- Field tuning infrastructure is present and maintained
- Field is no longer replay-only in the maintained corpus
- Field bootstrap is an explicit measured phase rather than an implicit lower
  threshold
- degraded-steady continuity is now an explicit measured band rather than an
  inferred “almost bootstrap” state
- the expanded corpus shows that Field was partly underexercised in earlier
  matrices, because dedicated anti-entropy and bootstrap-upgrade families do
  produce route-visible Field behavior
- the current recommendation is still narrower and weaker than the leading
  BATMAN and Pathway defaults
- the main remaining gap is not only coverage but implementation maturity:
  route presence improved once the runtime began degrading before bootstrap and
  carrying service evidence forward more smoothly, but bootstrap-to-steady
  upgrade is still narrower than it should be, so Field still trails the BATMAN
  and Pathway defaults in the maintained corpus

### Mixed Comparison

The comparison regimes remain useful for regime suitability:

- low-loss connected-only cases still favor BATMAN
- concurrent mixed workloads still favor Pathway
- high-loss bridge and some field-favorable comparison cases remain hard
  failure regimes

The comparison section should be read as “which engine family fits this regime
best” rather than “which engine is globally best”.

### Head-To-Head Engine Sets

The maintained report also includes a direct engine-set comparison over the
same regime families. This is separate from the mixed-engine comparison corpus:

- mixed-engine comparison asks which engine wins when several engines are
  available to the same router
- head-to-head comparison asks what happens when the host set is restricted to
  one explicit stack: `batman-bellman`, `pathway`, `field`, or `pathway-batman-bellman`

In the latest local artifact set:

- `batman-bellman` and `pathway-batman-bellman` are strongest in the low-loss connected and
  bridge-transition regimes
- `pathway` is strongest in the concurrent mixed and high-loss connected
  regimes
- `field` now activates and stays present in selected head-to-head families,
  especially `head-to-head-concurrent-mixed`, `head-to-head-connected-low-loss`,
  `head-to-head-partial-observability-bridge`, and
  `head-to-head-corridor-continuity-uncertainty`
- `field` still drops out entirely in the bridge-transition and connected
  high-loss head-to-head families

## Current Limits

The main open tuning limitation is still Field maturity:

- the simulator can extract meaningful Field replay and route-visible Field
  surfaces
- the maintained Field families and comparison families are present
- but the measured Field ceiling is still low, and the first maintained
  breakdown arrives earlier than for the leading BATMAN and Pathway defaults
- the expanded Field-favorable regimes suggest the earlier corpus was partly
  underexercising Field, but they also show a real bootstrap-to-steady
  continuity bottleneck rather than only a missing regime
- the most useful new diagnostic is the split between bootstrap hold, narrow,
  upgrade, and withdraw outcomes: that makes it possible to tell “could not
  activate” from “activated but never strengthened”

That limitation is important enough that the generated report treats Field as
an early route-visible baseline rather than silently presenting it as mature.

## Review Guidance

- prefer the PDF report and CSV tables over a single composite score
- use the transition table to distinguish robust settings from lucky averages
- use the boundary table to see where an engine stops being acceptable
- read Field results as a real but early recommendation surface, not yet as a
  mature peer of BATMAN and Pathway
- rerun the same matrix after meaningful BATMAN, Pathway, Field, router, or
  simulator changes before updating defaults
