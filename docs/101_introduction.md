# Introduction

Jacquard is a deterministic routing system for ad hoc shaped networks. It provides a stable routing abstraction and two in-tree explicit-path routing engines, `Pathway` and `batman`. It is designed so a host can add external routing engines through the same contract.

See [Core Types](201_core_types.md) for the model objects that carry the system. See [Time Model](202_time.md) for the deterministic time rules. See [Runtime Effects](301_runtime_effects.md) for the narrow runtime capability surface that hosts implement. See [Pipeline and World Observations](203_pipeline_observations.md) for the shared pipeline, the world schema, and the observation layer. See [Route Lifecycle](204_route_lifecycle.md) for how a route moves from objective through materialization, maintenance, and teardown. See [Crate Architecture](999_crate_architecture.md) for separation of concerns and implementation policies.

## Scope

Jacquard owns the shared routing contract and the first-party pathway routing engine today. The top-level router, runtime adapters, and simulation harness are planned future crates that land alongside the router control plane and simulator work. Protection-versus-connectivity policy may be supplied by a host, but Jacquard itself stays routing-engine-neutral at the contract layer.

The central split is between shared facts and local runtime state. Service descriptors, topology observations, admission checks, and route witnesses are explicit shared objects. Adaptive policy, selected routing actions, installed-route ownership, and engine-private runtime state stay local.

The routing model is shaped so admission, installation, maintenance, and replay remain explicit. The codebase is organized around shared model types, abstract trait boundaries, and a first-party explicit-path engine, with router orchestration and simulation reserved as future crates.

## Problem

Jacquard is aimed at networks that are unstable, capacity-constrained, and potentially adversarial. Nodes may churn, links may degrade quickly, identities may be weak or partially authenticated, and local coordination may be necessary without any reliable global authority.

That creates two competing pressures. The system needs stronger coordination than naive flooding or purely local heuristics, but it also cannot afford to hard-code one routing doctrine such as GPS-based clique membership, singleton leaders, or full consensus on every routing transition.

It also needs to support more than one routing engine being present at once. A host such as Aura may want to run onion and pathway side by side, migrate traffic gradually from one to the other, or use one engine as a limited lower-layer carrier for another. Those are different cases and should not be collapsed into one mechanism.

## System Shape

The top-level routing contract is routing-engine-neutral. A routing engine produces observational candidates, checks admission, admits a route, realizes it under router-provided canonical identity, publishes commitments, and handles engine-local maintenance. The control plane owns canonical route truth. The data plane forwards over already admitted truth.

When a routing engine needs local coordination, Jacquard allows it to expose a shared coordination result such as a committee selection. Jacquard does not require that every routing engine use committees, and it does not require that a committee have a distinguished leader. The shared layer standardizes the result shape, not the formation process. Formation may be engine-local, host-local, provisioned, or otherwise out of band.

Jacquard also allows a host-owned policy engine to compose routing engines through a neutral substrate contract. That means multiple routing engines may be used together, but the shared layer does not treat one canonical route as simultaneously owned by several unrelated engines. Composition happens through explicit carrier leases and layer parameters above the routing-engine boundary.

## Design Commitments

Jacquard is fully deterministic. It uses typed time, typed ordering, explicit bounds, and explicit ownership objects instead of ambient runtime assumptions.

Observation scopes are kept explicit. Local node state, peer estimates, link estimates, and neighborhood aggregates are separate model surfaces. This split keeps routing logic honest about what is known unequivocally, what is inferred about a peer, and what is an aggregate local view.

Jacquard is intentionally not opinionated about engine-local scoring, committee formation policy, or trust heuristics. The shared layer commits to the result shapes, evidence classes, ownership rules, and canonical transition path. The routing-engine layer owns the scoring rules, diversity logic, and misbehavior handling that depend on its routing semantics.

### Lifecycle and Integration

The system is committed to one explicit service lifecycle: observation to candidate to admission to router-owned canonical identity allocation to engine realization to materialized route to maintenance, replacement, or teardown. Major transitions stay typed and explicit. Data-plane health stays observational until the control plane publishes a canonical change.

It is equally committed to a composition boundary that stays narrow. The shared layer may expose substrate requirements, substrate leases, and layer parameters. It should not let one routing engine leak its internals into another, and it should not standardize one host policy for gradual migration.

Jacquard is also meant to be the integration point where multiple teams can contribute device-specific expertise without forking the routing model. One team may contribute a BLE node world extension, another a Wi-Fi link world extension, and another a platform-specific transport or service world extension. The cooperative effect comes from merging those self-describing observations into one shared world picture above the routing-engine boundary, then letting routing engines incorporate observed nodes and links when their own criteria are met.
