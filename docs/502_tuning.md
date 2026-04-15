# Routing Tuning

`jacquard-simulator` includes a maintained tuning harness for all seven in-tree
engines. The harness runs deterministic scenario matrices, sweeps maintained
public parameters, writes stable artifacts under `artifacts/analysis/`, and
generates CSV tables plus a PDF report with vector plots through the repo-local
Python, Polars, Altair, and ReportLab toolchain. It also includes a
dedicated head-to-head corpus that runs the same regimes under explicit
engine sets: `batman-bellman`, `batman-classic`, `babel`, `olsrv2`, `scatter`,
`pathway`, `field`, and `pathway-batman-bellman`.

The harness also emits a companion diffusion-oriented corpus in the same
artifact directory. That second track models mobility-driven contacts,
message persistence, bounded replication, resource cost, and observer leakage
for partition-tolerant delivery scenarios.

## Design Setting

The maintained corpus is designed for disrupted and mobility-driven mesh
environments. In that setting, end-to-end paths are often absent. Connectivity
appears through short contact windows, weak bridges, and repeated partial
recovery rather than through one stable connected graph. Nodes are also
resource-constrained, so routing quality depends on bounded state, bounded
work, and disciplined use of transmissions and custody.

The route-visible matrix gives useful evidence for this setting because it
stresses the conditions that determine whether a router-facing engine remains
usable at all. The maintained families vary bridge pressure, asymmetry, loss,
relink events, partitions, recovery, contention, and local node pressure.
Those are the same forces that determine whether a proactive engine keeps a
route, whether a search-driven engine finds one, and where each approach
breaks down.

The diffusion track adds the second half of the picture. It models cases where
movement is the transport mechanism and messages must persist across
disconnection. Its mobility-driven contacts, bounded replication, energy and
transmission accounting, storage utilization, and observer-leakage measures
give insight into whether a deferred-delivery policy remains viable in the
population-level setting described above, not only in easy connected regimes.

## Commands

Run the smaller smoke sweep:

```bash
cargo run --bin tuning_matrix -- smoke
```

Run the full local sweep and generate the report:

```bash
cargo run --bin tuning_matrix -- local
```

Regenerate the report for an existing artifact directory:

```bash
nix develop --command python3 -m analysis.report artifacts/analysis/local/latest
```

The local report is written to `artifacts/analysis/{suite}/latest/report.pdf`.
On `main`, GitHub Pages also publishes the latest CI-built routing report PDF
under the docs site root.

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
- `diffusion_runs.jsonl`: one run-level summary per diffusion scenario seed and
  policy setting
- the diffusion aggregate summary file: grouped means for delivery, coverage,
  transmissions, energy, boundedness, and leakage metrics
- the diffusion boundary summary file: per-policy viability, collapse, and
  overload boundaries across maintained diffusion families
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

The diffusion track adds a second set of metrics that are intentionally not
route-centric:

- delivery probability
- delivery latency
- coverage
- total transmissions
- energy per delivered message
- storage utilization
- estimated reproduction number
- corridor persistence
- observer leakage
- boundedness state (`collapse`, `viable`, `explosive`)

## Current Guidance

For the latest artifact set, run:

```bash
cargo run --bin tuning_matrix -- local
```

The report is generated automatically at
`artifacts/analysis/local/latest/report.pdf`.
On `main`, the latest CI-built copy is also published with the docs site.

### BATMAN Bellman

The BATMAN Bellman matrix is most informative in recoverable transition
families. Route-presence plateaus alone are too flat, so the report also looks
at stability accumulation, first-loss timing, and failure boundaries.

The responsive range clusters around the short-window settings.
`batman-bellman-1-1` leads the balanced default ranking, with
`batman-bellman-2-1` and `batman-bellman-3-1` close behind. Asymmetric bridge
breakdown regimes remain hard failures across the tested window range, so the
recommendation should be read as guidance for recoverable pressure rather than
impossible bridges.

### BATMAN Classic

BATMAN Classic converges more slowly than BATMAN Bellman due to its echo-only
bidirectionality and lack of bootstrap shortcut. The tested decay window
settings cluster tightly. The recommendation reflects the spec-faithful model's
need for larger windows to allow receive-window accumulation.

### Babel

Babel separates most clearly in the asymmetry-cost-penalty family, where the
bidirectional ETX formula produces measurably different route selection. The
partition-feasibility-recovery family shows the FD table's bounded
infeasible-fallback window. Decay window settings do not yet separate sharply,
suggesting the FD table and seqno refresh interval dominate convergence timing.

### Pathway

The Pathway matrix shows a clear minimum-budget boundary:

- budget `1` remains the hard cliff in the maintained service-pressure families
- budgets at and above `2` form the viable floor
- `pathway-4-zero` and `pathway-4-hop-lower-bound` lead the balanced default
  ranking

The practical interpretation: `2` is the minimum viable budget floor, `3` to
`4` is the sensible default range, and larger budgets need a regime-specific
justification.

### Field

The simulator includes dedicated Field families, Field replay extraction,
Field-specific CSV columns, and Field plot/report sections. The matrix
observes:

- corridor route-support evolution in the replay surface
- degraded-steady continuity-band entry, recovery, and downgrade timing
- bootstrap activation, hold, narrowing, upgrade, and withdrawal behavior
- dominant promotion decisions and dominant promotion blockers
- service-retention carry-forward and asymmetric continuation-shift success
- search and protocol replay metadata
- continuation-shift and reconfiguration counters
- field-favorable comparison regimes

The Field sweep produces non-zero activation success and route presence at the
router-visible route boundary. The tested Field settings cluster closely, so
the recommendation should be read as a viable range rather than one sharply
preferred point. The continuity profile table is the better place to choose
between lower-churn and broader-reselection behavior.

### Mixed Comparison

The comparison regimes are useful for regime suitability:

- low-loss connected-only cases favor the distance-vector stacks
- concurrent mixed workloads favor Pathway
- high-loss bridge and some field-favorable comparison cases remain hard
  failure regimes

The comparison section should be read as "which engine family fits this regime
best" rather than "which engine is globally best".

### Head-To-Head Engine Sets

The report includes a direct engine-set comparison over the same regime
families. This is separate from the mixed-engine comparison corpus:

- mixed-engine comparison asks which engine wins when several engines are
  available to the same router
- head-to-head comparison asks what happens when the host set is restricted to
  one explicit stack: `batman-bellman`, `batman-classic`, `babel`, `olsrv2`,
  `pathway`, `field`, or `pathway-batman-bellman`

## Review Guidance

- prefer the PDF report and CSV tables over a single composite score
- use the transition table to distinguish robust settings from lucky averages
- use the boundary table to see where an engine stops being acceptable
- rerun the same matrix after meaningful engine, router, or simulator changes
  before updating defaults
