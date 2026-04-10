import Field.Router.Lifecycle

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldQualityAPI

open FieldModelAPI
open FieldNetworkAPI
open FieldRouterLifecycle

inductive ComparisonObjective
  | supportDominance
  | hopBandConservativity
  | stableTieBreak
  | supportThenHopThenStableTieBreak
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ComparisonWinner
  | left
  | right
  | tie
  | inadmissible
  deriving Inhabited, Repr, DecidableEq, BEq

structure RouteComparisonView where
  destination : DestinationClass
  publisher : NodeId
  shape : CorridorShape
  support : Nat
  hopLower : Nat
  hopUpper : Nat
  status : LifecycleStatus
  deriving Repr, DecidableEq, BEq

structure RouteComparison where
  objective : ComparisonObjective
  left : RouteComparisonView
  right : RouteComparisonView
  winner : ComparisonWinner
  deriving Repr, DecidableEq, BEq

def routeComparisonView
    (route : LifecycleRoute) : RouteComparisonView :=
  { destination := route.candidate.destination
    publisher := route.candidate.publisher
    shape := route.candidate.shape
    support := route.candidate.support
    hopLower := route.candidate.hopLower
    hopUpper := route.candidate.hopUpper
    status := route.status }

def routeViewIsActive
    (view : RouteComparisonView) : Bool :=
  match view.status with
  | .installed => true
  | .refreshed => true
  | _ => false

def RouteViewAdmissible
    (view : RouteComparisonView) : Prop :=
  routeViewIsActive view = true

instance instDecidableRouteViewAdmissible
    (view : RouteComparisonView) :
    Decidable (RouteViewAdmissible view) := by
  unfold RouteViewAdmissible routeViewIsActive
  cases view.status <;> infer_instance

def RouteComparisonInputAdmissible
    (left right : RouteComparisonView) : Prop :=
  RouteViewAdmissible left ∧
    RouteViewAdmissible right ∧
    left.destination = right.destination

instance instDecidableRouteComparisonInputAdmissible
    (left right : RouteComparisonView) :
    Decidable (RouteComparisonInputAdmissible left right) := by
  unfold RouteComparisonInputAdmissible
  infer_instance

def publisherRank : NodeId → Nat
  | .alpha => 0
  | .beta => 1
  | .gamma => 2

def hopBandWidth
    (view : RouteComparisonView) : Nat :=
  view.hopUpper - view.hopLower

def comparisonWinner
    (objective : ComparisonObjective)
    (left right : RouteComparisonView) : ComparisonWinner :=
  if h : RouteComparisonInputAdmissible left right then
    match objective with
    | .supportDominance =>
        if right.support < left.support then .left
        else if left.support < right.support then .right
        else .tie
    | .hopBandConservativity =>
        if hopBandWidth left < hopBandWidth right then .left
        else if hopBandWidth right < hopBandWidth left then .right
        else .tie
    | .stableTieBreak =>
        if publisherRank left.publisher ≤ publisherRank right.publisher then .left
        else .right
    | .supportThenHopThenStableTieBreak =>
        if right.support < left.support then .left
        else if left.support < right.support then .right
        else if hopBandWidth left < hopBandWidth right then .left
        else if hopBandWidth right < hopBandWidth left then .right
        else if publisherRank left.publisher ≤ publisherRank right.publisher then .left
        else .right
  else
    .inadmissible

def compareRouteViews
    (objective : ComparisonObjective)
    (left right : RouteComparisonView) : RouteComparison :=
  { objective := objective
    left := left
    right := right
    winner := comparisonWinner objective left right }

def compareRoutes
    (objective : ComparisonObjective)
    (left right : LifecycleRoute) : RouteComparison :=
  compareRouteViews objective (routeComparisonView left) (routeComparisonView right)

def RouteComparison.preferredView?
    (comparison : RouteComparison) : Option RouteComparisonView :=
  match comparison.winner with
  | .left => some comparison.left
  | .right => some comparison.right
  | .tie => none
  | .inadmissible => none

def choosePreferredView
    (objective : ComparisonObjective)
    (current next : RouteComparisonView) : RouteComparisonView :=
  match (compareRouteViews objective current next).preferredView? with
  | some preferred => preferred
  | none => current

def destinationView
    (destination : DestinationClass)
    (route : LifecycleRoute) : Option RouteComparisonView :=
  let view := routeComparisonView route
  if h : RouteViewAdmissible view ∧ view.destination = destination then
    some view
  else
    none

def destinationViews
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : List RouteComparisonView :=
  routes.filterMap (destinationView destination)

def bestView
    (objective : ComparisonObjective)
    (views : List RouteComparisonView) : Option RouteComparisonView :=
  match views with
  | [] => none
  | head :: tail => some (tail.foldl (choosePreferredView objective) head)

def bestRouteView
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option RouteComparisonView :=
  bestView objective (destinationViews destination routes)

theorem routeViewIsActive_iff
    (view : RouteComparisonView) :
    RouteViewAdmissible view ↔
      view.status = .installed ∨ view.status = .refreshed := by
  cases view with
  | mk destination publisher shape support hopLower hopUpper status =>
      cases status <;> simp [RouteViewAdmissible, routeViewIsActive]

theorem comparison_depends_only_on_exported_fields
    (objective : ComparisonObjective)
    (left left' right right' : LifecycleRoute)
    (hLeft : routeComparisonView left = routeComparisonView left')
    (hRight : routeComparisonView right = routeComparisonView right') :
    compareRoutes objective left right =
      compareRoutes objective left' right' := by
  simp [compareRoutes, compareRouteViews, hLeft, hRight]

theorem equal_route_views_induce_equal_comparison_outcomes
    (objective : ComparisonObjective)
    (left left' right right' : LifecycleRoute)
    (hLeft : routeComparisonView left = routeComparisonView left')
    (hRight : routeComparisonView right = routeComparisonView right') :
    (compareRoutes objective left right).winner =
      (compareRoutes objective left' right').winner := by
  simp [comparison_depends_only_on_exported_fields objective left left' right right' hLeft hRight]

theorem comparison_winner_is_left_or_right_or_none
    (objective : ComparisonObjective)
    (left right preferred : RouteComparisonView)
    (hPreferred :
      (compareRouteViews objective left right).preferredView? = some preferred) :
    preferred = left ∨ preferred = right := by
  unfold compareRouteViews RouteComparison.preferredView? at hPreferred
  cases hWinner : comparisonWinner objective left right <;> simp [hWinner] at hPreferred
  · exact Or.inl hPreferred.symm
  · exact Or.inr hPreferred.symm

theorem comparison_preserves_shape_and_support
    (objective : ComparisonObjective)
    (left right preferred : RouteComparisonView)
    (hPreferred :
      (compareRouteViews objective left right).preferredView? = some preferred) :
    (preferred.shape = left.shape ∧ preferred.support = left.support) ∨
      (preferred.shape = right.shape ∧ preferred.support = right.support) := by
  rcases comparison_winner_is_left_or_right_or_none objective left right preferred hPreferred with
    hLeft | hRight
  · left
    simp [hLeft]
  · right
    simp [hRight]

theorem destinationView_some_implies_from_route
    (destination : DestinationClass)
    (route : LifecycleRoute)
    (view : RouteComparisonView)
    (hSome : destinationView destination route = some view) :
    routeComparisonView route = view := by
  by_cases hAdm :
      RouteViewAdmissible (routeComparisonView route) ∧
        (routeComparisonView route).destination = destination
  · simp [destinationView, hAdm] at hSome
    exact hSome
  · simp [destinationView, hAdm] at hSome

theorem destinationView_some_implies_admissible
    (destination : DestinationClass)
    (route : LifecycleRoute)
    (view : RouteComparisonView)
    (hSome : destinationView destination route = some view) :
    RouteViewAdmissible view ∧ view.destination = destination := by
  by_cases hAdm :
      RouteViewAdmissible (routeComparisonView route) ∧
        (routeComparisonView route).destination = destination
  · simp [destinationView, hAdm] at hSome
    subst hSome
    simpa using hAdm
  · simp [destinationView, hAdm] at hSome

theorem choosePreferredView_eq_current_or_next
    (objective : ComparisonObjective)
    (current next : RouteComparisonView) :
    choosePreferredView objective current next = current ∨
      choosePreferredView objective current next = next := by
  unfold choosePreferredView
  cases hSome : (compareRouteViews objective current next).preferredView? with
  | none =>
      simp
  | some preferred =>
      rcases comparison_winner_is_left_or_right_or_none objective current next preferred hSome with hCur | hNext
      · left
        simp [hCur]
      · right
        simp [hNext]

theorem fold_choosePreferredView_mem
    (objective : ComparisonObjective)
    (current : RouteComparisonView)
    (tail : List RouteComparisonView) :
    tail.foldl (choosePreferredView objective) current ∈ current :: tail := by
  induction tail generalizing current with
  | nil =>
      simp
  | cons head rest ih =>
      simp [List.foldl]
      rcases choosePreferredView_eq_current_or_next objective current head with hCurrent | hHead
      · have hIH :
            rest.foldl (choosePreferredView objective) current = current ∨
              rest.foldl (choosePreferredView objective) current ∈ rest := by
            simpa using ih current
        rcases hIH with hEq | hMem
        · simp [hCurrent, hEq]
        · simp [hCurrent, hMem]
      · have hIH :
            rest.foldl (choosePreferredView objective) head = head ∨
              rest.foldl (choosePreferredView objective) head ∈ rest := by
            simpa using ih head
        rcases hIH with hEq | hMem
        · simp [hHead, hEq]
        · simp [hHead, hMem]

theorem bestView_some_mem
    (objective : ComparisonObjective)
    (views : List RouteComparisonView)
    (winner : RouteComparisonView)
    (hWinner : bestView objective views = some winner) :
    winner ∈ views := by
  cases views with
  | nil =>
      simp [bestView] at hWinner
  | cons head tail =>
      simp [bestView] at hWinner
      subst hWinner
      exact fold_choosePreferredView_mem objective head tail

theorem destinationViews_mem_implies_from_route
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (view : RouteComparisonView)
    (hMem : view ∈ destinationViews destination routes) :
    ∃ route, route ∈ routes ∧ routeComparisonView route = view := by
  unfold destinationViews at hMem
  rcases List.mem_filterMap.1 hMem with ⟨route, hRouteMem, hSome⟩
  exact ⟨route, hRouteMem, destinationView_some_implies_from_route destination route view hSome⟩

theorem destinationViews_mem_implies_destination
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (view : RouteComparisonView)
    (hMem : view ∈ destinationViews destination routes) :
    view.destination = destination := by
  unfold destinationViews at hMem
  rcases List.mem_filterMap.1 hMem with ⟨route, _, hSome⟩
  exact (destinationView_some_implies_admissible destination route view hSome).2

theorem bestRouteView_some_implies_from_route
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : RouteComparisonView)
    (hWinner : bestRouteView objective destination routes = some winner) :
    ∃ route, route ∈ routes ∧ routeComparisonView route = winner := by
  unfold bestRouteView at hWinner
  have hMem : winner ∈ destinationViews destination routes :=
    bestView_some_mem objective (destinationViews destination routes) winner hWinner
  exact destinationViews_mem_implies_from_route destination routes winner hMem

theorem bestRouteView_some_has_destination
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : RouteComparisonView)
    (hWinner : bestRouteView objective destination routes = some winner) :
    winner.destination = destination := by
  unfold bestRouteView at hWinner
  have hMem : winner ∈ destinationViews destination routes :=
    bestView_some_mem objective (destinationViews destination routes) winner hWinner
  exact destinationViews_mem_implies_destination destination routes winner hMem

theorem bestRouteView_cannot_manufacture_explicit_path
    (objective : ComparisonObjective)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : RouteComparisonView)
    (hNoExplicit :
      ∀ route ∈ routes, route.candidate.shape ≠ CorridorShape.explicitPath)
    (hWinner : bestRouteView objective destination routes = some winner) :
    winner.shape ≠ CorridorShape.explicitPath := by
  rcases bestRouteView_some_implies_from_route objective destination routes winner hWinner with
    ⟨route, hRouteMem, hView⟩
  intro hShape
  have hRouteShape : route.candidate.shape = CorridorShape.explicitPath := by
    have hViewShape : (routeComparisonView route).shape = CorridorShape.explicitPath := by
      simpa [hView] using hShape
    simpa [routeComparisonView] using hViewShape
  exact hNoExplicit route hRouteMem hRouteShape

end FieldQualityAPI
