/-! # Protocol Correctness

Formal session-type properties for Jacquard's choreography protocols.

## Properties to establish

- **Session fidelity**: pathway choreography protocols (forward, repair, handoff,
  hold/replay, anti-entropy) satisfy their declared role and message-type contracts.
- **Progress**: well-typed protocol sessions cannot deadlock under the shared
  `RoutingEngine` lifecycle contract.
- **Fail-closed ordering**: checkpoint write precedes canonical in-memory
  publication in every lifecycle transition.
- **Isolation**: engine-private choreography effect interfaces are never
  observable at the shared `jacquard-traits` boundary.
-/

namespace ProtocolCorrectness

-- Placeholder: proofs go here.

end ProtocolCorrectness
