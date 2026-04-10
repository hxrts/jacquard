import Field.Adequacy.Projection
import Field.Adequacy.Optimality
import Field.Adequacy.Refinement
import Field.Adequacy.Runtime
import Field.Quality.Refinement
import Field.Router.CanonicalStrong

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyFixtures

open FieldAdequacyAPI
open FieldAdequacyOptimality
open FieldAdequacyProjection
open FieldAdequacyRefinement
open FieldAdequacyRuntime
open FieldQualityAPI
open FieldQualityRefinement
open FieldRouterCanonicalStrong
open FieldRouterOptimality
open FieldRouterLifecycle
open FieldSystemCanonical
open FieldSystemEndToEnd

def fixtureLowSupportRoute : LifecycleRoute :=
  { candidate :=
      { publisher := .alpha
        destination := .corridorA
        shape := .corridorEnvelope
        support := 2
        hopLower := 1
        hopUpper := 3 }
    status := .installed }

def fixtureHighSupportRoute : LifecycleRoute :=
  { candidate :=
      { publisher := .gamma
        destination := .corridorA
        shape := .corridorEnvelope
        support := 7
        hopLower := 1
        hopUpper := 4 }
    status := .refreshed }

def fixtureNarrowTieRoute : LifecycleRoute :=
  { candidate :=
      { publisher := .beta
        destination := .corridorA
        shape := .corridorEnvelope
        support := 5
        hopLower := 2
        hopUpper := 2 }
    status := .installed }

def fixtureWideTieRoute : LifecycleRoute :=
  { candidate :=
      { publisher := .gamma
        destination := .corridorA
        shape := .corridorEnvelope
        support := 5
        hopLower := 1
        hopUpper := 6 }
    status := .installed }

def fixtureSupportArtifacts : List RuntimeRoundArtifact :=
  runtimeArtifactsOfRoutes [fixtureLowSupportRoute, fixtureHighSupportRoute]

def fixtureTieArtifacts : List RuntimeRoundArtifact :=
  runtimeArtifactsOfRoutes [fixtureWideTieRoute, fixtureNarrowTieRoute]

def fixtureRuntimeStateOfArtifacts
    (artifacts : List RuntimeRoundArtifact) : RuntimeState :=
  initialRuntimeState artifacts

def generatedFixtureArtifactsOfSystem
    (state : EndToEndState) : List RuntimeRoundArtifact :=
  projectedRuntimeArtifactsOfState state

def generatedFixtureRuntimeStateOfSystem
    (state : EndToEndState) : RuntimeState :=
  projectedRuntimeStateOfSystem state

theorem fixture_support_artifacts_choose_high_support_route :
    runtimeCanonicalRoute .corridorA fixtureSupportArtifacts = some fixtureHighSupportRoute := by
  decide

theorem fixture_support_artifacts_choose_high_support_view :
    runtimeCanonicalRouteView .corridorA fixtureSupportArtifacts =
      some (routeComparisonView fixtureHighSupportRoute) := by
  decide

theorem fixture_empty_runtime_artifacts_yield_no_canonical_route :
    runtimeCanonicalRoute .corridorA [] = none := by
  rfl

theorem fixture_stronger_router_selector_prefers_narrower_tie_route :
    canonicalBestRouteSupportThenHopThenStableTieBreak
        .corridorA [fixtureWideTieRoute, fixtureNarrowTieRoute] =
      some fixtureNarrowTieRoute := by
  decide

theorem fixture_quality_nonclaim_stable_tie_break_can_prefer_lower_support :
    ∃ left right,
      RouteComparisonInputAdmissible left right ∧
        left.support < right.support ∧
        comparisonWinner .stableTieBreak left right = .left := by
  exact stableTieBreak_can_prefer_lower_support_view

theorem fixtureRuntimeStateOfArtifacts_has_empty_completed_prefix
    (artifacts : List RuntimeRoundArtifact) :
    runtimeArtifactsOfState (fixtureRuntimeStateOfArtifacts artifacts) = [] := by
  rfl

theorem generatedFixtureArtifactsOfSystem_are_admitted
    (state : EndToEndState) :
    RuntimeExecutionAdmitted (generatedFixtureArtifactsOfSystem state) := by
  exact projectedRuntimeArtifactsOfState_admitted state

theorem generatedFixtureRuntimeStateOfSystem_projects_system
    (state : EndToEndState) :
    RuntimeStateProjectsSystemState (generatedFixtureRuntimeStateOfSystem state) state := by
  exact projectedRuntimeStateOfSystem_projects_system state

theorem generated_fixture_budgeted_canonical_route_eq_system
    (destination : FieldNetworkAPI.DestinationClass)
    (budget : Nat)
    (state : EndToEndState)
    (hBudget :
      (FieldRouterCanonical.canonicalEligibleRoutes destination (systemStep state).lifecycle).length ≤
        budget) :
    budgetedCanonicalBestRoute destination budget (generatedFixtureArtifactsOfSystem state |>.filterMap runtimeLifecycleRouteOfArtifact) =
      canonicalSystemRoute destination state := by
  simpa [generatedFixtureArtifactsOfSystem, FieldAdequacyAPI.runtimeLifecycleRoutes] using
    projected_runtime_budgeted_canonical_route_eq_canonicalSystemRoute_of_budget_covers
      destination budget state hBudget

end FieldAdequacyFixtures
