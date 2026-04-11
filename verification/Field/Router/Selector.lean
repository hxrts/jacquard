import Field.CostAPI
import Field.Router.Lifecycle

/-! # Router.Selector — shared selector-family abstraction for lifecycle routes -/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterSelector

open FieldCostAPI
open FieldModelAPI
open FieldNetworkAPI
open FieldRouterLifecycle

structure LifecycleRouteSelector where
  eligible : DestinationClass → LifecycleRoute → Prop
  choose : LifecycleRoute → LifecycleRoute → LifecycleRoute

inductive SelectorObjectiveSurface
  | supportDominance
  | supportThenHopThenStableTieBreak
  deriving Inhabited, Repr, DecidableEq, BEq

inductive SelectorTieBreakSurface
  | keepCurrent
  | preferLowerHopBand
  | stablePublisher
  deriving Inhabited, Repr, DecidableEq, BEq

structure LifecycleSelectorSemantics where
  selector : LifecycleRouteSelector
  objective : SelectorObjectiveSurface
  tieBreak : SelectorTieBreakSurface

inductive SearchTraversalProfile
  | exhaustive
  | budgetedPrefix
  | cached
  deriving Inhabited, Repr, DecidableEq, BEq

inductive SearchExecutionRealization
  | serial
  | batched
  | parallel
  deriving Inhabited, Repr, DecidableEq, BEq

structure SearchExecutionPolicy where
  budget : Option WorkBudget
  traversal : SearchTraversalProfile
  realization : SearchExecutionRealization
  incremental : Bool

structure SelectorSearchInterface where
  semantics : LifecycleSelectorSemantics
  policy : SearchExecutionPolicy

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

noncomputable def searchedEligibleRoutes
    (semantics : LifecycleSelectorSemantics)
    (policy : SearchExecutionPolicy)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : List LifecycleRoute :=
  let eligible := eligibleRoutes semantics.selector destination routes
  match policy.traversal, policy.budget with
  | .budgetedPrefix, some budget => eligible.take budget.units.amount
  | _, _ => eligible

noncomputable def runSelectorSearch
    (search : SelectorSearchInterface)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option LifecycleRoute :=
  match searchedEligibleRoutes search.semantics search.policy destination routes with
  | [] => none
  | head :: tail => some (tail.foldl search.semantics.selector.choose head)

def exhaustiveSerialPolicy : SearchExecutionPolicy :=
  { budget := none
    traversal := .exhaustive
    realization := .serial
    incremental := false }

def executionPolicyOfPosture : RoutingPosture → SearchExecutionPolicy
  | .opportunistic =>
      { budget := none
        traversal := .exhaustive
        realization := .serial
        incremental := false }
  | .structured =>
      { budget := some (WorkBudget.ofNat 64)
        traversal := .budgetedPrefix
        realization := .batched
        incremental := true }
  | .retentionBiased =>
      { budget := some (WorkBudget.ofNat 96)
        traversal := .cached
        realization := .batched
        incremental := true }
  | .riskSuppressed =>
      { budget := some (WorkBudget.ofNat 32)
        traversal := .budgetedPrefix
        realization := .serial
        incremental := false }

def withExecutionPolicy
    (semantics : LifecycleSelectorSemantics)
    (policy : SearchExecutionPolicy) : SelectorSearchInterface :=
  { semantics := semantics, policy := policy }

def withPosturePolicy
    (semantics : LifecycleSelectorSemantics)
    (posture : RoutingPosture) : SelectorSearchInterface :=
  withExecutionPolicy semantics (executionPolicyOfPosture posture)

theorem searchedEligibleRoutes_eq_eligibleRoutes_of_nonbudgeted
    (semantics : LifecycleSelectorSemantics)
    (policy : SearchExecutionPolicy)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (hTraversal : policy.traversal ≠ .budgetedPrefix ∨ policy.budget = none) :
    searchedEligibleRoutes semantics policy destination routes =
      eligibleRoutes semantics.selector destination routes := by
  unfold searchedEligibleRoutes eligibleRoutes
  cases hTraversal with
  | inl hNotBudgeted =>
      cases hTrace : policy.traversal <;> cases hBudget : policy.budget <;>
        simp [hTrace] at hNotBudgeted ⊢
  | inr hNoBudget =>
      cases hTrace : policy.traversal <;> cases hBudget : policy.budget <;>
        simp [hBudget] at hNoBudget ⊢

theorem withPosturePolicy_preserves_semantics
    (semantics : LifecycleSelectorSemantics)
    (posture : RoutingPosture) :
    (withPosturePolicy semantics posture).semantics = semantics := by
  rfl

end FieldRouterSelector
