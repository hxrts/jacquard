# Introduction

Contour is a deterministic routing system for constrained and unstable networks. It provides a stable routing abstraction and one in-tree route family, `Mesh`. It is designed so a host can add an external routing family through the same contract.

See [Core Types](102_core_types.md) for the model objects that carry the system. See [Time Model](103_time.md) for the deterministic time rules. See [Routing Observation Boundary](104_routing_observation_boundary.md) for the world primitives, observation surfaces, and estimation layer used for routing. See [Routing Architecture](105_routing_architecture.md) for the crate and control-plane structure.

## Scope

Contour owns the shared routing contract, the first-party mesh family, the top-level router, runtime adapters, and simulation support. It does not own application policy, Aura-specific identity internals, or an in-tree onion implementation. Protection-versus-connectivity policy may be supplied by a host, but Contour itself stays family-neutral at the contract layer.

The central split is between shared facts and local runtime state. Service descriptors, topology observations, admission checks, and route witnesses are explicit shared objects. Adaptive policy, selected routing actions, installed-route ownership, and family-private runtime state stay local.

Contour depends on Telltale for choreography projection, runtime structure, and simulation support. The routing model is shaped so admission, installation, maintenance, and replay remain explicit. The codebase is organized around shared model types, abstract trait boundaries, first-party mesh logic, router orchestration, and simulation.

## System Shape

The system has four stable layers. `core` owns shared types. `traits` owns the abstract routing and runtime-effect boundaries. Later crates implement mesh, router orchestration, transport adapters, and simulation on top of those layers.

The top-level routing contract is family-neutral. A family produces observational candidates, checks admission, admits a route, installs it, publishes commitments, and handles family-local maintenance. The control plane owns canonical route truth. The data plane forwards over already admitted truth.

## Design Commitments

Contour is fully deterministic. It does not use floating-point scoring in the routing core. It uses typed time, typed ordering, explicit bounds, and explicit ownership objects instead of ambient runtime assumptions.

Contour also keeps observation scopes explicit. Local node state, peer estimates, link estimates, and neighborhood aggregates are separate model surfaces. That split keeps routing logic honest about what is self-known, what is inferred about a peer, and what is only an aggregate local view.
