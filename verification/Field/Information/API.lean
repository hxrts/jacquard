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

/-- Information-facing abstraction of the bounded finite belief object. -/
class Model where
  normalizeBelief : FiniteBelief → Distribution FieldHypothesis
  shannonUncertainty : FiniteBelief → ℝ
  explicitPathMass : FiniteBelief → ℝ
  corridorCapableMass : FiniteBelief → ℝ

section Wrappers

variable [Model]

def normalizeBelief (belief : FiniteBelief) : Distribution FieldHypothesis :=
  Model.normalizeBelief belief

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
      (@Model.normalizeBelief M belief).pmf FieldHypothesis.explicitPath

/-- The corridor-capable mass is the sum of normalized corridor and
explicit-path masses. -/
abbrev CorridorCapableMassMatches (M : Model) : Prop :=
  ∀ belief,
    @Model.corridorCapableMass M belief =
      (@Model.normalizeBelief M belief).pmf FieldHypothesis.corridor +
        (@Model.normalizeBelief M belief).pmf FieldHypothesis.explicitPath

/-- Shannon uncertainty is nonnegative on every normalized field belief. -/
abbrev ShannonUncertaintyNonneg (M : Model) : Prop :=
  ∀ belief, 0 ≤ @Model.shannonUncertainty M belief

/-- Explicit-path mass is nonnegative and bounded by corridor-capable mass. -/
abbrev ExplicitPathMassBounded (M : Model) : Prop :=
  ∀ belief,
    0 ≤ @Model.explicitPathMass M belief ∧
      @Model.explicitPathMass M belief ≤ @Model.corridorCapableMass M belief

class Laws extends Model where
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

end LawWrappers

end FieldInformationAPI
