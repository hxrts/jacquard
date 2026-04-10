import ClassicalAnalysisAPI
import Field.Information.Blindness
import Field.Model.Instance

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationQuantitative

open FieldInformationAPI
open FieldInformationBlindness
open FieldModelAPI
open EntropyAPI
open scoped BigOperators

/-- Simple `L1` distance on the normalized reduced belief simplex. -/
noncomputable def beliefL1Distance
    (left right : FiniteBelief) : ℝ :=
  ∑ hypothesis, |(normalizeBelief left).pmf hypothesis - (normalizeBelief right).pmf hypothesis|

/-- Local uncertainty potential following the same API/instance discipline as
Telltale's classical-analysis split: the API-level pieces come from the local
model and normalized belief object, and the first instance packages them into
one quantitative ranking candidate. -/
noncomputable def localUncertaintyPotential
    (state : LocalState) : ℝ :=
  shannonUncertainty state.posterior.belief +
    state.controller.congestionPrice + state.regime.residual

theorem beliefL1Distance_nonneg
    (left right : FiniteBelief) :
    0 ≤ beliefL1Distance left right := by
  unfold beliefL1Distance
  exact Finset.sum_nonneg (fun _ _ => abs_nonneg _)

theorem beliefL1Distance_eq_zero_of_equal
    (belief : FiniteBelief) :
    beliefL1Distance belief belief = 0 := by
  unfold beliefL1Distance
  simp

theorem localUncertaintyPotential_nonneg
    (state : LocalState) :
    0 ≤ localUncertaintyPotential state := by
  unfold localUncertaintyPotential
  nlinarith [shannon_uncertainty_nonneg state.posterior.belief]

theorem equal_beliefs_induce_zero_projection_loss
    (left right : FiniteBelief)
    (hEq : left = right) :
    publicProjectionOfBelief left = publicProjectionOfBelief right := by
  subst hEq
  rfl

end FieldInformationQuantitative
