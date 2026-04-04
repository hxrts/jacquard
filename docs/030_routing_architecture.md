# Routing Architecture

Contour is organized around a small stable stack. `core` owns shared model types. `traits` owns the abstract routing and runtime-effect boundaries. Later crates implement mesh, router orchestration, transport adapters, and simulation on top of those two layers.

See [Introduction](001_introduction.md) for repository scope. See [Core Types](010_core_types.md) for the semantic objects this architecture moves. See [Time Model](020_time.md) for the deterministic time and ordering rules that constrain the whole stack.

## Planes

The routing contract separates control-plane work from data-plane work. The control plane owns candidate gathering, admission, installation, commitments, maintenance, and anti-entropy. The data plane forwards payloads over already admitted route state.

This split prevents forwarding code from inventing canonical route truth. Data-plane observations may report health or failures, but the control plane decides whether that changes the installed route.

## Family Boundary

`RouteFamilyExtension` is the family boundary. A family produces observational candidates, checks admission, admits a route, installs it, publishes commitments, and handles family-local maintenance. Contour implements `Mesh` in-tree. Other families can integrate through the same boundary.

The top-level router stays family-neutral. It compares candidates, enforces fallback rules, tracks installed routes, and coordinates maintenance. Family-private planning and runtime state stay behind the extension boundary.

## Runtime Boundary

The routing core does not call platform APIs directly. Hashing, storage, audit emission, transport ingress, time, and ordering all cross explicit runtime-effect traits. That is how native execution, tests, and simulation share one semantic model instead of drifting apart.

This architecture is also the main reason the simulator can reuse the same routing contract. The simulator does not need a second routing model. It drives the same shared objects and effect boundaries under a different runtime implementation.
