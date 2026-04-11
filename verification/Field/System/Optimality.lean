import Field.Quality.Refinement
import Field.Router.Optimality
import Field.System.Canonical

/-! # System.Optimality — system budgeted selection with deadline safety and anytime monotonicity -/

/-
Lift budgeted canonical selection to system level and prove deadline-safety (budget is never
exceeded) and anytime monotonicity (better candidates are never discarded within budget).
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemOptimality

open FieldModelAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualityRefinement
open FieldQualityReference
open FieldQualitySystem
open FieldRouterCanonical
open FieldRouterCost
open FieldRouterOptimality
open FieldRouterLifecycle
open FieldSystemCanonical
open FieldSystemEndToEnd

/- Mechanical lift note:

The budgeted selectors in this file are mostly router optimality objects run on
`(systemStep state).lifecycle`. The genuinely new content is where those lifts
are tied back to system/reference views or budget coverage on the stepped
system state. -/

/-! ## Budgeted System Selection -/

def budgetedCanonicalSystemRoute
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState) : Option LifecycleRoute :=
  budgetedCanonicalBestRoute destination budget (systemStep state).lifecycle

def canonicalSystemSupportOptimumValue
    (destination : DestinationClass)
    (state : EndToEndState) : Nat :=
  canonicalSupportOptimumValue destination (systemStep state).lifecycle

def budgetedCanonicalSystemSupportValue
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState) : Nat :=
  budgetedCanonicalSupportValue destination budget (systemStep state).lifecycle

def budgetedCanonicalSystemSupportRegret
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState) : Nat :=
  budgetedCanonicalSupportRegret destination budget (systemStep state).lifecycle

theorem budgetedCanonicalSystemRoute_eq_canonicalSystemRoute_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState)
    (hBudget :
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget) :
    budgetedCanonicalSystemRoute destination budget state =
      canonicalSystemRoute destination state := by
  exact
    budgetedCanonicalBestRoute_eq_canonicalBestRoute_of_budget_covers
      destination budget (systemStep state).lifecycle hBudget

/-! ## Anytime Monotonicity -/

theorem budgetedCanonicalSystemRoute_anytime_monotone
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (state : EndToEndState)
    (winner₁ winner₂ : LifecycleRoute)
    (hBudget : budget₁ ≤ budget₂)
    (hSmall : budgetedCanonicalSystemRoute destination budget₁ state = some winner₁)
    (hLarge : budgetedCanonicalSystemRoute destination budget₂ state = some winner₂) :
    winner₁.candidate.support ≤ winner₂.candidate.support := by
  exact
    budgetedCanonicalBestRoute_anytime_monotone
      destination budget₁ budget₂ (systemStep state).lifecycle
      winner₁ winner₂ hBudget hSmall hLarge

/-! ## Deadline Safety -/

theorem budgetedCanonicalSystemRoute_deadline_safe_against_full_optimum
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState)
    (budgetWinner fullWinner : LifecycleRoute)
    (hBudgetWinner : budgetedCanonicalSystemRoute destination budget state = some budgetWinner)
    (hFullWinner : canonicalSystemRoute destination state = some fullWinner) :
    budgetWinner.candidate.support ≤ fullWinner.candidate.support := by
  exact
    budgetedCanonicalBestRoute_deadline_safe_against_full_optimum
      destination budget (systemStep state).lifecycle
      budgetWinner fullWinner hBudgetWinner hFullWinner

theorem budgetedCanonicalSystemSupportRegret_bounded
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState) :
    budgetedCanonicalSystemSupportRegret destination budget state ≤
      canonicalSystemSupportOptimumValue destination state := by
  exact
    budgetedCanonicalSupportRegret_bounded
      destination budget (systemStep state).lifecycle

theorem budgetedCanonicalSystemSupportRegret_eq_zero_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState)
    (hBudget :
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget) :
    budgetedCanonicalSystemSupportRegret destination budget state = 0 := by
  exact
    budgetedCanonicalSupportRegret_eq_zero_of_budget_covers
      destination budget (systemStep state).lifecycle hBudget

theorem canonicalSystemRouteView_supportDominance_is_sufficient_statistic
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRouteView destination state =
      referenceBestSystemRouteView destination state := by
  calc
    canonicalSystemRouteView destination state =
      bestSystemRouteView .supportDominance destination state := by
        symm
        exact bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView destination state
    _ = referenceBestSystemRouteView destination state := by
        exact bestSystemRouteView_supportDominance_eq_referenceBestSystemRouteView
          destination state

theorem supportDominance_reduction_preserves_dominance
    (destination : DestinationClass)
    (current next : LifecycleRoute)
    (hCurrent : CanonicalRouteEligible destination current)
    (hNext : CanonicalRouteEligible destination next)
    (hDominates : next.candidate.support ≤ current.candidate.support) :
    choosePreferredView .supportDominance
        (routeComparisonView current)
        (routeComparisonView next) =
      routeComparisonView current := by
  have hCurrentRef :
      ReferenceViewAdmissible destination (routeComparisonView current) :=
    (canonicalRouteEligible_iff_routeComparisonView_eligible destination current).mp hCurrent
  have hNextRef :
      ReferenceViewAdmissible destination (routeComparisonView next) :=
    (canonicalRouteEligible_iff_routeComparisonView_eligible destination next).mp hNext
  rw [choosePreferredView_supportDominance_eq_chooseHigherSupport
    destination (routeComparisonView current) (routeComparisonView next) hCurrentRef hNextRef]
  unfold chooseHigherSupport
  have hNotLt : ¬ current.candidate.support < next.candidate.support :=
    Nat.not_lt_of_ge hDominates
  simp [routeComparisonView, hNotLt]

theorem supportDominance_reduction_has_no_rank_inversion
    (destination : DestinationClass)
    (current next : LifecycleRoute)
    (hCurrent : CanonicalRouteEligible destination current)
    (hNext : CanonicalRouteEligible destination next)
    (hLt : current.candidate.support < next.candidate.support) :
    choosePreferredView .supportDominance
        (routeComparisonView current)
        (routeComparisonView next) =
      routeComparisonView next := by
  have hCurrentRef :
      ReferenceViewAdmissible destination (routeComparisonView current) :=
    (canonicalRouteEligible_iff_routeComparisonView_eligible destination current).mp hCurrent
  have hNextRef :
      ReferenceViewAdmissible destination (routeComparisonView next) :=
    (canonicalRouteEligible_iff_routeComparisonView_eligible destination next).mp hNext
  rw [choosePreferredView_supportDominance_eq_chooseHigherSupport
    destination (routeComparisonView current) (routeComparisonView next) hCurrentRef hNextRef]
  unfold chooseHigherSupport
  simp [routeComparisonView, hLt]

theorem supportDominance_reduction_zero_regret
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRouteView destination state =
      referenceBestSystemRouteView destination state := by
  exact canonicalSystemRouteView_supportDominance_is_sufficient_statistic destination state

theorem budgetedCanonicalSystemRoute_threshold_optimality_region
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState) :
    ((canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget ∧
        budgetedCanonicalSystemSupportRegret destination budget state = 0)
      ∨
      budgetedCanonicalSystemSupportRegret destination budget state ≤
        canonicalSystemSupportOptimumValue destination state := by
  exact
    budgetedCanonicalBestRoute_threshold_optimality_region
      destination budget (systemStep state).lifecycle

theorem budgetedCanonicalSystemParetoFrontier
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (state : EndToEndState)
    (hBudget : budget₁ ≤ budget₂) :
    canonicalSearchWorkUnits
        (budgetedCanonicalEligibleRoutes destination budget₁ (systemStep state).lifecycle) ≤
        canonicalSearchWorkUnits
          (budgetedCanonicalEligibleRoutes destination budget₂ (systemStep state).lifecycle)
      ∧
      budgetedCanonicalSystemSupportRegret destination budget₂ state ≤
        budgetedCanonicalSystemSupportRegret destination budget₁ state := by
  exact
    budgetedCanonicalPareto_frontier
      destination budget₁ budget₂ (systemStep state).lifecycle hBudget

theorem budgetedCanonicalSystemRoute_stable_after_exact_threshold
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (state : EndToEndState)
    (hBudget : budget₁ ≤ budget₂)
    (hExact :
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget₁) :
    budgetedCanonicalSystemRoute destination budget₂ state =
      canonicalSystemRoute destination state := by
  exact
    budgetedCanonicalBestRoute_stable_after_exact_threshold
      destination budget₁ budget₂ (systemStep state).lifecycle hBudget hExact

end FieldSystemOptimality
