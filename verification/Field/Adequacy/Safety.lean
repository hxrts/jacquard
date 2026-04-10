import Field.Adequacy.Refinement
import Field.Quality.System
import Field.Router.Canonical
import Field.System.Canonical

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacySafety

open FieldAdequacyAPI
open FieldAdequacyProjection
open FieldAdequacyRefinement
open FieldAdequacyRuntime
open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualitySystem
open FieldRouterCanonical
open FieldRouterLifecycle
open FieldSystemCanonical
open FieldSystemEndToEnd

theorem canonicalSystemRoute_eq_none_of_no_destination_lifecycle_match
    (destination : DestinationClass)
    (state : EndToEndState)
    (hSilent :
      ∀ route ∈ (systemStep state).lifecycle,
        route.candidate.destination ≠ destination) :
    canonicalSystemRoute destination state = none := by
  unfold canonicalSystemRoute
  exact
    canonicalBestRoute_eq_none_of_no_destination_match
      destination
      (systemStep state).lifecycle
      hSilent

theorem quiescent_runtime_state_no_route_creation_from_system_silence
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState)
    (hSilent :
      ∀ route ∈ (systemStep state).lifecycle,
        route.candidate.destination ≠ destination) :
    runtimeCanonicalRoute destination (runtimeArtifactsOfState runtimeState) = none := by
  rw [quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
    destination runtimeState state hRefinement hQuiescent]
  exact canonicalSystemRoute_eq_none_of_no_destination_lifecycle_match destination state hSilent

theorem quiescent_runtime_state_no_false_explicit_path_promotion
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (hNoExplicit :
      ∀ sender,
        (state.async.network.localStates sender destination).posterior.knowledge ≠
          ReachabilityKnowledge.explicitPath)
    (winner : RouteComparisonView)
    (hWinner :
      runtimeCanonicalRouteView destination (runtimeArtifactsOfState runtimeState) = some winner) :
    winner.shape ≠ CorridorShape.explicitPath := by
  intro hShape
  have hKnowledge :=
    quiescent_runtime_state_explicit_path_requires_explicit_sender_knowledge
      destination runtimeState state hRefinement hQuiescent
      hAssumptions hEmpty hHarmony winner hWinner hShape
  exact hNoExplicit winner.publisher hKnowledge

theorem quiescent_runtime_state_canonical_winner_has_admissible_system_origin
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState)
    (winner : FieldRouterLifecycle.LifecycleRoute)
    (hWinner :
      runtimeCanonicalRoute destination (runtimeArtifactsOfState runtimeState) = some winner) :
    ∃ source,
      source ∈ readyInstalledRoutes state.async ∧
        source.status = .installed ∧
        lifecycleMaintenance source = winner := by
  have hSystem :
      canonicalSystemRoute destination state = some winner := by
    rw [← quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
      destination runtimeState state hRefinement hQuiescent]
    exact hWinner
  have hWinnerMem :
      winner ∈ (systemStep state).lifecycle := by
    unfold canonicalSystemRoute at hSystem
    exact canonicalBestRoute_some_mem destination (systemStep state).lifecycle winner hSystem
  exact system_step_route_has_admissible_lifecycle_origin state winner hWinnerMem

theorem runtime_step_preserves_protocol_and_router_invariants
    {runtimeSource runtimeTarget : RuntimeState}
    {state : EndToEndState}
    (hAdmitted : RuntimeStateAdmitted runtimeSource)
    (hRefinement : RuntimeStateProjectsSystemState runtimeSource state)
    (hStep : RuntimeStep runtimeSource runtimeTarget) :
    RuntimeStateAdmitted runtimeTarget ∧
      RuntimeStateProjectsSystemState runtimeTarget state := by
  exact
    runtime_step_preserves_runtime_system_refinement_admitted
      hAdmitted hRefinement hStep

theorem quiescent_runtime_states_projecting_same_system_have_equal_canonical_route
    (destination : DestinationClass)
    (left right : RuntimeState)
    (state : EndToEndState)
    (hLeft : RuntimeStateProjectsSystemState left state)
    (hRight : RuntimeStateProjectsSystemState right state)
    (hLeftQuiescent : RuntimeStateQuiescent left)
    (hRightQuiescent : RuntimeStateQuiescent right) :
    runtimeCanonicalRoute destination (runtimeArtifactsOfState left) =
      runtimeCanonicalRoute destination (runtimeArtifactsOfState right) := by
  rw [quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
    destination left state hLeft hLeftQuiescent]
  rw [quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
    destination right state hRight hRightQuiescent]

theorem quiescent_runtime_states_projecting_same_system_have_equal_canonical_route_view
    (destination : DestinationClass)
    (left right : RuntimeState)
    (state : EndToEndState)
    (hLeft : RuntimeStateProjectsSystemState left state)
    (hRight : RuntimeStateProjectsSystemState right state)
    (hLeftQuiescent : RuntimeStateQuiescent left)
    (hRightQuiescent : RuntimeStateQuiescent right) :
    runtimeCanonicalRouteView destination (runtimeArtifactsOfState left) =
      runtimeCanonicalRouteView destination (runtimeArtifactsOfState right) := by
  rw [quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
    destination left state hLeft hLeftQuiescent]
  rw [quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
    destination right state hRight hRightQuiescent]

theorem quiescent_runtime_state_canonical_route_stable_under_reliable_immediate_empty
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    runtimeCanonicalRoute destination (runtimeArtifactsOfState runtimeState) =
      canonicalSystemRoute destination (systemStep state) := by
  rw [quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
    destination runtimeState state hRefinement hQuiescent]
  exact (canonical_system_route_stable_under_reliable_immediate_empty
    destination state hAssumptions hEmpty).symm

theorem runtime_projection_observational_equivalence_preserves_canonical_route
    (destination : DestinationClass)
    (left right : EndToEndState)
    (hEq : projectedRuntimeArtifactsOfState left = projectedRuntimeArtifactsOfState right) :
    canonicalSystemRoute destination left = canonicalSystemRoute destination right := by
  have hLeft :=
    projected_runtime_canonical_route_eq_canonicalSystemRoute destination left
  have hRight :=
    projected_runtime_canonical_route_eq_canonicalSystemRoute destination right
  rw [← hLeft, ← hRight, hEq]

theorem runtime_projection_observational_equivalence_preserves_canonical_route_view
    (destination : DestinationClass)
    (left right : EndToEndState)
    (hEq : projectedRuntimeArtifactsOfState left = projectedRuntimeArtifactsOfState right) :
    bestSystemRouteView .supportDominance destination left =
      bestSystemRouteView .supportDominance destination right := by
  have hLeft :=
    projected_runtime_canonical_route_view_eq_bestSystemRouteView_supportDominance destination left
  have hRight :=
    projected_runtime_canonical_route_view_eq_bestSystemRouteView_supportDominance destination right
  rw [← hLeft, ← hRight, hEq]

theorem canonical_route_order_insensitive_under_equal_projected_artifacts
    (destination : DestinationClass)
    (left right : EndToEndState)
    (hEq : projectedRuntimeArtifactsOfState left = projectedRuntimeArtifactsOfState right) :
    canonicalSystemRoute destination left = canonicalSystemRoute destination right := by
  exact runtime_projection_observational_equivalence_preserves_canonical_route
    destination left right hEq

theorem canonical_route_view_order_insensitive_under_equal_projected_artifacts
    (destination : DestinationClass)
    (left right : EndToEndState)
    (hEq : projectedRuntimeArtifactsOfState left = projectedRuntimeArtifactsOfState right) :
    bestSystemRouteView .supportDominance destination left =
      bestSystemRouteView .supportDominance destination right := by
  exact runtime_projection_observational_equivalence_preserves_canonical_route_view
    destination left right hEq

end FieldAdequacySafety
