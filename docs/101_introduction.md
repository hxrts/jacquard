# Introduction

Jacquard is a deterministic routing system for ad hoc shaped networks. It provides a stable routing abstraction above the concrete routing algorithm. A host composes one or several engines behind the same contract, whether those engines ship with Jacquard or come from a 3rd party.

The docs are grouped numerically. The 200s cover the shared model and time. The 300s specify contract surfaces, simulator architecture, and experimental methodology. 400s contain one implementation spec per in-tree engine plus the reference client composition. 500s are 3rd-party developer guides for running simulations, assembling clients, and building custom components. [Crate Architecture](999_crate_architecture.md) is the workspace architecture reference.

## Scope

Jacquard owns the shared routing contract and eight in-tree routing engines. The router control plane, runtime adapters, and simulation harness are implemented crates. Protection-versus-connectivity policy may be supplied by a host, but Jacquard itself stays routing-engine-neutral at the contract layer.

The central split is between shared facts and local runtime state. Service descriptors, topology observations, admission checks, and route witnesses are explicit shared objects. Adaptive policy, selected routing actions, installed-route ownership, and engine-private runtime state stay local.

The routing model is shaped so admission, installation, maintenance, and replay remain explicit.

## Problem

Jacquard is aimed at networks that are unstable, capacity-constrained, and potentially adversarial. Nodes may churn, links may degrade quickly, identities may be weak or partially authenticated, and local coordination may be necessary without any reliable global authority.

That creates two competing pressures. The system needs stronger coordination than naive flooding or purely local heuristics. It also cannot hard-code one routing doctrine such as GPS-based clique membership, singleton leaders, or full consensus on every routing transition.

It also needs to support more than one routing engine being present at once. A host such as Aura may want to run onion and pathway side by side, migrate traffic gradually from one to the other, or use one engine as a limited lower-layer carrier for another. Those are different cases and should not be collapsed into one mechanism.

## System Shape

The top-level routing contract is routing-engine-neutral. A routing engine produces observational candidates, checks admission, admits a routing result, realizes it under router-provided canonical identity, publishes commitments, and handles engine-local maintenance. The control plane owns canonical route truth. The data plane forwards over already admitted truth.

When a routing engine needs local coordination, Jacquard allows it to expose a shared coordination result such as a committee selection. Jacquard does not require that every routing engine use committees, and it does not require that a committee have a distinguished leader. The shared layer standardizes the result shape, not the formation process. Formation may be engine-local, host-local, provisioned, or otherwise out of band.

Jacquard also allows a host-owned policy engine to compose routing engines through a neutral substrate contract. That means multiple routing engines may be used together, but the shared layer does not treat one canonical route as simultaneously owned by several unrelated engines. Composition happens through explicit carrier leases and layer parameters above the routing-engine boundary.

## In-Tree Engines

Jacquard ships eight in-tree routing engines as concrete demonstrations of the contract:

- `pathway` for explicit-path routing
- `field` for corridor-envelope routing over a continuously updated field model
- `batman-bellman` for Bellman-Ford-enhanced next-hop routing
- `batman-classic` for spec-faithful BATMAN IV next-hop routing
- `babel` for RFC 8966 distance-vector routing with bidirectional ETX and feasibility distances
- `olsrv2` for OLSRv2 link-state routing
- `scatter` for bounded deferred-delivery diffusion routing
- `mercator` for hybrid corridor routing with stale-safe repair and bounded custody posture

These engines differ in what they publish. Pathway exposes an explicit path, field and mercator publish corridor envelopes, the proactive engines only next-hop visibility, and scatter an opaque viability claim. The shared routing contract carries canonical identity and lifecycle regardless of that published shape.

## Design Commitments

Jacquard is fully deterministic. It uses typed time, typed ordering, explicit bounds, and explicit ownership objects instead of ambient runtime assumptions.

Observation scopes are kept explicit. Local node state, peer estimates, link estimates, and neighborhood aggregates are separate model surfaces. This split keeps routing logic honest about what is known unequivocally, what is inferred about a peer, and what is an aggregate local view.

Jacquard is intentionally not opinionated about engine-local scoring, committee formation policy, or trust heuristics. The shared layer commits to the result shapes, evidence classes, ownership rules, and canonical transition path. The routing-engine layer owns the scoring rules, diversity logic, and misbehavior handling that depend on its routing semantics.

### Lifecycle and Integration

The system is committed to one explicit service lifecycle: observation → candidate → admission → router-owned canonical identity allocation → engine realization → materialized route → maintenance, replacement, or teardown. Major transitions stay typed and explicit. Data-plane health stays observational until the control plane publishes a canonical change.

The composition boundary is intentionally narrow. The shared layer exposes substrate requirements, substrate leases, and layer parameters, but does not let routing engines leak their internals into one another.

Jacquard is also meant to be the integration point where multiple teams can contribute device-specific expertise without forking the routing model. One team may contribute a BLE node extension, another a Wi-Fi link extension, and another a platform-specific transport or service extension. The cooperative effect comes from merging those self-describing observations into one shared world picture above the routing-engine boundary, then letting routing engines incorporate observed nodes and links when their own criteria are met.
