# Jacquard

Adaptive routing for ad hoc shaped networks. Jacquard's world model and shared routing contract support layered engines with common route objects. First party support provided for several common routing strategies, with an extension boundary for external routing engines. Routing state is fully deterministic, enabling exact simulation replay.

## Analysis

A deterministic simulator runs scenario matrices across all included routing engines. A generated [report](https://hxrts.com/jacquard/reports/router-tuning-report.pdf) covers per-engine recommendations, failure boundaries, cross-engine comparisons, and diffusion calibration.

## Development

```sh
# enter dev shell
nix develop

# build workspace
just build
```
