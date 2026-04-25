import Field.ActiveBeliefDefinitive

/-
The Problem. The active-belief proof stack includes small definitive-closure theorem surfaces for
receiver disagreement, raw/useful reproduction pressure, and
effective-independence potential. Keeping them in the main definitive file made
that file exceed the Lean source-style cap.

Solution Structure.
1. Keep the core active-belief definitive theorem file focused on the main
   demand, mergeable-statistic, controller, aggregation, stress, and observer
   records.
2. Put the small extension certificates in this imported module.
-/

/-! # Active Belief Definitive Closure Extensions -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldActiveBelief

open FieldCodedDiffusion

/-! ## Receiver Disagreement Explanations -/

inductive ReceiverDisagreementCause where
  | insufficientEvidence
  | differentAcceptedLedgers
  | staleDemand
  | biasedEvidence
  | duplicatePressure
  deriving Inhabited, Repr, DecidableEq

/-- Replay-visible explanation for a disagreement between guarded receivers. -/
structure ReceiverDisagreementExplanation where
  leftReceiver : ReceiverId
  rightReceiver : ReceiverId
  cause : ReceiverDisagreementCause
  evidenceDelta : Nat
  replayVisible : Bool
  deriving Inhabited, Repr, DecidableEq

def validReceiverDisagreementExplanation
    (explanation : ReceiverDisagreementExplanation) : Prop :=
  explanation.replayVisible = true

theorem receiver_disagreement_has_replay_visible_cause
    (explanation : ReceiverDisagreementExplanation)
    (hValid : validReceiverDisagreementExplanation explanation) :
    explanation.replayVisible = true ∧
      (explanation.cause = ReceiverDisagreementCause.insufficientEvidence ∨
        explanation.cause = ReceiverDisagreementCause.differentAcceptedLedgers ∨
        explanation.cause = ReceiverDisagreementCause.staleDemand ∨
        explanation.cause = ReceiverDisagreementCause.biasedEvidence ∨
        explanation.cause = ReceiverDisagreementCause.duplicatePressure) := by
  -- Disagreement is not hidden consensus failure; the artifact names a bounded cause.
  refine ⟨hValid, ?_⟩
  cases explanation.cause <;> simp

/-! ## Useful Reproduction And Effective-Independence Potential -/

/-- Raw and useful reproduction pressure are tracked as separate achieved values. -/
structure RawUsefulReproductionPressure where
  rawPermille : Permille
  usefulPermille : Permille
  lowerUsefulPermille : Permille
  upperUsefulPermille : Permille
  activeForwardingOpportunities : Nat
  independentUsefulSuccessors : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validRawUsefulReproductionPressure
    (pressure : RawUsefulReproductionPressure) : Prop :=
  pressure.usefulPermille ≤ pressure.rawPermille ∧
    pressure.lowerUsefulPermille ≤ pressure.usefulPermille ∧
    pressure.usefulPermille ≤ pressure.upperUsefulPermille ∧
    pressure.upperUsefulPermille ≤ 1000 ∧
    pressure.independentUsefulSuccessors ≤ pressure.activeForwardingOpportunities

theorem useful_reproduction_pressure_is_bounded_by_raw_pressure
    (pressure : RawUsefulReproductionPressure)
    (hValid : validRawUsefulReproductionPressure pressure) :
    pressure.usefulPermille ≤ pressure.rawPermille := by
  -- Raw spread can exceed independently useful spread; the reverse is not certified.
  exact hValid.left

theorem useful_reproduction_pressure_in_achieved_band
    (pressure : RawUsefulReproductionPressure)
    (hValid : validRawUsefulReproductionPressure pressure) :
    pressure.lowerUsefulPermille ≤ pressure.usefulPermille ∧
      pressure.usefulPermille ≤ pressure.upperUsefulPermille := by
  -- The useful pressure band is stated over achieved useful spread.
  exact ⟨hValid.right.left, hValid.right.right.left⟩

/-- Inference potential with an explicit effective-independence deficit term. -/
structure EffectiveIndependenceInferencePotential where
  uncertainty : Nat
  wrongBasinMass : Nat
  duplicatePressure : Nat
  storagePressure : Nat
  transmissionPressure : Nat
  effectiveIndependenceDeficit : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def EffectiveIndependenceInferencePotential.total
    (potential : EffectiveIndependenceInferencePotential) : Nat :=
  potential.uncertainty +
    potential.wrongBasinMass +
    potential.duplicatePressure +
    potential.storagePressure +
    potential.transmissionPressure +
    potential.effectiveIndependenceDeficit

theorem effective_independence_potential_total_is_accounted_sum
    (potential : EffectiveIndependenceInferencePotential) :
    potential.total =
      potential.uncertainty +
        potential.wrongBasinMass +
        potential.duplicatePressure +
        potential.storagePressure +
        potential.transmissionPressure +
        potential.effectiveIndependenceDeficit := by
  -- The useful-control potential names independence progress explicitly.
  rfl

structure EffectiveIndependencePotentialDrift where
  before : EffectiveIndependenceInferencePotential
  after : EffectiveIndependenceInferencePotential
  progressCredit : Nat
  pressureBudget : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

def validEffectiveIndependencePotentialDrift
    (drift : EffectiveIndependencePotentialDrift) : Prop :=
  drift.after.total + drift.progressCredit ≤
    drift.before.total + drift.pressureBudget

theorem effective_independence_potential_drift_bounded
    (drift : EffectiveIndependencePotentialDrift)
    (hValid : validEffectiveIndependencePotentialDrift drift) :
    drift.after.total + drift.progressCredit ≤
      drift.before.total + drift.pressureBudget := by
  -- Near-critical useful control is bounded by progress plus pressure budget.
  exact hValid

end FieldActiveBelief
