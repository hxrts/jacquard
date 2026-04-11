import Field.Router.Resilience
import Field.System.Canonical

/-! # System.Resilience — canonical support stabilises under bounded dropout -/

/-
Prove that system-level canonical support values stabilise when the fault count stays within
the dropout budget and the best-support route survives all faults in the budget.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemResilience

open FieldAsyncAPI
open FieldQualitySystem
open FieldRouterCanonical
open FieldRouterLifecycle
open FieldRouterResilience
open FieldSystemCanonical
open FieldSystemEndToEnd

/-- System resilience talks about composing router truth with delivery and
participation envelopes. It does not claim full transport correctness or
scheduler fairness beyond the stated assumptions. -/
def SystemTransportResilienceScope : Prop := True

/-! ## Stabilisation Under Faults -/

def dropoutCanonicalSystemSupportValue
    (budget : SilenceDropoutBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) : Option Nat :=
  dropoutCanonicalSupportValue budget destination (systemStep state).lifecycle

def nonParticipationCanonicalSystemSupportValue
    (budget : NonParticipationBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) : Option Nat :=
  nonParticipationCanonicalSupportValue budget destination (systemStep state).lifecycle

def DropoutCanonicalSystemStable
    (budget : SilenceDropoutBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) : Prop :=
  dropoutCanonicalSystemSupportValue budget destination (systemStep state) =
    dropoutCanonicalSystemSupportValue budget destination state

def NonParticipationCanonicalSystemStable
    (budget : NonParticipationBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) : Prop :=
  nonParticipationCanonicalSystemSupportValue budget destination (systemStep state) =
    nonParticipationCanonicalSystemSupportValue budget destination state

theorem dropoutCanonicalSystemSupportValue_stable_under_reliable_immediate_empty
    (budget : SilenceDropoutBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (_hBudget : SilenceOnlyDropoutBudgetValid budget)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    DropoutCanonicalSystemStable budget destination state := by
  unfold DropoutCanonicalSystemStable dropoutCanonicalSystemSupportValue
  simp [FieldQualitySystem.system_step_lifecycle_fixed_point_under_reliable_immediate_empty,
    hAssumptions, hEmpty]

theorem bounded_dropout_stabilizes_canonical_system_support_when_winner_survives
    (budget : SilenceDropoutBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hBudget : SilenceOnlyDropoutBudgetValid budget)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner)
    (hSurvives : routeSurvivesSilenceDropout budget winner) :
    dropoutCanonicalSystemSupportValue budget destination state =
      canonicalSupportValue destination (systemStep state).lifecycle := by
  unfold dropoutCanonicalSystemSupportValue
  unfold canonicalSystemRoute at hWinner
  rw [bounded_dropout_degradation_zero_when_support_best_survives
    budget destination (systemStep state).lifecycle winner hBudget hWinner hSurvives]

theorem dropoutCanonicalSystemSupportValue_eq_none_of_all_eligible_publishers_dropped
    (budget : SilenceDropoutBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hDropped :
      ∀ route ∈ (systemStep state).lifecycle,
        CanonicalRouteEligible destination route →
          route.candidate.publisher ∈ budget.droppedPublishers) :
    dropoutCanonicalSystemSupportValue budget destination state = none := by
  unfold dropoutCanonicalSystemSupportValue
  exact
    dropoutCanonicalSupportValue_eq_none_of_all_eligible_dropped
      budget destination (systemStep state).lifecycle hDropped

theorem dropoutCanonicalSystemSupportValue_eq_none_of_unique_bridge_publisher_dropped
    (budget : SilenceDropoutBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (route : LifecycleRoute)
    (hMem : route ∈ (systemStep state).lifecycle)
    (hEligible : CanonicalRouteEligible destination route)
    (hUnique :
      ∀ competitor,
        competitor ∈ (systemStep state).lifecycle →
          CanonicalRouteEligible destination competitor →
            competitor = route)
    (hDropped : route.candidate.publisher ∈ budget.droppedPublishers) :
    dropoutCanonicalSystemSupportValue budget destination state = none := by
  apply dropoutCanonicalSystemSupportValue_eq_none_of_all_eligible_publishers_dropped
  intro competitor hCompetitorMem hCompetitorEligible
  have hEq : competitor = route := hUnique competitor hCompetitorMem hCompetitorEligible
  simpa [hEq] using hDropped

theorem nonParticipationCanonicalSystemSupportValue_stable_under_reliable_immediate_empty
    (budget : NonParticipationBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (_hBudget : NonParticipationBudgetValid budget)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    NonParticipationCanonicalSystemStable budget destination state := by
  unfold NonParticipationCanonicalSystemStable nonParticipationCanonicalSystemSupportValue
  simp [FieldQualitySystem.system_step_lifecycle_fixed_point_under_reliable_immediate_empty,
    hAssumptions, hEmpty]

theorem bounded_non_participation_stabilizes_canonical_system_support_when_winner_survives
    (budget : NonParticipationBudget)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hBudget : NonParticipationBudgetValid budget)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (winner : LifecycleRoute)
    (hWinner : canonicalSystemRoute destination state = some winner)
    (hSurvives : routeSurvivesNonParticipation budget winner) :
    nonParticipationCanonicalSystemSupportValue budget destination state =
      canonicalSupportValue destination (systemStep state).lifecycle := by
  unfold nonParticipationCanonicalSystemSupportValue
  unfold canonicalSystemRoute at hWinner
  rw [bounded_non_participation_degradation_zero_when_support_best_survives
    budget destination (systemStep state).lifecycle winner hBudget hWinner hSurvives]

end FieldSystemResilience
