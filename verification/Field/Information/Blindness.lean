import ClassicalAnalysisAPI
import Field.Information.Instance

/-!
Field-side information-cost / blindness bridge for the normalized belief model.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationBlindness

open FieldInformationAPI
open FieldInformationInstance
open FieldModelAPI
open EntropyAPI

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

end FieldInformationBlindness
