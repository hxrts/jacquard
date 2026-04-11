/-! # Field.Architecture — shared taxonomy for lineage, projection, and refinement -/

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

def routeLineageMonotone
    (earlier later : RouteLineageStage) : Prop :=
  match earlier, later with
  | .localProjection, _ => True
  | .asyncEnvelope, .asyncEnvelope
  | .asyncEnvelope, .publicationCandidate
  | .asyncEnvelope, .admittedRoute
  | .asyncEnvelope, .installedRoute
  | .asyncEnvelope, .canonicalRoute => True
  | .publicationCandidate, .publicationCandidate
  | .publicationCandidate, .admittedRoute
  | .publicationCandidate, .installedRoute
  | .publicationCandidate, .canonicalRoute => True
  | .admittedRoute, .admittedRoute
  | .admittedRoute, .installedRoute
  | .admittedRoute, .canonicalRoute => True
  | .installedRoute, .installedRoute
  | .installedRoute, .canonicalRoute => True
  | .canonicalRoute, .canonicalRoute => True
  | _, _ => False

def evidenceLineageMonotone
    (earlier later : EvidenceLineageStage) : Prop :=
  match earlier, later with
  | .protocolTrace, _ => True
  | .controllerEvidence, .controllerEvidence
  | .controllerEvidence, .runtimeArtifact => True
  | .runtimeArtifact, .runtimeArtifact => True
  | _, _ => False

def selectorLineageMonotone
    (earlier later : SelectorLineageStage) : Prop :=
  match earlier, later with
  | .baseSelector, _ => True
  | .strongerSelector, .strongerSelector
  | .strongerSelector, .systemRefinement
  | .strongerSelector, .runtimeAdequacyRefinement => True
  | .systemRefinement, .systemRefinement
  | .systemRefinement, .runtimeAdequacyRefinement => True
  | .runtimeAdequacyRefinement, .runtimeAdequacyRefinement => True
  | _, _ => False

theorem canonical_route_is_terminal_route_lineage :
    routeLineageMonotone .localProjection .canonicalRoute := by
  trivial

theorem runtime_artifact_is_latest_evidence_lineage :
    evidenceLineageMonotone .protocolTrace .runtimeArtifact := by
  trivial

theorem runtime_refinement_is_latest_selector_lineage :
    selectorLineageMonotone .baseSelector .runtimeAdequacyRefinement := by
  trivial

end FieldArchitecture
