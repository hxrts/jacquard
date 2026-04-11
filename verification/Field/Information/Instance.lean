import ClassicalAnalysisInstance
import Field.Information.API
import Mathlib.Data.Real.Basic
import Mathlib.Tactic

/-! # Information.Instance — normalised pmf and weight calculations for finite belief distributions -/

/-
Implement and verify the arithmetic for finite probability mass functions over route hypothesis
sets, proving normalisation and non-negativity.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationInstance

open FieldInformationAPI
open FieldModelAPI
open EntropyAPI
open scoped BigOperators

/-! ## PMF Arithmetic -/

/-- Total finite weight carried by a reduced field belief. -/
def totalWeight (belief : FiniteBelief) : Nat :=
  belief.totalWeight

/-- Normalized pmf for the reduced field belief object. Zero-total beliefs
fall back to a point mass on `unknown`. -/
noncomputable def normalizedPmf
    (belief : FiniteBelief)
    (hypothesis : FieldHypothesis) : ℝ :=
  if hZero : totalWeight belief = 0 then
    if hypothesis = FieldHypothesis.unknown then
      1
    else
      0
  else
    (belief.weight hypothesis : ℝ) / (totalWeight belief : ℝ)

/-! ## Normalisation Proofs -/

theorem normalizedPmf_nonneg
    (belief : FiniteBelief)
    (hypothesis : FieldHypothesis) :
    0 ≤ normalizedPmf belief hypothesis := by
  by_cases hZero : totalWeight belief = 0
  · by_cases hUnknown : hypothesis = FieldHypothesis.unknown
    · simp [normalizedPmf, hZero, hUnknown]
    · simp [normalizedPmf, hZero, hUnknown]
  · have hTotalPosNat : 0 < totalWeight belief := Nat.pos_of_ne_zero hZero
    have hTotalPosReal : 0 < (totalWeight belief : ℝ) := by
      exact_mod_cast hTotalPosNat
    simpa [normalizedPmf, hZero] using
      (div_nonneg
        (show 0 ≤ (belief.weight hypothesis : ℝ) by exact_mod_cast Nat.zero_le _)
        hTotalPosReal.le)

theorem normalizedPmf_sum_one
    (belief : FiniteBelief) :
    ∑ h, normalizedPmf belief h = 1 := by
  by_cases hZero : totalWeight belief = 0
  · simp [normalizedPmf, hZero]
  · have hTotalPosNat : 0 < totalWeight belief := Nat.pos_of_ne_zero hZero
    have hTotalNeReal : (totalWeight belief : ℝ) ≠ 0 := by
      exact_mod_cast (Nat.ne_of_gt hTotalPosNat)
    have hWeightSumNat :
        belief.unknownWeight + belief.unreachableWeight +
          belief.corridorWeight + belief.explicitPathWeight =
            totalWeight belief := by
      rfl
    have hWeightSumReal :
        (belief.unknownWeight : ℝ) + belief.unreachableWeight +
          belief.corridorWeight + belief.explicitPathWeight =
            (totalWeight belief : ℝ) := by
      exact_mod_cast hWeightSumNat
    have hUniv :
        (Finset.univ : Finset FieldHypothesis) =
          { FieldHypothesis.unknown, FieldHypothesis.unreachable,
            FieldHypothesis.corridor, FieldHypothesis.explicitPath } := by
      native_decide
    calc
      ∑ h, normalizedPmf belief h
          =
        Finset.sum
          ({ FieldHypothesis.unknown, FieldHypothesis.unreachable,
             FieldHypothesis.corridor,
             FieldHypothesis.explicitPath } : Finset FieldHypothesis)
          (fun h => normalizedPmf belief h) := by
                simp [hUniv]
      _ =
        (belief.unknownWeight : ℝ) / (totalWeight belief : ℝ) +
          (belief.unreachableWeight : ℝ) / (totalWeight belief : ℝ) +
          (belief.corridorWeight : ℝ) / (totalWeight belief : ℝ) +
          (belief.explicitPathWeight : ℝ) / (totalWeight belief : ℝ) := by
            simp [normalizedPmf, hZero, FiniteBelief.weight]
            ring
      _ =
        ((belief.unknownWeight : ℝ) + belief.unreachableWeight +
            belief.corridorWeight + belief.explicitPathWeight) /
          (totalWeight belief : ℝ) := by
            ring
      _ = 1 := by
        field_simp [hTotalNeReal]
        simpa [add_assoc, add_left_comm, add_comm] using hWeightSumReal

/-- Concrete normalized distribution for reduced field beliefs. -/
noncomputable def normalizeBeliefImpl (belief : FiniteBelief) : Distribution FieldHypothesis where
  pmf := normalizedPmf belief
  nonneg := normalizedPmf_nonneg belief
  sum_one := normalizedPmf_sum_one belief

/-- Concrete probability-simplex style belief object for reduced field
beliefs. -/
noncomputable def simplexBeliefImpl
    (belief : FiniteBelief) : ProbabilitySimplexBelief :=
  { distribution := normalizeBeliefImpl belief }

/-- Concrete Shannon uncertainty on the normalized field belief. -/
noncomputable def shannonUncertaintyImpl (belief : FiniteBelief) : ℝ :=
  EntropyAPI.shannonEntropy (normalizeBeliefImpl belief).pmf

/-- Concrete normalized explicit-path mass. -/
noncomputable def explicitPathMassImpl (belief : FiniteBelief) : ℝ :=
  (normalizeBeliefImpl belief).pmf FieldHypothesis.explicitPath

/-- Concrete normalized corridor-capable mass. -/
noncomputable def corridorCapableMassImpl (belief : FiniteBelief) : ℝ :=
  (normalizeBeliefImpl belief).pmf FieldHypothesis.corridor +
    (normalizeBeliefImpl belief).pmf FieldHypothesis.explicitPath

noncomputable instance fieldInformationLaws : FieldInformationAPI.Laws where
  simplexBelief := simplexBeliefImpl
  shannonUncertainty := shannonUncertaintyImpl
  explicitPathMass := explicitPathMassImpl
  corridorCapableMass := corridorCapableMassImpl
  simplex_matches_finite_belief := by
    intro belief
    constructor
    · intro hZero
      have hZero' : totalWeight belief = 0 := hZero
      constructor
      · simp [simplexBeliefImpl, normalizeBeliefImpl, ProbabilitySimplexBelief.pmf,
          normalizedPmf, hZero']
      · constructor
        · simp [simplexBeliefImpl, normalizeBeliefImpl, ProbabilitySimplexBelief.pmf,
            normalizedPmf, hZero']
        · constructor
          · simp [simplexBeliefImpl, normalizeBeliefImpl, ProbabilitySimplexBelief.pmf,
              normalizedPmf, hZero']
          · simp [simplexBeliefImpl, normalizeBeliefImpl, ProbabilitySimplexBelief.pmf,
              normalizedPmf, hZero']
    · intro hNonzero hypothesis
      have hNonzero' : totalWeight belief ≠ 0 := hNonzero
      change normalizedPmf belief hypothesis =
        (belief.weight hypothesis : ℝ) / (belief.totalWeight : ℝ)
      rw [show normalizedPmf belief hypothesis =
          (belief.weight hypothesis : ℝ) / (totalWeight belief : ℝ) by
            simp [normalizedPmf, hNonzero']]
      rfl
  explicit_path_mass_matches := by
    intro belief
    rfl
  corridor_capable_mass_matches := by
    intro belief
    rfl
  shannon_uncertainty_nonneg := by
    intro belief
    simpa [shannonUncertaintyImpl] using
      EntropyAPI.shannon_entropy_nonneg (normalizeBeliefImpl belief)
  explicit_path_mass_bounded := by
    intro belief
    constructor
    · exact (normalizeBeliefImpl belief).nonneg FieldHypothesis.explicitPath
    · change
        (normalizeBeliefImpl belief).pmf FieldHypothesis.explicitPath ≤
          (normalizeBeliefImpl belief).pmf FieldHypothesis.corridor +
            (normalizeBeliefImpl belief).pmf FieldHypothesis.explicitPath
      exact le_add_of_nonneg_left ((normalizeBeliefImpl belief).nonneg FieldHypothesis.corridor)

/-- Positive explicit-path weight yields positive explicit-path mass whenever
the belief carries any total mass. -/
theorem positive_explicit_weight_yields_positive_mass
    (belief : FiniteBelief)
    (hTotal : totalWeight belief ≠ 0)
    (hWeight : 0 < belief.weight FieldHypothesis.explicitPath) :
    0 < FieldInformationAPI.explicitPathMass belief := by
  have hTotalPosNat : 0 < totalWeight belief := Nat.pos_of_ne_zero hTotal
  have hWeightReal : 0 < (belief.weight FieldHypothesis.explicitPath : ℝ) := by
    exact_mod_cast hWeight
  have hTotalReal : 0 < (totalWeight belief : ℝ) := by
    exact_mod_cast hTotalPosNat
  change 0 < explicitPathMassImpl belief
  simp [explicitPathMassImpl, normalizeBeliefImpl, normalizedPmf, hTotal]
  exact div_pos hWeightReal hTotalReal

/-- For nonzero total weight, explicit-path mass is exactly the normalized
weight ratio for the explicit-path hypothesis. -/
theorem explicit_path_mass_matches_weight_ratio
    (belief : FiniteBelief)
    (hTotal : totalWeight belief ≠ 0) :
    FieldInformationAPI.explicitPathMass belief =
      (belief.explicitPathWeight : ℝ) / (totalWeight belief : ℝ) := by
  change explicitPathMassImpl belief = _
  simp [explicitPathMassImpl, normalizeBeliefImpl, normalizedPmf, hTotal,
    FiniteBelief.weight]

/-- The concrete field information instance computes Shannon uncertainty from
the normalized finite belief distribution. -/
theorem shannon_uncertainty_is_entropy
    (belief : FiniteBelief) :
    FieldInformationAPI.shannonUncertainty belief =
      EntropyAPI.shannonEntropy ((FieldInformationAPI.normalizeBelief belief).pmf) := by
  rfl

/-- If all belief mass is concentrated on explicit-path, the normalized
explicit-path mass is exactly one. -/
theorem explicit_path_mass_eq_one_of_all_mass_on_explicit_path
    (belief : FiniteBelief)
    (hUnknown : belief.unknownWeight = 0)
    (hUnreachable : belief.unreachableWeight = 0)
    (hCorridor : belief.corridorWeight = 0)
    (hExplicit : belief.explicitPathWeight ≠ 0) :
    FieldInformationAPI.explicitPathMass belief = 1 := by
  have hTotal : totalWeight belief ≠ 0 := by
    intro hZero
    apply hExplicit
    simpa [FieldInformationInstance.totalWeight, FieldModelAPI.FiniteBelief.totalWeight,
      hUnknown, hUnreachable, hCorridor] using hZero
  rw [explicit_path_mass_matches_weight_ratio belief hTotal]
  have hTotalEq : totalWeight belief = belief.explicitPathWeight := by
    simp [FieldInformationInstance.totalWeight, FieldModelAPI.FiniteBelief.totalWeight,
      hUnknown, hUnreachable, hCorridor]
  rw [hTotalEq]
  field_simp [show (belief.explicitPathWeight : ℝ) ≠ 0 by exact_mod_cast hExplicit]

end FieldInformationInstance
