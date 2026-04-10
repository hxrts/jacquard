import Field.Async.Transport

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAsyncBounded

open FieldAsyncAPI
open FieldAsyncSafety
open FieldAsyncTransport
open FieldNetworkAPI

def boundedDelayRetryAssumptions : AsyncAssumptions :=
  { maxDelay := 1
    retryBound := 1
    lossPossible := True
    batchBound := FieldNetworkAPI.allNodes.length * FieldNetworkAPI.allDestinations.length }

def postRetryCycleEnvelope
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) : AsyncEnvelope :=
  lifecycleEnvelope assumptions (lifecycleEnvelope assumptions envelope)

def congestionLossBudget
    (state : AsyncState) : Nat :=
  state.inFlight.length +
    (state.inFlight.filter (eligibleForRetry state.assumptions)).length

theorem boundedDelayRetry_regime_has_explicit_budget :
    boundedDelayRetryAssumptions.maxDelay = 1 ∧
      boundedDelayRetryAssumptions.retryBound = 1 := by
  simp [boundedDelayRetryAssumptions]

theorem transportStep_existing_envelope_has_preserved_projection
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.inFlight) :
    ∃ lifted,
      lifted ∈ (transportStep state).inFlight ∧
        lifted.projection = envelope.projection := by
  refine ⟨lifecycleEnvelope state.assumptions envelope, ?_, ?_⟩
  · unfold transportStep
    exact List.mem_append_left _ (List.mem_map.2 ⟨envelope, hMem, rfl⟩)
  · exact lifecycle_envelope_preserves_projection state.assumptions envelope

theorem transportStep_existing_envelope_never_strengthens_claim
    (state : AsyncState)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.inFlight) :
    ∃ lifted,
      lifted ∈ (transportStep state).inFlight ∧
        lifted.projection.shape = envelope.projection.shape ∧
        lifted.projection.support = envelope.projection.support := by
  rcases transportStep_existing_envelope_has_preserved_projection state envelope hMem with
    ⟨lifted, hLiftedMem, hProjection⟩
  refine ⟨lifted, hLiftedMem, ?_, ?_⟩
  · simp [hProjection]
  · simp [hProjection]

theorem transportStep_inflight_length_bounded_by_current_plus_publications
    (state : AsyncState) :
    (transportStep state).inFlight.length ≤
      state.inFlight.length + (enqueuePublications state.network state.assumptions).length := by
  unfold transportStep
  simp

theorem boundedDelayRetry_transport_step_never_strengthens_existing_claims
    (state : AsyncState)
    (_hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.inFlight) :
    ∃ lifted,
      lifted ∈ (transportStep state).inFlight ∧
        lifted.projection.shape = envelope.projection.shape ∧
        lifted.projection.support = envelope.projection.support := by
  exact transportStep_existing_envelope_never_strengthens_claim state envelope hMem

theorem boundedDelayRetry_ready_messages_bounded_by_queue
    (state : AsyncState)
    (_hAssumptions : state.assumptions = boundedDelayRetryAssumptions) :
    (observerView (transportStep state)).readyCount ≤
      (observerView (transportStep state)).inFlightCount := by
  unfold observerView
  exact List.length_filter_le _ _

theorem transportStep_ready_count_bounded_by_current_plus_publications
    (state : AsyncState) :
    (observerView (transportStep state)).readyCount ≤
      state.inFlight.length + (enqueuePublications state.network state.assumptions).length := by
  calc
    (observerView (transportStep state)).readyCount
        ≤ (observerView (transportStep state)).inFlightCount :=
          by
            unfold observerView
            exact List.length_filter_le _ _
    _ = (transportStep state).inFlight.length := by
          rfl
    _ ≤ state.inFlight.length + (enqueuePublications state.network state.assumptions).length :=
          transportStep_inflight_length_bounded_by_current_plus_publications state

theorem drainReadyMessages_after_transport_inflight_bounded_by_current_plus_publications
    (state : AsyncState) :
    (drainReadyMessages (transportStep state)).inFlight.length ≤
      state.inFlight.length + (enqueuePublications state.network state.assumptions).length := by
  calc
    (drainReadyMessages (transportStep state)).inFlight.length
        ≤ (transportStep state).inFlight.length :=
          drain_ready_messages_never_increases_queue (transportStep state)
    _ ≤ state.inFlight.length + (enqueuePublications state.network state.assumptions).length :=
          transportStep_inflight_length_bounded_by_current_plus_publications state

theorem drainReadyMessages_after_transport_inflight_bounded_by_congestion_loss_budget
    (state : AsyncState) :
    (drainReadyMessages (transportStep state)).inFlight.length ≤
      congestionLossBudget state + (enqueuePublications state.network state.assumptions).length := by
  calc
    (drainReadyMessages (transportStep state)).inFlight.length
        ≤ state.inFlight.length + (enqueuePublications state.network state.assumptions).length :=
          drainReadyMessages_after_transport_inflight_bounded_by_current_plus_publications state
    _ ≤ congestionLossBudget state + (enqueuePublications state.network state.assumptions).length := by
          unfold congestionLossBudget
          omega

theorem retry_eligible_lifecycle_envelope_eq_bounded_reset
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hRetry : eligibleForRetry state.assumptions envelope = true) :
    lifecycleEnvelope state.assumptions envelope =
      { envelope with
          dropped := False
          delay := 1
          retryCount := envelope.retryCount + 1 } := by
  rw [hAssumptions] at hRetry ⊢
  have hDroppedRetry : envelope.dropped = true ∧ envelope.retryCount = 0 := by
    simpa [eligibleForRetry, boundedDelayRetryAssumptions] using hRetry
  by_cases hDelay : envelope.delay = 0
  · simp [lifecycleEnvelope, retryEnvelope, eligibleForRetry, stepEnvelope,
      boundedDelayRetryAssumptions, hDroppedRetry.1, hDroppedRetry.2, hDelay]
  · simp [lifecycleEnvelope, retryEnvelope, eligibleForRetry, stepEnvelope,
      boundedDelayRetryAssumptions, hDroppedRetry.1, hDroppedRetry.2, hDelay]

theorem postRetryCycleEnvelope_eq_bounded_ready
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hRetry : eligibleForRetry state.assumptions envelope = true) :
    postRetryCycleEnvelope state.assumptions envelope =
      { envelope with
          dropped := False
          delay := 0
          retryCount := envelope.retryCount + 1 } := by
  rw [postRetryCycleEnvelope]
  rw [retry_eligible_lifecycle_envelope_eq_bounded_reset state hAssumptions envelope hRetry]
  rw [hAssumptions]
  simp [lifecycleEnvelope, retryEnvelope, eligibleForRetry, stepEnvelope, boundedDelayRetryAssumptions]

theorem retry_eligible_envelope_not_ready_after_first_bounded_retry
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hRetry : eligibleForRetry state.assumptions envelope = true) :
    readyForDelivery (lifecycleEnvelope state.assumptions envelope) = false := by
  rw [retry_eligible_lifecycle_envelope_eq_bounded_reset state hAssumptions envelope hRetry]
  simp [readyForDelivery]

theorem retry_eligible_envelope_survives_first_drain_under_bounded_delay_retry
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.inFlight)
    (hRetry : eligibleForRetry state.assumptions envelope = true) :
    lifecycleEnvelope state.assumptions envelope ∈
      (drainReadyMessages (transportStep state)).inFlight := by
  have hLifted :
      lifecycleEnvelope state.assumptions envelope ∈ (transportStep state).inFlight := by
    unfold transportStep
    exact List.mem_append_left _ (List.mem_map.2 ⟨envelope, hMem, rfl⟩)
  unfold drainReadyMessages
  apply List.mem_filter.2
  constructor
  · exact hLifted
  · simp [retry_eligible_envelope_not_ready_after_first_bounded_retry
      state hAssumptions envelope hRetry]

theorem retry_eligible_envelope_ready_after_one_retry_cycle
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.inFlight)
    (hRetry : eligibleForRetry state.assumptions envelope = true) :
    postRetryCycleEnvelope state.assumptions envelope ∈
        (transportStep (drainReadyMessages (transportStep state))).inFlight.filter readyForDelivery ∧
      (postRetryCycleEnvelope state.assumptions envelope).projection = envelope.projection := by
  let lifted1 := lifecycleEnvelope state.assumptions envelope
  let lifted2 := postRetryCycleEnvelope state.assumptions envelope
  have hLifted1Mem :
      lifted1 ∈ (drainReadyMessages (transportStep state)).inFlight :=
    retry_eligible_envelope_survives_first_drain_under_bounded_delay_retry
      state hAssumptions envelope hMem hRetry
  constructor
  · apply List.mem_filter.2
    constructor
    · unfold transportStep
      exact List.mem_append_left _ (List.mem_map.2 ⟨lifted1, hLifted1Mem,
        by simp [lifted1, postRetryCycleEnvelope, drainReadyMessages]⟩)
    · change readyForDelivery (postRetryCycleEnvelope state.assumptions envelope) = true
      rw [postRetryCycleEnvelope_eq_bounded_ready state hAssumptions envelope hRetry]
      simp [readyForDelivery]
  · simp [postRetryCycleEnvelope, lifecycle_envelope_preserves_projection]

/-- Under the current bounded-delay / bounded-retry regime, if the queue
contains only retry-eligible dropped envelopes and there are no fresh
publications to inject, then one full retry cycle drains the queue. -/
theorem retry_only_queue_drains_after_one_retry_cycle_without_new_publications
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (hNoFresh : enqueuePublications state.network state.assumptions = [])
    (hAllRetry :
      ∀ envelope, envelope ∈ state.inFlight →
        eligibleForRetry state.assumptions envelope = true) :
    (drainReadyMessages (transportStep (drainReadyMessages (transportStep state)))).inFlight = [] := by
  rcases state with ⟨network, assumptions, inFlight, tick⟩
  subst hAssumptions
  let baseState : AsyncState :=
    { network := network
      assumptions := boundedDelayRetryAssumptions
      inFlight := inFlight
      tick := tick }
  have hFirstDrain :
      List.filter (fun envelope => !readyForDelivery envelope)
          (List.map (lifecycleEnvelope boundedDelayRetryAssumptions) inFlight) =
        List.map (lifecycleEnvelope boundedDelayRetryAssumptions) inFlight := by
    apply List.filter_eq_self.2
    intro lifted hMem
    rcases List.mem_map.1 hMem with ⟨envelope, hEnvelopeMem, rfl⟩
    have hRetry := hAllRetry envelope hEnvelopeMem
    have hNotReady :
        readyForDelivery (lifecycleEnvelope boundedDelayRetryAssumptions envelope) = false :=
      retry_eligible_envelope_not_ready_after_first_bounded_retry
        baseState rfl envelope hRetry
    simp [hNotReady]
  have hSecondTransport :
      List.map (lifecycleEnvelope boundedDelayRetryAssumptions)
          (List.map (lifecycleEnvelope boundedDelayRetryAssumptions) inFlight) =
        List.map (postRetryCycleEnvelope boundedDelayRetryAssumptions) inFlight := by
    simp [postRetryCycleEnvelope, List.map_map]
  have hFinal :
      List.filter (fun envelope => !readyForDelivery envelope)
          (List.map (postRetryCycleEnvelope boundedDelayRetryAssumptions) inFlight) = [] := by
    apply List.filter_eq_nil_iff.2
    intro lifted hMem
    rcases List.mem_map.1 hMem with ⟨envelope, hEnvelopeMem, rfl⟩
    have hRetry := hAllRetry envelope hEnvelopeMem
    have hReady :
        readyForDelivery (postRetryCycleEnvelope boundedDelayRetryAssumptions envelope) = true := by
      rw [postRetryCycleEnvelope_eq_bounded_ready baseState rfl envelope hRetry]
      simp [readyForDelivery]
    simp [hReady]
  simpa [baseState, drainReadyMessages, transportStep, hNoFresh, hFirstDrain, hSecondTransport] using hFinal

end FieldAsyncBounded
