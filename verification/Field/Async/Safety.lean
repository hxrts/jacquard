import Field.Async.API
import Field.Network.Safety

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAsyncSafety

open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI

theorem publication_envelope_matches_local_projection
    (network : NetworkState)
    (assumptions : AsyncAssumptions)
    (sender receiver : NodeId)
    (destination : DestinationClass) :
    (publicationEnvelope network assumptions sender receiver destination).projection =
      (publishMessage sender destination (network.localStates sender destination)).projection := by
  rfl

theorem publication_envelope_mem_enqueue_publications
    (network : NetworkState)
    (assumptions : AsyncAssumptions)
    (sender receiver : NodeId)
    (destination : DestinationClass)
    (hNeighbor : network.neighbors sender receiver = true) :
    publicationEnvelope network assumptions sender receiver destination ∈
      enqueuePublications network assumptions := by
  cases sender <;> cases receiver <;> cases destination <;>
    simp [enqueuePublications, allNodes, allDestinations, hNeighbor]

theorem immediate_reliable_step_contains_synchronous_publications
    (state : AsyncState)
    (sender receiver : NodeId)
    (destination : DestinationClass)
    (hNeighbor : state.network.neighbors sender receiver = true)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = []) :
    ∃ envelope ∈ (asyncStep state).inFlight,
      envelope.sender = sender ∧
        envelope.receiver = receiver ∧
        envelope.destination = destination ∧
        envelope.delay = 0 ∧
        envelope.projection =
          ((networkRound state.network).roundBuffer sender destination).projection := by
  rcases state with ⟨network, assumptions, inFlight, tick⟩
  simp at hAssumptions hEmpty
  subst hAssumptions
  subst hEmpty
  let envelope := publicationEnvelope network reliableImmediateAssumptions sender receiver destination
  refine ⟨envelope, ?_, ?_⟩
  · unfold envelope
    simpa [asyncStep] using
      (publication_envelope_mem_enqueue_publications
        network reliableImmediateAssumptions sender receiver destination (by simpa using hNeighbor))
  · constructor
    · rfl
    constructor
    · rfl
    constructor
    · rfl
    constructor
    · rfl
    · rfl

theorem publication_envelope_explicit_path_requires_explicit_local_knowledge
    (network : NetworkState)
    (assumptions : AsyncAssumptions)
    (hHarmony : NetworkLocallyHarmonious network)
    (sender receiver : NodeId)
    (destination : DestinationClass)
    (hShape :
      (publicationEnvelope network assumptions sender receiver destination).projection.shape =
        CorridorShape.explicitPath) :
    (network.localStates sender destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  have hProjected :
      (network.localStates sender destination).projection.shape = CorridorShape.explicitPath := by
    simpa [publicationEnvelope, publishMessage] using hShape
  rcases hHarmony sender destination with ⟨_, _, hExplicit, _, _⟩
  exact hExplicit.mp hProjected

theorem drain_ready_messages_never_increases_queue
    (state : AsyncState) :
    (drainReadyMessages state).inFlight.length ≤ state.inFlight.length := by
  unfold drainReadyMessages
  exact List.length_filter_le _ _

theorem zero_delay_queue_drains_in_one_step
    (state : AsyncState)
    (hZero : ∀ envelope ∈ state.inFlight, envelope.delay = 0) :
    (observerView (drainReadyMessages state)).readyCount = 0 := by
  unfold observerView drainReadyMessages
  simp

end FieldAsyncSafety
