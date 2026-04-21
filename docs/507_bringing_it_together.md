# Bringing It Together

This is the capstone guide. It shows how the earlier guides compose into an end-to-end custom experiment: a fully custom host running a custom engine over a custom transport and device profile, driven by a custom suite, reduced through a custom report section. Its purpose is to thread the other guides together so a 3rd party can see how the pieces fit.

A fully custom experiment may replace six components: a routing engine, a transport and link profile, a device and node profile, the client composition, the experiment suite definition, and the report pipeline. Each has a dedicated guide. This capstone references them rather than re-explaining them.

## Assembling The Client

The client composition pattern is already covered in [Client Assembly](503_client_assembly.md). Reuse it here with the custom components in place. A `ClientBuilder`-shaped composition wires the custom engine from [Custom Engine](504_custom_engine.md), the custom transport from [Custom Transport](505_custom_transport.md), and the custom device profile from [Custom Device](506_custom_device.md) into one `ReferenceClient` equivalent.

The current reference `ClientBuilder` expects the in-tree engines and the in-memory transport, so a fully custom stack bypasses it. Compose a router, register the custom engine on that router, and wire the custom transport surfaces into a host bridge directly. The minimum host composition is documented in [Reference Client](407_reference_client.md).

## Driving The Custom Client From The Simulator

Lift the custom stack into the simulator harness by implementing `JacquardHostAdapter`. The adapter's `build_hosts` method returns one host runtime per scenario host. Pass the adapter to `JacquardSimulator::new` instead of the default `ReferenceClientAdapter`. The walkthrough for the host adapter lives under [Running Simulations](501_running_simulations.md) in the "Swapping the Host Adapter" section.

```rust
use jacquard_simulator::{JacquardHostAdapter, JacquardScenario, JacquardSimulator, SimulationError};
use jacquard_core::NodeId;
use std::collections::BTreeMap;

struct CustomStackAdapter {
    // runtime-specific wiring for the custom engine and transport
}

impl JacquardHostAdapter for CustomStackAdapter {
    fn build_hosts(
        &self,
        scenario: &JacquardScenario,
    ) -> Result<BTreeMap<NodeId, jacquard_reference_client::ReferenceClient>, SimulationError> {
        todo!("construct the custom client per host in the scenario")
    }
}

let mut simulator = JacquardSimulator::new(CustomStackAdapter { /* ... */ });
```

The adapter is the only simulator-facing integration point the capstone introduces. Everything else about the simulation flow is identical to [Running Simulations](501_running_simulations.md): scenarios, environment hooks, replay inspection, and assertions all work unchanged.

## Wrapping In An Experiment Suite

Promote the custom scenario into an `ExperimentSuite` to sweep it across seeds and parameter sets. The suite assembly pattern is already documented in [Running Experiments](502_running_experiments.md). Reuse it with the custom `JacquardHostAdapter` instead of the default.

An experiment over a fully custom stack should honor the methodology in [Experimental Methodology](307_experimental_methodology.md). If the custom engine exposes tunable parameters, run a tuning sweep for that engine before any comparative run. Hold the resulting operating point fixed in comparative runs so the contrast measures engines, not tuning.

Artifacts land under `artifacts/analysis/{suite}/{timestamp}/` like the in-tree suites. The Python report pipeline ingests them unchanged because the artifact shape is stable across releases.

## Extending The Report Pipeline

The `analysis/` Python package reads simulator artifacts and assembles the PDF report. `report.py` is the entry. `data.py` loads per-run and aggregate data into Polars frames. `scoring.py` derives per-run metrics. `tables.py` produces CSV tables. `plots.py` produces vector plots. `sections.py` composes report sections.

A 3rd party adding a custom metric or plot for a custom experiment touches three places. Add the metric derivation in `scoring.py`. Add the CSV or plot rendering in `tables.py` or `plots.py`. Add the section layout in `sections.py` and hook it into the report assembly in `report.py`.

```python
# scoring.py
def custom_engine_stability(run):
    return run["my_custom_metric"].mean()

# sections.py
def custom_engine_section(runs):
    return [
        Paragraph("Custom engine stability by regime"),
        custom_engine_stability_table(runs),
    ]
```

The call shape and the data frames follow the conventions already used for the in-tree sections. Read the existing implementations of the BATMAN, Babel, or Pathway sections as living templates.

## Artifact Schema Stability

The Rust simulator guarantees stable schemas for the per-run JSONL logs plus aggregate, breakdown, diffusion aggregate, and diffusion boundary JSON summaries. Schema changes go through explicit versioning. A 3rd party can rely on the shape across patch releases and plan migrations for minor releases.

Model-lane artifacts are additive and subject to looser guarantees. They serve as a validation companion rather than a scoring input. Consumers who need long-term stability of their custom reductions should target the full-stack artifacts first.

For contract rules the simulator and report pipeline live within, see [Crate Architecture](999_crate_architecture.md). For the methodology these artifacts ultimately support, see [Experimental Methodology](307_experimental_methodology.md).
