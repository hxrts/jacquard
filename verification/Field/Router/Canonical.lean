import Field.Router.Lifecycle

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterCanonical

open FieldNetworkAPI
open FieldRouterLifecycle

def CanonicalRouteEligible
    (destination : DestinationClass)
    (route : LifecycleRoute) : Prop :=
  (route.status = .installed ∨ route.status = .refreshed) ∧
    route.candidate.destination = destination

instance instDecidableCanonicalRouteEligible
    (destination : DestinationClass)
    (route : LifecycleRoute) :
    Decidable (CanonicalRouteEligible destination route) := by
  unfold CanonicalRouteEligible
  infer_instance

def eligibleCanonicalRoute
    (destination : DestinationClass)
    (route : LifecycleRoute) : Option LifecycleRoute :=
  if h : CanonicalRouteEligible destination route then
    some route
  else
    none

def canonicalEligibleRoutes
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : List LifecycleRoute :=
  routes.filterMap (eligibleCanonicalRoute destination)

def chooseCanonicalRouteBySupport
    (current next : LifecycleRoute) : LifecycleRoute :=
  if current.candidate.support < next.candidate.support then next else current

def canonicalBestRoute
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option LifecycleRoute :=
  match canonicalEligibleRoutes destination routes with
  | [] => none
  | head :: tail => some (tail.foldl chooseCanonicalRouteBySupport head)

def CanonicalSupportBest
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute) : Prop :=
  CanonicalRouteEligible destination winner ∧
    winner ∈ routes ∧
    ∀ competitor,
      competitor ∈ routes →
        CanonicalRouteEligible destination competitor →
        competitor.candidate.support ≤ winner.candidate.support

theorem eligibleCanonicalRoute_some_implies_route
    (destination : DestinationClass)
    (route winner : LifecycleRoute)
    (hSome : eligibleCanonicalRoute destination route = some winner) :
    route = winner := by
  by_cases hEligible : CanonicalRouteEligible destination route
  · simp [eligibleCanonicalRoute, hEligible] at hSome
    exact hSome
  · simp [eligibleCanonicalRoute, hEligible] at hSome

theorem eligibleCanonicalRoute_some_implies_eligible
    (destination : DestinationClass)
    (route winner : LifecycleRoute)
    (hSome : eligibleCanonicalRoute destination route = some winner) :
    CanonicalRouteEligible destination winner := by
  have hEq := eligibleCanonicalRoute_some_implies_route destination route winner hSome
  subst hEq
  by_cases hEligible : CanonicalRouteEligible destination route
  · simpa [eligibleCanonicalRoute, hEligible] using hEligible
  · simp [eligibleCanonicalRoute, hEligible] at hSome

theorem chooseCanonicalRouteBySupport_eq_current_or_next
    (current next : LifecycleRoute) :
    chooseCanonicalRouteBySupport current next = current ∨
      chooseCanonicalRouteBySupport current next = next := by
  unfold chooseCanonicalRouteBySupport
  by_cases hLt : current.candidate.support < next.candidate.support
  · right
    simp [hLt]
  · left
    simp [hLt]

theorem chooseCanonicalRouteBySupport_support_ge_current
    (current next : LifecycleRoute) :
    current.candidate.support ≤
      (chooseCanonicalRouteBySupport current next).candidate.support := by
  unfold chooseCanonicalRouteBySupport
  by_cases hLt : current.candidate.support < next.candidate.support
  · simp [hLt, Nat.le_of_lt hLt]
  · simp [hLt]

theorem chooseCanonicalRouteBySupport_support_ge_next
    (current next : LifecycleRoute) :
    next.candidate.support ≤
      (chooseCanonicalRouteBySupport current next).candidate.support := by
  unfold chooseCanonicalRouteBySupport
  by_cases hLt : current.candidate.support < next.candidate.support
  · simp [hLt]
  · have hLe : next.candidate.support ≤ current.candidate.support :=
      Nat.le_of_not_gt hLt
    simp [hLt, hLe]

theorem chooseCanonicalRouteBySupport_preserves_eligible
    (destination : DestinationClass)
    (current next : LifecycleRoute)
    (hCurrent : CanonicalRouteEligible destination current)
    (hNext : CanonicalRouteEligible destination next) :
    CanonicalRouteEligible destination (chooseCanonicalRouteBySupport current next) := by
  rcases chooseCanonicalRouteBySupport_eq_current_or_next current next with hEq | hEq
  · simpa [hEq] using hCurrent
  · simpa [hEq] using hNext

theorem fold_chooseCanonicalRouteBySupport_mem
    (current : LifecycleRoute)
    (tail : List LifecycleRoute) :
    tail.foldl chooseCanonicalRouteBySupport current ∈ current :: tail := by
  induction tail generalizing current with
  | nil =>
      simp
  | cons head rest ih =>
      simp [List.foldl]
      rcases chooseCanonicalRouteBySupport_eq_current_or_next current head with hCurrent | hHead
      · have hIH :
            rest.foldl chooseCanonicalRouteBySupport current = current ∨
              rest.foldl chooseCanonicalRouteBySupport current ∈ rest := by
            simpa using ih current
        rcases hIH with hEq | hMem
        · simp [hCurrent, hEq]
        · simp [hCurrent, hMem]
      · have hIH :
            rest.foldl chooseCanonicalRouteBySupport head = head ∨
              rest.foldl chooseCanonicalRouteBySupport head ∈ rest := by
            simpa using ih head
        rcases hIH with hEq | hMem
        · simp [hHead, hEq]
        · simp [hHead, hMem]

theorem fold_chooseCanonicalRouteBySupport_support_dominates
    (current competitor : LifecycleRoute)
    (tail : List LifecycleRoute)
    (hMem : competitor ∈ current :: tail) :
    competitor.candidate.support ≤
      (tail.foldl chooseCanonicalRouteBySupport current).candidate.support := by
  induction tail generalizing current competitor with
  | nil =>
      simp at hMem
      subst hMem
      simp
  | cons head rest ih =>
      let chosen := chooseCanonicalRouteBySupport current head
      have hChosenCurrent :
          current.candidate.support ≤ chosen.candidate.support := by
        exact chooseCanonicalRouteBySupport_support_ge_current current head
      have hChosenHead :
          head.candidate.support ≤ chosen.candidate.support := by
        exact chooseCanonicalRouteBySupport_support_ge_next current head
      have hChosenFixed :
          chosen.candidate.support ≤
            (rest.foldl chooseCanonicalRouteBySupport chosen).candidate.support := by
        exact ih chosen chosen (by simp)
      simp at hMem
      rcases hMem with rfl | rfl | hRest
      · exact Nat.le_trans hChosenCurrent hChosenFixed
      · exact Nat.le_trans hChosenHead hChosenFixed
      · exact ih chosen competitor (by simp [hRest])

theorem canonicalEligibleRoutes_mem_implies_from_routes
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hMem : winner ∈ canonicalEligibleRoutes destination routes) :
    winner ∈ routes := by
  unfold canonicalEligibleRoutes at hMem
  rcases List.mem_filterMap.1 hMem with ⟨route, hRouteMem, hSome⟩
  have hEq := eligibleCanonicalRoute_some_implies_route destination route winner hSome
  simpa [hEq] using hRouteMem

theorem canonicalEligibleRoutes_mem_implies_eligible
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hMem : winner ∈ canonicalEligibleRoutes destination routes) :
    CanonicalRouteEligible destination winner := by
  unfold canonicalEligibleRoutes at hMem
  rcases List.mem_filterMap.1 hMem with ⟨route, _, hSome⟩
  exact eligibleCanonicalRoute_some_implies_eligible destination route winner hSome

theorem canonicalBestRoute_some_mem
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner : canonicalBestRoute destination routes = some winner) :
    winner ∈ routes := by
  unfold canonicalBestRoute at hWinner
  cases hEligible : canonicalEligibleRoutes destination routes with
  | nil =>
      simp [hEligible] at hWinner
  | cons head tail =>
      simp [hEligible] at hWinner
      subst hWinner
      have hMemEligible :
          tail.foldl chooseCanonicalRouteBySupport head ∈ head :: tail :=
        fold_chooseCanonicalRouteBySupport_mem head tail
      rw [← hEligible] at hMemEligible
      exact canonicalEligibleRoutes_mem_implies_from_routes destination routes _ hMemEligible

theorem canonicalBestRoute_some_is_eligible
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner : canonicalBestRoute destination routes = some winner) :
    CanonicalRouteEligible destination winner := by
  unfold canonicalBestRoute at hWinner
  cases hEligible : canonicalEligibleRoutes destination routes with
  | nil =>
      simp [hEligible] at hWinner
  | cons head tail =>
      simp [hEligible] at hWinner
      subst hWinner
      have hMemEligible :
          tail.foldl chooseCanonicalRouteBySupport head ∈ head :: tail :=
        fold_chooseCanonicalRouteBySupport_mem head tail
      rw [← hEligible] at hMemEligible
      exact canonicalEligibleRoutes_mem_implies_eligible destination routes _ hMemEligible

theorem canonicalBestRoute_some_is_support_best
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner : canonicalBestRoute destination routes = some winner) :
    CanonicalSupportBest destination routes winner := by
  unfold canonicalBestRoute at hWinner
  cases hEligible : canonicalEligibleRoutes destination routes with
  | nil =>
      simp [hEligible] at hWinner
  | cons head tail =>
      simp [hEligible] at hWinner
      subst hWinner
      have hMemEligible :
          tail.foldl chooseCanonicalRouteBySupport head ∈ head :: tail :=
        fold_chooseCanonicalRouteBySupport_mem head tail
      have hEligibleWinner :
          CanonicalRouteEligible destination (tail.foldl chooseCanonicalRouteBySupport head) := by
        rw [← hEligible] at hMemEligible
        exact canonicalEligibleRoutes_mem_implies_eligible destination routes _ hMemEligible
      have hWinnerMemRoutes :
          tail.foldl chooseCanonicalRouteBySupport head ∈ routes := by
        rw [← hEligible] at hMemEligible
        exact canonicalEligibleRoutes_mem_implies_from_routes destination routes _ hMemEligible
      refine ⟨hEligibleWinner, hWinnerMemRoutes, ?_⟩
      intro competitor hCompetitor hCompetitorEligible
      have hCompetitorEligibleMem :
          competitor ∈ head :: tail := by
        rw [← hEligible]
        unfold canonicalEligibleRoutes
        exact List.mem_filterMap.2 ⟨competitor, hCompetitor, by simp [eligibleCanonicalRoute, hCompetitorEligible]⟩
      exact
        fold_chooseCanonicalRouteBySupport_support_dominates
          head competitor tail hCompetitorEligibleMem

end FieldRouterCanonical
