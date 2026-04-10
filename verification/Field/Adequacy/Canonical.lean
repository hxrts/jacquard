import Field.Adequacy.Instance
import Field.System.Canonical

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyCanonical

open FieldAdequacyAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualitySystem
open FieldSystemCanonical
open FieldSystemEndToEnd

/-- Explicit reduced alignment boundary between extracted runtime router routes
and the Lean end-to-end lifecycle view. -/
def RuntimeSystemCanonicalAligned
    (artifacts : List RuntimeRoundArtifact)
    (state : EndToEndState) : Prop :=
  runtimeLifecycleRoutes artifacts = (systemStep state).lifecycle

theorem runtime_canonical_route_eq_canonicalSystemRoute_of_alignment
    (destination : DestinationClass)
    (artifacts : List RuntimeRoundArtifact)
    (state : EndToEndState)
    (hAlign : RuntimeSystemCanonicalAligned artifacts state) :
    runtimeCanonicalRoute destination artifacts =
      canonicalSystemRoute destination state := by
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
  unfold runtimeCanonicalRouteView
  rw [runtime_canonical_route_eq_canonicalSystemRoute_of_alignment destination artifacts state hAlign]
  change canonicalSystemRouteView destination state =
    bestSystemRouteView .supportDominance destination state
  exact (bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView destination state).symm

end FieldAdequacyCanonical
