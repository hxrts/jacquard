import Field.Async.Bounded
import Field.System.Canonical
import Field.System.Convergence
import Field.System.EndToEnd

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemBounded

open FieldAsyncAPI
open FieldAsyncBounded
open FieldModelAPI
open FieldNetworkAPI
open FieldRouterCanonical
open FieldRouterLifecycle
open FieldSystemCanonical
open FieldSystemConvergence
open FieldSystemEndToEnd

def reliableImmediateRecoveryState
    (state : EndToEndState) : EndToEndState :=
  { async :=
      { network := state.async.network
        assumptions := reliableImmediateAssumptions
        inFlight := []
        tick := state.async.tick }
    lifecycle := state.lifecycle }

def systemStepWorkUnits
    (state : EndToEndState) : Nat :=
  state.async.inFlight.length +
    (enqueuePublications state.async.network state.async.assumptions).length +
    (readyInstalledRoutes state.async).length +
    (systemStep state).lifecycle.length

theorem installLifecycleOfEnvelope_some_preserves_projection_shape_support
    (network : FieldNetworkAPI.NetworkState)
    (envelope : AsyncEnvelope)
    (route : LifecycleRoute)
    (hInstall : installLifecycleOfEnvelope network envelope = some route) :
    route.candidate.shape = envelope.projection.shape ∧
      route.candidate.support = envelope.projection.support := by
  unfold installLifecycleOfEnvelope at hInstall
  cases hAdmit : admitEnvelopeCandidate network envelope with
  | none =>
      simp [hAdmit] at hInstall
  | some admitted =>
      simp [hAdmit] at hInstall
      cases hInstall
      have hCandidate :
          admitted.candidate = candidateOfEnvelope envelope :=
        admit_envelope_candidate_preserves_candidate network envelope admitted hAdmit
      rcases candidate_of_envelope_matches_projection envelope with ⟨hShape, hSupport⟩
      constructor
      · calc
          (installCandidateLifecycle admitted).candidate.shape = admitted.candidate.shape := by
            simp [installCandidateLifecycle]
          _ = (candidateOfEnvelope envelope).shape := by simp [hCandidate]
          _ = envelope.projection.shape := hShape
      · calc
          (installCandidateLifecycle admitted).candidate.support = admitted.candidate.support := by
            simp [installCandidateLifecycle]
          _ = (candidateOfEnvelope envelope).support := by simp [hCandidate]
          _ = envelope.projection.support := hSupport

theorem readyInstalledRoute_preserves_source_projection_shape_support
    (state : AsyncState)
    (route : LifecycleRoute)
    (hMem : route ∈ readyInstalledRoutes state) :
    ∃ envelope,
      envelope ∈ (transportStep state).inFlight.filter readyForDelivery ∧
        route.candidate.shape = envelope.projection.shape ∧
        route.candidate.support = envelope.projection.support := by
  unfold readyInstalledRoutes at hMem
  rcases List.mem_filterMap.1 hMem with ⟨envelope, hEnvelopeMem, hInstall⟩
  refine ⟨envelope, hEnvelopeMem, ?_, ?_⟩
  exact (installLifecycleOfEnvelope_some_preserves_projection_shape_support
    (transportStep state).network envelope route hInstall).1
  exact (installLifecycleOfEnvelope_some_preserves_projection_shape_support
    (transportStep state).network envelope route hInstall).2

theorem readyInstalledRoutes_length_bounded_by_transport_ready_queue
    (state : AsyncState) :
    (readyInstalledRoutes state).length ≤
      ((transportStep state).inFlight.filter readyForDelivery).length := by
  unfold readyInstalledRoutes
  exact List.length_filterMap_le _ _

theorem systemStep_inflight_length_bounded_by_current_plus_publications
    (state : EndToEndState) :
    (systemStep state).async.inFlight.length ≤
      state.async.inFlight.length +
        (enqueuePublications state.async.network state.async.assumptions).length := by
  unfold systemStep
  exact
    drainReadyMessages_after_transport_inflight_bounded_by_current_plus_publications state.async

theorem systemStep_inflight_length_bounded_by_congestion_loss_budget
    (state : EndToEndState) :
    (systemStep state).async.inFlight.length ≤
      congestionLossBudget state.async +
        (enqueuePublications state.async.network state.async.assumptions).length := by
  unfold systemStep
  exact
    drainReadyMessages_after_transport_inflight_bounded_by_congestion_loss_budget state.async

theorem systemStep_lifecycle_length_bounded_by_transport_ready_queue
    (state : EndToEndState) :
    (systemStep state).lifecycle.length ≤
      ((transportStep state.async).inFlight.filter readyForDelivery).length := by
  unfold systemStep maintainLifecycle
  calc
    (List.map lifecycleMaintenance (readyInstalledRoutes state.async)).length
        = (readyInstalledRoutes state.async).length := by simp
    _ ≤ ((transportStep state.async).inFlight.filter readyForDelivery).length :=
          readyInstalledRoutes_length_bounded_by_transport_ready_queue state.async

/-- Abstract one-step latency bound for the reduced end-to-end semantics.
The bound is stated in proof-facing work units, not wall-clock time. -/
theorem system_step_work_units_bounded_by_transport_volume
    (state : EndToEndState) :
    systemStepWorkUnits state ≤
      4 * (state.async.inFlight.length +
        (enqueuePublications state.async.network state.async.assumptions).length) := by
  have hReady :
      (readyInstalledRoutes state.async).length ≤
        state.async.inFlight.length +
          (enqueuePublications state.async.network state.async.assumptions).length := by
    calc
      (readyInstalledRoutes state.async).length ≤
          ((transportStep state.async).inFlight.filter readyForDelivery).length :=
        readyInstalledRoutes_length_bounded_by_transport_ready_queue state.async
      _ = (observerView (transportStep state.async)).readyCount := by
            rfl
      _ ≤ state.async.inFlight.length +
            (enqueuePublications state.async.network state.async.assumptions).length :=
        transportStep_ready_count_bounded_by_current_plus_publications state.async
  have hLifecycle :
      (systemStep state).lifecycle.length ≤
        state.async.inFlight.length +
          (enqueuePublications state.async.network state.async.assumptions).length := by
    calc
      (systemStep state).lifecycle.length ≤
          ((transportStep state.async).inFlight.filter readyForDelivery).length :=
        systemStep_lifecycle_length_bounded_by_transport_ready_queue state
      _ = (observerView (transportStep state.async)).readyCount := by
            rfl
      _ ≤ state.async.inFlight.length +
            (enqueuePublications state.async.network state.async.assumptions).length :=
        transportStep_ready_count_bounded_by_current_plus_publications state.async
  unfold systemStepWorkUnits
  omega

theorem system_step_route_never_amplifies_source_projection
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle) :
    ∃ envelope,
      envelope ∈ (transportStep state.async).inFlight.filter readyForDelivery ∧
        route.candidate.shape = envelope.projection.shape ∧
        route.candidate.support = envelope.projection.support := by
  rcases system_step_route_has_ready_installed_origin state route hMem with
    ⟨source, hSourceMem, hMaintained⟩
  rcases readyInstalledRoute_preserves_source_projection_shape_support state.async source hSourceMem with
    ⟨envelope, hEnvelopeMem, hShape, hSupport⟩
  have hCandidate :
      (lifecycleMaintenance source).candidate = source.candidate :=
    lifecycle_maintenance_preserves_candidate source
  refine ⟨envelope, hEnvelopeMem, ?_, ?_⟩
  · calc
      route.candidate.shape = (lifecycleMaintenance source).candidate.shape := by
        simp [hMaintained]
      _ = source.candidate.shape := by simp [hCandidate]
      _ = envelope.projection.shape := hShape
  · calc
      route.candidate.support = (lifecycleMaintenance source).candidate.support := by
        simp [hMaintained]
      _ = source.candidate.support := by simp [hCandidate]
      _ = envelope.projection.support := hSupport

theorem system_step_overload_monotone_degradation
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle) :
    ∃ envelope,
      envelope ∈ (transportStep state.async).inFlight.filter readyForDelivery ∧
        route.candidate.shape = envelope.projection.shape ∧
        route.candidate.support = envelope.projection.support := by
  exact system_step_route_never_amplifies_source_projection state route hMem

theorem retry_eligible_admissible_update_processed_after_one_retry_cycle
    (state : AsyncState)
    (hAssumptions : state.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.inFlight)
    (hRetry : eligibleForRetry state.assumptions envelope = true)
    (route : LifecycleRoute)
    (hInstall :
      installLifecycleOfEnvelope
          (transportStep (drainReadyMessages (transportStep state))).network
          (postRetryCycleEnvelope state.assumptions envelope) = some route) :
    route ∈ readyInstalledRoutes (drainReadyMessages (transportStep state)) := by
  have hReady :
      postRetryCycleEnvelope state.assumptions envelope ∈
        (transportStep (drainReadyMessages (transportStep state))).inFlight.filter readyForDelivery := by
    exact (retry_eligible_envelope_ready_after_one_retry_cycle
      state hAssumptions envelope hMem hRetry).1
  unfold readyInstalledRoutes
  exact List.mem_filterMap.2 ⟨postRetryCycleEnvelope state.assumptions envelope, hReady, hInstall⟩

theorem candidate_view_recovers_after_queue_clears_under_reliable_immediate
    (n : Nat)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    lifecycleCandidateView (iterateSystemStep (n + 1) state).lifecycle =
      lifecycleCandidateView (systemStep state).lifecycle := by
  exact
    candidate_view_recovers_within_one_step_under_reliable_immediate_empty
      n state hAssumptions hEmpty

theorem canonical_route_recovers_after_queue_clears_under_reliable_immediate
    (n : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRoute destination (iterateSystemStep (n + 1) state) =
      canonicalSystemRoute destination state := by
  exact
    canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
      n destination state hAssumptions hEmpty

theorem intermittent_loss_eventually_converges_after_recovery
    (n : Nat)
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRoute destination
        (iterateSystemStep (n + 1) (reliableImmediateRecoveryState state)) =
      canonicalSystemRoute destination (reliableImmediateRecoveryState state) := by
  exact
    canonical_route_recovers_after_queue_clears_under_reliable_immediate
      n destination (reliableImmediateRecoveryState state) rfl rfl

/-- System-facing queue-drain horizon for the current broader async regime.
If no fresh publications are injected and every queued envelope is already
retry-eligible, then one full retry cycle drains the async backlog. -/
theorem system_queue_drains_after_one_retry_cycle_without_new_publications
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = boundedDelayRetryAssumptions)
    (hNoFresh :
      enqueuePublications state.async.network state.async.assumptions = [])
    (hAllRetry :
      ∀ envelope, envelope ∈ state.async.inFlight →
        eligibleForRetry state.async.assumptions envelope = true) :
    (drainReadyMessages (transportStep (drainReadyMessages (transportStep state.async)))).inFlight = [] := by
  exact
    retry_only_queue_drains_after_one_retry_cycle_without_new_publications
      state.async hAssumptions hNoFresh hAllRetry

/-- Under the current mixed saturation/loss model, once the reduced execution
has crossed the explicit recovery threshold of an empty queue together with the
reliable-immediate assumptions, later iterates cannot keep flipping the
canonical winner. -/
theorem partial_delivery_does_not_oscillate_after_recovery_threshold
    (n m : Nat)
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRoute destination
        (iterateSystemStep (n + 1) (reliableImmediateRecoveryState state)) =
      canonicalSystemRoute destination
        (iterateSystemStep (m + 1) (reliableImmediateRecoveryState state)) := by
  calc
    canonicalSystemRoute destination
        (iterateSystemStep (n + 1) (reliableImmediateRecoveryState state)) =
      canonicalSystemRoute destination (reliableImmediateRecoveryState state) :=
        intermittent_loss_eventually_converges_after_recovery n destination state
    _ =
      canonicalSystemRoute destination
        (iterateSystemStep (m + 1) (reliableImmediateRecoveryState state)) := by
          symm
          exact intermittent_loss_eventually_converges_after_recovery m destination state

/-- The current explicit recovery threshold is the reduced state in which
backlog has drained to `[]` and the async regime has returned to
`reliableImmediateAssumptions`; once that threshold is reached, convergence of
the canonical route resumes after one reduced step and remains fixed
thereafter. -/
theorem recovery_threshold_resumes_convergence
    (n : Nat)
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRoute destination
        (iterateSystemStep (n + 1) (reliableImmediateRecoveryState state)) =
      canonicalSystemRoute destination (reliableImmediateRecoveryState state) := by
  exact intermittent_loss_eventually_converges_after_recovery n destination state

theorem single_retry_loss_preserves_canonical_support_after_one_retry_cycle
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.async.inFlight)
    (hRetry : eligibleForRetry state.async.assumptions envelope = true)
    (route : LifecycleRoute)
    (hInstall :
      installLifecycleOfEnvelope
          (transportStep (drainReadyMessages (transportStep state.async))).network
          (postRetryCycleEnvelope state.async.assumptions envelope) = some route)
    (hSupport : route.candidate.support ≠ 0)
    (hShape : route.candidate.shape ≠ CorridorShape.opaque)
    (hDominates :
      ∀ competitor,
        competitor ∈
            (systemStep
              { async := drainReadyMessages (transportStep state.async)
                lifecycle := state.lifecycle }).lifecycle →
          CanonicalRouteEligible route.candidate.destination competitor →
            competitor.candidate.support ≤ route.candidate.support) :
    ∃ winner,
      canonicalSystemRoute route.candidate.destination
          { async := drainReadyMessages (transportStep state.async)
            lifecycle := state.lifecycle } = some winner ∧
        winner.candidate.support = route.candidate.support := by
  let retryState : EndToEndState :=
    { async := drainReadyMessages (transportStep state.async)
      lifecycle := state.lifecycle }
  have hRouteReady :
      route ∈ readyInstalledRoutes retryState.async := by
    exact
      retry_eligible_admissible_update_processed_after_one_retry_cycle
        state.async hAssumptions envelope hMem hRetry route hInstall
  have hMaintainedMem :
      lifecycleMaintenance route ∈ (systemStep retryState).lifecycle :=
    ready_installed_route_appears_in_system_step_lifecycle retryState route hRouteReady
  have hMaintainedEq :
      lifecycleMaintenance route = { route with status := .refreshed } :=
    lifecycleMaintenance_refreshes_positive_nonopaque_route route hSupport hShape
  have hMaintainedEligible :
      CanonicalRouteEligible route.candidate.destination (lifecycleMaintenance route) := by
    rw [hMaintainedEq]
    simp [CanonicalRouteEligible]
  have hWinner :
      ∃ winner,
        canonicalBestRoute route.candidate.destination (systemStep retryState).lifecycle = some winner ∧
          winner.candidate.support = (lifecycleMaintenance route).candidate.support := by
    apply canonicalBestRoute_some_with_support_of_dominating_route
    · exact hMaintainedMem
    · exact hMaintainedEligible
    · intro competitor hCompetitorMem hCompetitorEligible
      calc
        competitor.candidate.support ≤ route.candidate.support :=
          hDominates competitor hCompetitorMem hCompetitorEligible
        _ = (lifecycleMaintenance route).candidate.support := by
          simp [lifecycle_maintenance_preserves_candidate]
  rcases hWinner with ⟨winner, hWinnerCanonical, hWinnerSupport⟩
  refine ⟨winner, ?_, ?_⟩
  · exact hWinnerCanonical
  · calc
      winner.candidate.support = (lifecycleMaintenance route).candidate.support := hWinnerSupport
      _ = route.candidate.support := by simp [lifecycle_maintenance_preserves_candidate]

/-- First reduced redundancy-threshold theorem for the broader async regime.
Threshold `1` is enough in the current support-dominance model when the one
recovered admissible update support-dominates every eligible competitor after
the retry cycle. -/
theorem redundancy_threshold_one_preserves_canonical_support
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.async.inFlight)
    (hRetry : eligibleForRetry state.async.assumptions envelope = true)
    (route : LifecycleRoute)
    (hInstall :
      installLifecycleOfEnvelope
          (transportStep (drainReadyMessages (transportStep state.async))).network
          (postRetryCycleEnvelope state.async.assumptions envelope) = some route)
    (hSupport : route.candidate.support ≠ 0)
    (hShape : route.candidate.shape ≠ CorridorShape.opaque)
    (hDominates :
      ∀ competitor,
        competitor ∈
            (systemStep
              { async := drainReadyMessages (transportStep state.async)
                lifecycle := state.lifecycle }).lifecycle →
          CanonicalRouteEligible route.candidate.destination competitor →
            competitor.candidate.support ≤ route.candidate.support) :
    ∃ winner,
      canonicalSystemRoute route.candidate.destination
          { async := drainReadyMessages (transportStep state.async)
            lifecycle := state.lifecycle } = some winner ∧
        winner.candidate.support = route.candidate.support := by
  exact
    single_retry_loss_preserves_canonical_support_after_one_retry_cycle
      state hAssumptions envelope hMem hRetry route hInstall hSupport hShape hDominates

theorem recovered_invalid_update_clears_canonical_route_after_one_retry_cycle
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.async.inFlight)
    (hRetry : eligibleForRetry state.async.assumptions envelope = true)
    (route : LifecycleRoute)
    (hInstall :
      installLifecycleOfEnvelope
          (transportStep (drainReadyMessages (transportStep state.async))).network
          (postRetryCycleEnvelope state.async.assumptions envelope) = some route)
    (hInvalid : route.candidate.support = 0 ∨ route.candidate.shape = CorridorShape.opaque)
    (hNoOtherEligible :
      ∀ competitor,
        competitor ∈
            (systemStep
              { async := drainReadyMessages (transportStep state.async)
                lifecycle := state.lifecycle }).lifecycle →
          competitor ≠ lifecycleMaintenance route →
          competitor.candidate.destination = route.candidate.destination →
            ¬ CanonicalRouteEligible route.candidate.destination competitor) :
    canonicalSystemRoute route.candidate.destination
        { async := drainReadyMessages (transportStep state.async)
          lifecycle := state.lifecycle } = none := by
  let retryState : EndToEndState :=
    { async := drainReadyMessages (transportStep state.async)
      lifecycle := state.lifecycle }
  have hRouteReady :
      route ∈ readyInstalledRoutes retryState.async := by
    exact
      retry_eligible_admissible_update_processed_after_one_retry_cycle
        state.async hAssumptions envelope hMem hRetry route hInstall
  have hMaintainedMem :
      lifecycleMaintenance route ∈ (systemStep retryState).lifecycle :=
    ready_installed_route_appears_in_system_step_lifecycle retryState route hRouteReady
  have hMaintainedNotEligible :
      ¬ CanonicalRouteEligible route.candidate.destination (lifecycleMaintenance route) := by
    rcases hInvalid with hZero | hOpaque
    · unfold CanonicalRouteEligible lifecycleMaintenance
      simp [hZero, expireLifecycleRoute]
    · unfold CanonicalRouteEligible lifecycleMaintenance
      by_cases hZero : route.candidate.support = 0
      · simp [hZero, expireLifecycleRoute]
      · simp [hZero, hOpaque, withdrawLifecycleRoute]
  unfold canonicalSystemRoute
  apply canonicalBestRoute_eq_none_of_no_eligible
  intro competitor hCompetitorMem hCompetitorEligible
  by_cases hEq : competitor = lifecycleMaintenance route
  · subst hEq
    exact hMaintainedNotEligible hCompetitorEligible
  · exact
      hNoOtherEligible competitor hCompetitorMem hEq
        (by exact hCompetitorEligible.2) hCompetitorEligible

theorem single_retry_loss_graceful_degradation_envelope
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = boundedDelayRetryAssumptions)
    (envelope : AsyncEnvelope)
    (hMem : envelope ∈ state.async.inFlight)
    (hRetry : eligibleForRetry state.async.assumptions envelope = true)
    (route : LifecycleRoute)
    (hInstall :
      installLifecycleOfEnvelope
          (transportStep (drainReadyMessages (transportStep state.async))).network
          (postRetryCycleEnvelope state.async.assumptions envelope) = some route)
    (hEnvelope :
      (route.candidate.support ≠ 0 ∧
        route.candidate.shape ≠ CorridorShape.opaque ∧
        (∀ competitor,
          competitor ∈
              (systemStep
                { async := drainReadyMessages (transportStep state.async)
                  lifecycle := state.lifecycle }).lifecycle →
            CanonicalRouteEligible route.candidate.destination competitor →
              competitor.candidate.support ≤ route.candidate.support))
        ∨
      ((route.candidate.support = 0 ∨ route.candidate.shape = CorridorShape.opaque) ∧
        (∀ competitor,
          competitor ∈
              (systemStep
                { async := drainReadyMessages (transportStep state.async)
                  lifecycle := state.lifecycle }).lifecycle →
            competitor ≠ lifecycleMaintenance route →
            competitor.candidate.destination = route.candidate.destination →
              ¬ CanonicalRouteEligible route.candidate.destination competitor))) :
    (∃ winner,
      canonicalSystemRoute route.candidate.destination
          { async := drainReadyMessages (transportStep state.async)
            lifecycle := state.lifecycle } = some winner ∧
        winner.candidate.support = route.candidate.support)
      ∨
      canonicalSystemRoute route.candidate.destination
          { async := drainReadyMessages (transportStep state.async)
            lifecycle := state.lifecycle } = none := by
  rcases hEnvelope with ⟨hSupport, hShape, hDominates⟩ | ⟨hInvalid, hNoOtherEligible⟩
  · left
    exact
      single_retry_loss_preserves_canonical_support_after_one_retry_cycle
        state hAssumptions envelope hMem hRetry route hInstall hSupport hShape hDominates
  · right
    exact
      recovered_invalid_update_clears_canonical_route_after_one_retry_cycle
        state hAssumptions envelope hMem hRetry route hInstall hInvalid hNoOtherEligible

end FieldSystemBounded
