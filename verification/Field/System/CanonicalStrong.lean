import Field.Quality.System
import Field.Router.CanonicalStrong
import Field.Router.Selector
import Field.System.Canonical

/-! # System.CanonicalStrong — system-level multi-criteria selection stability -/

/-
Define the system-level multi-criteria canonical selector and prove it is stable (produces
the same winner across repeated calls) under reliable-immediate assumptions.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemCanonicalStrong

open FieldAsyncAPI
open FieldNetworkAPI
open FieldQualitySystem
open FieldRouterCanonical
open FieldRouterCanonicalStrong
open FieldRouterLifecycle
open FieldRouterSelector
open FieldSystemCanonical
open FieldSystemEndToEnd

/- Mechanical lift note:

The selector definitions here are router-level selectors evaluated on
`(systemStep state).lifecycle`. Only the stability theorems below add genuine
system reasoning beyond that lift. -/

/-! ## Multi-Criteria Selection -/

def canonicalSystemRouteSupportThenHopThenStableTieBreak
    (destination : DestinationClass)
    (state : EndToEndState) : Option LifecycleRoute :=
  canonicalBestRouteSupportThenHopThenStableTieBreak destination (systemStep state).lifecycle

theorem canonicalSystemRouteSupportThenHopThenStableTieBreak_eq_selector_bestRoute
    (destination : DestinationClass)
    (state : EndToEndState) :
    canonicalSystemRouteSupportThenHopThenStableTieBreak destination state =
      bestRoute canonicalStrongSelector destination (systemStep state).lifecycle := by
  simpa [canonicalSystemRouteSupportThenHopThenStableTieBreak] using
    canonicalBestRouteSupportThenHopThenStableTieBreak_eq_selector_bestRoute
      destination (systemStep state).lifecycle

/-! ## Stability -/

theorem canonical_system_route_supportThenHopThenStableTieBreak_stable_under_reliable_immediate_empty
    (destination : DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    canonicalSystemRouteSupportThenHopThenStableTieBreak destination (systemStep state) =
      canonicalSystemRouteSupportThenHopThenStableTieBreak destination state := by
  unfold canonicalSystemRouteSupportThenHopThenStableTieBreak
  rw [FieldQualitySystem.system_step_lifecycle_fixed_point_under_reliable_immediate_empty
    state hAssumptions hEmpty]

theorem canonicalSystemRouteSupportThenHopThenStableTieBreak_some_mem
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : LifecycleRoute)
    (hWinner :
      canonicalSystemRouteSupportThenHopThenStableTieBreak destination state = some winner) :
    winner ∈ (systemStep state).lifecycle := by
  exact
    canonicalBestRouteSupportThenHopThenStableTieBreak_some_mem
      destination (systemStep state).lifecycle winner hWinner

theorem canonicalSystemRouteSupportThenHopThenStableTieBreak_some_is_eligible
    (destination : DestinationClass)
    (state : EndToEndState)
    (winner : LifecycleRoute)
    (hWinner :
      canonicalSystemRouteSupportThenHopThenStableTieBreak destination state = some winner) :
    CanonicalRouteEligible destination winner := by
  exact
    canonicalBestRouteSupportThenHopThenStableTieBreak_some_is_eligible
      destination (systemStep state).lifecycle winner hWinner

end FieldSystemCanonicalStrong
