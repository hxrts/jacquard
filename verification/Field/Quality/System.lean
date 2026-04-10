import Field.Quality.API
import Field.System.Convergence

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldQualitySystem

open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldRouterLifecycle
open FieldSystemConvergence
open FieldSystemEndToEnd

def bestSystemRouteView
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (state : EndToEndState) : Option RouteComparisonView :=
  bestRouteView objective destination (systemStep state).lifecycle

theorem system_step_lifecycle_eq_canonical_under_reliable_immediate_empty
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    (systemStep state).lifecycle =
      maintainLifecycle (canonicalInstalledRoutes state.async.network) := by
  unfold systemStep
  simp [ready_installed_routes_eq_canonical_under_reliable_immediate_empty state.async hAssumptions hEmpty]

theorem system_step_lifecycle_fixed_point_under_reliable_immediate_empty
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    (systemStep (systemStep state)).lifecycle = (systemStep state).lifecycle := by
  have hPres :=
    system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
  rw [system_step_lifecycle_eq_canonical_under_reliable_immediate_empty (systemStep state) hPres.1 hPres.2]
  rw [system_step_lifecycle_eq_canonical_under_reliable_immediate_empty state hAssumptions hEmpty]
  simp [system_step_preserves_network]

theorem best_system_route_view_stable_under_reliable_immediate_empty
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    bestSystemRouteView objective destination (systemStep state) =
      bestSystemRouteView objective destination state := by
  unfold bestSystemRouteView
  rw [system_step_lifecycle_fixed_point_under_reliable_immediate_empty state hAssumptions hEmpty]

theorem best_system_route_view_cannot_manufacture_explicit_path
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : RouteComparisonView)
    (hNoExplicit :
      ∀ route ∈ (systemStep state).lifecycle,
        route.candidate.shape ≠ CorridorShape.explicitPath)
    (hWinner : bestSystemRouteView objective destination state = some winner) :
    winner.shape ≠ CorridorShape.explicitPath := by
  exact
    bestRouteView_cannot_manufacture_explicit_path
      objective destination (systemStep state).lifecycle winner hNoExplicit hWinner

theorem best_system_route_view_support_conservative
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (winner : RouteComparisonView)
    (hWinner : bestSystemRouteView objective destination state = some winner) :
    winner.support ≤
      (state.async.network.localStates winner.publisher destination).posterior.support := by
  rcases
      bestRouteView_some_implies_from_route
        objective destination (systemStep state).lifecycle winner hWinner with
    ⟨route, hRouteMem, hView⟩
  have hWinnerDestination :
      winner.destination = destination :=
    bestRouteView_some_has_destination
      objective destination (systemStep state).lifecycle winner hWinner
  have hWinnerPublisher :
      winner.publisher = route.candidate.publisher := by
    simpa [routeComparisonView] using (congrArg RouteComparisonView.publisher hView).symm
  have hWinnerSupport :
      winner.support = route.candidate.support := by
    simpa [routeComparisonView] using (congrArg RouteComparisonView.support hView).symm
  have hRouteDestination :
      route.candidate.destination = destination := by
    calc
      route.candidate.destination = winner.destination := by
        simpa [routeComparisonView] using congrArg RouteComparisonView.destination hView
      _ = destination := hWinnerDestination
  have hCandidateMem :
      route.candidate ∈ lifecycleCandidateView (systemStep state).lifecycle := by
    unfold lifecycleCandidateView
    exact List.mem_map.2 ⟨route, hRouteMem, rfl⟩
  have hProduced :=
    candidate_mem_system_step_view_implies_produced
      state hAssumptions hEmpty route.candidate hCandidateMem
  have hSupport :=
    produced_candidate_support_conservative
      state.async hAssumptions hEmpty hHarmony route.candidate hProduced
  calc
    winner.support = route.candidate.support := hWinnerSupport
    _ ≤
        (state.async.network.localStates route.candidate.publisher route.candidate.destination).posterior.support :=
          hSupport
    _ =
        (state.async.network.localStates winner.publisher destination).posterior.support := by
          simp [hWinnerPublisher, hRouteDestination]

theorem best_system_route_view_explicit_path_requires_explicit_sender_knowledge
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (winner : RouteComparisonView)
    (hWinner : bestSystemRouteView objective destination state = some winner)
    (hShape : winner.shape = CorridorShape.explicitPath) :
    (state.async.network.localStates winner.publisher destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  rcases
      bestRouteView_some_implies_from_route
        objective destination (systemStep state).lifecycle winner hWinner with
    ⟨route, hRouteMem, hView⟩
  have hWinnerDestination :
      winner.destination = destination :=
    bestRouteView_some_has_destination
      objective destination (systemStep state).lifecycle winner hWinner
  have hWinnerPublisher :
      winner.publisher = route.candidate.publisher := by
    simpa [routeComparisonView] using (congrArg RouteComparisonView.publisher hView).symm
  have hRouteDestination :
      route.candidate.destination = destination := by
    calc
      route.candidate.destination = winner.destination := by
        simpa [routeComparisonView] using congrArg RouteComparisonView.destination hView
      _ = destination := hWinnerDestination
  have hCandidateMem :
      route.candidate ∈ lifecycleCandidateView (systemStep state).lifecycle := by
    unfold lifecycleCandidateView
    exact List.mem_map.2 ⟨route, hRouteMem, rfl⟩
  have hProduced :=
    candidate_mem_system_step_view_implies_produced
      state hAssumptions hEmpty route.candidate hCandidateMem
  have hCandidateShape : route.candidate.shape = CorridorShape.explicitPath := by
    have hViewShape : (routeComparisonView route).shape = CorridorShape.explicitPath := by
      simpa [hView] using hShape
    simpa [routeComparisonView] using hViewShape
  have hKnowledge :=
    produced_candidate_requires_explicit_sender_knowledge
      state.async hAssumptions hEmpty hHarmony route.candidate hProduced hCandidateShape
  calc
    (state.async.network.localStates winner.publisher destination).posterior.knowledge
        =
          (state.async.network.localStates route.candidate.publisher route.candidate.destination).posterior.knowledge := by
            simp [hWinnerPublisher, hRouteDestination]
    _ = ReachabilityKnowledge.explicitPath := hKnowledge

theorem ready_installed_route_eventually_appears_in_system_destination_views
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ readyInstalledRoutes state.async)
    (hSupport : route.candidate.support ≠ 0)
    (hShape : route.candidate.shape ≠ CorridorShape.opaque) :
    routeComparisonView (lifecycleMaintenance route) ∈
      destinationViews route.candidate.destination (systemStep state).lifecycle := by
  unfold destinationViews
  apply List.mem_filterMap.2
  refine ⟨lifecycleMaintenance route,
    ready_installed_route_appears_in_system_step_lifecycle state route hMem, ?_⟩
  have hMaintained :
      lifecycleMaintenance route =
        { route with status := .refreshed } := by
    cases route with
    | mk candidate status =>
        simp [lifecycleMaintenance, hSupport, hShape, refreshLifecycleRoute]
  rw [hMaintained]
  simp [destinationView, routeComparisonView, RouteViewAdmissible, routeViewIsActive]

theorem best_system_route_view_idempotent_under_lifecycle_maintenance
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    bestRouteView objective destination (maintainLifecycle (maintainLifecycle routes)) =
      bestRouteView objective destination (maintainLifecycle routes) := by
  exact bestRouteView_maintainLifecycle_idempotent objective destination routes

end FieldQualitySystem
