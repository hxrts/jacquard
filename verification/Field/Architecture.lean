/-! # Field.Architecture — shared taxonomy for projection, lineage, and roles -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldArchitecture

/-- Distinguish the three projection families that currently coexist in the
field stack. -/
inductive ProjectionKind
  | protocol
  | localPublic
  | runtimeAdequacy
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Main semantic ladder used across docs and theorem-pack summaries. -/
inductive RefinementLadderStage
  | localPrivateSemantics
  | publicSystemSemantics
  | routerOwnedTruth
  | runtimeAdequacyArtifacts
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Publication lineage for route-shaped objects. This is a taxonomy only; the
owning semantics still live in the corresponding subsystem files. -/
inductive RouteLineageStage
  | localProjection
  | asyncEnvelope
  | publicationCandidate
  | admittedRoute
  | installedRoute
  | canonicalRoute
  deriving Inhabited, Repr, DecidableEq, BEq, Ord

/-- Evidence lineage for controller-facing and runtime-facing observational
objects. -/
inductive EvidenceLineageStage
  | protocolTrace
  | controllerEvidence
  | runtimeArtifact
  deriving Inhabited, Repr, DecidableEq, BEq, Ord

/-- Selector-refinement lineage used by router/system/adequacy theorem packs. -/
inductive SelectorLineageStage
  | baseSelector
  | strongerSelector
  | systemRefinement
  | runtimeAdequacyRefinement
  deriving Inhabited, Repr, DecidableEq, BEq, Ord

/-- Shared semantic-vs-proof-artifact split used for cleanup and docs. -/
inductive ObjectRole
  | semanticCore
  | theoremPackaging
  | syntheticFixture
  deriving Inhabited, Repr, DecidableEq, BEq

end FieldArchitecture
