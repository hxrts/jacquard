# DualTide Split Parity

The migrated research engine is no longer an in-tree Jacquard engine. DualTide is the canonical repository for the engine, LaTeX paper package, Rust research surfaces, and Lean theorem boundary.

The parity point is Jacquard commit `0607d3b` and DualTide commit `5400555`. At this point Jacquard keeps the deterministic multi-engine runtime, router contract, reference-client bridge, simulator, general reports, and historical report-reader compatibility fields. DualTide owns carrier objects, certificate evidence, extraction `rho`, effective Fisher volume, supported transformation, and the paper-facing theorem map.

Jacquard docs list seven in-tree engines in [Routing Engines](303_routing_engines.md). The reference client exposes pathway, batman-bellman, batman-classic, babel, olsrv2, scatter, and mercator as selectable in-tree engines.

Historical report schemas may still contain `field_*` columns. Those names are compatibility fields for preserved `analysis/` and `analysis_2/` artifacts. They are not an in-tree engine surface.

Future work on the migrated research engine belongs in DualTide. Jacquard changes should be limited to explicit integration boundaries, historical artifact readers, or shared runtime contracts that remain engine-neutral.
