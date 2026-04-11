import Field.Quality.Refinement
import Field.Router.Canonical
import Field.Router.Selector

/-! # System.Canonical — system-level canonical selection and equivalence with reference -/

/-
Lift the canonical route selection algorithm to system level and prove that system canonical
selection is equivalent to the reference best-view selection objective.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemCanonical

open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualityReference
open FieldQualityRefinement
open FieldQualitySystem
open FieldRouterCanonical
open FieldRouterLifecycle
open FieldRouterSelector
open FieldSystemConvergence
open FieldSystemEndToEnd

/-! ## System Canonical Selection -/

def canonicalSystemRoute
    (destination : DestinationClass)
    (state : EndToEndState) : Option LifecycleRoute :=
  canonicalBestRoute destination (systemStep state).lifecycle

def canonicalSystemRouteView
    (destination : DestinationClass)
    (state : EndToEndState) : Option RouteComparisonView :=
  Option.map routeComparisonView (canonicalSystemRoute destination state)

def CanonicalSystemSupportAtLeast
    (threshold : Nat)
    (destination : DestinationClass)
    (state : EndToEndState) : Prop :=
  CanonicalSupportAtLeast threshold destination (systemStep state).lifecycle

theorem canonicalSystemRoute_eq_selector_bestRoute
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRoute destination state =
      bestRoute canonicalSupportSelector destination (systemStep state).lifecycle := by
  simpa [canonicalSystemRoute] using
    canonicalBestRoute_eq_selector_bestRoute destination (systemStep state).lifecycle

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

/-! ## Reference Equivalence -/

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
      simp
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
      simp [hFold]

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

/-- The current support-only router-owned selector is an exact global optimum
over the full reduced lifecycle surface for that router objective. -/
theorem canonicalSystemRoute_some_is_global_support_optimum_under_full_information
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner) :
    CanonicalSupportBest destination (systemStep state).lifecycle winner := by
  exact canonicalSystemRoute_some_is_support_best destination state winner hWinner

theorem canonicalSystemRoute_eq_none_of_no_active_destination_match
    (destination : DestinationClass)
    (state : EndToEndState)
    (hNoActive :
      ∀ route ∈ (systemStep state).lifecycle,
        (route.status ≠ .installed ∧ route.status ≠ .refreshed) ∨
          route.candidate.destination ≠ destination) :
    canonicalSystemRoute destination state = none := by
  unfold canonicalSystemRoute
  exact
    canonicalBestRoute_eq_none_of_no_active_destination_match
      destination (systemStep state).lifecycle hNoActive

theorem canonicalSystemRoute_eq_some_of_unique_eligible
    (destination : DestinationClass)
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle)
    (hEligible : CanonicalRouteEligible destination route)
    (hUnique :
      ∀ competitor,
        competitor ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination competitor →
            competitor = route) :
    canonicalSystemRoute destination state = some route := by
  unfold canonicalSystemRoute
  exact
    canonicalBestRoute_eq_some_of_unique_eligible
      destination (systemStep state).lifecycle route hMem hEligible hUnique

theorem canonicalSystemSupportAtLeast_of_dominating_route
    (threshold : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle)
    (hEligible : CanonicalRouteEligible destination route)
    (hThreshold : threshold ≤ route.candidate.support)
    (hDominates :
      ∀ competitor,
        competitor ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination competitor →
            competitor.candidate.support ≤ route.candidate.support) :
    CanonicalSystemSupportAtLeast threshold destination state := by
  exact
    canonical_support_at_least_of_dominating_route
      threshold destination (systemStep state).lifecycle route
      hMem hEligible hThreshold hDominates

theorem not_canonicalSystemSupportAtLeast_of_all_eligible_below_threshold
    (threshold : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hBelow :
      ∀ route,
        route ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination route →
            route.candidate.support < threshold) :
    ¬ CanonicalSystemSupportAtLeast threshold destination state := by
  exact
    not_canonical_support_at_least_of_all_eligible_below_threshold
      threshold destination (systemStep state).lifecycle hBelow

theorem canonicalSystemRoute_support_bounded_by_threshold_of_all_eligible_bounded
    (threshold : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner)
    (hBound :
      ∀ route,
        route ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination route →
            route.candidate.support ≤ threshold) :
    winner.candidate.support ≤ threshold := by
  unfold canonicalSystemRoute at hWinner
  exact
    canonicalBestRoute_support_bounded_by_threshold_of_all_eligible_bounded
      threshold destination (systemStep state).lifecycle winner hWinner hBound

theorem canonicalSystemRoute_support_conservative
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner) :
    winner.candidate.support ≤
      (state.async.network.localStates winner.candidate.publisher destination).posterior.support := by
  have hView :
      canonicalSystemRouteView destination state = some (routeComparisonView winner) := by
    simp [canonicalSystemRouteView, hWinner]
  have hBestView :
      bestSystemRouteView .supportDominance destination state = some (routeComparisonView winner) := by
    rw [bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView]
    exact hView
  have hSupport :=
    best_system_route_view_support_conservative
      .supportDominance destination state hAssumptions hEmpty hHarmony
      (routeComparisonView winner) hBestView
  simpa [routeComparisonView] using hSupport

theorem canonicalSystemRoute_explicit_path_requires_explicit_sender_knowledge
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner)
    (hShape : winner.candidate.shape = CorridorShape.explicitPath) :
    (state.async.network.localStates winner.candidate.publisher destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  have hView :
      canonicalSystemRouteView destination state = some (routeComparisonView winner) := by
    simp [canonicalSystemRouteView, hWinner]
  have hBestView :
      bestSystemRouteView .supportDominance destination state = some (routeComparisonView winner) := by
    rw [bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView]
    exact hView
  have hKnowledge :=
    best_system_route_view_explicit_path_requires_explicit_sender_knowledge
      .supportDominance destination state hAssumptions hEmpty hHarmony
      (routeComparisonView winner) hBestView (by simpa [routeComparisonView] using hShape)
  simpa [routeComparisonView] using hKnowledge

/-- After one reduced end-to-end step under the reliable-immediate / empty-
queue regime, the current canonical support selector has already absorbed one
changed input and remains fixed on later iterates. -/
theorem canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
    (n : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRoute destination (iterateSystemStep (n + 1) state) =
      canonicalSystemRoute destination state := by
  induction n generalizing state with
  | zero =>
      simpa [iterateSystemStep] using
        canonical_system_route_stable_under_reliable_immediate_empty
          destination state hAssumptions hEmpty
  | succ n ih =>
      have hStep :=
        system_step_preserves_reliable_immediate_empty_queue state hAssumptions hEmpty
      calc
        canonicalSystemRoute destination (iterateSystemStep (Nat.succ n + 1) state)
            =
              canonicalSystemRoute destination (systemStep state) := by
                simpa [iterateSystemStep, Nat.add_assoc, Nat.add_comm, Nat.add_left_comm] using
                  ih (systemStep state) hStep.1 hStep.2
        _ = canonicalSystemRoute destination state := by
              exact canonical_system_route_stable_under_reliable_immediate_empty
                destination state hAssumptions hEmpty

/-- The reliable-immediate / empty-queue corner is the current bounded-delay
stability regime. Once a state is in that regime, later iterates cannot keep
alternating the canonical winner. -/
theorem canonical_system_route_no_oscillation_under_reliable_immediate_empty
    (n m : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRoute destination (iterateSystemStep (n + 1) state) =
      canonicalSystemRoute destination (iterateSystemStep (m + 1) state) := by
  calc
    canonicalSystemRoute destination (iterateSystemStep (n + 1) state) =
      canonicalSystemRoute destination state :=
        canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
          n destination state hAssumptions hEmpty
    _ = canonicalSystemRoute destination (iterateSystemStep (m + 1) state) := by
          symm
          exact
            canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
              m destination state hAssumptions hEmpty

/-- Current bounded convergence-time statement for canonical selection: in the
reliable-immediate / empty-queue regime, one reduced end-to-end step is enough
to reach the stable canonical winner and later iterates keep that winner. -/
theorem canonical_system_route_converges_within_one_step_under_reliable_immediate_empty
    (n : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRoute destination (iterateSystemStep (n + 1) state) =
      canonicalSystemRoute destination state := by
  exact
    canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
      n destination state hAssumptions hEmpty

theorem canonicalSystemSupportAtLeast_stable_under_reliable_immediate_empty
    (threshold : Nat)
    (n : Nat)
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    CanonicalSystemSupportAtLeast threshold destination (iterateSystemStep (n + 1) state) ↔
      CanonicalSystemSupportAtLeast threshold destination state := by
  unfold CanonicalSystemSupportAtLeast CanonicalSupportAtLeast
  constructor <;> intro hAtLeast
  · rcases hAtLeast with ⟨winner, hWinner, hThreshold⟩
    refine ⟨winner, ?_, hThreshold⟩
    calc
      canonicalSystemRoute destination state =
        canonicalSystemRoute destination (iterateSystemStep (n + 1) state) := by
          symm
          exact
            canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
              n destination state hAssumptions hEmpty
      _ = some winner := hWinner
  · rcases hAtLeast with ⟨winner, hWinner, hThreshold⟩
    refine ⟨winner, ?_, hThreshold⟩
    calc
      canonicalSystemRoute destination (iterateSystemStep (n + 1) state) =
        canonicalSystemRoute destination state :=
          canonical_system_route_recovers_within_one_step_under_reliable_immediate_empty
            n destination state hAssumptions hEmpty
      _ = some winner := hWinner

theorem vanishing_support_limit_blocks_positive_canonical_support
    (destination : DestinationClass)
    (state : EndToEndState)
    (hBelow :
      ∀ route,
        route ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination route →
            route.candidate.support < 1) :
    ¬ CanonicalSystemSupportAtLeast 1 destination state := by
  exact
    not_canonicalSystemSupportAtLeast_of_all_eligible_below_threshold
      1 destination state hBelow

theorem canonicalSystemSupport_threshold_boundary
    (threshold : Nat)
    (destination : DestinationClass)
    (state : EndToEndState) :
    ((∃ route,
        route ∈ (systemStep state).lifecycle ∧
          CanonicalRouteEligible destination route ∧
          threshold ≤ route.candidate.support ∧
          (∀ competitor,
            competitor ∈ (systemStep state).lifecycle →
              CanonicalRouteEligible destination competitor →
                competitor.candidate.support ≤ route.candidate.support)) →
      CanonicalSystemSupportAtLeast threshold destination state)
    ∧
    ((∀ route,
        route ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination route →
            route.candidate.support < threshold) →
      ¬ CanonicalSystemSupportAtLeast threshold destination state) := by
  constructor
  · intro hEmerges
    rcases hEmerges with ⟨route, hMem, hEligible, hThreshold, hDominates⟩
    exact
      canonicalSystemSupportAtLeast_of_dominating_route
        threshold destination state route hMem hEligible hThreshold hDominates
  · intro hDisappears
    exact
      not_canonicalSystemSupportAtLeast_of_all_eligible_below_threshold
        threshold destination state hDisappears

end FieldSystemCanonical
