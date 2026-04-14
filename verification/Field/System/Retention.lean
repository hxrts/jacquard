import Field.Retention.Fixtures
import Field.System.EndToEnd

/-
The Problem. The reduced retention layer needs one system-facing integration
point above async transport and lifecycle state.

Solution Structure.
1. Define a small composite state that keeps reduced system state and retention
   custody side by side.
2. Define a delay-only carry step and a full system-retention step.
3. Prove that retention preserves canonical route truth and that custody work is
   bounded per step.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemRetention

open FieldAsyncAPI
open FieldNetworkAPI
open FieldRouterCanonical
open FieldRetentionAPI
open FieldRetentionInstance
open FieldRetentionRefinement
open FieldSystemEndToEnd

/-! ## Composite State And Steps -/

structure RetentionSystemState where
  system : EndToEndState
  retention : RetentionState

def asyncDelayCarryStep
    (state : RetentionSystemState) : RetentionSystemState :=
  { system :=
      { async := transportStep state.system.async
        lifecycle := state.system.lifecycle }
    retention := state.retention }

def admittedForwardingStep
    (input : RetentionPolicyInput)
    (state : RetentionSystemState) : RetentionSystemState :=
  { system := systemStep state.system
    retention := retentionStepImpl input state.retention }

def retentionWorkUnitsOfStep
    (input : RetentionPolicyInput)
    (state : RetentionSystemState) : Nat :=
  match (retentionStepImpl input state.retention).lastDecision with
  | none => 0
  | some _ => 1

/-! ## System-Level Retention Theorems -/

theorem retained_tokens_survive_async_delay
    (state : RetentionSystemState) :
    (asyncDelayCarryStep state).retention = state.retention := by
  rfl

theorem admitted_runtime_forwarding_consumes_retained_token
    (input : RetentionPolicyInput)
    (state : RetentionSystemState)
    (_hCoherent : state.retention.coherent)
    (hBounded : state.retention.buffer.length ≤ retentionCapacity)
    (hForward : (retentionStepImpl input state.retention).lastDecision = some .forward) :
    (admittedForwardingStep input state).retention.deliveredCount =
      state.retention.deliveredCount + 1 ∧
      (admittedForwardingStep input state).retention.buffer.length + 1 =
        state.retention.buffer.length := by
  unfold admittedForwardingStep
  unfold retentionStepImpl at hForward ⊢
  cases hBuffer : ageBuffer state.retention.buffer with
  | nil =>
      simp [hBuffer, mkRetentionState] at hForward
  | cons token tail =>
      have hLen : (token :: tail).length = state.retention.buffer.length := by
        rw [← hBuffer, ageBuffer_length]
      have hBoundedCons : (token :: tail).length ≤ retentionCapacity := by
        simpa [hLen] using hBounded
      have hBoundedTail : tail.length ≤ retentionCapacity := by
        exact Nat.le_trans (Nat.le_succ tail.length) hBoundedCons
      have hTakeTail : normalizeBuffer tail = tail := by
        exact (List.take_eq_self_iff tail).2 hBoundedTail
      cases hDecision : selectRetentionDecisionImpl input token
      · simp [hBuffer, hDecision, mkRetentionState] at hForward
      · simp [hBuffer, hDecision, mkRetentionState] at hForward
      · have hLen' : state.retention.buffer.length = tail.length + 1 := by
            simpa using hLen.symm
        simp [hBuffer, hDecision, mkRetentionState, hTakeTail, hLen']
      · simp [hBuffer, hDecision, mkRetentionState] at hForward

theorem silence_does_not_strengthen_delivery_claims
    (_input : RetentionPolicyInput)
    (state : RetentionSystemState)
    (destination : DestinationClass) :
    canonicalBestRoute destination (asyncDelayCarryStep state).system.lifecycle =
      canonicalBestRoute destination state.system.lifecycle := by
  rfl

theorem no_delivery_without_prior_custody
    (input : RetentionPolicyInput)
    (state : RetentionSystemState)
    (hForward : (retentionStepImpl input state.retention).lastDecision = some .forward) :
    state.retention.buffer ≠ [] := by
  unfold retentionStepImpl at hForward
  cases hBuffer : ageBuffer state.retention.buffer with
  | nil =>
      simp [hBuffer, mkRetentionState] at hForward
  | cons token tail =>
      intro hEmpty
      simp [hEmpty, ageBuffer] at hBuffer

theorem retained_tokens_do_not_strengthen_canonical_route_truth
    (input : RetentionPolicyInput)
    (state : RetentionSystemState)
    (destination : DestinationClass) :
    canonicalBestRoute destination (admittedForwardingStep input state).system.lifecycle =
      canonicalBestRoute destination (systemStep state.system).lifecycle := by
  rfl

theorem retention_work_stays_bounded_per_system_step
    (input : RetentionPolicyInput)
    (state : RetentionSystemState) :
    retentionWorkUnitsOfStep input state ≤ 1 := by
  unfold retentionWorkUnitsOfStep
  cases (retentionStepImpl input state.retention).lastDecision <;> simp

end FieldSystemRetention
