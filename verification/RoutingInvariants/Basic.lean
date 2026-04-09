/-! # Routing Invariants

Formal statements of Jacquard's core routing invariants.

## Invariants to establish

- **Determinism**: for the same `Observation<Configuration>` and `RoutingObjective`,
  the planner produces the same candidate ordering across replays.
- **Boundedness**: all counters, budgets, and scored values are bounded by
  their declared `*_max` limits.
- **Ordering**: `Tick` is monotone non-decreasing; `OrderStamp` is strictly
  increasing; neither may be converted to the other by rewrapping the inner
  integer.
- **No float**: routing logic, routing state, and policy values contain no
  floating-point types.
-/

namespace RoutingInvariants

-- Placeholder: proofs go here.

end RoutingInvariants
