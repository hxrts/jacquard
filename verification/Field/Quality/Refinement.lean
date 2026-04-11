import Field.Quality.Reference
import Field.Quality.System

/-! # Quality.Refinement — support-dominance folding and system-level route view equivalences -/

/-
Prove that folding support-dominance comparison across a route set produces the same winner
as reference selection, and lift the equivalence to system-level route views.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldQualityRefinement

open FieldNetworkAPI
open FieldQualityAPI
open FieldQualityReference
open FieldQualitySystem
open FieldRouterLifecycle
open FieldSystemEndToEnd

/-! ## Folding Equivalence -/

def referenceBestSystemRouteView
    (destination : DestinationClass)
    (state : EndToEndState) : Option RouteComparisonView :=
  referenceBestRouteView destination (systemStep state).lifecycle

theorem choosePreferredView_supportDominance_eq_chooseHigherSupport
    (destination : DestinationClass)
    (current next : RouteComparisonView)
    (hCurrent : ReferenceViewAdmissible destination current)
    (hNext : ReferenceViewAdmissible destination next) :
    choosePreferredView .supportDominance current next =
      chooseHigherSupport current next := by
  rcases hCurrent with ⟨hCurrentAdm, hCurrentDest⟩
  rcases hNext with ⟨hNextAdm, hNextDest⟩
  have hInput :
      RouteComparisonInputAdmissible current next := by
    refine ⟨hCurrentAdm, hNextAdm, ?_⟩
    calc
      current.destination = destination := hCurrentDest
      _ = next.destination := hNextDest.symm
  unfold choosePreferredView compareRouteViews RouteComparison.preferredView?
  by_cases hLt : current.support < next.support
  · have hNotRev : ¬ next.support < current.support := by
      exact Nat.not_lt_of_ge (Nat.le_of_lt hLt)
    simp [comparisonWinner, hInput, hLt, hNotRev, chooseHigherSupport]
  · by_cases hRev : next.support < current.support
    · simp [comparisonWinner, hInput, hLt, hRev, chooseHigherSupport]
    · simp [comparisonWinner, hInput, hLt, hRev, chooseHigherSupport]

theorem fold_supportDominance_eq_referenceFold
    (destination : DestinationClass)
    (current : RouteComparisonView)
    (tail : List RouteComparisonView)
    (hCurrent : ReferenceViewAdmissible destination current)
    (hTail : ∀ view ∈ tail, ReferenceViewAdmissible destination view) :
    tail.foldl (choosePreferredView .supportDominance) current =
      tail.foldl chooseHigherSupport current := by
  induction tail generalizing current with
  | nil =>
      rfl
  | cons head rest ih =>
      have hHead : ReferenceViewAdmissible destination head :=
        hTail head (by simp)
      have hStep :
          choosePreferredView .supportDominance current head =
            chooseHigherSupport current head :=
        choosePreferredView_supportDominance_eq_chooseHigherSupport
          destination current head hCurrent hHead
      have hChosen :
          ReferenceViewAdmissible destination (chooseHigherSupport current head) :=
        chooseHigherSupport_preserves_reference_admissible
          destination current head hCurrent hHead
      have hRest :
          ∀ view ∈ rest, ReferenceViewAdmissible destination view := by
        intro view hMem
        exact hTail view (by simp [hMem])
      simpa [List.foldl, hStep] using
        ih (chooseHigherSupport current head) hChosen hRest

theorem bestView_supportDominance_eq_referenceBestView
    (destination : DestinationClass)
    (views : List RouteComparisonView)
    (hAll : ∀ view ∈ views, ReferenceViewAdmissible destination view) :
    bestView .supportDominance views = referenceBestView views := by
  cases views with
  | nil =>
      rfl
  | cons head tail =>
      have hHead : ReferenceViewAdmissible destination head :=
        hAll head (by simp)
      have hTail :
          ∀ view ∈ tail, ReferenceViewAdmissible destination view := by
        intro view hMem
        exact hAll view (by simp [hMem])
      simp [bestView, referenceBestView,
        fold_supportDominance_eq_referenceFold destination head tail hHead hTail]

theorem bestRouteView_supportDominance_eq_referenceBestRouteView
    (destination : DestinationClass)
    (routes : List LifecycleRoute) :
    bestRouteView .supportDominance destination routes =
      referenceBestRouteView destination routes := by
  unfold bestRouteView referenceBestRouteView
  apply bestView_supportDominance_eq_referenceBestView
  intro view hMem
  exact destinationViews_mem_implies_reference_admissible destination routes view hMem

theorem bestRouteView_supportDominance_refines_reference
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : RouteComparisonView)
    (hWinner : bestRouteView .supportDominance destination routes = some winner) :
    ReferenceSupportBestRouteView destination routes winner := by
  have hReference :
      referenceBestRouteView destination routes = some winner := by
    simpa [bestRouteView_supportDominance_eq_referenceBestRouteView destination routes] using hWinner
  exact referenceBestRouteView_some_is_reference_best destination routes winner hReference

/-! ## System-Level Equivalences -/

theorem bestSystemRouteView_supportDominance_eq_referenceBestSystemRouteView
    (destination : DestinationClass)
    (state : EndToEndState) :
    bestSystemRouteView .supportDominance destination state =
      referenceBestSystemRouteView destination state := by
  unfold bestSystemRouteView referenceBestSystemRouteView
  exact bestRouteView_supportDominance_eq_referenceBestRouteView
    destination (systemStep state).lifecycle

theorem bestSystemRouteView_supportDominance_refines_reference
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : RouteComparisonView)
    (hWinner : bestSystemRouteView .supportDominance destination state = some winner) :
    ReferenceSupportBestRouteView destination (systemStep state).lifecycle winner := by
  have hReference :
      referenceBestSystemRouteView destination state = some winner := by
    simpa [bestSystemRouteView_supportDominance_eq_referenceBestSystemRouteView destination state] using
      hWinner
  exact referenceBestRouteView_some_is_reference_best
    destination (systemStep state).lifecycle winner hReference

def stableTieBreakLowSupportView : RouteComparisonView :=
  { destination := .corridorA
    publisher := .alpha
    shape := .corridorEnvelope
    support := 2
    hopLower := 1
    hopUpper := 1
    status := .installed }

def stableTieBreakHighSupportView : RouteComparisonView :=
  { destination := .corridorA
    publisher := .gamma
    shape := .corridorEnvelope
    support := 7
    hopLower := 1
    hopUpper := 4
    status := .installed }

theorem stableTieBreak_can_prefer_lower_support_view :
    ∃ left right,
      RouteComparisonInputAdmissible left right ∧
        left.support < right.support ∧
        comparisonWinner .stableTieBreak left right = .left := by
  refine ⟨stableTieBreakLowSupportView, stableTieBreakHighSupportView, ?_⟩
  refine ⟨by decide, by decide, by decide⟩

def hopBandLowSupportView : RouteComparisonView :=
  { destination := .corridorA
    publisher := .alpha
    shape := .corridorEnvelope
    support := 3
    hopLower := 2
    hopUpper := 2
    status := .installed }

def hopBandHighSupportView : RouteComparisonView :=
  { destination := .corridorA
    publisher := .beta
    shape := .corridorEnvelope
    support := 8
    hopLower := 1
    hopUpper := 6
    status := .installed }

theorem hopBandConservativity_can_prefer_lower_support_view :
    ∃ left right,
      RouteComparisonInputAdmissible left right ∧
        left.support < right.support ∧
        comparisonWinner .hopBandConservativity left right = .left := by
  refine ⟨hopBandLowSupportView, hopBandHighSupportView, ?_⟩
  refine ⟨by decide, by decide, by decide⟩

end FieldQualityRefinement
