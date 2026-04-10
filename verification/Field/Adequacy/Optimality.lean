import Field.Adequacy.Cost
import Field.System.Optimality

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyOptimality

open FieldAdequacyAPI
open FieldAdequacyProjection
open FieldNetworkAPI
open FieldRouterCanonical
open FieldRouterOptimality
open FieldSystemCanonical
open FieldSystemEndToEnd
open FieldSystemOptimality

theorem projected_runtime_budgeted_canonical_route_eq_canonicalSystemRoute_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState)
    (hBudget :
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget) :
    budgetedCanonicalBestRoute destination budget
        (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) =
      canonicalSystemRoute destination state := by
  rw [runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState]
  exact
    budgetedCanonicalSystemRoute_eq_canonicalSystemRoute_of_budget_covers
      destination budget state hBudget

theorem projected_runtime_budgeted_search_zero_regret_of_budget_covers
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState)
    (hBudget :
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget) :
    budgetedCanonicalSupportRegret destination budget
        (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) = 0 := by
  rw [runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState]
  exact
    budgetedCanonicalSystemSupportRegret_eq_zero_of_budget_covers
      destination budget state hBudget

theorem projected_runtime_budgeted_search_threshold_optimality_region
    (destination : DestinationClass)
    (budget : Nat)
    (state : EndToEndState) :
    ((canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget ∧
        budgetedCanonicalSupportRegret destination budget
          (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) = 0)
      ∨
      budgetedCanonicalSupportRegret destination budget
          (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) ≤
        canonicalSupportOptimumValue destination (systemStep state).lifecycle := by
  rw [runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState]
  exact
    budgetedCanonicalSystemRoute_threshold_optimality_region
      destination budget state

theorem projected_runtime_budgeted_search_stable_after_exact_threshold
    (destination : DestinationClass)
    (budget₁ budget₂ : Nat)
    (state : EndToEndState)
    (hBudget : budget₁ ≤ budget₂)
    (hExact :
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤ budget₁) :
    budgetedCanonicalBestRoute destination budget₂
        (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) =
      canonicalSystemRoute destination state := by
  rw [runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState]
  exact
    budgetedCanonicalSystemRoute_stable_after_exact_threshold
      destination budget₁ budget₂ state hBudget hExact

end FieldAdequacyOptimality
