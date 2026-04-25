# Certified Kernel Transformation Paper Package

`analysis_2` contains the paper-facing certified temporal kernel
transformation material, including active belief diffusion as the constructive
mechanism, separate from Jacquard's route-visible router analysis.

This package owns:

- `research_boundary.md`, the research boundary and implementation/proof split
  previously kept in `docs/`,
- `text.md`, the manuscript-facing paper text following the outline from
  `work/research_proposal.md`,
- the report generator that assembles paper text, active-belief CSV rows,
  figures, captions, and the reproducibility manifest,
- sanity checks for the generated report artifacts.

Generate the paper report with:

```bash
just active-belief-report
just active-belief-sanity
```

The PDF lands at `artifacts/analysis_2/latest/active-belief-report.pdf`.

The split is intentional: `docs/` remains Jacquard routing and simulator
documentation, while this directory is the extraction boundary for the
certified temporal kernel transformation paper package.

Focused simulator checks for the paper-facing artifacts currently include:

```bash
cargo test -p jacquard-simulator core_experiment
cargo test -p jacquard-simulator experiment_a_landscape
cargo test -p jacquard-simulator experiment_a2_evidence_modes
cargo test -p jacquard-simulator experiment_b_path_free_recovery
cargo test -p jacquard-simulator experiment_c_phase_diagram
cargo test -p jacquard-simulator experiment_d_coding_vs_replication
cargo test -p jacquard-simulator experiment_e_observer_frontier
```

Near-critical and observer-ambiguity fixtures remain part of the paper package:

```bash
cargo test -p jacquard-simulator reproduction_pressure
cargo test -p jacquard-simulator near_critical_controller
cargo test -p jacquard-simulator potential_accounting
cargo test -p jacquard-simulator near_critical_sweep
cargo test -p jacquard-simulator near_critical_artifacts
cargo test -p jacquard-simulator near_critical_theory
cargo test -p jacquard-simulator observer_projection
cargo test -p jacquard-simulator observer_attacker
cargo test -p jacquard-simulator observer_metrics
cargo test -p jacquard-simulator observer_sweep
cargo test -p jacquard-simulator observer_robustness
cargo test -p jacquard-simulator observer_artifacts
```
