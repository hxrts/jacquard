import Field.System.EndToEnd

/-! # System.Convergence — system step preserves queue properties and route views converge -/

/-
Prove that one system step preserves the reliable/immediate queue properties and that
iterated system steps cause route view selections to converge to a fixpoint.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemConvergence

open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI
open FieldRouterPublication
open FieldSystemEndToEnd

/-! ## Queue Preservation -/

def iterateSystemStep : Nat → EndToEndState → EndToEndState
  | 0, state => state
  | n + 1, state => iterateSystemStep n (systemStep state)

theorem system_step_preserves_reliable_immediate_empty_queue
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    (systemStep state).async.assumptions = reliableImmediateAssumptions ∧
      (systemStep state).async.inFlight = [] := by
  rcases state with ⟨async, lifecycle⟩
  rcases async with ⟨network, assumptions, inFlight, tick⟩
  subst hAssumptions
  subst hEmpty
  constructor
  · simp [systemStep, transportStep, drainReadyMessages, reliableImmediateAssumptions]
  · change List.filter (fun envelope => !readyForDelivery envelope)
        (enqueuePublications network reliableImmediateAssumptions) = []
    apply List.filter_eq_nil_iff.2
    intro envelope hMem
    have hMemReady : envelope ∈ List.filter readyForDelivery
        (enqueuePublications network reliableImmediateAssumptions) := by
      rw [reliable_immediate_enqueued_publications_are_ready]
      exact hMem
    have hReady : readyForDelivery envelope = true := (List.mem_filter.1 hMemReady).2
    simp [hReady]

/-! ## View Convergence -/

theorem candidate_view_fixed_point_under_reliable_immediate_empty
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    lifecycleCandidateView (systemStep (systemStep state)).lifecycle =
      lifecycleCandidateView (systemStep state).lifecycle := by
  have hPres := system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
  rw [system_step_candidate_view_eq_canonical_under_reliable_immediate_empty (systemStep state) hPres.1 hPres.2]
  rw [system_step_candidate_view_eq_canonical_under_reliable_immediate_empty state hAssumptions hEmpty]
  simp [system_step_preserves_network]

theorem candidate_view_iterate_stable_under_reliable_immediate_empty
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    lifecycleCandidateView (iterateSystemStep (n + 1) state).lifecycle =
      lifecycleCandidateView (systemStep state).lifecycle := by
  induction n generalizing state with
  | zero =>
      simp [iterateSystemStep]
  | succ n ih =>
      have hPres := system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
      simp [iterateSystemStep]
      calc
        lifecycleCandidateView (iterateSystemStep (n + 1) (systemStep state)).lifecycle
            = lifecycleCandidateView (systemStep (systemStep state)).lifecycle := by
                exact ih (systemStep state) hPres.1 hPres.2
        _ = lifecycleCandidateView (systemStep state).lifecycle := by
              exact candidate_view_fixed_point_under_reliable_immediate_empty state hAssumptions hEmpty

theorem iterateSystemStep_preserves_reliable_immediate_empty_queue
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    (iterateSystemStep n state).async.assumptions = reliableImmediateAssumptions ∧
      (iterateSystemStep n state).async.inFlight = [] := by
  induction n generalizing state with
  | zero =>
      simpa [iterateSystemStep] using And.intro hAssumptions hEmpty
  | succ n ih =>
      simp [iterateSystemStep]
      have hStep :=
        system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
      exact ih (systemStep state) hStep.1 hStep.2

/-- Under the clean reliable-immediate / empty-queue regime, one reduced
end-to-end step is enough to absorb one changed input into the candidate view;
every later iterate keeps the same candidate view. -/
theorem candidate_view_recovers_within_one_step_under_reliable_immediate_empty
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    lifecycleCandidateView (iterateSystemStep (n + 1) state).lifecycle =
      lifecycleCandidateView (systemStep state).lifecycle := by
  exact candidate_view_iterate_stable_under_reliable_immediate_empty n state hAssumptions hEmpty

theorem candidate_mem_system_step_view_implies_produced
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (candidate : PublishedCandidate)
    (hMem : candidate ∈ lifecycleCandidateView (systemStep state).lifecycle) :
    ProducedInstalledCandidate state.async candidate := by
  rw [system_step_candidate_view_eq_canonical_under_reliable_immediate_empty state hAssumptions hEmpty] at hMem
  unfold FieldSystemEndToEnd.lifecycleCandidateView canonicalInstalledRoutes at hMem
  rcases List.mem_map.1 hMem with ⟨route, hRouteMem, hEq⟩
  rcases List.mem_filterMap.1 hRouteMem with ⟨envelope, hEnvelopeMem, hRouteSome⟩
  unfold installLifecycleOfEnvelope at hRouteSome
  cases hAdmit : admitEnvelopeCandidate state.async.network envelope with
  | none =>
      simp [hAdmit] at hRouteSome
  | some admitted =>
      simp [hAdmit] at hRouteSome
      rcases hRouteSome with rfl
      refine ⟨envelope, admitted, ?_, hAdmit, ?_⟩
      · have hInFlight :
            (transportStep state.async).inFlight =
              enqueuePublications state.async.network reliableImmediateAssumptions := by
            have hStateAssumptions := hAssumptions
            have hStateEmpty := hEmpty
            rcases state with ⟨async, lifecycle⟩
            rcases async with ⟨network, assumptions, inFlight, tick⟩
            simp at hStateAssumptions hStateEmpty
            subst assumptions
            subst inFlight
            simp [transportStep, reliableImmediateAssumptions]
        rw [hInFlight]
        rw [reliable_immediate_enqueued_publications_are_ready]
        exact hEnvelopeMem
      · simpa [FieldRouterLifecycle.installCandidateLifecycle] using hEq

theorem no_spontaneous_explicit_path_promotion_over_iterated_steps
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (hNoExplicit :
      ∀ sender destination,
        (state.async.network.localStates sender destination).posterior.knowledge ≠
          ReachabilityKnowledge.explicitPath)
    (candidate : PublishedCandidate)
    (hMem : candidate ∈ lifecycleCandidateView (iterateSystemStep (n + 1) state).lifecycle) :
    candidate.shape ≠ CorridorShape.explicitPath := by
  have hStable :=
    candidate_view_iterate_stable_under_reliable_immediate_empty n state hAssumptions hEmpty
  rw [hStable] at hMem
  intro hShape
  have hProduced := candidate_mem_system_step_view_implies_produced state hAssumptions hEmpty candidate hMem
  have hKnowledge :=
    produced_candidate_requires_explicit_sender_knowledge
      state.async hAssumptions hEmpty hHarmony candidate hProduced hShape
  exact hNoExplicit candidate.publisher candidate.destination hKnowledge

end FieldSystemConvergence
