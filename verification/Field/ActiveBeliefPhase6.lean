import Field.ActiveBeliefDefinitive

/-
Phase 6 closes the critique-driven theorem-surface gaps without lengthening the
main definitive active-belief file. These records are certificate boundaries:
they state what replay/profile rows must expose, not arbitrary simulator facts.
-/

/-! # Active Belief Phase 6 theorem surfaces -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Demand Variance Deflection -/

/-- Replay-facing bound on how much bounded demand can deflect receiver variance. -/
structure DemandVarianceDeflectionCertificate where
  demand : DemandSummary
  noDemandVariance : Nat
  demandGuidedVariance : Nat
  deflectionBound : Nat
  demandByteBudget : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid certificates keep demand bounded and expose a deterministic deflection cap. -/
def validDemandVarianceDeflectionCertificate
    (certificate : DemandVarianceDeflectionCertificate) : Prop :=
  validDemandSummary certificate.demand ∧
    certificate.demandGuidedVariance ≤
      certificate.noDemandVariance + certificate.deflectionBound ∧
    certificate.deflectionBound ≤ certificate.demandByteBudget

theorem demand_induced_allocation_variance_deflection_bounded
    (certificate : DemandVarianceDeflectionCertificate)
    (hValid : validDemandVarianceDeflectionCertificate certificate) :
    certificate.demandGuidedVariance ≤
        certificate.noDemandVariance + certificate.deflectionBound ∧
      certificate.deflectionBound ≤ certificate.demandByteBudget ∧
      (ActiveMessage.demand certificate.demand).contributionId? = none := by
  -- Demand remains non-evidential while steering allocation inside this bound.
  exact
    ⟨ hValid.right.left
    , hValid.right.right
    , demand_message_carries_no_contribution certificate.demand ⟩

/-! ## Partial Accumulation And Quality -/

/-- Certificate that a task quality map factors through the mergeable statistic. -/
structure MonoidHomomorphicQualityCertificate
    (task : MergeableStatistic) where
  partialStatistic : task.Carrier
  remainderStatistic : task.Carrier
  globalStatistic : task.Carrier
  partialQuality : Nat
  globalQuality : Nat
  partialEvidenceCount : Nat
  globalEvidenceCount : Nat

/-- Valid certificates expose the homomorphic merge and monotone quality facts. -/
def validMonoidHomomorphicQualityCertificate
    (task : MergeableStatistic)
    (certificate : MonoidHomomorphicQualityCertificate task) : Prop :=
  certificate.globalStatistic =
      task.merge certificate.partialStatistic certificate.remainderStatistic ∧
    certificate.partialQuality ≤ certificate.globalQuality ∧
    certificate.partialEvidenceCount ≤ certificate.globalEvidenceCount

theorem monoid_homomorphism_preserves_decision_quality_under_partial_accumulation
    (task : MergeableStatistic)
    (certificate : MonoidHomomorphicQualityCertificate task)
    (hValid : validMonoidHomomorphicQualityCertificate task certificate) :
    certificate.partialQuality ≤ certificate.globalQuality ∧
      certificate.partialEvidenceCount ≤ certificate.globalEvidenceCount := by
  -- This applies only where the task quality functional has this certificate.
  exact ⟨hValid.right.left, hValid.right.right⟩

end FieldActiveBelief
