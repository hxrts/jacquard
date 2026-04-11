import Field.Adequacy.Projection
import Field.Router.Cost

/-! # Adequacy.Cost — projection preserves canonical search cost metrics -/

/-
Prove that projecting runtime artifacts down to the adequacy layer preserves the canonical
search work-unit and space-bound metrics defined in Router.Cost.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyCost

open FieldAdequacyAPI
open FieldAdequacyProjection
open FieldRouterCanonical
open FieldRouterCost
open FieldSystemEndToEnd

/-! ## Cost Projection -/

theorem projected_runtime_canonical_search_work_units_preserved
    (state : EndToEndState) :
    canonicalSearchWorkUnits
        (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) =
      canonicalSearchWorkUnits (systemStep state).lifecycle := by
  rw [runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState]

theorem projected_runtime_canonical_search_input_size_preserved
    (state : EndToEndState) :
    canonicalSearchWorkUnits
        (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) =
      (systemStep state).lifecycle.length := by
  rw [projected_runtime_canonical_search_work_units_preserved]
  exact canonical_search_worst_case_cost (systemStep state).lifecycle

theorem projected_runtime_canonical_search_space_preserved
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) :
    (canonicalEligibleRoutes destination
        (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state))).length =
      (canonicalEligibleRoutes destination (systemStep state).lifecycle).length := by
  rw [runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState]

theorem projected_runtime_canonical_search_inputs_complete
    (state : EndToEndState) :
    runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state) =
      (systemStep state).lifecycle := by
  exact runtimeLifecycleRoutes_projectedRuntimeArtifactsOfState state

/-! ## Bound Preservation -/

theorem projected_runtime_reduction_complete_for_canonical_search_complexity
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) :
    runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state) =
        (systemStep state).lifecycle ∧
      canonicalSearchWorkUnits
          (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state)) =
        canonicalSearchWorkUnits (systemStep state).lifecycle ∧
      (canonicalEligibleRoutes destination
          (runtimeLifecycleRoutes (projectedRuntimeArtifactsOfState state))).length =
        (canonicalEligibleRoutes destination (systemStep state).lifecycle).length := by
  constructor
  · exact projected_runtime_canonical_search_inputs_complete state
  constructor
  · exact projected_runtime_canonical_search_work_units_preserved state
  · exact projected_runtime_canonical_search_space_preserved destination state

end FieldAdequacyCost
