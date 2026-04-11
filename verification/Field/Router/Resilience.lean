import Field.Router.Canonical

/-! # Router.Resilience — fault budgets and route survival under bounded dropout -/

/-
Define silence-dropout and non-participation fault budgets and prove that canonical support
values survive bounded faults when the best route persists.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterResilience

open FieldNetworkAPI
open FieldRouterCanonical
open FieldRouterLifecycle

/-! ## Fault Budgets -/

inductive ParticipationFaultClass
  | silenceDropout
  | nonCooperation
  | dishonestPublication
  deriving Inhabited, Repr, DecidableEq, BEq

structure SilenceDropoutBudget where
  faultClass : ParticipationFaultClass
  droppedPublishers : List NodeId
  maxDroppedPublishers : Nat
  deriving Repr, DecidableEq, BEq

structure NonParticipationBudget where
  faultClass : ParticipationFaultClass
  inactivePublishers : List NodeId
  maxInactivePublishers : Nat
  deriving Repr, DecidableEq, BEq

def SilenceOnlyDropoutBudgetValid
    (budget : SilenceDropoutBudget) : Prop :=
  budget.faultClass = .silenceDropout ∧
    budget.droppedPublishers.length ≤ budget.maxDroppedPublishers

def NonParticipationBudgetValid
    (budget : NonParticipationBudget) : Prop :=
  budget.faultClass = .nonCooperation ∧
    budget.inactivePublishers.length ≤ budget.maxInactivePublishers

def silenceBudgetOfNonParticipation
    (budget : NonParticipationBudget) : SilenceDropoutBudget :=
  { faultClass := .silenceDropout
    droppedPublishers := budget.inactivePublishers
    maxDroppedPublishers := budget.maxInactivePublishers }

def publisherDroppedBySilence
    (publisher : NodeId) : List NodeId → Bool
  | [] => false
  | head :: tail =>
      if head = publisher then true else publisherDroppedBySilence publisher tail

def routeDroppedOutBySilence
    (budget : SilenceDropoutBudget)
    (route : LifecycleRoute) : Bool :=
  publisherDroppedBySilence route.candidate.publisher budget.droppedPublishers

def routeSurvivesSilenceDropout
    (budget : SilenceDropoutBudget)
    (route : LifecycleRoute) : Prop :=
  routeDroppedOutBySilence budget route = false

def routeBlockedByNonParticipation
    (budget : NonParticipationBudget)
    (route : LifecycleRoute) : Bool :=
  publisherDroppedBySilence route.candidate.publisher budget.inactivePublishers

def routeSurvivesNonParticipation
    (budget : NonParticipationBudget)
    (route : LifecycleRoute) : Prop :=
  routeBlockedByNonParticipation budget route = false

/-! ## Survival Theorems -/

theorem publisherDroppedBySilence_eq_true_of_mem
    (publisher : NodeId)
    (publishers : List NodeId)
    (hMem : publisher ∈ publishers) :
    publisherDroppedBySilence publisher publishers = true := by
  induction publishers with
  | nil =>
      simp at hMem
  | cons head tail ih =>
      simp at hMem
      rcases hMem with rfl | hTail
      · simp [publisherDroppedBySilence]
      · simp [publisherDroppedBySilence, ih hTail]

theorem silenceBudgetOfNonParticipation_valid
    (budget : NonParticipationBudget)
    (hValid : NonParticipationBudgetValid budget) :
    SilenceOnlyDropoutBudgetValid (silenceBudgetOfNonParticipation budget) := by
  rcases hValid with ⟨_, hLen⟩
  constructor
  · simp [silenceBudgetOfNonParticipation]
  · simpa [silenceBudgetOfNonParticipation] using hLen

theorem routeSurvivesNonParticipation_iff_survives_silence_budget
    (budget : NonParticipationBudget)
    (route : LifecycleRoute) :
    routeSurvivesNonParticipation budget route ↔
      routeSurvivesSilenceDropout (silenceBudgetOfNonParticipation budget) route := by
  rfl

def survivingRoutesAfterSilenceDropout
    (budget : SilenceDropoutBudget)
    (routes : List LifecycleRoute) : List LifecycleRoute :=
  routes.filter (fun route => !routeDroppedOutBySilence budget route)

def canonicalSupportValue
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option Nat :=
  Option.map (fun route => route.candidate.support) (canonicalBestRoute destination routes)

def dropoutCanonicalSupportValue
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option Nat :=
  canonicalSupportValue destination (survivingRoutesAfterSilenceDropout budget routes)

def survivingRoutesAfterNonParticipation
    (budget : NonParticipationBudget)
    (routes : List LifecycleRoute) : List LifecycleRoute :=
  survivingRoutesAfterSilenceDropout (silenceBudgetOfNonParticipation budget) routes

def nonParticipationCanonicalSupportValue
    (budget : NonParticipationBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option Nat :=
  canonicalSupportValue destination (survivingRoutesAfterNonParticipation budget routes)

def DropoutCanonicalSupportStable
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Prop :=
  dropoutCanonicalSupportValue budget destination routes =
    canonicalSupportValue destination routes

theorem survivingRoutesAfterSilenceDropout_mem
    (budget : SilenceDropoutBudget)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ survivingRoutesAfterSilenceDropout budget routes) :
    route ∈ routes ∧ routeSurvivesSilenceDropout budget route := by
  simpa [survivingRoutesAfterSilenceDropout, routeSurvivesSilenceDropout] using
    (List.mem_filter.1 hMem)

theorem survivingRoutesAfterSilenceDropout_mem_of_mem
    (budget : SilenceDropoutBudget)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ routes)
    (hSurvives : routeSurvivesSilenceDropout budget route) :
    route ∈ survivingRoutesAfterSilenceDropout budget routes := by
  unfold survivingRoutesAfterSilenceDropout
  apply List.mem_filter.2
  constructor
  · exact hMem
  · simpa using hSurvives

theorem survivingRoutesAfterSilenceDropout_length_bounded
    (budget : SilenceDropoutBudget)
    (routes : List LifecycleRoute) :
    (survivingRoutesAfterSilenceDropout budget routes).length ≤ routes.length := by
  unfold survivingRoutesAfterSilenceDropout
  exact List.length_filter_le _ _

theorem survivingRoutesAfterSilenceDropout_preserves_eligible
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ survivingRoutesAfterSilenceDropout budget routes)
    (hEligible : CanonicalRouteEligible destination route) :
    route ∈ routes ∧ CanonicalRouteEligible destination route := by
  exact ⟨(survivingRoutesAfterSilenceDropout_mem budget routes route hMem).1, hEligible⟩

theorem dropoutCanonicalSupportValue_some_of_surviving_support_best
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner : canonicalBestRoute destination routes = some winner)
    (hSurvives : routeSurvivesSilenceDropout budget winner) :
    dropoutCanonicalSupportValue budget destination routes = some winner.candidate.support := by
  have hWinnerMem :
      winner ∈ routes :=
    canonicalBestRoute_some_mem destination routes winner hWinner
  have hWinnerEligible :
      CanonicalRouteEligible destination winner :=
    canonicalBestRoute_some_is_eligible destination routes winner hWinner
  have hWinnerSurvivorMem :
      winner ∈ survivingRoutesAfterSilenceDropout budget routes :=
    survivingRoutesAfterSilenceDropout_mem_of_mem budget routes winner hWinnerMem hSurvives
  have hWinnerEligibleMem :
      winner ∈ canonicalEligibleRoutes destination
        (survivingRoutesAfterSilenceDropout budget routes) := by
    unfold canonicalEligibleRoutes
    exact List.mem_filterMap.2 ⟨winner, hWinnerSurvivorMem, by
      simp [eligibleCanonicalRoute, hWinnerEligible]⟩
  have hSurvivorSome :
      ∃ survivingWinner,
        canonicalBestRoute destination (survivingRoutesAfterSilenceDropout budget routes) =
          some survivingWinner := by
    unfold canonicalBestRoute
    cases hEligible :
        canonicalEligibleRoutes destination (survivingRoutesAfterSilenceDropout budget routes) with
    | nil =>
        simp [hEligible] at hWinnerEligibleMem
    | cons head tail =>
        refine ⟨tail.foldl chooseCanonicalRouteBySupport head, ?_⟩
        simp
  rcases hSurvivorSome with ⟨survivingWinner, hSurvivingWinner⟩
  have hSurvivorBest :
      CanonicalSupportBest destination
        (survivingRoutesAfterSilenceDropout budget routes) survivingWinner :=
    canonicalBestRoute_some_is_support_best destination
      (survivingRoutesAfterSilenceDropout budget routes) survivingWinner hSurvivingWinner
  have hWinnerBest :
      CanonicalSupportBest destination routes winner :=
    canonicalBestRoute_some_is_support_best destination routes winner hWinner
  have hLeLeft :
      winner.candidate.support ≤ survivingWinner.candidate.support :=
    hSurvivorBest.2.2 winner hWinnerSurvivorMem hWinnerEligible
  have hSurvivingWinnerMemRoutes :
      survivingWinner ∈ routes :=
    (survivingRoutesAfterSilenceDropout_mem
      budget routes survivingWinner
      (canonicalBestRoute_some_mem destination
        (survivingRoutesAfterSilenceDropout budget routes) survivingWinner hSurvivingWinner)).1
  have hSurvivingWinnerEligible :
      CanonicalRouteEligible destination survivingWinner :=
    canonicalBestRoute_some_is_eligible destination
      (survivingRoutesAfterSilenceDropout budget routes) survivingWinner hSurvivingWinner
  have hLeRight :
      survivingWinner.candidate.support ≤ winner.candidate.support :=
    hWinnerBest.2.2 survivingWinner hSurvivingWinnerMemRoutes hSurvivingWinnerEligible
  have hEqSupport :
      survivingWinner.candidate.support = winner.candidate.support :=
    Nat.le_antisymm hLeRight hLeLeft
  unfold dropoutCanonicalSupportValue canonicalSupportValue
  rw [hSurvivingWinner]
  simp [hEqSupport]

theorem bounded_dropout_degradation_zero_when_support_best_survives
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hValid : SilenceOnlyDropoutBudgetValid budget)
    (hWinner : canonicalBestRoute destination routes = some winner)
    (hSurvives : routeSurvivesSilenceDropout budget winner) :
    dropoutCanonicalSupportValue budget destination routes =
      canonicalSupportValue destination routes := by
  rcases hValid with ⟨_, _⟩
  unfold canonicalSupportValue
  rw [dropoutCanonicalSupportValue_some_of_surviving_support_best
    budget destination routes winner hWinner hSurvives]
  simp [hWinner]

theorem bounded_non_participation_degradation_zero_when_support_best_survives
    (budget : NonParticipationBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hValid : NonParticipationBudgetValid budget)
    (hWinner : canonicalBestRoute destination routes = some winner)
    (hSurvives : routeSurvivesNonParticipation budget winner) :
    nonParticipationCanonicalSupportValue budget destination routes =
      canonicalSupportValue destination routes := by
  unfold nonParticipationCanonicalSupportValue survivingRoutesAfterNonParticipation
  exact
    bounded_dropout_degradation_zero_when_support_best_survives
      (silenceBudgetOfNonParticipation budget) destination routes winner
      (silenceBudgetOfNonParticipation_valid budget hValid)
      hWinner
      ((routeSurvivesNonParticipation_iff_survives_silence_budget budget winner).mp hSurvives)

theorem dropoutCanonicalSupportValue_eq_none_of_all_eligible_dropped
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (hDropped :
      ∀ route ∈ routes,
        CanonicalRouteEligible destination route →
          route.candidate.publisher ∈ budget.droppedPublishers) :
    dropoutCanonicalSupportValue budget destination routes = none := by
  unfold dropoutCanonicalSupportValue canonicalSupportValue
  have hNone :
      canonicalBestRoute destination (survivingRoutesAfterSilenceDropout budget routes) = none := by
    apply canonicalBestRoute_eq_none_of_no_eligible
    intro route hMem hEligible
    have hRouteMem :
        route ∈ routes :=
      (survivingRoutesAfterSilenceDropout_mem budget routes route hMem).1
    have hSurvives :
        routeSurvivesSilenceDropout budget route :=
      (survivingRoutesAfterSilenceDropout_mem budget routes route hMem).2
    have hDroppedBool :
        routeDroppedOutBySilence budget route = true := by
      exact publisherDroppedBySilence_eq_true_of_mem route.candidate.publisher
        budget.droppedPublishers (hDropped route hRouteMem hEligible)
    simp [routeSurvivesSilenceDropout, hDroppedBool] at hSurvives
  simp [hNone]

theorem dropoutCanonicalSupportValue_some_of_surviving_eligible_route
    (budget : SilenceDropoutBudget)
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (route : LifecycleRoute)
    (hMem : route ∈ routes)
    (hEligible : CanonicalRouteEligible destination route)
    (hSurvives : routeSurvivesSilenceDropout budget route) :
    ∃ winner,
      canonicalBestRoute destination (survivingRoutesAfterSilenceDropout budget routes) =
        some winner := by
  have hRouteSurvivorMem :
      route ∈ survivingRoutesAfterSilenceDropout budget routes :=
    survivingRoutesAfterSilenceDropout_mem_of_mem budget routes route hMem hSurvives
  have hEligibleMem :
      route ∈ canonicalEligibleRoutes destination
        (survivingRoutesAfterSilenceDropout budget routes) := by
    unfold canonicalEligibleRoutes
    exact List.mem_filterMap.2 ⟨route, hRouteSurvivorMem, by simp [eligibleCanonicalRoute, hEligible]⟩
  unfold canonicalBestRoute
  cases hRoutes :
      canonicalEligibleRoutes destination (survivingRoutesAfterSilenceDropout budget routes) with
  | nil =>
      simp [hRoutes] at hEligibleMem
  | cons head tail =>
      refine ⟨tail.foldl chooseCanonicalRouteBySupport head, ?_⟩
      simp

theorem silence_dropout_nonclaim_does_not_extend_to_dishonest_publication :
    ∃ honest dishonest,
      CanonicalRouteEligible .corridorA honest ∧
        CanonicalRouteEligible .corridorA dishonest ∧
        honest.candidate.support < dishonest.candidate.support ∧
        canonicalBestRoute .corridorA [honest, dishonest] = some dishonest := by
  let honest : LifecycleRoute :=
    { candidate :=
        { publisher := .alpha
          destination := .corridorA
          shape := FieldModelAPI.CorridorShape.corridorEnvelope
          support := 5
          hopLower := 1
          hopUpper := 2 }
      status := .installed }
  let dishonest : LifecycleRoute :=
    { candidate :=
        { publisher := .beta
          destination := .corridorA
          shape := FieldModelAPI.CorridorShape.explicitPath
          support := 900
          hopLower := 1
          hopUpper := 1 }
      status := .installed }
  refine ⟨honest, dishonest, ?_, ?_, ?_, ?_⟩
  · simp [CanonicalRouteEligible, honest]
  · simp [CanonicalRouteEligible, dishonest]
  · simp [honest, dishonest]
  · decide

end FieldRouterResilience
