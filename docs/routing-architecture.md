# Routing Architecture

Contour splits routing into a small set of stable layers. The `core` crate owns shared data types. The `traits` crate owns the abstract routing contract. Later crates provide the first-party mesh implementation, the top-level router, runtime adapters, and the simulator.

The routing core is deterministic. Canonical route decisions use typed ordering objects, typed time, and explicit capability and lease objects. The routing core does not treat wall clock as distributed truth. The routing core does not depend on floating-point scoring.

The top-level contract is family-neutral. A route family produces candidates, checks admissibility, admits one route, installs runtime state, and performs family-local maintenance. The control plane owns canonical route truth. The data plane forwards over already admitted truth.

## Phase 1 Surface

Phase 1 implements the shared object model and the abstract routing traits. This includes routing identity, service and topology surfaces, admission objects, witnesses, leases, transitions, runtime health objects, hashing, and content IDs. It also includes the top-level router and control-plane trait boundaries.

Phase 1 does not implement the first-party mesh family yet. It also does not implement any onion family in this repository. The onion spec remains an external extension target for a host such as Aura.

## Ownership Boundary

Contour keeps shared facts and local policy separate. Service descriptors, topology observations, admission checks, and route witnesses are shared semantic objects. Local adaptive policy and live route ownership remain runtime-local state.

This split prevents transport observations or diagnostics from becoming canonical route truth on their own. Canonical route installation and mutation require explicit route lifecycle objects and the correct capability class.
