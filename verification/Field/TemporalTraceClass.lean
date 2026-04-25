import Field.TemporalIndependenceLimits

/-
This file carries the Path A theorem selected for the critique pass: an
explicit trace class implies the effective-independence limit consumed by the
paper. Keeping it separate preserves the Lean source-size policy.
-/

/-! # Temporal trace-class certificates -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-- Path-A trace class: finite contact conditions that certify effective rank. -/
structure TemporalTraceClassAssumptions where
  nodeCount : Nat
  timeHorizon : Nat
  minEffectiveRank : Nat
  observedEffectiveRank : Nat
  contactEntropyFloorPermille : Permille
  bridgeDiversityFloor : Nat
  receiverArrivalPermilleFloor : Permille
  deriving Inhabited, Repr, DecidableEq, BEq

/-- Valid trace-class assumptions expose finite horizon, diversity, and arrival floors. -/
def validTemporalTraceClassAssumptions
    (assumptions : TemporalTraceClassAssumptions) : Prop :=
  0 < assumptions.nodeCount ∧
    0 < assumptions.timeHorizon ∧
    assumptions.minEffectiveRank ≤ assumptions.observedEffectiveRank ∧
    assumptions.contactEntropyFloorPermille ≤ 1000 ∧
    assumptions.receiverArrivalPermilleFloor ≤ 1000

/-- Certificate tying an explicit trace class to the effective-independence limit. -/
structure TraceClassIndependenceCertificate where
  window : CodingWindow
  summary : TemporalContactDiversitySummary
  recoveryBound : IndependenceLimitedRecoveryBound
  assumptions : TemporalTraceClassAssumptions
  deriving Inhabited, Repr, DecidableEq, BEq

def validTraceClassIndependenceCertificate
    (certificate : TraceClassIndependenceCertificate) : Prop :=
  validTemporalTraceClassAssumptions certificate.assumptions ∧
    validTemporalContactDiversitySummary certificate.summary ∧
    validIndependenceLimitedRecoveryBound certificate.recoveryBound ∧
    certificate.recoveryBound.window = certificate.window ∧
    certificate.recoveryBound.summary = certificate.summary ∧
    certificate.assumptions.minEffectiveRank ≤
      certificate.summary.effectiveRank ∧
    certificate.window.k ≤ certificate.assumptions.minEffectiveRank

theorem trace_class_temporal_contact_implies_independence_limit
    (certificate : TraceClassIndependenceCertificate)
    (hValid : validTraceClassIndependenceCertificate certificate) :
    effectiveReconstructable certificate.window certificate.summary ∧
      certificate.recoveryBound.recoverPermille ≤
        certificate.recoveryBound.effectiveRankAtLeastKPermille ∧
      certificate.assumptions.receiverArrivalPermilleFloor ≤ 1000 := by
  -- Path A proves coverage from an explicit trace class, not arbitrary traces.
  unfold effectiveReconstructable
  exact
    ⟨ Nat.le_trans hValid.right.right.right.right.right.right
        hValid.right.right.right.right.right.left
    , hValid.right.right.left.right.left
    , hValid.left.right.right.right.right ⟩

end FieldActiveBelief
