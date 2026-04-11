import ClassicalAnalysisAPI
import Field.Information.Instance
import Field.Model.Instance

/-
The Problem. The information layer defines a richer probabilistic belief
surface, but downstream public observers do not see that full microstate. We
need one explicit blindness/coarse-graining layer that states what public
projection preserves and what it erases.

Solution Structure.
1. Define the public projection over finite beliefs and reduced summaries.
2. Prove the projection is lossy and coarse-grained by construction.
3. Prove representative erasure lemmas for support, freshness, and unknown vs
   unreachable distinctions.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationBlindness

open FieldInformationAPI
open FieldInformationInstance
open FieldModelAPI
open FieldModelInstance
open EntropyAPI

/-! ## Public Projection And Erasure -/

/-- Conservative public observer over the normalized belief object. -/
noncomputable def publicProjectionOfBelief
    (belief : FiniteBelief) : CorridorShape :=
  if explicitPathMass belief > 0 then
    CorridorShape.explicitPath
  else if corridorCapableMass belief > 0 then
    CorridorShape.corridorEnvelope
  else
    CorridorShape.opaque

/-- Joint label/observation distribution induced by the public projection. -/
noncomputable def beliefProjectionJoint
    (L : Type*)
    (beliefs : L → FiniteBelief)
    (labelDist : L → ℝ) :
    L × CorridorShape → ℝ :=
  fun lo =>
    if publicProjectionOfBelief (beliefs lo.1) = lo.2 then
      labelDist lo.1
    else
      0

theorem public_projection_mutual_info_zero_of_erasure
    {L : Type} [Fintype L] [Fintype CorridorShape] [DecidableEq CorridorShape]
    [EntropyAPI.Laws]
    (beliefs : L → FiniteBelief)
    (labelDist : L → ℝ)
    (h_nn : ∀ l, 0 ≤ labelDist l)
    (h_sum : ∑ l, labelDist l = 1) :
    EntropyAPI.IsErasureKernel labelDist (beliefProjectionJoint L beliefs labelDist) →
      EntropyAPI.mutualInfo (beliefProjectionJoint L beliefs labelDist) = 0 := by
  intro hErase
  exact EntropyAPI.Laws.mutual_info_zero_of_erasure
    (self := inferInstance) (L := L) (O := CorridorShape)
    labelDist h_nn h_sum (beliefProjectionJoint L beliefs labelDist) hErase

/-- The public projection is an explicit lossy observer over the normalized
belief object: it never reveals more than one coarse corridor shape. -/
theorem public_projection_is_lossy_observer
    (belief : FiniteBelief) :
    publicProjectionOfBelief belief = CorridorShape.opaque ∨
    publicProjectionOfBelief belief = CorridorShape.corridorEnvelope ∨
      publicProjectionOfBelief belief = CorridorShape.explicitPath := by
  unfold publicProjectionOfBelief
  split_ifs <;> simp

/-- The controller-facing summary still exposes one public macrostate, but it
forgets the support and uncertainty coordinates when only the public observer
is consulted. -/
def publicProjectionOfReducedSummary
    (summary : ReducedBeliefSummary) : CorridorShape :=
  summary.publicMacrostate

theorem reduction_erases_probabilistic_belief_choice
    (posterior : PosteriorState)
    (leftBelief rightBelief : ProbabilisticRouteBelief) :
    FieldModelAPI.reducePosterior posterior leftBelief =
      FieldModelAPI.reducePosterior posterior rightBelief := by
  change FieldModelInstance.reducePosteriorImpl posterior leftBelief =
    FieldModelInstance.reducePosteriorImpl posterior rightBelief
  rfl

theorem reduction_erases_freshness_under_fixed_belief_and_knowledge
    (beliefState : FiniteBelief)
    (knowledge : ReachabilityKnowledge)
    (leftFreshness rightFreshness : ObservationFreshness)
    (leftBelief rightBelief : ProbabilisticRouteBelief) :
    FieldModelAPI.reducePosterior
        { belief := beliefState
          freshness := leftFreshness
          knowledge := knowledge } leftBelief =
      FieldModelAPI.reducePosterior
        { belief := beliefState
          freshness := rightFreshness
          knowledge := knowledge } rightBelief := by
  change FieldModelInstance.reducePosteriorImpl
      { belief := beliefState
        freshness := leftFreshness
        knowledge := knowledge } leftBelief =
    FieldModelInstance.reducePosteriorImpl
      { belief := beliefState
        freshness := rightFreshness
        knowledge := knowledge } rightBelief
  cases knowledge <;>
    simp [FieldModelInstance.reducePosteriorImpl, PosteriorState.support,
      PosteriorState.entropy]

theorem reduction_erases_unknown_unreachable_distinction_under_equal_uncertainty
    (uncertainty : Nat)
    (leftFreshness rightFreshness : ObservationFreshness)
    (leftBelief rightBelief : ProbabilisticRouteBelief) :
    FieldModelAPI.reducePosterior
        { belief :=
            { unknownWeight := uncertainty
              unreachableWeight := 0
              corridorWeight := 0
              explicitPathWeight := 0 }
          freshness := leftFreshness
          knowledge := ReachabilityKnowledge.unknown } leftBelief =
      FieldModelAPI.reducePosterior
        { belief :=
            { unknownWeight := 0
              unreachableWeight := uncertainty
              corridorWeight := 0
              explicitPathWeight := 0 }
          freshness := rightFreshness
          knowledge := ReachabilityKnowledge.unreachable } rightBelief := by
  change FieldModelInstance.reducePosteriorImpl
      { belief :=
          { unknownWeight := uncertainty
            unreachableWeight := 0
            corridorWeight := 0
            explicitPathWeight := 0 }
        freshness := leftFreshness
        knowledge := ReachabilityKnowledge.unknown } leftBelief =
    FieldModelInstance.reducePosteriorImpl
      { belief :=
          { unknownWeight := 0
            unreachableWeight := uncertainty
            corridorWeight := 0
            explicitPathWeight := 0 }
        freshness := rightFreshness
        knowledge := ReachabilityKnowledge.unreachable } rightBelief
  simp [FieldModelInstance.reducePosteriorImpl, PosteriorState.support,
    PosteriorState.entropy, FiniteBelief.supportMass, FiniteBelief.uncertaintyMass]

theorem public_projection_of_reduced_summary_forgets_support_and_uncertainty
    (left right : ReducedBeliefSummary)
    (hMacro : left.publicMacrostate = right.publicMacrostate) :
    publicProjectionOfReducedSummary left =
      publicProjectionOfReducedSummary right := by
  exact hMacro

theorem explicit_projection_of_positive_explicit_mass
    (belief : FiniteBelief)
    (hExplicit : explicitPathMass belief > 0) :
    publicProjectionOfBelief belief = CorridorShape.explicitPath := by
  unfold publicProjectionOfBelief
  simp [hExplicit]

theorem corridor_projection_of_zero_explicit_and_positive_corridor_mass
    (belief : FiniteBelief)
    (hExplicit : explicitPathMass belief = 0)
    (hCorridor : corridorCapableMass belief > 0) :
    publicProjectionOfBelief belief = CorridorShape.corridorEnvelope := by
  unfold publicProjectionOfBelief
  simp [hExplicit, hCorridor]

theorem opaque_projection_of_zero_corridor_mass
    (belief : FiniteBelief)
    (hCorridor : corridorCapableMass belief = 0) :
    publicProjectionOfBelief belief = CorridorShape.opaque := by
  have hExplicit : explicitPathMass belief = 0 := by
    have hBound := FieldInformationAPI.explicit_path_mass_bounded belief
    rw [hCorridor] at hBound
    nlinarith
  unfold publicProjectionOfBelief
  simp [hCorridor, hExplicit]

/-- Field-side erasure theorem: once corridor-capable mass is zero, the public
projection forgets how the remaining belief mass is split between `unknown`
and `unreachable`. -/
theorem opaque_projection_erases_unknown_unreachable_split
    (left right : FiniteBelief)
    (hLeft : corridorCapableMass left = 0)
    (hRight : corridorCapableMass right = 0) :
    publicProjectionOfBelief left = publicProjectionOfBelief right := by
  have hLeftExplicit : ¬ explicitPathMass left > 0 := by
    intro hPos
    have hBound := FieldInformationAPI.explicit_path_mass_bounded left
    linarith
  have hRightExplicit : ¬ explicitPathMass right > 0 := by
    intro hPos
    have hBound := FieldInformationAPI.explicit_path_mass_bounded right
    linarith
  unfold publicProjectionOfBelief
  simp [hLeft, hRight, hLeftExplicit, hRightExplicit]

end FieldInformationBlindness
