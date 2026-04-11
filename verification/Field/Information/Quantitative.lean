import ClassicalAnalysisAPI
import Field.Information.Blindness
import Field.Model.Refinement

/-! # Information.Quantitative — L1 belief distance and local uncertainty potential -/

/-
Define a quantitative measure of belief change (L1 simplex distance) and a scalar uncertainty
potential that combines belief entropy with congestion and residual pressure.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationQuantitative

open FieldInformationAPI
open FieldInformationBlindness
open FieldModelAPI
open FieldModelRefinement
open EntropyAPI
open scoped BigOperators

/-! ## Belief Distance -/

/-- Simple `L1` distance on the normalized reduced belief simplex. -/
noncomputable def beliefL1Distance
    (left right : FiniteBelief) : ℝ :=
  ∑ hypothesis, |(normalizeBelief left).pmf hypothesis - (normalizeBelief right).pmf hypothesis|

/-- Small symmetric aggregate gap on naturals used for reduction-level
support/uncertainty comparisons. -/
def natGap (left right : Nat) : Nat :=
  max left right - min left right

/-- Reduction-level support difference carried by the controller-facing
summary. -/
def reducedSupportGap
    (left right : ReducedBeliefSummary) : Nat :=
  natGap left.supportMass right.supportMass

/-- Reduction-level uncertainty difference carried by the controller-facing
summary. -/
def reducedUncertaintyGap
    (left right : ReducedBeliefSummary) : Nat :=
  natGap left.uncertaintyMass right.uncertaintyMass

/-- Aggregate controller-facing difference over the reduced summary. This does
not include public-macrostate disagreement, which is tracked separately as a
coarse observer. -/
def reducedSummaryAggregateGap
    (left right : ReducedBeliefSummary) : Nat :=
  reducedSupportGap left right + reducedUncertaintyGap left right

/-! ## Uncertainty Potential -/

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

theorem natGap_eq_zero_of_equal
    (value : Nat) :
    natGap value value = 0 := by
  simp [natGap]

theorem reducedSupportGap_eq_zero_of_equal
    (summary : ReducedBeliefSummary) :
    reducedSupportGap summary summary = 0 := by
  simp [reducedSupportGap, natGap_eq_zero_of_equal]

theorem reducedUncertaintyGap_eq_zero_of_equal
    (summary : ReducedBeliefSummary) :
    reducedUncertaintyGap summary summary = 0 := by
  simp [reducedUncertaintyGap, natGap_eq_zero_of_equal]

theorem reducedSummaryAggregateGap_eq_zero_of_equal
    (summary : ReducedBeliefSummary) :
    reducedSummaryAggregateGap summary summary = 0 := by
  simp [reducedSummaryAggregateGap, reducedSupportGap_eq_zero_of_equal,
    reducedUncertaintyGap_eq_zero_of_equal]

theorem reducedSupportGap_matches_posterior_support_gap
    (left right : PosteriorState)
    (leftBelief rightBelief : ProbabilisticRouteBelief) :
    reducedSupportGap
        (FieldModelAPI.reducePosterior left leftBelief)
        (FieldModelAPI.reducePosterior right rightBelief) =
      natGap left.support right.support := by
  simp [reducedSupportGap, natGap, reduced_summary_preserves_support_mass]

theorem reducedUncertaintyGap_matches_posterior_uncertainty_gap
    (left right : PosteriorState)
    (leftBelief rightBelief : ProbabilisticRouteBelief) :
    reducedUncertaintyGap
        (FieldModelAPI.reducePosterior left leftBelief)
        (FieldModelAPI.reducePosterior right rightBelief) =
      natGap left.entropy right.entropy := by
  simp [reducedUncertaintyGap, natGap, reduced_summary_preserves_uncertainty_mass]

theorem equal_reduced_summaries_induce_zero_aggregate_gap
    (left right : ReducedBeliefSummary)
    (hEq : left = right) :
    reducedSummaryAggregateGap left right = 0 := by
  subst hEq
  exact reducedSummaryAggregateGap_eq_zero_of_equal left

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
