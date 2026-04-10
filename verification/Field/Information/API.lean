import ClassicalAnalysisAPI
import Field.Model.API

/-!
The Problem. The local field model now has a finite belief object, but the
proof surface still needs an explicit information-theoretic boundary so
downstream work can depend on abstract distribution/entropy operations rather
than one concrete normalization formula.

Solution Structure.
1. Define an abstract model that normalizes `FiniteBelief` into a finite
   distribution over `FieldHypothesis`.
2. Define abstract real-valued information operations over that normalized
   belief.
3. Re-export stable wrappers and law bundles for downstream field proofs.
4. Keep concrete normalization and real-analysis realization in the companion
   instance layer.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldInformationAPI

open FieldModelAPI
open EntropyAPI

/-- Explicit probability-simplex style belief object over the reduced field
hypothesis space. This keeps the richer probabilistic semantics visible at the
API boundary instead of burying them inside one concrete normalization
formula. -/
structure ProbabilitySimplexBelief where
  distribution : Distribution FieldHypothesis

namespace ProbabilitySimplexBelief

def pmf (belief : ProbabilitySimplexBelief) : FieldHypothesis → ℝ :=
  belief.distribution.pmf

theorem nonneg
    (belief : ProbabilitySimplexBelief)
    (hypothesis : FieldHypothesis) :
    0 ≤ belief.pmf hypothesis :=
  belief.distribution.nonneg hypothesis

theorem sum_one (belief : ProbabilitySimplexBelief) :
    ∑ h, belief.pmf h = 1 :=
  belief.distribution.sum_one

end ProbabilitySimplexBelief

/-- The simplex belief must agree with the finite weight object, including the
zero-mass fallback to `unknown`. -/
abbrev SimplexMatchesFiniteBelief
    (finite : FiniteBelief)
    (simplex : ProbabilitySimplexBelief) : Prop :=
  (finite.totalWeight = 0 →
      simplex.pmf FieldHypothesis.unknown = 1 ∧
        simplex.pmf FieldHypothesis.unreachable = 0 ∧
        simplex.pmf FieldHypothesis.corridor = 0 ∧
        simplex.pmf FieldHypothesis.explicitPath = 0) ∧
    (finite.totalWeight ≠ 0 →
      ∀ hypothesis,
        simplex.pmf hypothesis =
          (finite.weight hypothesis : ℝ) / (finite.totalWeight : ℝ))

/-- Information-facing abstraction of the bounded finite belief object. -/
class Model where
  simplexBelief : FiniteBelief → ProbabilitySimplexBelief
  shannonUncertainty : FiniteBelief → ℝ
  explicitPathMass : FiniteBelief → ℝ
  corridorCapableMass : FiniteBelief → ℝ

section Wrappers

variable [Model]

def simplexBelief (belief : FiniteBelief) : ProbabilitySimplexBelief :=
  Model.simplexBelief belief

def normalizeBelief (belief : FiniteBelief) : Distribution FieldHypothesis :=
  (simplexBelief belief).distribution

def shannonUncertainty (belief : FiniteBelief) : ℝ :=
  Model.shannonUncertainty belief

def explicitPathMass (belief : FiniteBelief) : ℝ :=
  Model.explicitPathMass belief

def corridorCapableMass (belief : FiniteBelief) : ℝ :=
  Model.corridorCapableMass belief

end Wrappers

/-- The explicit-path mass is exactly the normalized mass of the explicit-path
hypothesis. -/
abbrev ExplicitPathMassMatches (M : Model) : Prop :=
  ∀ belief,
    @Model.explicitPathMass M belief =
      ((@Model.simplexBelief M belief).distribution).pmf FieldHypothesis.explicitPath

/-- The corridor-capable mass is the sum of normalized corridor and
explicit-path masses. -/
abbrev CorridorCapableMassMatches (M : Model) : Prop :=
  ∀ belief,
    @Model.corridorCapableMass M belief =
      ((@Model.simplexBelief M belief).distribution).pmf FieldHypothesis.corridor +
        ((@Model.simplexBelief M belief).distribution).pmf FieldHypothesis.explicitPath

/-- Shannon uncertainty is nonnegative on every normalized field belief. -/
abbrev ShannonUncertaintyNonneg (M : Model) : Prop :=
  ∀ belief, 0 ≤ @Model.shannonUncertainty M belief

/-- Explicit-path mass is nonnegative and bounded by corridor-capable mass. -/
abbrev ExplicitPathMassBounded (M : Model) : Prop :=
  ∀ belief,
    0 ≤ @Model.explicitPathMass M belief ∧
      @Model.explicitPathMass M belief ≤ @Model.corridorCapableMass M belief

class Laws extends Model where
  simplex_matches_finite_belief :
    ∀ belief, SimplexMatchesFiniteBelief belief (toModel.simplexBelief belief)
  explicit_path_mass_matches : ExplicitPathMassMatches toModel
  corridor_capable_mass_matches : CorridorCapableMassMatches toModel
  shannon_uncertainty_nonneg : ShannonUncertaintyNonneg toModel
  explicit_path_mass_bounded : ExplicitPathMassBounded toModel

instance (priority := 100) lawsToModel [Laws] : Model := Laws.toModel

section LawWrappers

variable [Laws]

theorem explicit_path_mass_matches
    (belief : FiniteBelief) :
    explicitPathMass belief =
      (normalizeBelief belief).pmf FieldHypothesis.explicitPath :=
  Laws.explicit_path_mass_matches belief

theorem corridor_capable_mass_matches
    (belief : FiniteBelief) :
    corridorCapableMass belief =
      (normalizeBelief belief).pmf FieldHypothesis.corridor +
        (normalizeBelief belief).pmf FieldHypothesis.explicitPath :=
  Laws.corridor_capable_mass_matches belief

theorem shannon_uncertainty_nonneg
    (belief : FiniteBelief) :
    0 ≤ shannonUncertainty belief :=
  Laws.shannon_uncertainty_nonneg belief

theorem explicit_path_mass_bounded
    (belief : FiniteBelief) :
    0 ≤ explicitPathMass belief ∧
      explicitPathMass belief ≤ corridorCapableMass belief :=
  Laws.explicit_path_mass_bounded belief

theorem simplex_matches_finite_belief
    (belief : FiniteBelief) :
    SimplexMatchesFiniteBelief belief (simplexBelief belief) :=
  Laws.simplex_matches_finite_belief belief

end LawWrappers

end FieldInformationAPI
