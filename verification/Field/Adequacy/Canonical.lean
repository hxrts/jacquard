import Field.Adequacy.Instance
import Field.System.Canonical

/-
The Problem. Downstream proofs need a low-level statement that an admitted
runtime artifact list and a system state agree on router-facing lifecycle
routes. This file should state that explicit alignment boundary and package the
two direct consequences it unlocks.

Solution Structure.
1. Define the explicit runtime/system lifecycle-alignment predicate.
2. Prove agreement of runtime canonical selection with the system selector.
3. Re-express that agreement at the route-view level used by the quality layer.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyCanonical

open FieldAdequacyAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualitySystem
open FieldSystemCanonical
open FieldSystemEndToEnd

/-! ## Alignment Predicate -/

/-- Explicit reduced alignment boundary between extracted runtime router routes
and the Lean end-to-end lifecycle view. -/
def RuntimeSystemCanonicalAligned
    (artifacts : List RuntimeRoundArtifact)
    (state : EndToEndState) : Prop :=
  runtimeLifecycleRoutes artifacts = (systemStep state).lifecycle

/-! ## Canonical Consequences -/

theorem runtime_canonical_route_eq_canonicalSystemRoute_of_alignment
    (destination : DestinationClass)
    (artifacts : List RuntimeRoundArtifact)
    (state : EndToEndState)
    (hAlign : RuntimeSystemCanonicalAligned artifacts state) :
    runtimeCanonicalRoute destination artifacts =
      canonicalSystemRoute destination state := by
  -- Reduce both sides to canonical selection over the same lifecycle list and
  -- transport the result across the explicit alignment hypothesis.
  exact
    congrArg (FieldRouterCanonical.canonicalBestRoute destination)
      (show runtimeLifecycleRoutes artifacts = (systemStep state).lifecycle from hAlign)

theorem runtime_canonical_route_view_eq_bestSystemRouteView_supportDominance_of_alignment
    (destination : DestinationClass)
    (artifacts : List RuntimeRoundArtifact)
    (state : EndToEndState)
    (hAlign : RuntimeSystemCanonicalAligned artifacts state) :
    runtimeCanonicalRouteView destination artifacts =
      bestSystemRouteView .supportDominance destination state := by
  -- First align the lifecycle-level canonical selectors, then reuse the
  -- system-side theorem equating support dominance with canonical truth.
  unfold runtimeCanonicalRouteView
  rw [runtime_canonical_route_eq_canonicalSystemRoute_of_alignment destination artifacts state hAlign]
  change canonicalSystemRouteView destination state =
    bestSystemRouteView .supportDominance destination state
  exact (bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView destination state).symm

end FieldAdequacyCanonical
