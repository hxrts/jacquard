import Field.Async.Transport
import Field.Router.Lifecycle
import Field.Network.Safety

/-! # System.EndToEnd — end-to-end system composition of async transport and lifecycle -/

/-
Define the composite end-to-end system step that sequences asynchronous message delivery with
route lifecycle updates, mapping received envelopes to installed route objects.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemEndToEnd

open FieldAsyncAPI
open FieldAsyncTransport
open FieldModelAPI
open FieldNetworkAPI
open FieldNetworkSafety
open FieldRouterAdmission
open FieldRouterLifecycle
open FieldRouterPublication

/-! ## Composite System Step -/

/-- Execution-state family note: `EndToEndState` is the composite execution
object that sequences async transport with router-owned lifecycle state. It is
not itself canonical truth; it is the execution layer from which truth is
selected. -/
structure EndToEndState where
  async : AsyncState
  lifecycle : List LifecycleRoute

def candidateOfEnvelope
    (envelope : AsyncEnvelope) : PublishedCandidate :=
  { publisher := envelope.sender
    destination := envelope.destination
    shape := envelope.projection.shape
    support := envelope.projection.support
    hopLower := envelope.projection.hopLower
    hopUpper := envelope.projection.hopUpper }

def admitEnvelopeCandidate
    (network : NetworkState)
    (envelope : AsyncEnvelope) : Option AdmittedCandidate :=
  let candidate := candidateOfEnvelope envelope
  let localState := network.localStates envelope.receiver envelope.destination
  if h : CandidateAdmissible localState candidate then
    some { localState := localState, candidate := candidate, admissible := h }
  else
    none

def installLifecycleOfEnvelope
    (network : NetworkState)
    (envelope : AsyncEnvelope) : Option LifecycleRoute :=
  Option.map installCandidateLifecycle (admitEnvelopeCandidate network envelope)

def readyInstalledRoutes
    (state : AsyncState) : List LifecycleRoute :=
  let stepped := transportStep state
  (stepped.inFlight.filter readyForDelivery).filterMap (installLifecycleOfEnvelope stepped.network)

def canonicalInstalledRoutes
    (network : NetworkState) : List LifecycleRoute :=
  (enqueuePublications network reliableImmediateAssumptions).filterMap
    (installLifecycleOfEnvelope network)

def lifecycleCandidateView
    (routes : List LifecycleRoute) : List PublishedCandidate :=
  routes.map LifecycleRoute.candidate

def systemStep
    (state : EndToEndState) : EndToEndState :=
  let stepped := transportStep state.async
  { async := drainReadyMessages stepped
    lifecycle := maintainLifecycle (readyInstalledRoutes state.async) }

def ProducedInstalledCandidate
    (state : AsyncState)
    (candidate : PublishedCandidate) : Prop :=
  ∃ envelope admitted,
    envelope ∈ (transportStep state).inFlight.filter readyForDelivery ∧
      admitEnvelopeCandidate (transportStep state).network envelope = some admitted ∧
      admitted.candidate = candidate

/-! ## Envelope-to-Route Mapping -/

theorem admit_envelope_candidate_preserves_candidate
    (network : NetworkState)
    (envelope : AsyncEnvelope)
    (admitted : AdmittedCandidate)
    (hAdmit : admitEnvelopeCandidate network envelope = some admitted) :
    admitted.candidate = candidateOfEnvelope envelope := by
  let localState := network.localStates envelope.receiver envelope.destination
  let candidate := candidateOfEnvelope envelope
  by_cases hAdm : CandidateAdmissible localState candidate
  · simp [admitEnvelopeCandidate, localState, candidate, hAdm] at hAdmit
    cases hAdmit
    rfl
  · simp [admitEnvelopeCandidate, localState, candidate, hAdm] at hAdmit

theorem ready_envelope_from_reliable_immediate_empty_matches_local_projection
    (state : AsyncState)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = [])
    (envelope : AsyncEnvelope)
    (hReady : envelope ∈ (transportStep state).inFlight.filter readyForDelivery) :
    envelope.projection =
      (publishMessage envelope.sender envelope.destination
        (state.network.localStates envelope.sender envelope.destination)).projection := by
  rcases state with ⟨network, assumptions, inFlight, tick⟩
  subst hAssumptions
  subst hEmpty
  cases envelope with
  | mk sender receiver destination projection delay retryCount dropped =>
      simp [transportStep, readyForDelivery, enqueuePublications, publicationEnvelope,
        reliableImmediateAssumptions] at hReady ⊢
      exact hReady.2.2.1.2.2.1.symm

theorem candidate_of_envelope_matches_projection
    (envelope : AsyncEnvelope) :
    (candidateOfEnvelope envelope).shape = envelope.projection.shape ∧
      (candidateOfEnvelope envelope).support = envelope.projection.support := by
  simp [candidateOfEnvelope]

theorem produced_candidate_requires_explicit_sender_knowledge
    (state : AsyncState)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.network)
    (candidate : PublishedCandidate)
    (hProduced : ProducedInstalledCandidate state candidate)
    (hShape : candidate.shape = CorridorShape.explicitPath) :
    (state.network.localStates candidate.publisher candidate.destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  rcases hProduced with ⟨envelope, admitted, hReady, hAdmit, hCandidate⟩
  subst hCandidate
  have hCandidateEq :
      admitted.candidate = candidateOfEnvelope envelope :=
    admit_envelope_candidate_preserves_candidate (transportStep state).network envelope admitted hAdmit
  have hProjectionEq :
      envelope.projection =
        (publishMessage envelope.sender envelope.destination
          (state.network.localStates envelope.sender envelope.destination)).projection :=
    ready_envelope_from_reliable_immediate_empty_matches_local_projection
      state hAssumptions hEmpty envelope hReady
  have hEnvelopeShape : (candidateOfEnvelope envelope).shape = CorridorShape.explicitPath := by
    simpa [hCandidateEq] using hShape
  have hLocalShape :
      (state.network.localStates envelope.sender envelope.destination).projection.shape =
        CorridorShape.explicitPath := by
    simpa [candidateOfEnvelope, hProjectionEq]
      using hEnvelopeShape
  exact
    by
      simpa [hCandidateEq, candidateOfEnvelope] using
        (explicit_path_publication_requires_explicit_knowledge
      (state.network.localStates envelope.sender envelope.destination)
      (hHarmony envelope.sender envelope.destination)
      (by simpa [publishCandidate] using hLocalShape))

theorem produced_candidate_support_conservative
    (state : AsyncState)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.network)
    (candidate : PublishedCandidate)
    (hProduced : ProducedInstalledCandidate state candidate) :
    candidate.support ≤
      (state.network.localStates candidate.publisher candidate.destination).posterior.support := by
  rcases hProduced with ⟨envelope, admitted, hReady, hAdmit, hCandidate⟩
  subst hCandidate
  have hCandidateEq :
      admitted.candidate = candidateOfEnvelope envelope :=
    admit_envelope_candidate_preserves_candidate (transportStep state).network envelope admitted hAdmit
  have hProjectionEq :
      envelope.projection =
        (publishMessage envelope.sender envelope.destination
          (state.network.localStates envelope.sender envelope.destination)).projection :=
    ready_envelope_from_reliable_immediate_empty_matches_local_projection
      state hAssumptions hEmpty envelope hReady
  have hSupport :
      envelope.projection.support ≤
        (state.network.localStates envelope.sender envelope.destination).posterior.support := by
    simpa [hProjectionEq, publishCandidate] using
      publication_support_le_local_support
        (state.network.localStates envelope.sender envelope.destination)
        (hHarmony envelope.sender envelope.destination)
  simpa [hCandidateEq, candidateOfEnvelope] using hSupport

theorem system_step_candidate_view
    (state : EndToEndState) :
    lifecycleCandidateView (systemStep state).lifecycle =
      lifecycleCandidateView (readyInstalledRoutes state.async) := by
  unfold systemStep lifecycleCandidateView
  simp [FieldRouterLifecycle.maintain_lifecycle_preserves_candidate_view]

theorem system_step_preserves_network
    (state : EndToEndState) :
    (systemStep state).async.network = state.async.network := by
  rfl

theorem reliable_immediate_enqueued_publications_are_ready
    (network : NetworkState) :
    List.filter readyForDelivery (enqueuePublications network reliableImmediateAssumptions) =
      enqueuePublications network reliableImmediateAssumptions := by
  apply List.filter_eq_self.2
  intro envelope hMem
  cases envelope
  case mk sender receiver destination projection delayNat retryCountNat droppedFlag =>
    simp [enqueuePublications, publicationEnvelope, reliableImmediateAssumptions, readyForDelivery] at hMem ⊢
    have hDelay : 0 = delayNat := hMem.2.2.2.2.2.1
    have hDropped : droppedFlag = false := hMem.2.2.2.2.2.2.2
    constructor
    · simpa using hDelay.symm
    · simpa using hDropped

theorem ready_installed_routes_eq_canonical_under_reliable_immediate_empty
    (state : AsyncState)
    (hAssumptions : state.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.inFlight = []) :
    readyInstalledRoutes state = canonicalInstalledRoutes state.network := by
  rcases state with ⟨network, assumptions, inFlight, tick⟩
  subst hAssumptions
  subst hEmpty
  unfold readyInstalledRoutes canonicalInstalledRoutes transportStep
  simp [reliable_immediate_enqueued_publications_are_ready]

theorem system_step_candidate_view_eq_canonical_under_reliable_immediate_empty
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    lifecycleCandidateView (systemStep state).lifecycle =
      lifecycleCandidateView (canonicalInstalledRoutes state.async.network) := by
  rw [system_step_candidate_view]
  rw [ready_installed_routes_eq_canonical_under_reliable_immediate_empty state.async hAssumptions hEmpty]

theorem system_step_route_has_ready_installed_origin
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle) :
    ∃ source,
      source ∈ readyInstalledRoutes state.async ∧
        lifecycleMaintenance source = route := by
  unfold systemStep at hMem
  unfold maintainLifecycle at hMem
  rcases List.mem_map.1 hMem with ⟨source, hSourceMem, hEq⟩
  exact ⟨source, hSourceMem, hEq⟩

theorem system_step_route_origin_is_installed
    (state : EndToEndState)
    (route source : LifecycleRoute)
    (hOrigin : source ∈ readyInstalledRoutes state.async)
    (_hMaintained : lifecycleMaintenance source = route) :
    source.status = .installed := by
  unfold readyInstalledRoutes at hOrigin
  rcases List.mem_filterMap.1 hOrigin with ⟨envelope, _, hSome⟩
  unfold installLifecycleOfEnvelope at hSome
  cases hAdmit : admitEnvelopeCandidate (transportStep state.async).network envelope with
  | none =>
      simp [hAdmit] at hSome
  | some admitted =>
      simp [hAdmit] at hSome
      subst hSome
      simp [installCandidateLifecycle]

theorem system_step_route_has_admissible_lifecycle_origin
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle) :
    ∃ source,
      source ∈ readyInstalledRoutes state.async ∧
        source.status = .installed ∧
        lifecycleMaintenance source = route := by
  rcases system_step_route_has_ready_installed_origin state route hMem with
    ⟨source, hSourceMem, hMaintained⟩
  refine ⟨source, hSourceMem, ?_, hMaintained⟩
  exact system_step_route_origin_is_installed state route source hSourceMem hMaintained

theorem ready_installed_route_appears_in_system_step_lifecycle
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ readyInstalledRoutes state.async) :
    lifecycleMaintenance route ∈ (systemStep state).lifecycle := by
  unfold systemStep maintainLifecycle
  exact List.mem_map.2 ⟨route, hMem, rfl⟩

theorem ready_installed_route_candidate_appears_in_system_step_candidate_view
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ readyInstalledRoutes state.async) :
    (lifecycleMaintenance route).candidate ∈
      lifecycleCandidateView (systemStep state).lifecycle := by
  unfold lifecycleCandidateView
  exact List.mem_map.2 ⟨lifecycleMaintenance route,
    ready_installed_route_appears_in_system_step_lifecycle state route hMem, rfl⟩

end FieldSystemEndToEnd
