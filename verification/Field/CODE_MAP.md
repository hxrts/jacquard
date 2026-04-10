# Field Verification Code Map

This map describes the current organization of `verification/Field`.

## Top-Level Theorem Packs

- `Field/LocalModel.lean`
  - imports the local observer-controller model, the finite-belief information layer, and the first decision procedure
- `Field/PrivateProtocol.lean`
  - imports the reduced private choreography/runtime layer and the Telltale-shaped protocol bridge
- `Field/Boundary.lean`
  - imports the observational controller-boundary theorems
- `Field/Adequacy.lean`
  - imports the Rust-runtime adequacy bridge, low-level runtime-to-canonical alignment theorems, and stronger projected runtime/system refinement theorems
- `Field/Network.lean`
  - imports the reduced finite network layer and its first safety theorems
- `Field/Router.lean`
  - imports the reduced publication, admission, installation, lifecycle, and canonical-selection layers
- `Field/Async.lean`
  - imports the reduced async delivery semantics, transport lifecycle lemmas, and first async safety theorems
- `Field/System.lean`
  - imports system-level summaries, reduced end-to-end semantics, convergence theorems, and refinement to router-owned canonical selection above the async layer
- `Field/Quality.lean`
  - imports the reduced routing-quality / comparison, reference-best, and support-only refinement layer above the router and system boundaries
- `Field/Field.lean`
  - umbrella import for the whole current field verification stack

## Local Model

- `Field/Model/API.lean`
  - semantic state vocabulary, abstract round-step operations, boundedness/harmony laws
- `Field/Model/Instance.lean`
  - first bounded concrete realization, structural theorems, temporal theorems, and first quantitative ranking law
- `Field/Model/Decision.lean`
  - one-step finite exploration / decision procedure over a small evidence alphabet

## Information Layer

- `Field/Information/API.lean`
  - abstract probability-simplex style normalization and information-theoretic operations over `FiniteBelief`
- `Field/Information/Instance.lean`
  - first concrete probability-simplex belief object, weight-normalized distribution, and entropy/mass theorems
- `Field/Information/Blindness.lean`
  - field-side information-cost / blindness bridge over the normalized public projection, including a first erasure theorem

## Private Protocol

- `Field/Protocol/API.lean`
  - reduced protocol roles, labels, machine state, global choreography, abstract projection/step/export laws
- `Field/Protocol/Instance.lean`
  - first reduced summary-exchange instance
- `Field/Protocol/Bridge.lean`
  - Telltale-shaped reduced protocol-machine fragment and replay/observer bridge
- `Field/Protocol/Conservation.lean`
  - field-side conservation pack for evidence, authority, and replay-equivalent fragment traces, with direct-family instantiations kept separate from remaining local glue
- `Field/Protocol/Coherence.lean`
  - reduced updated-edge / incident-edge / unrelated-edge coherence lemmas
- `Field/Protocol/ReceiveRefinement.lean`
  - first typed receive-refinement hook aligned to `Consume` / subtype-replacement shape
- `Field/Protocol/Reconfiguration.lean`
  - fixed-participant audit note proving the current reduced protocol has no reconfiguration semantics

## Boundary And Adequacy

- `Field/Model/Boundary.lean`
  - controller-evidence boundary from protocol exports and traces
- `Field/Adequacy/API.lean`
  - abstract Rust-runtime artifact boundary, reduced router-facing runtime projection, and reduced runtime-to-trace simulation witness
- `Field/Adequacy/Canonical.lean`
  - runtime-to-canonical refinement theorems connecting extracted runtime lifecycle routes to the system/router canonical selector under an explicit alignment boundary
- `Field/Adequacy/Projection.lean`
  - reduced runtime artifact projection generated from `systemStep`, admission/honesty lemmas for that projection, and stronger runtime/system canonical refinement theorems with no extra alignment hypothesis
- `Field/Adequacy/Instance.lean`
  - first concrete runtime extraction, execution-level observational trace theorem, reduced simulation theorem, router-projection honesty facts, and evidence-agreement theorems
- `Field/Assumptions.lean`
  - packaged proof-contract assumptions for semantic, protocol-envelope, runtime-envelope, and optional strengthening theorems, including reduced-quality, support-only refinement, canonical-router refinement, runtime-to-canonical refinement, projected runtime/system refinement, and non-optimality boundaries

## Network And Router

- `Field/Network/API.lean`
  - finite node/destination vocabulary, synchronous round buffer, delivered-message view, and local-harmony lift
- `Field/Network/Safety.lean`
  - first reduced network safety theorems connecting local honesty to publication, admission, and installation
- `Field/Router/Publication.lean`
  - router-facing publication candidates and publication honesty / well-formedness theorems
- `Field/Router/Admission.lean`
  - reduced observed/admitted/rejected boundary and first admission conservativity theorems
- `Field/Router/Installation.lean`
  - minimal canonical installed-route object and installation honesty theorems
- `Field/Router/Lifecycle.lean`
  - reduced observed/admitted/installed/withdrawn/expired/refreshed lifecycle object plus maintenance and conservativity theorems
- `Field/Router/Canonical.lean`
  - router-owned destination-local canonical support selector over lifecycle routes, plus support-best and eligibility theorems for canonical route truth

## Async And System Layers

- `Field/Async/API.lean`
  - reduced async envelopes, explicit delay/retry/loss assumptions, queue stepping, ready-message view, and observer view
- `Field/Async/Safety.lean`
  - first async publication-safety theorems and queue-drain facts connecting the async layer back to local honesty
- `Field/Async/Transport.lean`
  - transport lifecycle lemmas for retry/delivery/drop behavior, publication injection, and the reliable-immediate refinement to the synchronous publication model
- `Field/System/Statistics.lean`
  - aggregate local-support summaries and in-flight support-mass bounds over the async layer
- `Field/System/Boundary.lean`
  - system-level assumption-boundary statements above the async/runtime stack
- `Field/System/EndToEnd.lean`
  - reduced end-to-end state and step relation combining async transport, router lifecycle installation, and lifecycle maintenance, plus first safety/observer lemmas
- `Field/System/Convergence.lean`
  - reduced reliable-immediate fixed-point and no-spontaneous-promotion theorems over iterated end-to-end steps
- `Field/System/Canonical.lean`
  - system-facing refinement theorems connecting `supportDominance` winners to the router-owned canonical selector, plus reliable-immediate stability for the canonical system route
- `Field/Quality/API.lean`
  - reduced route-comparison views, admissibility rules, objective vocabulary, pairwise comparison objects, and destination-filtered best-view selection
- `Field/Quality/Reference.lean`
  - reference admissibility and support-best semantics over exported route views, plus a destination-filtered support-only reference selector
- `Field/Quality/Refinement.lean`
  - support-only refinement theorems connecting `supportDominance` to the reference-best semantics, plus explicit counterexamples showing why tie-break and hop-band objectives are not promoted to global optimality
- `Field/Quality/System.lean`
  - system-facing quality theorems over `systemStep` lifecycle outputs, including stability, explicit-path non-manufacture, and sender-local support/knowledge observer results

## Notes

- `Field/Docs/Model.md`
  - mathematical description of the local field model, plus its place in the wider field stack
- `Field/Docs/Protocol.md`
  - protocol, Telltale mapping, and replay/authority notes
- `Field/Docs/Adequacy.md`
  - runtime artifact bridge, reduced runtime router projection, low-level alignment theorem, and stronger projected runtime/system adequacy note
- `Field/Docs/Guide.md`
  - contributor guidance, maturity summary, router-canonical truth versus quality/comparison scope, convergence assumptions, and stack-level module map including the network/router/async/system layers

## Maturity Snapshot

- most mature:
  - local boundedness/harmony/honesty theorems
  - reduced private protocol and observational boundary
- moderate:
  - reduced finite network, publication, admission, installation, and lifecycle semantics
  - first network-level safety theorems
  - reduced async semantics, transport lifecycle lemmas, and first async safety theorems
  - router-owned canonical selection over lifecycle routes
  - system-level aggregate summaries, reduced end-to-end safety/observer theorems, reliable-immediate convergence results, and canonical-router refinement
  - reduced route-comparison / ranking semantics and support-only reference refinement above system-facing lifecycle outputs
  - projected reduced runtime/system refinement to router-owned canonical truth
  - probability-simplex information layer
  - normalized public-projection blindness bridge
  - one-step decision layer
  - reduced protocol-machine fragment
- earliest:
  - stronger extracted-Rust runtime correctness theorem beyond the current reduced simulation bridge and projected runtime/system refinement
  - convergence beyond the reliable-immediate / empty-queue / unchanged-network regime
  - stronger global routing optimality theorem beyond the current router-canonical support selector and its system refinement
  - deeper Telltale-native reuse of conservation and subtype-replacement families
