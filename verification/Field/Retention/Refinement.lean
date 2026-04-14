import Field.Retention.Instance
import Field.Router.Publication

/-
The Problem. The retention instance must be shown to sit below the existing
Field truth surfaces rather than competing with them.

Solution Structure.
1. Prove that stepping only the retention component leaves the local model,
   publication candidate, and canonical-route input unchanged.
2. Prove that the first executable retention instance conserves custody and does
   not invent payload identities.
3. Add small forwarding-admissibility theorems over reduced runtime witnesses.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRetentionRefinement

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterPublication
open FieldRouterCanonical
open FieldRetentionAPI
open FieldRetentionInstance

/-! ## Controlled-View Separation -/

def publishFromControlledView
    (publisher : NodeId)
    (destination : DestinationClass)
    (view : RetentionControlledView) : PublishedCandidate :=
  publishCandidate publisher destination view.localState

theorem retained_payloads_do_not_change_posterior_truth
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (view : RetentionControlledView) :
    (stepControlledView interface input view).localState.posterior =
      view.localState.posterior := by
  rfl

theorem retained_payloads_do_not_change_reduced_summary
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (view : RetentionControlledView) :
    (stepControlledView interface input view).localState.summary =
      view.localState.summary := by
  rfl

theorem retained_payloads_do_not_change_local_order_parameter
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (view : RetentionControlledView) :
    (stepControlledView interface input view).localState.orderParameter =
      view.localState.orderParameter := by
  rfl

theorem retained_payloads_do_not_publish_routes
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (publisher : NodeId)
    (destination : DestinationClass)
    (view : RetentionControlledView) :
    publishFromControlledView publisher destination (stepControlledView interface input view) =
      publishFromControlledView publisher destination view := by
  rfl

theorem retained_payloads_do_not_create_canonical_truth
    (interface : RetentionInterface)
    (input : RetentionPolicyInput)
    (destination : DestinationClass)
    (view : RetentionControlledView) :
    canonicalBestRoute destination (stepControlledView interface input view).routes =
      canonicalBestRoute destination view.routes := by
  rfl

/-! ## Custody Preservation -/

theorem payload_custody_conserved_under_retain_carry_forward_drop
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (hCoherent : state.coherent)
    (hBounded : state.buffer.length ≤ retentionCapacity) :
    (retentionStepImpl input state).accountedCount = state.accountedCount := by
  unfold RetentionState.accountedCount retentionStepImpl
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      cases hState : state.buffer with
      | nil =>
          simp [mkRetentionState, normalizeBuffer]
      | cons head tail =>
          simp [ageBuffer, hState] at hBuffer
  | cons token tail =>
      have hLen : (token :: tail).length = state.buffer.length := by
        rw [← hBuffer, ageBuffer_length]
      have hBoundedCons : (token :: tail).length ≤ retentionCapacity := by
        simpa [hLen] using hBounded
      have hBoundedTail : tail.length ≤ retentionCapacity := by
        exact Nat.le_trans (Nat.le_succ tail.length) hBoundedCons
      have hTakeCons : normalizeBuffer (token :: tail) = token :: tail := by
        exact (List.take_eq_self_iff (token :: tail)).2 hBoundedCons
      have hTakeTail : normalizeBuffer tail = tail := by
        exact (List.take_eq_self_iff tail).2 hBoundedTail
      have hCount : state.buffer.length = tail.length + 1 := by
        simpa using hLen.symm
      cases hDecision : selectRetentionDecisionImpl input token
      · simp [hDecision, mkRetentionState, hTakeCons, hCount]
      · simp [hDecision, mkRetentionState, hTakeCons, hCount]
      · simp [hDecision, mkRetentionState, hTakeTail, hCount, Nat.add_left_comm, Nat.add_comm]
      · simp [hDecision, mkRetentionState, hTakeTail, hCount, Nat.add_assoc, Nat.add_left_comm,
          Nat.add_comm]

theorem no_payload_creation_from_silence
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (token : PayloadToken)
    (hMem : token ∈ (retentionStepImpl input state).buffer) :
    ∃ prior ∈ state.buffer, prior.messageId = token.messageId :=
  retentionStepImpl_preserves_message_origin input state token hMem

theorem checkpoint_restore_preserves_retained_multiset
    (state : RetentionState)
    (hBounded : state.buffer.length ≤ retentionCapacity) :
    (restoreRetentionStateImpl state).buffer = state.buffer :=
  restoreRetentionStateImpl_preserves_buffer_of_bounded_state state hBounded

theorem checkpoint_restore_preserves_retention_timestamps
    (state : RetentionState)
    (hBounded : state.buffer.length ≤ retentionCapacity) :
    ((restoreRetentionStateImpl state).buffer.map PayloadToken.retainedAtTick) =
      (state.buffer.map PayloadToken.retainedAtTick) := by
  rw [restoreRetentionStateImpl_preserves_buffer_of_bounded_state state hBounded]

theorem checkpoint_restore_preserves_retention_epochs
    (state : RetentionState)
    (hBounded : state.buffer.length ≤ retentionCapacity) :
    ((restoreRetentionStateImpl state).buffer.map PayloadToken.admittedRouteEpoch) =
      (state.buffer.map PayloadToken.admittedRouteEpoch) := by
  rw [restoreRetentionStateImpl_preserves_buffer_of_bounded_state state hBounded]

/-! ## Forwarding Admissibility -/

structure ContinuationEnvelope where
  selectedNeighbor : NodeId
  admissibleNeighbors : List NodeId
  deriving Inhabited, Repr, DecidableEq, BEq

def ForwardWithinContinuationEnvelope
    (envelope : ContinuationEnvelope)
    (neighbor : NodeId) : Prop :=
  neighbor = envelope.selectedNeighbor ∨ neighbor ∈ envelope.admissibleNeighbors

def InstalledRouteWitness
    (input : RetentionPolicyInput)
    (view : RetentionControlledView)
    (destination : DestinationClass) : Prop :=
  input.routeInstalled = true →
    ∃ route ∈ view.routes, CanonicalRouteEligible destination route

theorem forward_decision_characterizes_runtime_preconditions
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (hForward : selectRetentionDecisionImpl input token = .forward) :
    input.routeInstalled = true ∧ input.continuity = .steady ∧
      input.continuationAvailable = true ∧
      token.admittedRouteEpoch = input.activeRouteEpoch ∧
      (token.custodyAge input.nowTick).value ≤ input.custodyTimeout.value := by
  by_cases hTimeout : tokenCustodyExpired input token
  · simp [selectRetentionDecisionImpl, hTimeout] at hForward
  · by_cases hRoute : input.routeInstalled
    · by_cases hEpoch : tokenRouteEpochCurrent input token
      · by_cases hDrop : token.ageClass = .stale ∧ input.supportBand = .low
        · simp [selectRetentionDecisionImpl, hTimeout, hRoute, hEpoch, hDrop] at hForward
        · by_cases hBootstrap : input.continuity = .bootstrap ∧ input.uncertaintyBand = .risky
          · simp [selectRetentionDecisionImpl, hTimeout, hRoute, hEpoch, hDrop, hBootstrap] at hForward
          · by_cases hCarry : input.posture = .retentionBiased ∧ input.supportBand ≠ .low
            · simp [selectRetentionDecisionImpl, hTimeout, hRoute, hEpoch, hBootstrap, hCarry] at hForward
            · have hFresh :
                  (token.custodyAge input.nowTick).value ≤ input.custodyTimeout.value := by
                  unfold tokenCustodyExpired at hTimeout
                  have hNotLt : ¬ input.custodyTimeout.value < (token.custodyAge input.nowTick).value := by
                    simpa using hTimeout
                  exact Nat.le_of_not_gt hNotLt
              simp [selectRetentionDecisionImpl, hTimeout, hRoute, hEpoch, hDrop, hBootstrap,
                hCarry] at hForward
              exact ⟨hRoute, hForward.1, hForward.2, by simpa [tokenRouteEpochCurrent] using hEpoch,
                hFresh⟩
      · simp [selectRetentionDecisionImpl, hTimeout, hRoute, hEpoch] at hForward
    · simp [selectRetentionDecisionImpl, hTimeout, hRoute] at hForward

theorem forward_requires_admitted_runtime_path
    (input : RetentionPolicyInput)
    (view : RetentionControlledView)
    (token : PayloadToken)
    (hForward : selectRetentionDecisionImpl input token = .forward)
    (hWitness : InstalledRouteWitness input view token.destination) :
    ∃ route ∈ view.routes,
      CanonicalRouteEligible token.destination route ∧
        input.continuationAvailable = true ∧
        token.admittedRouteEpoch = input.activeRouteEpoch := by
  rcases forward_decision_characterizes_runtime_preconditions input token hForward with
    ⟨hRoute, _hSteady, hContinuation, hEpoch, _hFresh⟩
  rcases hWitness hRoute with ⟨route, hMem, hEligible⟩
  exact ⟨route, hMem, hEligible, hContinuation, hEpoch⟩

theorem forward_stays_inside_continuation_envelope
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (envelope : ContinuationEnvelope)
    (neighbor : NodeId)
    (hForward : selectRetentionDecisionImpl input token = .forward)
    (hAllowed : neighbor = envelope.selectedNeighbor ∨
      neighbor ∈ envelope.admissibleNeighbors) :
    input.continuationAvailable = true ∧
      token.admittedRouteEpoch = input.activeRouteEpoch ∧
      ForwardWithinContinuationEnvelope envelope neighbor := by
  rcases forward_decision_characterizes_runtime_preconditions input token hForward with
    ⟨_hRoute, _hSteady, hContinuation, hEpoch, _hFresh⟩
  exact ⟨hContinuation, hEpoch, hAllowed⟩

theorem retention_biased_posture_has_carry_witness :
    ∃ input token,
      input.posture = .retentionBiased ∧
        input.supportBand = .medium ∧
        selectRetentionDecisionImpl input token = .carry := by
  let input : RetentionPolicyInput :=
    { regime := .retentionFavorable
      posture := .retentionBiased
      supportBand := .medium
      uncertaintyBand := .stable
      continuity := .degradedSteady
      continuationAvailable := false
      routeInstalled := true
      nowTick := ⟨4⟩
      activeRouteEpoch := ⟨9⟩
      custodyTimeout := ⟨5⟩ }
  let token : PayloadToken :=
    { messageId := 1
      destination := .corridorA
      sizeClass := .small
      ageClass := .fresh
      custodyClass := .localOnly
      retainedAtTick := ⟨2⟩
      lastRetryTick := ⟨2⟩
      admittedRouteEpoch := ⟨9⟩ }
  refine ⟨input, token, rfl, rfl, ?_⟩
  unfold input token
  exact
    selectRetentionDecision_retention_biased_carries
      _ _ (by decide) rfl rfl
      (Or.inl (by decide))
      (Or.inl (by decide))
      rfl
      (by decide)

theorem retention_biased_posture_has_forward_witness :
    ∃ input token,
      input.posture = .retentionBiased ∧
        input.supportBand = .low ∧
        selectRetentionDecisionImpl input token = .forward := by
  let input : RetentionPolicyInput :=
    { regime := .retentionFavorable
      posture := .retentionBiased
      supportBand := .low
      uncertaintyBand := .stable
      continuity := .steady
      continuationAvailable := true
      routeInstalled := true
      nowTick := ⟨4⟩
      activeRouteEpoch := ⟨9⟩
      custodyTimeout := ⟨5⟩ }
  let token : PayloadToken :=
    { messageId := 2
      destination := .corridorA
      sizeClass := .small
      ageClass := .fresh
      custodyClass := .localOnly
      retainedAtTick := ⟨2⟩
      lastRetryTick := ⟨2⟩
      admittedRouteEpoch := ⟨9⟩ }
  refine ⟨input, token, rfl, rfl, ?_⟩
  unfold input token
  exact
    selectRetentionDecision_steady_with_continuation_forwards
      _ _ (by decide) rfl rfl
      (Or.inl (by decide))
      (Or.inl (by decide))
      (Or.inr rfl)
      rfl
      rfl

theorem retention_biased_posture_permits_but_does_not_force_retention :
    (∃ input token,
      input.posture = .retentionBiased ∧
        input.supportBand = .medium ∧
        selectRetentionDecisionImpl input token = .carry) ∧
    (∃ input token,
      input.posture = .retentionBiased ∧
        input.supportBand = .low ∧
        selectRetentionDecisionImpl input token = .forward) := by
  exact ⟨retention_biased_posture_has_carry_witness,
    retention_biased_posture_has_forward_witness⟩

end FieldRetentionRefinement
