# Introduction

Jacquard is a deterministic routing system for constrained and unstable networks. It provides a stable routing abstraction and one in-tree routing engine, `Mesh`. It is designed so a host can add an external routing engine through the same contract.

See [Core Types](102_core_types.md) for the model objects that carry the system. See [Time Model](103_time.md) for the deterministic time rules. See [Routing Observation Boundary](104_routing_observation_boundary.md) for the world primitives, observation surfaces, and estimation layer used for routing. See [Routing Logic](105_routing_logic.md) for the decision path and routing-engine boundary. See [Crate Architecture](106_crate_architecture.md) for separation of concerns and implementation policies.

## Scope

Jacquard owns the shared routing contract, the first-party mesh routing engine, the top-level router, runtime adapters, and simulation support. Protection-versus-connectivity policy may be supplied by a host, but Jacquard itself stays routing-engine-neutral at the contract layer.

The central split is between shared facts and local runtime state. Service descriptors, topology observations, admission checks, and route witnesses are explicit shared objects. Adaptive policy, selected routing actions, installed-route ownership, and engine-private runtime state stay local.

Jacquard depends on Telltale for choreography projection, runtime structure, and simulation support. The routing model is shaped so admission, installation, maintenance, and replay remain explicit. The codebase is organized around shared model types, abstract trait boundaries, first-party mesh logic, router orchestration, and simulation.

## Problem

Jacquard is aimed at networks that are unstable, capacity-constrained, and potentially adversarial. Nodes may churn, links may degrade quickly, identities may be weak or partially authenticated, and local coordination may be necessary without any reliable global authority.

That creates two competing pressures. The system needs stronger coordination than naive flooding or purely local heuristics, but it also cannot afford to hard-code one routing doctrine such as GPS-based clique membership, singleton leaders, or full consensus on every routing transition.

It also needs to support more than one routing engine being present at once. A host such as Aura may want to run onion and mesh side by side, migrate traffic gradually from one to the other, or use one engine as a limited lower-layer carrier for another. Those are different cases and should not be collapsed into one mechanism.

## System Shape

The top-level routing contract is routing-engine-neutral. A routing engine produces observational candidates, checks admission, admits a route, realizes it under router-provided canonical identity, publishes commitments, and handles engine-local maintenance. The control plane owns canonical route truth. The data plane forwards over already admitted truth.

When a routing engine needs local coordination, Jacquard allows it to expose a shared coordination result such as a committee selection. Jacquard does not require that every routing engine use committees, and it does not require that a committee have a distinguished leader. The shared layer standardizes the result shape, not the formation process. Formation may be engine-local, host-local, provisioned, or otherwise out of band.

Jacquard also allows a host-owned policy engine to compose routing engines through a neutral substrate contract. That means multiple routing engines may be used together, but the shared layer does not treat one canonical route as simultaneously owned by several unrelated engines. Composition happens through explicit carrier leases and layer parameters above the routing-engine boundary.

## Design Commitments

Jacquard is fully deterministic. It uses typed time, typed ordering, explicit bounds, and explicit ownership objects instead of ambient runtime assumptions.

Observation scopes are kept explicit. Local node state, peer estimates, link estimates, and neighborhood aggregates are separate model surfaces. This split keeps routing logic honest about what is known unequivocally, what is inferred about a peer, and what is an aggregate local view.

Jacquard is intentionally not opinionated about engine-local scoring, committee formation policy, or trust heuristics. The shared layer commits to the result shapes, evidence classes, ownership rules, and canonical transition path. The routing-engine layer owns the scoring rules, diversity logic, and misbehavior handling that depend on its routing semantics.

The system is committed to one explicit service lifecycle: observation to candidate to admission to router-owned canonical identity allocation to family realization to materialized route to maintenance, replacement, or teardown. Major transitions stay typed and explicit. Data-plane health stays observational until the control plane publishes a canonical change.

It is equally committed to a composition boundary that stays narrow. The shared layer may expose substrate requirements, substrate leases, and layer parameters. It should not let one routing engine leak its internals into another, and it should not standardize one host policy for gradual migration.

Jacquard is also meant to be the integration point where multiple teams can contribute device-specific expertise without forking the routing model. One team may contribute a BLE node world extension, another a Wi-Fi link world extension, and another a platform-specific transport or service world extension. The cooperative effect comes from merging those self-describing observations into one shared world picture above the routing-engine boundary, then letting routing engines incorporate observed nodes and links when their own criteria are met.
