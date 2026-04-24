# Coded Diffusion Research Boundary

This page is the active implementation boundary for the coded-diffusion research reset initially hosted inside `jacquard-field`.

The research contribution is not named Jacquard or Field. Jacquard is the deterministic implementation framework, and Field is the reusable experimental engine workspace. The result should remain name-independent so it can be split out later without inheriting Jacquard's routing-engine identity or Field's old corridor-routing framing.

## Active Research Direction

The active direction is resource-bounded diffusion-coded inference in temporal networks.

The semantic center is:

- message identity,
- fragment identity,
- coding width and k-of-n reconstruction,
- independent receiver rank,
- fragment custody,
- innovative versus duplicate arrivals,
- diffusion pressure,
- storage pressure,
- rank deficit,
- observer-visible fragment movement,
- reconstruction quorum.

The initial Rust research boundary is `crates/field/src/research.rs`. It defines the coded-diffusion vocabulary used by new work:

- `DiffusionMessageId`
- `DiffusionFragmentId`
- `CodingWindow`
- `FragmentCustody`
- `ReceiverRankState`
- `ReconstructionQuorum`
- `DiffusionPressure`
- `FragmentSpreadBelief`
- `DiffusionOrderParameters`
- `NearCriticalControlState`
- `FragmentRetentionPolicy`
- `DelayedFragmentEvent`
- `FragmentReplayEvent`
- `PrivateProtocolRole`

The initial Lean theorem boundary is `verification/Field/CodedDiffusion.lean`, imported as `Field.CodedDiffusion`. It owns the first proof-facing placeholders for k-of-n reconstruction, duplicate non-inflation, observer projection, and diffusion-potential accounting.

## Legacy Field Baseline

`docs/406_field_routing.md` is legacy context. It documents the old corridor-envelope Field engine that still exists as a runnable baseline.

The legacy baseline may still compile and run for comparison:

- corridor-envelope route candidate generation,
- private Telltale route search,
- route admission and materialization,
- bootstrap and continuity maintenance,
- route-shaped replay fixtures,
- reference-client and simulator profiles that still instantiate `FieldEngine`.

The baseline is not the research contribution. New research code should not depend on planner admission, route search, selected private paths, route-quality ranking, or corridor publication.

## Removed Or Renamed From The Research Path

The reset removed active research-facing corridor terminology from the coded-diffusion path:

- old route/search/replay exports are grouped under `jacquard_field::baseline`,
- replay narrowing counters use continuity-facing names,
- simulator diffusion forwarding uses `ContinuityBiased` instead of corridor-aware naming,
- diffusion reuse and persistence metrics use continuity and cluster-pair naming,
- Router/Search/Quality Lean packs are marked as legacy baseline context,
- Field corridor docs are marked baseline-only.

Some compatibility re-exports remain at the crate root while downstream simulator and reference-client code is converted. Those exports are compatibility surface, not the active research interface.

## Implementation, Proof, And Experiment Split

Implementation work lives initially in `jacquard-field` because it already has deterministic runtime, observer, control, replay, retention, and private protocol scaffolding.

Proof scaffolding lives under `verification/Field`. The active coded-diffusion proof entry point is `Field/CodedDiffusion.lean`; reusable support remains in Information, Model, Retention, Async, and Protocol modules after their statements are converted to fragment/reconstruction semantics.

Experimental evaluation lives in `jacquard-simulator` and the analysis pipeline. Field's old corridor baseline remains useful as a comparator, but the experimental metrics for the research path should report fragment spread, reconstruction progress, duplicate pressure, storage pressure, and diffusion-potential behavior.

## Hard Boundary

New coded-diffusion research code must use the research boundary and may only use legacy Field route machinery as an explicit baseline comparator. If a route-centered surface is retained, it must be documented as baseline-only or converted to fragment/reconstruction semantics before it is used in the research path.
