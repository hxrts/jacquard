import Field.Adequacy.Projection
import Field.Adequacy.Runtime
import Field.Quality.System
import Field.System.Canonical

/-
The Problem. `Field/Adequacy/Projection` proves strong theorems for synthetic
projected runtime artifact lists, but the next refinement layer should talk
about reduced runtime states and runtime steps rather than bare lists. This
file should define that runtime/system relation and package the first stuttering
refinement theorems it unlocks. The runtime states here are still semantic
reduced execution objects; the theorem consequences are packaging layered above
those objects, not a second execution semantics.

Solution Structure.
1. Define a runtime/system relation that decomposes projected runtime artifacts
   into a completed runtime prefix and a pending suffix.
2. Prove that reduced runtime steps preserve that relation against a fixed
   `EndToEndState`, giving a stuttering refinement theorem.
3. Derive quiescent runtime-state consequences for canonical outcomes and the
   first system-level safety facts.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyRefinement

open FieldAdequacyAPI
open FieldAdequacyProjection
open FieldAdequacyRuntime
open FieldAsyncAPI
open FieldModelAPI
open FieldNetworkAPI
open FieldQualityAPI
open FieldQualitySystem
open FieldSystemCanonical
open FieldSystemEndToEnd

def runtimeSystemRefinementObjectRole : FieldArchitecture.ObjectRole :=
  .theoremPackaging

/-! ## Runtime/System Relation -/

/-- Reduced runtime state projected from one end-to-end system state before any
runtime artifacts are consumed. -/
def projectedRuntimeStateOfSystem
    (state : EndToEndState) : RuntimeState :=
  initialRuntimeState (projectedRuntimeArtifactsOfState state)

/-- Runtime/system stuttering refinement relation. The completed runtime prefix
plus the remaining pending runtime suffix must match the projected runtime
artifact stream induced by the current system state. -/
def RuntimeStateProjectsSystemState
    (runtimeState : RuntimeState)
    (state : EndToEndState) : Prop :=
  runtimeArtifactsOfState runtimeState ++ runtimeState.pendingArtifacts =
    projectedRuntimeArtifactsOfState state

/-- A reduced runtime state is quiescent when all projected runtime artifacts
have been consumed into the completed execution prefix. -/
def RuntimeStateQuiescent
    (runtimeState : RuntimeState) : Prop :=
  runtimeState.pendingArtifacts = []

theorem projectedRuntimeStateOfSystem_projects_system
    (state : EndToEndState) :
    RuntimeStateProjectsSystemState (projectedRuntimeStateOfSystem state) state := by
  simp [RuntimeStateProjectsSystemState, projectedRuntimeStateOfSystem,
    initialRuntimeState, runtimeArtifactsOfState]

theorem projectedRuntimeStateOfSystem_admitted
    (state : EndToEndState) :
    RuntimeStateAdmitted (projectedRuntimeStateOfSystem state) := by
  exact
    initialRuntimeState_admitted
      (projectedRuntimeArtifactsOfState state)
      (projectedRuntimeArtifactsOfState_admitted state)

/-! ## Stuttering Refinement -/

theorem runtime_step_preserves_runtime_system_refinement
    {runtimeSource runtimeTarget : RuntimeState}
    {state : EndToEndState}
    (hRefinement : RuntimeStateProjectsSystemState runtimeSource state)
    (hStep : RuntimeStep runtimeSource runtimeTarget) :
    RuntimeStateProjectsSystemState runtimeTarget state := by
  cases hStep with
  | consume artifact pendingTail completed =>
      simpa [RuntimeStateProjectsSystemState, runtimeArtifactsOfState, List.append_assoc]
        using hRefinement

theorem runtime_step_preserves_runtime_system_refinement_admitted
    {runtimeSource runtimeTarget : RuntimeState}
    {state : EndToEndState}
    (hAdmitted : RuntimeStateAdmitted runtimeSource)
    (hRefinement : RuntimeStateProjectsSystemState runtimeSource state)
    (hStep : RuntimeStep runtimeSource runtimeTarget) :
    RuntimeStateAdmitted runtimeTarget ∧
      RuntimeStateProjectsSystemState runtimeTarget state := by
  exact
    ⟨runtime_step_preserves_state_admitted hAdmitted hStep,
      runtime_step_preserves_runtime_system_refinement hRefinement hStep⟩

/-! ## Quiescent Canonical Consequences -/

theorem quiescent_runtime_state_completed_artifacts_eq_projected
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState) :
    runtimeArtifactsOfState runtimeState = projectedRuntimeArtifactsOfState state := by
  rw [RuntimeStateQuiescent] at hQuiescent
  rw [RuntimeStateProjectsSystemState] at hRefinement
  simpa [hQuiescent] using hRefinement

theorem quiescent_runtime_state_canonical_route_eq_canonicalSystemRoute
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState) :
    runtimeCanonicalRoute destination (runtimeArtifactsOfState runtimeState) =
      canonicalSystemRoute destination state := by
  rw [quiescent_runtime_state_completed_artifacts_eq_projected runtimeState state hRefinement hQuiescent]
  exact projected_runtime_canonical_route_eq_canonicalSystemRoute destination state

theorem quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState) :
    runtimeCanonicalRouteView destination (runtimeArtifactsOfState runtimeState) =
      bestSystemRouteView .supportDominance destination state := by
  rw [quiescent_runtime_state_completed_artifacts_eq_projected runtimeState state hRefinement hQuiescent]
  exact projected_runtime_canonical_route_view_eq_bestSystemRouteView_supportDominance destination state

theorem quiescent_runtime_state_support_conservative
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (winner : RouteComparisonView)
    (hWinner :
      runtimeCanonicalRouteView destination (runtimeArtifactsOfState runtimeState) = some winner) :
    winner.support ≤
      (state.async.network.localStates winner.publisher destination).posterior.support := by
  rw [quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
    destination runtimeState state hRefinement hQuiescent] at hWinner
  exact
    best_system_route_view_support_conservative
      .supportDominance destination state hAssumptions hEmpty hHarmony winner hWinner

theorem quiescent_runtime_state_explicit_path_requires_explicit_sender_knowledge
    (destination : DestinationClass)
    (runtimeState : RuntimeState)
    (state : EndToEndState)
    (hRefinement : RuntimeStateProjectsSystemState runtimeState state)
    (hQuiescent : RuntimeStateQuiescent runtimeState)
    (hAssumptions : state.async.assumptions = reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : NetworkLocallyHarmonious state.async.network)
    (winner : RouteComparisonView)
    (hWinner :
      runtimeCanonicalRouteView destination (runtimeArtifactsOfState runtimeState) = some winner)
    (hShape : winner.shape = CorridorShape.explicitPath) :
    (state.async.network.localStates winner.publisher destination).posterior.knowledge =
      ReachabilityKnowledge.explicitPath := by
  rw [quiescent_runtime_state_route_view_eq_bestSystemRouteView_supportDominance
    destination runtimeState state hRefinement hQuiescent] at hWinner
  exact
    best_system_route_view_explicit_path_requires_explicit_sender_knowledge
      .supportDominance destination state hAssumptions hEmpty hHarmony winner hWinner hShape

end FieldAdequacyRefinement
