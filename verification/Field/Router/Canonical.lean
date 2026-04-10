import Field.Router.Lifecycle

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterCanonical

open FieldModelAPI
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

def CanonicalSupportAtLeast
    (threshold : Nat)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Prop :=
  ∃ winner,
    canonicalBestRoute destination routes = some winner ∧
      threshold ≤ winner.candidate.support

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
  · simp [hEligible]
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

theorem canonicalBestRoute_some_with_support_of_dominating_route
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ routes)
    (hEligible : CanonicalRouteEligible destination route)
    (hDominates :
      ∀ competitor,
        competitor ∈ routes →
          CanonicalRouteEligible destination competitor →
            competitor.candidate.support ≤ route.candidate.support) :
    ∃ winner,
      canonicalBestRoute destination routes = some winner ∧
        winner.candidate.support = route.candidate.support := by
  have hSome :
      ∃ winner, canonicalBestRoute destination routes = some winner := by
    unfold canonicalBestRoute
    have hEligibleMem : route ∈ canonicalEligibleRoutes destination routes := by
      unfold canonicalEligibleRoutes
      exact List.mem_filterMap.2 ⟨route, hMem, by simp [eligibleCanonicalRoute, hEligible]⟩
    cases hRoutes : canonicalEligibleRoutes destination routes with
    | nil =>
        rw [hRoutes] at hEligibleMem
        simp at hEligibleMem
    | cons head tail =>
        exact ⟨tail.foldl chooseCanonicalRouteBySupport head, by simp⟩
  rcases hSome with ⟨winner, hWinner⟩
  have hBest : CanonicalSupportBest destination routes winner :=
    canonicalBestRoute_some_is_support_best destination routes winner hWinner
  have hWinnerLe :
      winner.candidate.support ≤ route.candidate.support :=
    hDominates winner hBest.2.1 hBest.1
  have hRouteLe :
      route.candidate.support ≤ winner.candidate.support :=
    hBest.2.2 route hMem hEligible
  exact ⟨winner, hWinner, Nat.le_antisymm hWinnerLe hRouteLe⟩

theorem canonicalBestRoute_cons_ineligible
    (destination : DestinationClass)
    (route : LifecycleRoute)
    (routes : List LifecycleRoute)
    (hIneligible : ¬ CanonicalRouteEligible destination route) :
    canonicalBestRoute destination (route :: routes) =
      canonicalBestRoute destination routes := by
  unfold canonicalBestRoute canonicalEligibleRoutes
  simp [eligibleCanonicalRoute, hIneligible]

theorem canonicalBestRoute_ignores_off_destination_route
    (destination : DestinationClass)
    (route : LifecycleRoute)
    (routes : List LifecycleRoute)
    (hDestination : route.candidate.destination ≠ destination) :
    canonicalBestRoute destination (route :: routes) =
      canonicalBestRoute destination routes := by
  apply canonicalBestRoute_cons_ineligible
  intro hEligible
  exact hDestination hEligible.2

theorem canonicalBestRoute_front_off_destination_routes_irrelevant
    (destination : DestinationClass)
    (front routes : List LifecycleRoute)
    (hFront :
      ∀ route ∈ front, route.candidate.destination ≠ destination) :
    canonicalBestRoute destination (front ++ routes) =
      canonicalBestRoute destination routes := by
  induction front with
  | nil =>
      simp
  | cons route rest ih =>
      have hRoute : route.candidate.destination ≠ destination :=
        hFront route (by simp)
      have hRest :
          ∀ competitor ∈ rest, competitor.candidate.destination ≠ destination := by
        intro competitor hMem
        exact hFront competitor (by simp [hMem])
      calc
        canonicalBestRoute destination ((route :: rest) ++ routes) =
          canonicalBestRoute destination (rest ++ routes) := by
            simpa using
              canonicalBestRoute_ignores_off_destination_route
                destination route (rest ++ routes) hRoute
        _ = canonicalBestRoute destination routes := ih hRest

theorem canonicalBestRoute_eq_some_of_unique_eligible
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ routes)
    (hEligible : CanonicalRouteEligible destination route)
    (hUnique :
      ∀ competitor,
        competitor ∈ routes →
          CanonicalRouteEligible destination competitor →
            competitor = route) :
    canonicalBestRoute destination routes = some route := by
  have hDominates :
      ∀ competitor,
        competitor ∈ routes →
          CanonicalRouteEligible destination competitor →
            competitor.candidate.support ≤ route.candidate.support := by
    intro competitor hCompetitorMem hCompetitorEligible
    have hEq := hUnique competitor hCompetitorMem hCompetitorEligible
    simp [hEq]
  rcases canonicalBestRoute_some_with_support_of_dominating_route
      destination routes route hMem hEligible hDominates with ⟨winner, hWinner, _⟩
  have hWinnerMem : winner ∈ routes :=
    canonicalBestRoute_some_mem destination routes winner hWinner
  have hWinnerEligible : CanonicalRouteEligible destination winner :=
    canonicalBestRoute_some_is_eligible destination routes winner hWinner
  have hEq : winner = route := hUnique winner hWinnerMem hWinnerEligible
  simpa [hEq] using hWinner

theorem canonical_support_at_least_of_dominating_route
    (threshold : Nat)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ routes)
    (hEligible : CanonicalRouteEligible destination route)
    (hThreshold : threshold ≤ route.candidate.support)
    (hDominates :
      ∀ competitor,
        competitor ∈ routes →
          CanonicalRouteEligible destination competitor →
            competitor.candidate.support ≤ route.candidate.support) :
    CanonicalSupportAtLeast threshold destination routes := by
  rcases canonicalBestRoute_some_with_support_of_dominating_route
      destination routes route hMem hEligible hDominates with ⟨winner, hWinner, hSupport⟩
  refine ⟨winner, hWinner, ?_⟩
  calc
    threshold ≤ route.candidate.support := hThreshold
    _ = winner.candidate.support := by simp [hSupport]

theorem not_canonical_support_at_least_of_all_eligible_below_threshold
    (threshold : Nat)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (hBelow :
      ∀ route,
        route ∈ routes →
          CanonicalRouteEligible destination route →
            route.candidate.support < threshold) :
    ¬ CanonicalSupportAtLeast threshold destination routes := by
  intro hAtLeast
  rcases hAtLeast with ⟨winner, hWinner, hThreshold⟩
  have hWinnerMem : winner ∈ routes :=
    canonicalBestRoute_some_mem destination routes winner hWinner
  have hWinnerEligible : CanonicalRouteEligible destination winner :=
    canonicalBestRoute_some_is_eligible destination routes winner hWinner
  have hLt := hBelow winner hWinnerMem hWinnerEligible
  exact Nat.not_lt_of_ge hThreshold hLt

theorem canonicalBestRoute_support_bounded_by_threshold_of_all_eligible_bounded
    (threshold : Nat)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner : canonicalBestRoute destination routes = some winner)
    (hBound :
      ∀ route,
        route ∈ routes →
          CanonicalRouteEligible destination route →
            route.candidate.support ≤ threshold) :
    winner.candidate.support ≤ threshold := by
  exact hBound winner
    (canonicalBestRoute_some_mem destination routes winner hWinner)
    (canonicalBestRoute_some_is_eligible destination routes winner hWinner)

theorem threshold_one_discontinuity_example :
    ∃ low high,
      canonicalBestRoute .corridorA [low] = some low ∧
        canonicalBestRoute .corridorA [high] = some high ∧
        ¬ CanonicalSupportAtLeast 1 .corridorA [low] ∧
        CanonicalSupportAtLeast 1 .corridorA [high] := by
  let low : LifecycleRoute :=
    { candidate :=
        { publisher := .alpha
          destination := .corridorA
          shape := CorridorShape.corridorEnvelope
          support := 0
          hopLower := 1
          hopUpper := 1 }
      status := .installed }
  let high : LifecycleRoute :=
    { candidate :=
        { publisher := .alpha
          destination := .corridorA
          shape := CorridorShape.corridorEnvelope
          support := 1
          hopLower := 1
          hopUpper := 1 }
      status := .installed }
  refine ⟨low, high, ?_, ?_, ?_, ?_⟩
  · simp [canonicalBestRoute, canonicalEligibleRoutes, eligibleCanonicalRoute,
      CanonicalRouteEligible, low]
  · simp [canonicalBestRoute, canonicalEligibleRoutes, eligibleCanonicalRoute,
      CanonicalRouteEligible, high]
  · apply not_canonical_support_at_least_of_all_eligible_below_threshold 1 .corridorA [low]
    intro route hMem hEligible
    simp [low] at hMem
    subst hMem
    simp
  · refine ⟨high, ?_, by decide⟩
    simp [canonicalBestRoute, canonicalEligibleRoutes, eligibleCanonicalRoute,
      CanonicalRouteEligible, high]

theorem canonicalBestRoute_eq_none_of_no_eligible
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (hNoEligible : ∀ route ∈ routes, ¬ CanonicalRouteEligible destination route) :
    canonicalBestRoute destination routes = none := by
  unfold canonicalBestRoute
  have hEligibleNil : canonicalEligibleRoutes destination routes = [] := by
    unfold canonicalEligibleRoutes
    induction routes with
    | nil =>
        simp
    | cons route rest ih =>
        have hRoute : ¬ CanonicalRouteEligible destination route :=
          hNoEligible route (by simp)
        have hRest :
            ∀ route' ∈ rest, ¬ CanonicalRouteEligible destination route' := by
          intro route' hMem
          exact hNoEligible route' (by simp [hMem])
        simp [eligibleCanonicalRoute, hRoute, ih hRest]
  simp [hEligibleNil]

theorem canonicalBestRoute_eq_none_of_no_destination_match
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (hNoDestination : ∀ route ∈ routes, route.candidate.destination ≠ destination) :
    canonicalBestRoute destination routes = none := by
  apply canonicalBestRoute_eq_none_of_no_eligible
  intro route hRouteMem hEligible
  exact hNoDestination route hRouteMem hEligible.2

theorem canonicalBestRoute_eq_none_of_no_active_destination_match
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (hNoActive :
      ∀ route ∈ routes,
        (route.status ≠ .installed ∧ route.status ≠ .refreshed) ∨
          route.candidate.destination ≠ destination) :
    canonicalBestRoute destination routes = none := by
  apply canonicalBestRoute_eq_none_of_no_eligible
  intro route hRouteMem hEligible
  rcases hNoActive route hRouteMem with hInactive | hDestination
  · rcases hEligible with ⟨hStatus, _⟩
    rcases hStatus with hInstalled | hRefreshed
    · exact hInactive.1 hInstalled
    · exact hInactive.2 hRefreshed
  · exact hDestination hEligible.2

end FieldRouterCanonical
