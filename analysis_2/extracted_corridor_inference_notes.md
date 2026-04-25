# Extracted Corridor-Inference Notes

This note preserves useful ideas removed from the active `analysis/` report
while preparing the paper-facing analysis surface.

The retired corridor-routing report surfaces were useful mostly as diagnostics:

- lifecycle split between weak bootstrap evidence and steady admission
- reconfiguration cost as a first-class control-motion metric
- bounded continuation shifts under asymmetric bridge repair
- service continuity under overlapping or stale candidate evidence
- diffusion posture signals for scarcity, congestion, and observer leakage

For the paper direction, these ideas are better treated as inference-limit
questions than as another route-visible engine comparison. The useful
translation is:

- bootstrap versus steady admission becomes a temporal evidence threshold
- continuation shift count becomes a reconstruction instability measure
- service carry-forward becomes delayed evidence reuse under bounded memory
- observer leakage becomes failed non-reconstructability under projection
- congestion or scarcity posture becomes an explicit resource-bounded inference
  regime

Do not reintroduce these as active `analysis/` report tables unless the paper
needs a separate extraction pipeline. The current `analysis/` package is the
base-router comparison surface and excludes this retired corridor-inference
material.
