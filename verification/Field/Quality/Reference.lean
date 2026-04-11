import Field.Quality.API

/-! # Quality.Reference — reference highest-support route selection -/

/-
Implement the reference best-route selector that picks the highest-support admissible route
and prove its consistency with support-dominance ordering.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldQualityReference

open FieldNetworkAPI
open FieldQualityAPI

/-! ## Reference Selector -/

def ReferenceViewAdmissible
    (destination : DestinationClass)
    (view : RouteComparisonView) : Prop :=
  RouteViewAdmissible view ∧ view.destination = destination

instance instDecidableReferenceViewAdmissible
    (destination : DestinationClass)
    (view : RouteComparisonView) :
    Decidable (ReferenceViewAdmissible destination view) := by
  unfold ReferenceViewAdmissible
  infer_instance

def chooseHigherSupport
    (current next : RouteComparisonView) : RouteComparisonView :=
  if current.support < next.support then next else current

def referenceBestView
    (views : List RouteComparisonView) : Option RouteComparisonView :=
  match views with
  | [] => none
  | head :: tail => some (tail.foldl chooseHigherSupport head)

def referenceBestRouteView
    (destination : DestinationClass)
    (routes : List FieldRouterLifecycle.LifecycleRoute) : Option RouteComparisonView :=
  referenceBestView (destinationViews destination routes)

def ReferenceSupportBest
    (destination : DestinationClass)
    (views : List RouteComparisonView)
    (candidate : RouteComparisonView) : Prop :=
  ReferenceViewAdmissible destination candidate ∧
    candidate ∈ views ∧
    ∀ competitor,
      competitor ∈ views →
        ReferenceViewAdmissible destination competitor →
        competitor.support ≤ candidate.support

def ReferenceSupportBestRouteView
    (destination : DestinationClass)
    (routes : List FieldRouterLifecycle.LifecycleRoute)
    (candidate : RouteComparisonView) : Prop :=
  ReferenceSupportBest destination (destinationViews destination routes) candidate

/-! ## Consistency Proofs -/

theorem chooseHigherSupport_eq_current_or_next
    (current next : RouteComparisonView) :
    chooseHigherSupport current next = current ∨
      chooseHigherSupport current next = next := by
  unfold chooseHigherSupport
  by_cases hLt : current.support < next.support
  · right
    simp [hLt]
  · left
    simp [hLt]

theorem chooseHigherSupport_support_ge_current
    (current next : RouteComparisonView) :
    current.support ≤ (chooseHigherSupport current next).support := by
  unfold chooseHigherSupport
  by_cases hLt : current.support < next.support
  · simp [hLt, Nat.le_of_lt hLt]
  · simp [hLt]

theorem chooseHigherSupport_support_ge_next
    (current next : RouteComparisonView) :
    next.support ≤ (chooseHigherSupport current next).support := by
  unfold chooseHigherSupport
  by_cases hLt : current.support < next.support
  · simp [hLt]
  · have hLe : next.support ≤ current.support :=
      Nat.le_of_not_gt hLt
    simp [hLt, hLe]

theorem chooseHigherSupport_preserves_reference_admissible
    (destination : DestinationClass)
    (current next : RouteComparisonView)
    (hCurrent : ReferenceViewAdmissible destination current)
    (hNext : ReferenceViewAdmissible destination next) :
    ReferenceViewAdmissible destination (chooseHigherSupport current next) := by
  rcases chooseHigherSupport_eq_current_or_next current next with hEq | hEq
  · simpa [hEq] using hCurrent
  · simpa [hEq] using hNext

theorem fold_chooseHigherSupport_mem
    (current : RouteComparisonView)
    (tail : List RouteComparisonView) :
    tail.foldl chooseHigherSupport current ∈ current :: tail := by
  induction tail generalizing current with
  | nil =>
      simp
  | cons head rest ih =>
      simp [List.foldl]
      rcases chooseHigherSupport_eq_current_or_next current head with hCurrent | hHead
      · have hIH :
            rest.foldl chooseHigherSupport current = current ∨
              rest.foldl chooseHigherSupport current ∈ rest := by
            simpa using ih current
        rcases hIH with hEq | hMem
        · simp [hCurrent, hEq]
        · simp [hCurrent, hMem]
      · have hIH :
            rest.foldl chooseHigherSupport head = head ∨
              rest.foldl chooseHigherSupport head ∈ rest := by
            simpa using ih head
        rcases hIH with hEq | hMem
        · simp [hHead, hEq]
        · simp [hHead, hMem]

theorem fold_chooseHigherSupport_support_dominates
    (current competitor : RouteComparisonView)
    (tail : List RouteComparisonView)
    (hMem : competitor ∈ current :: tail) :
    competitor.support ≤ (tail.foldl chooseHigherSupport current).support := by
  induction tail generalizing current competitor with
  | nil =>
      simp at hMem
      subst hMem
      simp
  | cons head rest ih =>
      let chosen := chooseHigherSupport current head
      have hChosenCurrent :
          current.support ≤ chosen.support := by
        exact chooseHigherSupport_support_ge_current current head
      have hChosenHead :
          head.support ≤ chosen.support := by
        exact chooseHigherSupport_support_ge_next current head
      have hChosenFixed :
          chosen.support ≤ (rest.foldl chooseHigherSupport chosen).support := by
        exact ih chosen chosen (by simp)
      simp at hMem
      rcases hMem with rfl | rfl | hRest
      · exact Nat.le_trans hChosenCurrent hChosenFixed
      · exact Nat.le_trans hChosenHead hChosenFixed
      · exact ih chosen competitor (by simp [hRest])

theorem destinationViews_mem_implies_reference_admissible
    (destination : DestinationClass)
    (routes : List FieldRouterLifecycle.LifecycleRoute)
    (view : RouteComparisonView)
    (hMem : view ∈ destinationViews destination routes) :
    ReferenceViewAdmissible destination view := by
  unfold destinationViews at hMem
  rcases List.mem_filterMap.1 hMem with ⟨route, _, hSome⟩
  exact destinationView_some_implies_admissible destination route view hSome

theorem referenceBestView_some_mem
    (views : List RouteComparisonView)
    (winner : RouteComparisonView)
    (hWinner : referenceBestView views = some winner) :
    winner ∈ views := by
  cases views with
  | nil =>
      simp [referenceBestView] at hWinner
  | cons head tail =>
      simp [referenceBestView] at hWinner
      subst hWinner
      exact fold_chooseHigherSupport_mem head tail

theorem referenceBestView_some_is_reference_best
    (destination : DestinationClass)
    (views : List RouteComparisonView)
    (winner : RouteComparisonView)
    (hAll : ∀ view ∈ views, ReferenceViewAdmissible destination view)
    (hWinner : referenceBestView views = some winner) :
    ReferenceSupportBest destination views winner := by
  cases views with
  | nil =>
      simp [referenceBestView] at hWinner
  | cons head tail =>
      simp [referenceBestView] at hWinner
      subst hWinner
      have hMem :
          tail.foldl chooseHigherSupport head ∈ head :: tail :=
        fold_chooseHigherSupport_mem head tail
      have hAdm :
          ReferenceViewAdmissible destination (tail.foldl chooseHigherSupport head) :=
        hAll _ hMem
      refine ⟨hAdm, hMem, ?_⟩
      intro competitor hCompetitor _
      exact fold_chooseHigherSupport_support_dominates head competitor tail hCompetitor

theorem referenceBestRouteView_some_is_reference_best
    (destination : DestinationClass)
    (routes : List FieldRouterLifecycle.LifecycleRoute)
    (winner : RouteComparisonView)
    (hWinner : referenceBestRouteView destination routes = some winner) :
    ReferenceSupportBestRouteView destination routes winner := by
  simp [ReferenceSupportBestRouteView]
  apply referenceBestView_some_is_reference_best
  · intro view hMem
    exact destinationViews_mem_implies_reference_admissible destination routes view hMem
  · exact hWinner

end FieldQualityReference
