import Field.Router.Cost

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterOptimality

open FieldNetworkAPI
open FieldRouterCanonical
open FieldRouterCost
open FieldRouterLifecycle

def budgetedCanonicalEligibleRoutes
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute) : List LifecycleRoute :=
  (canonicalEligibleRoutes destination routes).take budget

def budgetedCanonicalBestRoute
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute) : Option LifecycleRoute :=
  canonicalBestRoute destination (budgetedCanonicalEligibleRoutes destination budget routes)

def canonicalSupportOptimumValue
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Nat :=
  match canonicalBestRoute destination routes with
  | some route => route.candidate.support
  | none => 0

def budgetedCanonicalSupportValue
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute) : Nat :=
  match budgetedCanonicalBestRoute destination budget routes with
  | some route => route.candidate.support
  | none => 0

def budgetedCanonicalSupportRegret
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute) : Nat :=
  canonicalSupportOptimumValue destination routes -
    budgetedCanonicalSupportValue destination budget routes

theorem canonicalEligibleRoutes_eq_self_of_all_eligible
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    (∀ route ∈ routes, CanonicalRouteEligible destination route) →
    canonicalEligibleRoutes destination routes = routes := by
  intro hAll
  induction routes with
  | nil =>
      simp [canonicalEligibleRoutes]
  | cons route rest ih =>
      have hRouteEligible : CanonicalRouteEligible destination route :=
        hAll route (by simp)
      have hRestEligible :
          ∀ next ∈ rest, CanonicalRouteEligible destination next := by
        intro next hMem
        exact hAll next (by simp [hMem])
      have hRestEq : canonicalEligibleRoutes destination rest = rest := ih hRestEligible
      unfold canonicalEligibleRoutes
      simp [eligibleCanonicalRoute, hRouteEligible]
      exact hRestEq

theorem canonicalEligibleRoutes_idempotent
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    canonicalEligibleRoutes destination (canonicalEligibleRoutes destination routes) =
      canonicalEligibleRoutes destination routes := by
  have hAll :
      ∀ route ∈ canonicalEligibleRoutes destination routes,
        CanonicalRouteEligible destination route := by
    intro route hMem
    exact canonicalEligibleRoutes_mem_implies_eligible destination routes route hMem
  exact canonicalEligibleRoutes_eq_self_of_all_eligible
    destination (canonicalEligibleRoutes destination routes) hAll

theorem budgetedCanonicalEligibleRoutes_eq_all_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute)
    (hBudget :
      (canonicalEligibleRoutes destination routes).length ≤ budget) :
    budgetedCanonicalEligibleRoutes destination budget routes =
      canonicalEligibleRoutes destination routes := by
  unfold budgetedCanonicalEligibleRoutes
  exact List.take_of_length_le hBudget

theorem budgetedCanonicalBestRoute_eq_canonicalBestRoute_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute)
    (hBudget :
      (canonicalEligibleRoutes destination routes).length ≤ budget) :
    budgetedCanonicalBestRoute destination budget routes =
      canonicalBestRoute destination routes := by
  unfold budgetedCanonicalBestRoute canonicalBestRoute
  rw [budgetedCanonicalEligibleRoutes_eq_all_of_budget_covers destination budget routes hBudget]
  rw [canonicalEligibleRoutes_idempotent]

theorem budgetedCanonicalBestRoute_some_is_support_best_within_budget
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner : budgetedCanonicalBestRoute destination budget routes = some winner) :
    CanonicalSupportBest destination
      (budgetedCanonicalEligibleRoutes destination budget routes) winner := by
  exact
    canonicalBestRoute_some_is_support_best
      destination (budgetedCanonicalEligibleRoutes destination budget routes) winner hWinner

theorem budgetedCanonicalBestRoute_deadline_safe_against_full_optimum
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute)
    (budgetWinner fullWinner : LifecycleRoute)
    (hBudgetWinner :
      budgetedCanonicalBestRoute destination budget routes = some budgetWinner)
    (hFullWinner : canonicalBestRoute destination routes = some fullWinner) :
    budgetWinner.candidate.support ≤ fullWinner.candidate.support := by
  have hBudgetMem :
      budgetWinner ∈ budgetedCanonicalEligibleRoutes destination budget routes :=
    canonicalBestRoute_some_mem
      destination (budgetedCanonicalEligibleRoutes destination budget routes)
      budgetWinner hBudgetWinner
  have hEligibleMem :
      budgetWinner ∈ canonicalEligibleRoutes destination routes :=
    List.mem_of_mem_take hBudgetMem
  have hBudgetWinnerMemRoutes :
      budgetWinner ∈ routes :=
    canonicalEligibleRoutes_mem_implies_from_routes destination routes budgetWinner hEligibleMem
  have hBudgetWinnerEligible :
      CanonicalRouteEligible destination budgetWinner :=
    canonicalEligibleRoutes_mem_implies_eligible destination routes budgetWinner hEligibleMem
  have hFullBest :
      CanonicalSupportBest destination routes fullWinner :=
    canonicalBestRoute_some_is_support_best destination routes fullWinner hFullWinner
  exact
    hFullBest.2.2 budgetWinner hBudgetWinnerMemRoutes hBudgetWinnerEligible

theorem mem_budgetedCanonicalEligibleRoutes_mono
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂)
    (hMem : route ∈ budgetedCanonicalEligibleRoutes destination budget₁ routes) :
    route ∈ budgetedCanonicalEligibleRoutes destination budget₂ routes := by
  have hMemTake :
      route ∈ (budgetedCanonicalEligibleRoutes destination budget₂ routes).take budget₁ := by
    simpa [budgetedCanonicalEligibleRoutes, List.take_take, Nat.min_eq_left hBudget] using hMem
  exact List.mem_of_mem_take hMemTake

theorem budgetedCanonicalBestRoute_anytime_monotone
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (winner₁ winner₂ : LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂)
    (hSmall :
      budgetedCanonicalBestRoute destination budget₁ routes = some winner₁)
    (hLarge :
      budgetedCanonicalBestRoute destination budget₂ routes = some winner₂) :
    winner₁.candidate.support ≤ winner₂.candidate.support := by
  let eligible := canonicalEligibleRoutes destination routes
  have hSmallMem :
      winner₁ ∈ budgetedCanonicalEligibleRoutes destination budget₁ routes :=
    canonicalBestRoute_some_mem
      destination (budgetedCanonicalEligibleRoutes destination budget₁ routes)
      winner₁ hSmall
  have hSmallMemLarge :
      winner₁ ∈ budgetedCanonicalEligibleRoutes destination budget₂ routes := by
    exact
      mem_budgetedCanonicalEligibleRoutes_mono
        destination budget₁ budget₂ routes winner₁ hBudget hSmallMem
  have hSmallMemEligible : winner₁ ∈ eligible := by
    exact List.mem_of_mem_take hSmallMemLarge
  have hSmallEligible :
      CanonicalRouteEligible destination winner₁ :=
    canonicalEligibleRoutes_mem_implies_eligible destination routes winner₁ hSmallMemEligible
  have hLargeBest :
      CanonicalSupportBest destination
        (budgetedCanonicalEligibleRoutes destination budget₂ routes) winner₂ :=
    budgetedCanonicalBestRoute_some_is_support_best_within_budget
      destination budget₂ routes winner₂ hLarge
  exact hLargeBest.2.2 winner₁ hSmallMemLarge hSmallEligible

theorem budgetedCanonicalSupportRegret_bounded
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute) :
    budgetedCanonicalSupportRegret destination budget routes ≤
      canonicalSupportOptimumValue destination routes := by
  unfold budgetedCanonicalSupportRegret
  omega

theorem budgetedCanonicalSupportRegret_eq_zero_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute)
    (hBudget :
      (canonicalEligibleRoutes destination routes).length ≤ budget) :
    budgetedCanonicalSupportRegret destination budget routes = 0 := by
  unfold budgetedCanonicalSupportRegret budgetedCanonicalSupportValue canonicalSupportOptimumValue
  rw [budgetedCanonicalBestRoute_eq_canonicalBestRoute_of_budget_covers
    destination budget routes hBudget]
  split <;> simp

theorem budgetedCanonicalSearchWorkUnits_monotone
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂) :
    canonicalSearchWorkUnits
        (budgetedCanonicalEligibleRoutes destination budget₁ routes) ≤
      canonicalSearchWorkUnits
        (budgetedCanonicalEligibleRoutes destination budget₂ routes) := by
  unfold canonicalSearchWorkUnits budgetedCanonicalEligibleRoutes
  simp [List.length_take]
  omega

theorem budgetedCanonicalBestRoute_exists_of_larger_budget
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (winner₁ : LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂)
    (hSmall :
      budgetedCanonicalBestRoute destination budget₁ routes = some winner₁) :
    ∃ winner₂,
      budgetedCanonicalBestRoute destination budget₂ routes = some winner₂ := by
  have hSmallMem :
      winner₁ ∈ budgetedCanonicalEligibleRoutes destination budget₁ routes :=
    canonicalBestRoute_some_mem
      destination (budgetedCanonicalEligibleRoutes destination budget₁ routes)
      winner₁ hSmall
  have hSmallMemLarge :
      winner₁ ∈ budgetedCanonicalEligibleRoutes destination budget₂ routes :=
    mem_budgetedCanonicalEligibleRoutes_mono
      destination budget₁ budget₂ routes winner₁ hBudget hSmallMem
  cases hLargeRoutes : budgetedCanonicalEligibleRoutes destination budget₂ routes with
  | nil =>
      simp [hLargeRoutes] at hSmallMemLarge
  | cons head tail =>
      have hAllEligible :
          ∀ route ∈ head :: tail, CanonicalRouteEligible destination route := by
        intro route hMem
        have hMemBudgeted : route ∈ budgetedCanonicalEligibleRoutes destination budget₂ routes := by
          simpa [hLargeRoutes] using hMem
        have hMemEligible : route ∈ canonicalEligibleRoutes destination routes :=
          List.mem_of_mem_take hMemBudgeted
        exact canonicalEligibleRoutes_mem_implies_eligible destination routes route hMemEligible
      refine ⟨tail.foldl chooseCanonicalRouteBySupport head, ?_⟩
      unfold budgetedCanonicalBestRoute canonicalBestRoute
      rw [hLargeRoutes]
      rw [canonicalEligibleRoutes_eq_self_of_all_eligible destination (head :: tail) hAllEligible]

theorem budgetedCanonicalSupportValue_monotone
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂) :
    budgetedCanonicalSupportValue destination budget₁ routes ≤
      budgetedCanonicalSupportValue destination budget₂ routes := by
  unfold budgetedCanonicalSupportValue
  cases hSmall : budgetedCanonicalBestRoute destination budget₁ routes with
  | none =>
      simp
  | some winner₁ =>
      rcases budgetedCanonicalBestRoute_exists_of_larger_budget
        destination budget₁ budget₂ routes winner₁ hBudget hSmall with
        ⟨winner₂, hLarge⟩
      simp [hLarge]
      exact
        budgetedCanonicalBestRoute_anytime_monotone
          destination budget₁ budget₂ routes winner₁ winner₂ hBudget hSmall hLarge

theorem budgetedCanonicalSupportRegret_monotone
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂) :
    budgetedCanonicalSupportRegret destination budget₂ routes ≤
      budgetedCanonicalSupportRegret destination budget₁ routes := by
  unfold budgetedCanonicalSupportRegret
  have hSupport :=
    budgetedCanonicalSupportValue_monotone destination budget₁ budget₂ routes hBudget
  omega

theorem budgetedCanonicalPareto_frontier
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂) :
    canonicalSearchWorkUnits
        (budgetedCanonicalEligibleRoutes destination budget₁ routes) ≤
        canonicalSearchWorkUnits
          (budgetedCanonicalEligibleRoutes destination budget₂ routes)
      ∧
      budgetedCanonicalSupportRegret destination budget₂ routes ≤
        budgetedCanonicalSupportRegret destination budget₁ routes := by
  exact
    ⟨budgetedCanonicalSearchWorkUnits_monotone destination budget₁ budget₂ routes hBudget,
      budgetedCanonicalSupportRegret_monotone destination budget₁ budget₂ routes hBudget⟩

theorem budgetedCanonicalBestRoute_stable_after_exact_threshold
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (routes : List LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂)
    (hExact :
      (canonicalEligibleRoutes destination routes).length ≤ budget₁) :
    budgetedCanonicalBestRoute destination budget₂ routes =
      canonicalBestRoute destination routes := by
  apply budgetedCanonicalBestRoute_eq_canonicalBestRoute_of_budget_covers
  omega

theorem budgetedCanonicalBestRoute_threshold_optimality_region
    (destination : DestinationClass)
    (budget : Nat)
    (routes : List LifecycleRoute) :
    ((canonicalEligibleRoutes destination routes).length ≤ budget ∧
        budgetedCanonicalSupportRegret destination budget routes = 0)
      ∨
      budgetedCanonicalSupportRegret destination budget routes ≤
        canonicalSupportOptimumValue destination routes := by
  by_cases hBudget : (canonicalEligibleRoutes destination routes).length ≤ budget
  · left
    exact ⟨hBudget,
      budgetedCanonicalSupportRegret_eq_zero_of_budget_covers destination budget routes hBudget⟩
  · right
    exact budgetedCanonicalSupportRegret_bounded destination budget routes

end FieldRouterOptimality
