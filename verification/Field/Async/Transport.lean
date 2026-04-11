import Field.Async.Safety

/-! # Async.Transport — envelope transformation and monotonicity under delayed delivery -/

/-
Prove that lifecycle envelope transformations preserve message projections and that delayed
messages never strengthen claims beyond what was published.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAsyncTransport

open FieldAsyncAPI
open FieldAsyncSafety
open FieldModelAPI
open FieldNetworkAPI

/-! ## Envelope Transformation -/

theorem retry_envelope_preserves_projection
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) :
    (retryEnvelope assumptions envelope).projection = envelope.projection := by
  unfold retryEnvelope
  by_cases hRetry : eligibleForRetry assumptions envelope
  · simp [hRetry]
  · simp [hRetry]

theorem lifecycle_envelope_preserves_projection
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) :
    (lifecycleEnvelope assumptions envelope).projection = envelope.projection := by
  unfold lifecycleEnvelope retryEnvelope eligibleForRetry
  by_cases hZero : envelope.delay = 0
  · by_cases hRetry : envelope.dropped = true ∧ envelope.retryCount < assumptions.retryBound
    · simp [stepEnvelope, hZero, hRetry]
    · simp [stepEnvelope, hZero, hRetry]
  · by_cases hRetry : envelope.dropped = true ∧ envelope.retryCount < assumptions.retryBound
    · simp [stepEnvelope, hZero, hRetry]
    · simp [stepEnvelope, hZero, hRetry]

theorem lifecycle_envelope_preserves_shape
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) :
    (lifecycleEnvelope assumptions envelope).projection.shape = envelope.projection.shape := by
  simp [lifecycle_envelope_preserves_projection assumptions envelope]

theorem lifecycle_envelope_preserves_support
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) :
    (lifecycleEnvelope assumptions envelope).projection.support = envelope.projection.support := by
  simp [lifecycle_envelope_preserves_projection assumptions envelope]

theorem transport_step_preserves_network
    (state : AsyncState) :
    (transportStep state).network = state.network := by
  rfl

theorem transport_step_preserves_assumptions
    (state : AsyncState) :
    (transportStep state).assumptions = state.assumptions := by
  rfl

/-! ## Monotonicity -/

theorem injected_publication_mem_transport_step
    (state : AsyncState)
    (sender receiver : NodeId)
    (destination : DestinationClass)
    (hNeighbor : state.network.neighbors sender receiver = true) :
    publicationEnvelope state.network state.assumptions sender receiver destination ∈
      (transportStep state).inFlight := by
  unfold transportStep
  exact List.mem_append_right _ <|
    publication_envelope_mem_enqueue_publications
      state.network state.assumptions sender receiver destination hNeighbor

theorem reliable_immediate_transport_refines_async_step
    (state : AsyncState)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions) :
    transportStep state = asyncStep state := by
  rcases state with ⟨network, assumptions, inFlight, tick⟩
  subst hAssumptions
  simp [transportStep, asyncStep, lifecycleEnvelope, retryEnvelope, eligibleForRetry, stepEnvelope,
    reliableImmediateAssumptions]

theorem delayed_or_dropped_transport_never_strengthens_projection
    (assumptions : AsyncAssumptions)
    (envelope : AsyncEnvelope) :
    (lifecycleEnvelope assumptions envelope).projection.shape = envelope.projection.shape ∧
      (lifecycleEnvelope assumptions envelope).projection.support = envelope.projection.support := by
  exact ⟨lifecycle_envelope_preserves_shape assumptions envelope,
    lifecycle_envelope_preserves_support assumptions envelope⟩

theorem transport_step_publication_requires_explicit_local_knowledge
    (state : AsyncState)
    (hHarmony : NetworkLocallyHarmonious state.network)
    (sender receiver : NodeId)
    (destination : DestinationClass)
    (hNeighbor : state.network.neighbors sender receiver = true)
    (hShape :
      (publicationEnvelope state.network state.assumptions sender receiver destination).projection.shape =
        CorridorShape.explicitPath) :
    (state.network.localStates sender destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  exact publication_envelope_explicit_path_requires_explicit_local_knowledge
    state.network state.assumptions hHarmony sender receiver destination hShape

end FieldAsyncTransport
