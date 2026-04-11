import Field.Router.Lifecycle

/-! # Router.Selector — shared selector-family abstraction for lifecycle routes -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterSelector

open FieldNetworkAPI
open FieldRouterLifecycle

structure LifecycleRouteSelector where
  eligible : DestinationClass → LifecycleRoute → Prop
  choose : LifecycleRoute → LifecycleRoute → LifecycleRoute

noncomputable def eligibleRoute
    (selector : LifecycleRouteSelector)
    (destination : DestinationClass)
    (route : LifecycleRoute) : Option LifecycleRoute :=
  by
    classical
    exact if h : selector.eligible destination route then some route else none

noncomputable def eligibleRoutes
    (selector : LifecycleRouteSelector)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : List LifecycleRoute :=
  routes.filterMap (eligibleRoute selector destination)

noncomputable def bestRoute
    (selector : LifecycleRouteSelector)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option LifecycleRoute :=
  match eligibleRoutes selector destination routes with
  | [] => none
  | head :: tail => some (tail.foldl selector.choose head)

theorem eligibleRoute_some_implies_route
    (selector : LifecycleRouteSelector)
    (destination : DestinationClass)
    (route winner : LifecycleRoute)
    (hSome : eligibleRoute selector destination route = some winner) :
    route = winner := by
  classical
  by_cases hEligible : selector.eligible destination route
  · simp [eligibleRoute, hEligible] at hSome
    exact hSome
  · simp [eligibleRoute, hEligible] at hSome

theorem eligibleRoutes_mem_implies_from_routes
    (selector : LifecycleRouteSelector)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hMem : winner ∈ eligibleRoutes selector destination routes) :
    winner ∈ routes := by
  unfold eligibleRoutes at hMem
  rcases List.mem_filterMap.1 hMem with ⟨route, hRouteMem, hSome⟩
  have hEq := eligibleRoute_some_implies_route selector destination route winner hSome
  simpa [hEq] using hRouteMem

end FieldRouterSelector
