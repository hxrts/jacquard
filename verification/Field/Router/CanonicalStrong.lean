import Field.Router.Canonical

/-! # Router.CanonicalStrong — multi-criteria canonical selection with tie-breaking -/

/-
Extend canonical selection with secondary and tertiary criteria: support dominance first,
then hop-band tightness, then stable publisher-rank tie-break.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldRouterCanonicalStrong

open FieldNetworkAPI
open FieldRouterCanonical
open FieldRouterLifecycle

/-! ## Selection Criteria -/

def canonicalPublisherRank : NodeId → Nat
  | .alpha => 0
  | .beta => 1
  | .gamma => 2

def canonicalHopBandWidth
    (route : LifecycleRoute) : Nat :=
  route.candidate.hopUpper - route.candidate.hopLower

def chooseCanonicalRouteSupportThenHopThenStableTieBreak
    (current next : LifecycleRoute) : LifecycleRoute :=
  if current.candidate.support < next.candidate.support then
    next
  else if next.candidate.support < current.candidate.support then
    current
  else if canonicalHopBandWidth next < canonicalHopBandWidth current then
    next
  else if canonicalHopBandWidth current < canonicalHopBandWidth next then
    current
  else if canonicalPublisherRank current.candidate.publisher ≤
      canonicalPublisherRank next.candidate.publisher then
    current
  else
    next

def canonicalBestRouteSupportThenHopThenStableTieBreak
    (destination : DestinationClass)
    (routes : List LifecycleRoute) : Option LifecycleRoute :=
  match canonicalEligibleRoutes destination routes with
  | [] => none
  | head :: tail =>
      some (tail.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak head)

/-! ## Tie-Break -/

theorem chooseCanonicalRouteSupportThenHopThenStableTieBreak_eq_current_or_next
    (current next : LifecycleRoute) :
    chooseCanonicalRouteSupportThenHopThenStableTieBreak current next = current ∨
      chooseCanonicalRouteSupportThenHopThenStableTieBreak current next = next := by
  unfold chooseCanonicalRouteSupportThenHopThenStableTieBreak
  by_cases hSupport : current.candidate.support < next.candidate.support
  · right
    simp [hSupport]
  · by_cases hRevSupport : next.candidate.support < current.candidate.support
    · left
      simp [hSupport, hRevSupport]
    · by_cases hHop : canonicalHopBandWidth next < canonicalHopBandWidth current
      · right
        simp [hSupport, hRevSupport, hHop]
      · by_cases hRevHop : canonicalHopBandWidth current < canonicalHopBandWidth next
        · left
          simp [hSupport, hRevSupport, hHop, hRevHop]
        · by_cases hRank :
            canonicalPublisherRank current.candidate.publisher ≤
              canonicalPublisherRank next.candidate.publisher
          · left
            simp [hSupport, hRevSupport, hHop, hRevHop, hRank]
          · right
            simp [hSupport, hRevSupport, hHop, hRevHop, hRank]

theorem chooseCanonicalRouteSupportThenHopThenStableTieBreak_preserves_eligible
    (destination : DestinationClass)
    (current next : LifecycleRoute)
    (hCurrent : CanonicalRouteEligible destination current)
    (hNext : CanonicalRouteEligible destination next) :
    CanonicalRouteEligible destination
      (chooseCanonicalRouteSupportThenHopThenStableTieBreak current next) := by
  rcases chooseCanonicalRouteSupportThenHopThenStableTieBreak_eq_current_or_next current next with
    hEq | hEq
  · simpa [hEq] using hCurrent
  · simpa [hEq] using hNext

theorem fold_chooseCanonicalRouteSupportThenHopThenStableTieBreak_mem
    (current : LifecycleRoute)
    (tail : List LifecycleRoute) :
    tail.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak current ∈ current :: tail := by
  induction tail generalizing current with
  | nil =>
      simp
  | cons head rest ih =>
      simp [List.foldl]
      rcases chooseCanonicalRouteSupportThenHopThenStableTieBreak_eq_current_or_next current head with
        hCurrent | hHead
      · have hIH :
            rest.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak current = current ∨
              rest.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak current ∈ rest := by
            simpa using ih current
        rcases hIH with hEq | hMem
        · simp [hCurrent, hEq]
        · simp [hCurrent, hMem]
      · have hIH :
            rest.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak head = head ∨
              rest.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak head ∈ rest := by
            simpa using ih head
        rcases hIH with hEq | hMem
        · simp [hHead, hEq]
        · simp [hHead, hMem]

theorem canonicalBestRouteSupportThenHopThenStableTieBreak_some_mem
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner :
      canonicalBestRouteSupportThenHopThenStableTieBreak destination routes = some winner) :
    winner ∈ routes := by
  unfold canonicalBestRouteSupportThenHopThenStableTieBreak at hWinner
  cases hEligible : canonicalEligibleRoutes destination routes with
  | nil =>
      simp [hEligible] at hWinner
  | cons head tail =>
      simp [hEligible] at hWinner
      subst hWinner
      have hMemEligible :
          tail.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak head ∈ head :: tail :=
        fold_chooseCanonicalRouteSupportThenHopThenStableTieBreak_mem head tail
      rw [← hEligible] at hMemEligible
      exact canonicalEligibleRoutes_mem_implies_from_routes destination routes _ hMemEligible

theorem canonicalBestRouteSupportThenHopThenStableTieBreak_some_is_eligible
    (destination : DestinationClass)
    (routes : List LifecycleRoute)
    (winner : LifecycleRoute)
    (hWinner :
      canonicalBestRouteSupportThenHopThenStableTieBreak destination routes = some winner) :
    CanonicalRouteEligible destination winner := by
  unfold canonicalBestRouteSupportThenHopThenStableTieBreak at hWinner
  cases hEligible : canonicalEligibleRoutes destination routes with
  | nil =>
      simp [hEligible] at hWinner
  | cons head tail =>
      simp [hEligible] at hWinner
      subst hWinner
      have hMemEligible :
          tail.foldl chooseCanonicalRouteSupportThenHopThenStableTieBreak head ∈ head :: tail :=
        fold_chooseCanonicalRouteSupportThenHopThenStableTieBreak_mem head tail
      rw [← hEligible] at hMemEligible
      exact canonicalEligibleRoutes_mem_implies_eligible destination routes _ hMemEligible

end FieldRouterCanonicalStrong
