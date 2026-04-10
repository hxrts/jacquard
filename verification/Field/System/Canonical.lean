import Field.Quality.Refinement
import Field.Router.Canonical

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemCanonical

open FieldAsyncAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualityReference
open FieldQualityRefinement
open FieldQualitySystem
open FieldRouterCanonical
open FieldRouterLifecycle
open FieldSystemConvergence
open FieldSystemEndToEnd

def canonicalSystemRoute
    (destination : DestinationClass)
    (state : EndToEndState) : Option LifecycleRoute :=
  canonicalBestRoute destination (systemStep state).lifecycle

def canonicalSystemRouteView
    (destination : DestinationClass)
    (state : EndToEndState) : Option RouteComparisonView :=
  Option.map routeComparisonView (canonicalSystemRoute destination state)

theorem canonicalRouteEligible_iff_routeComparisonView_eligible
    (destination : DestinationClass)
    (route : LifecycleRoute) :
    CanonicalRouteEligible destination route ↔
      (RouteViewAdmissible (routeComparisonView route) ∧
        (routeComparisonView route).destination = destination) := by
  cases route with
  | mk candidate status =>
      cases status <;>
        simp [CanonicalRouteEligible, RouteViewAdmissible, routeViewIsActive, routeComparisonView]

theorem canonicalEligibleRoutes_map_routeComparisonView_eq_destinationViews
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    (canonicalEligibleRoutes destination routes).map routeComparisonView =
      destinationViews destination routes := by
  induction routes with
  | nil =>
      simp [canonicalEligibleRoutes, destinationViews]
  | cons route rest ih =>
      by_cases hEligible : CanonicalRouteEligible destination route
      · have hView :
            RouteViewAdmissible (routeComparisonView route) ∧
              (routeComparisonView route).destination = destination :=
          (canonicalRouteEligible_iff_routeComparisonView_eligible destination route).mp hEligible
        simp [canonicalEligibleRoutes, destinationViews, eligibleCanonicalRoute, destinationView,
          hEligible, hView]
        exact ih
      · have hView :
            ¬ (RouteViewAdmissible (routeComparisonView route) ∧
                (routeComparisonView route).destination = destination) := by
          intro h
          exact hEligible ((canonicalRouteEligible_iff_routeComparisonView_eligible destination route).mpr h)
        simp [canonicalEligibleRoutes, destinationViews, eligibleCanonicalRoute, destinationView,
          hEligible, hView]
        exact ih

theorem routeComparisonView_chooseCanonicalRouteBySupport_eq_choosePreferredView_supportDominance
    (destination : DestinationClass)
    (current next : LifecycleRoute)
    (hCurrent : CanonicalRouteEligible destination current)
    (hNext : CanonicalRouteEligible destination next) :
    routeComparisonView (chooseCanonicalRouteBySupport current next) =
      choosePreferredView .supportDominance (routeComparisonView current) (routeComparisonView next) := by
  have hCurrentRef :
      ReferenceViewAdmissible destination (routeComparisonView current) :=
    (canonicalRouteEligible_iff_routeComparisonView_eligible destination current).mp hCurrent
  have hNextRef :
      ReferenceViewAdmissible destination (routeComparisonView next) :=
    (canonicalRouteEligible_iff_routeComparisonView_eligible destination next).mp hNext
  have hQuality :
      choosePreferredView .supportDominance (routeComparisonView current) (routeComparisonView next) =
        chooseHigherSupport (routeComparisonView current) (routeComparisonView next) :=
    choosePreferredView_supportDominance_eq_chooseHigherSupport
      destination (routeComparisonView current) (routeComparisonView next) hCurrentRef hNextRef
  have hCanonical :
      routeComparisonView (chooseCanonicalRouteBySupport current next) =
        chooseHigherSupport (routeComparisonView current) (routeComparisonView next) := by
    unfold chooseCanonicalRouteBySupport chooseHigherSupport
    by_cases hLt : current.candidate.support < next.candidate.support
    · simp [hLt, routeComparisonView]
    · simp [hLt, routeComparisonView]
  exact hCanonical.trans hQuality.symm

theorem fold_chooseCanonicalRouteBySupport_routeComparison_eq_supportDominance
    (destination : DestinationClass)
    (current : LifecycleRoute)
    (tail : List LifecycleRoute)
    (hCurrent : CanonicalRouteEligible destination current)
    (hTail : ∀ route ∈ tail, CanonicalRouteEligible destination route) :
    routeComparisonView (tail.foldl chooseCanonicalRouteBySupport current) =
      (tail.map routeComparisonView).foldl
        (choosePreferredView .supportDominance)
        (routeComparisonView current) := by
  induction tail generalizing current with
  | nil =>
      rfl
  | cons head rest ih =>
      have hHead : CanonicalRouteEligible destination head :=
        hTail head (by simp)
      have hStep :
          routeComparisonView (chooseCanonicalRouteBySupport current head) =
            choosePreferredView .supportDominance
              (routeComparisonView current)
              (routeComparisonView head) :=
        routeComparisonView_chooseCanonicalRouteBySupport_eq_choosePreferredView_supportDominance
          destination current head hCurrent hHead
      have hChosen :
          CanonicalRouteEligible destination (chooseCanonicalRouteBySupport current head) :=
        chooseCanonicalRouteBySupport_preserves_eligible destination current head hCurrent hHead
      have hRest :
          ∀ route ∈ rest, CanonicalRouteEligible destination route := by
        intro route hMem
        exact hTail route (by simp [hMem])
      simpa [List.foldl, hStep] using
        ih (chooseCanonicalRouteBySupport current head) hChosen hRest

theorem canonicalBestRouteView_eq_bestRouteView_supportDominance
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    Option.map routeComparisonView (canonicalBestRoute destination routes) =
      bestRouteView .supportDominance destination routes := by
  unfold bestRouteView
  rw [← canonicalEligibleRoutes_map_routeComparisonView_eq_destinationViews destination routes]
  unfold canonicalBestRoute bestView
  cases hEligible : canonicalEligibleRoutes destination routes with
  | nil =>
      simp [hEligible]
  | cons head tail =>
      have hHead : CanonicalRouteEligible destination head := by
        have hMemHead : head ∈ canonicalEligibleRoutes destination routes := by
          rw [hEligible]
          simp
        exact canonicalEligibleRoutes_mem_implies_eligible destination routes head hMemHead
      have hTail :
          ∀ route ∈ tail, CanonicalRouteEligible destination route := by
        intro route hMem
        have hMemEligible : route ∈ canonicalEligibleRoutes destination routes := by
          rw [hEligible]
          simp [hMem]
        exact canonicalEligibleRoutes_mem_implies_eligible destination routes route hMemEligible
      have hFold :=
        fold_chooseCanonicalRouteBySupport_routeComparison_eq_supportDominance
          destination head tail hHead hTail
      simp [hEligible, hFold]

theorem canonicalSystemRoute_eq_router_canonical_under_reliable_immediate_empty
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRoute destination state =
      canonicalBestRoute destination
        (maintainLifecycle (canonicalInstalledRoutes state.async.network)) := by
  unfold canonicalSystemRoute
  rw [system_step_lifecycle_eq_canonical_under_reliable_immediate_empty state hAssumptions hEmpty]

theorem canonical_system_route_stable_under_reliable_immediate_empty
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRoute destination (systemStep state) =
      canonicalSystemRoute destination state := by
  unfold canonicalSystemRoute
  rw [system_step_lifecycle_fixed_point_under_reliable_immediate_empty state hAssumptions hEmpty]

theorem bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView
    (destination : DestinationClass)
    (state : EndToEndState) :
    bestSystemRouteView .supportDominance destination state =
      canonicalSystemRouteView destination state := by
  unfold bestSystemRouteView canonicalSystemRouteView canonicalSystemRoute
  exact
    (canonicalBestRouteView_eq_bestRouteView_supportDominance
      destination (systemStep state).lifecycle).symm

theorem canonicalSystemRoute_some_is_support_best
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner) :
    CanonicalSupportBest destination (systemStep state).lifecycle winner := by
  exact canonicalBestRoute_some_is_support_best destination (systemStep state).lifecycle winner hWinner

end FieldSystemCanonical
