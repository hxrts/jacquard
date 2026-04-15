import Field.Retention.API

/-
The Problem. The retention API needs one small executable instance so later
proofs can reason about bounded custody without appealing only to abstract law
bundles.

Solution Structure.
1. Choose one small fixed capacity and one simple retain/carry/forward/drop policy.
2. Implement bounded aging, injection, restore, and one-step retention update.
3. Prove the first coherence, boundedness, and explicit-outcome lemmas.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRetentionInstance

open FieldModelAPI
open FieldRetentionAPI

/-! ## Capacity And Aging -/

def retentionCapacity : Nat := 4

def ageRank : AgeClass → Nat
  | .fresh => 0
  | .warm => 1
  | .stale => 2

def advanceAge : AgeClass → AgeClass
  | .fresh => .warm
  | .warm => .stale
  | .stale => .stale

def ageToken (token : PayloadToken) : PayloadToken :=
  { token with ageClass := advanceAge token.ageClass }

def ageBuffer (buffer : List PayloadToken) : List PayloadToken :=
  buffer.map ageToken

def normalizeBuffer (buffer : List PayloadToken) : List PayloadToken :=
  buffer.take retentionCapacity

def mkRetentionState
    (buffer : List PayloadToken)
    (deliveredCount droppedCount : Nat)
    (lastDecision : Option RetentionDecision) : RetentionState :=
  let bounded := normalizeBuffer buffer
  { buffer := bounded
    retainedCount := bounded.length
    deliveredCount := deliveredCount
    droppedCount := droppedCount
    lastDecision := lastDecision }

/-! ## Executable Retention Policy -/

def tokenCustodyExpired
    (input : RetentionPolicyInput)
    (token : PayloadToken) : Bool :=
  input.custodyTimeout.value < (token.custodyAge input.nowTick).value

def tokenRouteEpochCurrent
    (input : RetentionPolicyInput)
    (token : PayloadToken) : Bool :=
  token.admittedRouteEpoch = input.activeRouteEpoch

def selectRetentionDecisionImpl
    (input : RetentionPolicyInput)
    (token : PayloadToken) : RetentionDecision :=
  if tokenCustodyExpired input token then
    .drop
  else if !input.routeInstalled then
    .retain
  else if !tokenRouteEpochCurrent input token then
    .retain
  else if token.ageClass = .stale && input.supportBand = .low then
    .drop
  else if input.continuity = .bootstrap && input.uncertaintyBand = .risky then
    .retain
  else if input.posture = .retentionBiased && input.supportBand ≠ .low then
    .carry
  else if input.continuity = .steady && input.continuationAvailable then
    .forward
  else
    .retain

def injectPayloadImpl
    (token : PayloadToken)
    (state : RetentionState) : RetentionState :=
  mkRetentionState (state.buffer ++ [token]) state.deliveredCount state.droppedCount state.lastDecision

def restoreRetentionStateImpl
    (state : RetentionState) : RetentionState :=
  mkRetentionState state.buffer state.deliveredCount state.droppedCount none

def retentionStepImpl
    (input : RetentionPolicyInput)
    (state : RetentionState) : RetentionState :=
  match ageBuffer state.buffer with
  | [] =>
      mkRetentionState [] state.deliveredCount state.droppedCount none
  | token :: tail =>
      match selectRetentionDecisionImpl input token with
      | .retain =>
          mkRetentionState (token :: tail) state.deliveredCount state.droppedCount (some .retain)
      | .carry =>
          mkRetentionState (token :: tail) state.deliveredCount state.droppedCount (some .carry)
      | .forward =>
          mkRetentionState tail (state.deliveredCount + 1) state.droppedCount (some .forward)
      | .drop =>
          mkRetentionState tail state.deliveredCount (state.droppedCount + 1) (some .drop)

def retentionInterfaceImpl : RetentionInterface :=
  { selectRetentionDecision := selectRetentionDecisionImpl
    retentionStep := retentionStepImpl
    injectPayload := injectPayloadImpl
    restoreRetentionState := restoreRetentionStateImpl }

/-! ## Boundedness And Coherence -/

theorem normalizeBuffer_length_le_capacity
    (buffer : List PayloadToken) :
    (normalizeBuffer buffer).length ≤ retentionCapacity := by
  unfold normalizeBuffer retentionCapacity
  simp

theorem mkRetentionState_coherent
    (buffer : List PayloadToken)
    (deliveredCount droppedCount : Nat)
    (lastDecision : Option RetentionDecision) :
    (mkRetentionState buffer deliveredCount droppedCount lastDecision).coherent := by
  simp [RetentionState.coherent, mkRetentionState]

theorem ageBuffer_length
    (buffer : List PayloadToken) :
    (ageBuffer buffer).length = buffer.length := by
  simp [ageBuffer]

theorem ageRank_advanceAge_monotone
    (age : AgeClass) :
    ageRank age ≤ ageRank (advanceAge age) := by
  cases age <;> decide

theorem ageToken_not_younger
    (token : PayloadToken) :
    ageRank token.ageClass ≤ ageRank (ageToken token).ageClass := by
  simpa [ageToken] using ageRank_advanceAge_monotone token.ageClass

theorem ageToken_preserves_retainedAtTick
    (token : PayloadToken) :
    (ageToken token).retainedAtTick = token.retainedAtTick := by
  rfl

theorem ageToken_preserves_lastRetryTick
    (token : PayloadToken) :
    (ageToken token).lastRetryTick = token.lastRetryTick := by
  rfl

theorem ageToken_preserves_admittedRouteEpoch
    (token : PayloadToken) :
    (ageToken token).admittedRouteEpoch = token.admittedRouteEpoch := by
  rfl

/-! ## Decision And Step Lemmas -/

theorem selectRetentionDecision_no_route_retains
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (hTimeout : tokenCustodyExpired input token = false)
    (hRoute : input.routeInstalled = false) :
    selectRetentionDecisionImpl input token = .retain := by
  by_cases hEpoch : tokenRouteEpochCurrent input token
  · simp [selectRetentionDecisionImpl, hTimeout, hRoute]
  · simp [selectRetentionDecisionImpl, hTimeout, hRoute]

theorem selectRetentionDecision_expired_drops
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (hTimeout : tokenCustodyExpired input token = true) :
    selectRetentionDecisionImpl input token = .drop := by
  simp [selectRetentionDecisionImpl, hTimeout]

theorem selectRetentionDecision_stale_epoch_retains
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (hTimeout : tokenCustodyExpired input token = false)
    (hRoute : input.routeInstalled = true)
    (hEpoch : tokenRouteEpochCurrent input token = false) :
    selectRetentionDecisionImpl input token = .retain := by
  simp [selectRetentionDecisionImpl, hTimeout, hRoute, hEpoch]

theorem selectRetentionDecision_retention_biased_carries
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (hTimeout : tokenCustodyExpired input token = false)
    (hRoute : input.routeInstalled = true)
    (hEpoch : tokenRouteEpochCurrent input token = true)
    (hNoDrop : token.ageClass ≠ .stale ∨ input.supportBand ≠ .low)
    (hNoBootstrap : input.continuity ≠ .bootstrap ∨ input.uncertaintyBand ≠ .risky)
    (hPosture : input.posture = .retentionBiased)
    (hSupport : input.supportBand ≠ .low) :
    selectRetentionDecisionImpl input token = .carry := by
  have hDropFalse : ¬ (token.ageClass = .stale ∧ input.supportBand = .low) := by
    intro h
    rcases hNoDrop with hAge | hSupport'
    · exact hAge h.1
    · exact hSupport' h.2
  have hBootstrapFalse : ¬ (input.continuity = .bootstrap ∧ input.uncertaintyBand = .risky) := by
    intro h
    rcases hNoBootstrap with hCont | hUncertainty
    · exact hCont h.1
    · exact hUncertainty h.2
  unfold selectRetentionDecisionImpl
  simp [hTimeout, hRoute, hEpoch, hBootstrapFalse, hPosture, hSupport]

theorem selectRetentionDecision_steady_with_continuation_forwards
    (input : RetentionPolicyInput)
    (token : PayloadToken)
    (hTimeout : tokenCustodyExpired input token = false)
    (hRoute : input.routeInstalled = true)
    (hEpoch : tokenRouteEpochCurrent input token = true)
    (hNoDrop : token.ageClass ≠ .stale ∨ input.supportBand ≠ .low)
    (hNoBootstrap : input.continuity ≠ .bootstrap ∨ input.uncertaintyBand ≠ .risky)
    (hNoCarry : input.posture ≠ .retentionBiased ∨ input.supportBand = .low)
    (hSteady : input.continuity = .steady)
    (hContinuation : input.continuationAvailable = true) :
    selectRetentionDecisionImpl input token = .forward := by
  have hDropFalse : ¬ (token.ageClass = .stale ∧ input.supportBand = .low) := by
    intro h
    rcases hNoDrop with hAge | hSupport'
    · exact hAge h.1
    · exact hSupport' h.2
  have hBootstrapFalse : ¬ (input.continuity = .bootstrap ∧ input.uncertaintyBand = .risky) := by
    intro h
    rcases hNoBootstrap with hCont | hUncertainty
    · exact hCont h.1
    · exact hUncertainty h.2
  have hCarryFalse : ¬ (input.posture = .retentionBiased ∧ input.supportBand ≠ .low) := by
    intro h
    rcases hNoCarry with hPosture | hLow
    · exact hPosture h.1
    · exact h.2 hLow
  unfold selectRetentionDecisionImpl
  simp [hTimeout, hRoute, hEpoch, hDropFalse, hCarryFalse, hSteady,
    hContinuation]

theorem retentionStepImpl_coherent
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (_hCoherent : state.coherent) :
    (retentionStepImpl input state).coherent := by
  unfold retentionStepImpl
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      simpa [hBuffer] using mkRetentionState_coherent [] state.deliveredCount state.droppedCount none
  | cons token tail =>
      cases hDecision : selectRetentionDecisionImpl input token
      · simpa [hBuffer, hDecision] using
          mkRetentionState_coherent (token :: tail) state.deliveredCount state.droppedCount (some .retain)
      · simpa [hBuffer, hDecision] using
          mkRetentionState_coherent (token :: tail) state.deliveredCount state.droppedCount (some .carry)
      · simpa [hBuffer, hDecision] using
          mkRetentionState_coherent tail (state.deliveredCount + 1) state.droppedCount (some .forward)
      · simpa [hBuffer, hDecision] using
          mkRetentionState_coherent tail state.deliveredCount (state.droppedCount + 1) (some .drop)

theorem injectPayloadImpl_coherent
    (token : PayloadToken)
    (state : RetentionState)
    (_hCoherent : state.coherent) :
    (injectPayloadImpl token state).coherent := by
  simpa [injectPayloadImpl] using
    mkRetentionState_coherent (state.buffer ++ [token]) state.deliveredCount state.droppedCount state.lastDecision

theorem restoreRetentionStateImpl_coherent
    (state : RetentionState)
    (_hCoherent : state.coherent) :
    (restoreRetentionStateImpl state).coherent := by
  simpa [restoreRetentionStateImpl] using
    mkRetentionState_coherent state.buffer state.deliveredCount state.droppedCount none

theorem retentionStepImpl_bounded
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (_hCoherent : state.coherent) :
    (retentionStepImpl input state).buffer.length ≤ retentionCapacity := by
  unfold retentionStepImpl
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      simpa [hBuffer, mkRetentionState] using normalizeBuffer_length_le_capacity ([] : List PayloadToken)
  | cons token tail =>
      cases hDecision : selectRetentionDecisionImpl input token <;>
        simp [hDecision, mkRetentionState, normalizeBuffer_length_le_capacity]

theorem injectPayloadImpl_bounded
    (token : PayloadToken)
    (state : RetentionState)
    (_hCoherent : state.coherent) :
    (injectPayloadImpl token state).buffer.length ≤ retentionCapacity := by
  simpa [injectPayloadImpl, mkRetentionState] using
    normalizeBuffer_length_le_capacity (state.buffer ++ [token])

theorem restoreRetentionStateImpl_bounded
    (state : RetentionState)
    (_hCoherent : state.coherent) :
    (restoreRetentionStateImpl state).buffer.length ≤ retentionCapacity := by
  simpa [restoreRetentionStateImpl, mkRetentionState] using
    normalizeBuffer_length_le_capacity state.buffer

theorem retentionStepImpl_drop_explicit
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (hChanged : (retentionStepImpl input state).droppedCount ≠ state.droppedCount) :
    (retentionStepImpl input state).lastDecision = some .drop := by
  unfold retentionStepImpl at hChanged ⊢
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      simp [hBuffer, mkRetentionState] at hChanged
  | cons token tail =>
      cases hDecision : selectRetentionDecisionImpl input token <;>
        simp [hBuffer, hDecision, mkRetentionState] at hChanged ⊢

theorem retentionStepImpl_delivery_explicit
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (hChanged : (retentionStepImpl input state).deliveredCount ≠ state.deliveredCount) :
    (retentionStepImpl input state).lastDecision = some .forward := by
  unfold retentionStepImpl at hChanged ⊢
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      simp [hBuffer, mkRetentionState] at hChanged
  | cons token tail =>
      cases hDecision : selectRetentionDecisionImpl input token <;>
        simp [hBuffer, hDecision, mkRetentionState] at hChanged ⊢

theorem mem_ageBuffer_implies_origin
    (buffer : List PayloadToken)
    (token : PayloadToken)
    (hMem : token ∈ ageBuffer buffer) :
    ∃ prior ∈ buffer, prior.messageId = token.messageId := by
  unfold ageBuffer at hMem
  rcases List.mem_map.1 hMem with ⟨prior, hPrior, hEq⟩
  refine ⟨prior, hPrior, ?_⟩
  cases hEq
  rfl

theorem mem_take_implies_mem
    (n : Nat)
    (buffer : List PayloadToken)
    (token : PayloadToken)
    (hMem : token ∈ buffer.take n) :
    token ∈ buffer := by
  induction n generalizing buffer with
  | zero =>
      cases buffer <;> simp at hMem
  | succ n ih =>
      cases buffer with
      | nil =>
          simp at hMem
      | cons head tail =>
          simp at hMem ⊢
          rcases hMem with rfl | hTail
          · exact Or.inl rfl
          · exact Or.inr (ih tail hTail)

theorem mem_normalizeBuffer_implies_mem
    (buffer : List PayloadToken)
    (token : PayloadToken)
    (hMem : token ∈ normalizeBuffer buffer) :
    token ∈ buffer := by
  exact mem_take_implies_mem retentionCapacity buffer token (by simpa [normalizeBuffer] using hMem)

theorem retentionStepImpl_preserves_message_origin
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (token : PayloadToken)
    (hMem : token ∈ (retentionStepImpl input state).buffer) :
    ∃ prior ∈ state.buffer, prior.messageId = token.messageId := by
  unfold retentionStepImpl at hMem
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      have hImpossible : token ∈ ([] : List PayloadToken) := by
        simp [hBuffer, mkRetentionState, normalizeBuffer] at hMem
      cases hImpossible
  | cons head tail =>
      cases hDecision : selectRetentionDecisionImpl input head
      · have hAged : token ∈ ageBuffer state.buffer := by
          simp [hBuffer, hDecision, mkRetentionState] at hMem
          rw [hBuffer]
          exact mem_normalizeBuffer_implies_mem (head :: tail) token hMem
        exact mem_ageBuffer_implies_origin state.buffer token hAged
      · have hAged : token ∈ ageBuffer state.buffer := by
          simp [hBuffer, hDecision, mkRetentionState] at hMem
          rw [hBuffer]
          exact mem_normalizeBuffer_implies_mem (head :: tail) token hMem
        exact mem_ageBuffer_implies_origin state.buffer token hAged
      · have hTail : token ∈ tail := by
          simp [hBuffer, hDecision, mkRetentionState] at hMem
          exact mem_normalizeBuffer_implies_mem tail token hMem
        have hAged : token ∈ ageBuffer state.buffer := by
          rw [hBuffer]
          simp [hTail]
        exact mem_ageBuffer_implies_origin state.buffer token hAged
      · have hTail : token ∈ tail := by
          simp [hBuffer, hDecision, mkRetentionState] at hMem
          exact mem_normalizeBuffer_implies_mem tail token hMem
        have hAged : token ∈ ageBuffer state.buffer := by
          rw [hBuffer]
          simp [hTail]
        exact mem_ageBuffer_implies_origin state.buffer token hAged

theorem restoreRetentionStateImpl_preserves_message_origin
    (state : RetentionState)
    (token : PayloadToken)
    (hMem : token ∈ (restoreRetentionStateImpl state).buffer) :
    ∃ prior ∈ state.buffer, prior.messageId = token.messageId := by
  unfold restoreRetentionStateImpl mkRetentionState at hMem
  have hIn : token ∈ state.buffer := by
    exact mem_normalizeBuffer_implies_mem state.buffer token (by simpa [normalizeBuffer] using hMem)
  exact ⟨token, hIn, rfl⟩

theorem forward_step_removes_at_most_one_token
    (input : RetentionPolicyInput)
    (state : RetentionState)
    (_hCoherent : state.coherent)
    (hBounded : state.buffer.length ≤ retentionCapacity) :
    state.buffer.length ≤ (retentionStepImpl input state).buffer.length + 1 := by
  unfold retentionStepImpl
  cases hBuffer : ageBuffer state.buffer with
  | nil =>
      simp [ageBuffer] at hBuffer
      simp [hBuffer, mkRetentionState]
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
      have hLen' : state.buffer.length = tail.length + 1 := by
        simpa using hLen.symm
      cases hDecision : selectRetentionDecisionImpl input token
      · simp [hDecision, mkRetentionState, hTakeCons, hLen']
      · simp [hDecision, mkRetentionState, hTakeCons, hLen']
      · simp [hDecision, mkRetentionState, hTakeTail, hLen']
      · simp [hDecision, mkRetentionState, hTakeTail, hLen']

theorem restoreRetentionStateImpl_preserves_buffer_of_bounded_state
    (state : RetentionState)
    (hBounded : state.buffer.length ≤ retentionCapacity) :
    (restoreRetentionStateImpl state).buffer = state.buffer := by
  unfold restoreRetentionStateImpl mkRetentionState normalizeBuffer
  exact (List.take_eq_self_iff state.buffer).2 hBounded

def boundednessLaws : RetentionBoundednessLaws retentionInterfaceImpl :=
  { capacity := retentionCapacity
    step_coherent := retentionStepImpl_coherent
    inject_coherent := injectPayloadImpl_coherent
    restore_coherent := restoreRetentionStateImpl_coherent
    step_bounded := retentionStepImpl_bounded
    inject_bounded := injectPayloadImpl_bounded
    restore_bounded := restoreRetentionStateImpl_bounded }

def explicitDropDeliveryLaws : ExplicitDropDeliveryLaws retentionInterfaceImpl :=
  { dropped_count_changes_are_explicit := retentionStepImpl_drop_explicit
    delivered_count_changes_are_explicit := retentionStepImpl_delivery_explicit }

def noCreationFromSilenceLaws : NoCreationFromSilenceLaws retentionInterfaceImpl :=
  { step_preserves_message_origin := retentionStepImpl_preserves_message_origin
    restore_preserves_message_origin := restoreRetentionStateImpl_preserves_message_origin }

def routeTruthSeparationLaws : RouteTruthSeparationLaws retentionInterfaceImpl :=
  { local_state_unchanged := by
      intro input view
      rfl
    routes_unchanged := by
      intro input view
      rfl
    canonical_truth_unchanged := by
      intro input view destination
      rfl }

end FieldRetentionInstance
