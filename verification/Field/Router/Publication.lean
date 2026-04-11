import Field.Architecture
import Field.Model.API
import Field.Network.API

/-! # Router.Publication — published candidate structure and explicit-path honesty -/

/-
Define the published route candidate record, its honesty and well-formedness constraints,
and prove that explicit-path publication requires the publishing node to hold explicit local
knowledge of the path.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterPublication

open FieldModelAPI
open FieldNetworkAPI
open FieldArchitecture

/-! ## Published Candidate -/

/-- Publication-lineage ownership note for route-shaped objects:
local projection -> async envelope -> publication candidate -> admitted route
-> installed route -> canonical route. This file owns the publication-candidate
surface only. -/
def publishedCandidateLineageStage : RouteLineageStage :=
  .publicationCandidate

/-- Router-facing field publication candidate. This is the first point where a
local observational projection becomes eligible for canonical control-plane
handling, but it is not itself canonical truth. -/
structure PublishedCandidate where
  publisher : NodeId
  destination : DestinationClass
  shape : CorridorShape
  support : Nat
  hopLower : Nat
  hopUpper : Nat
  deriving Repr, DecidableEq, BEq

def publishCandidate
    (publisher : NodeId)
    (destination : DestinationClass)
    (localState : LocalState) : PublishedCandidate :=
  { publisher := publisher
    destination := destination
    shape := localState.projection.shape
    support := localState.projection.support
    hopLower := localState.projection.hopLower
    hopUpper := localState.projection.hopUpper }

def PublicationHonest
    (localState : LocalState)
    (candidate : PublishedCandidate) : Prop :=
  candidate.shape = localState.projection.shape ∧
    candidate.support = localState.projection.support ∧
    candidate.hopLower = localState.projection.hopLower ∧
    candidate.hopUpper = localState.projection.hopUpper

def PublicationWellFormed
    (candidate : PublishedCandidate) : Prop :=
  candidate.support ≤ 1000 ∧ candidate.hopLower ≤ candidate.hopUpper

/-! ## Honesty Constraints -/

theorem publishCandidate_honest
    (publisher : NodeId)
    (destination : DestinationClass)
    (localState : LocalState) :
    PublicationHonest localState (publishCandidate publisher destination localState) := by
  simp [PublicationHonest, publishCandidate]

theorem publishCandidate_well_formed
    (publisher : NodeId)
    (destination : DestinationClass)
    (localState : LocalState)
    (hBound : ProjectionBounded localState.projection) :
    PublicationWellFormed (publishCandidate publisher destination localState) := by
  rcases hBound with ⟨hSupport, hHops⟩
  exact ⟨by simpa [PublicationWellFormed, publishCandidate] using hSupport,
    by simpa [PublicationWellFormed, publishCandidate] using hHops⟩

theorem explicit_path_publication_requires_explicit_knowledge
    (localState : LocalState)
    (hHarmony : Harmony localState)
    (hShape :
      (publishCandidate NodeId.alpha DestinationClass.corridorA localState).shape =
        CorridorShape.explicitPath) :
    localState.posterior.knowledge = ReachabilityKnowledge.explicitPath := by
  rcases hHarmony with ⟨_, _, hShapeIff, _, _⟩
  exact hShapeIff.mp (by simpa [publishCandidate] using hShape)

theorem publication_support_le_local_support
    (localState : LocalState)
    (hHarmony : Harmony localState) :
    (publishCandidate NodeId.alpha DestinationClass.corridorA localState).support ≤
      localState.posterior.support := by
  rcases hHarmony with ⟨_, _, _, hSupport, _⟩
  simpa [publishCandidate] using hSupport

theorem publication_hops_match_local_projection
    (publisher : NodeId)
    (destination : DestinationClass)
    (localState : LocalState) :
    (publishCandidate publisher destination localState).hopLower =
        localState.projection.hopLower ∧
      (publishCandidate publisher destination localState).hopUpper =
        localState.projection.hopUpper := by
  simp [publishCandidate]

end FieldRouterPublication
