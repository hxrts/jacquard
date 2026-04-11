import Field.Assumptions

/-
The Problem. Higher layers need a small system-facing summary of the packaged
contract boundaries without reopening the full assumptions/theorem file. This
module should stay a thin re-export surface, not a second theorem-definition
site.

Solution Structure.
1. Group the explicit non-claims about stronger contracts.
2. Group the explicit unlock theorems for stronger contracts.
3. Keep the file thin by forwarding to `FieldAssumptions`.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldSystemBoundary

open FieldAssumptions

/-! ## Explicit Non-Claims -/

/-- The default contract does not justify strong global optimality claims. -/
theorem default_contract_does_not_claim_global_optimality_ready :
    ¬ defaultContract.optional.globalOptimalityReady :=
  FieldAssumptions.default_contract_does_not_claim_global_optimality_ready

/-- The reduced-quality contract still does not justify support-optimality refinement. -/
theorem reduced_quality_contract_does_not_claim_support_optimality_refinement_ready :
    ¬ reducedQualityContract.optional.supportOptimalityRefinementReady :=
  FieldAssumptions.reduced_quality_contract_does_not_claim_support_optimality_refinement_ready

/-- The support-refinement contract still does not justify canonical-router refinement. -/
theorem support_optimality_contract_does_not_claim_canonical_router_refinement_ready :
    ¬ supportOptimalityContract.optional.canonicalRouterRefinementReady :=
  FieldAssumptions.support_optimality_contract_does_not_claim_canonical_router_refinement_ready

/-- The canonical-router contract still does not justify runtime-canonical refinement. -/
theorem canonical_router_contract_does_not_claim_runtime_canonical_refinement_ready :
    ¬ canonicalRouterContract.optional.runtimeCanonicalRefinementReady :=
  FieldAssumptions.canonical_router_contract_does_not_claim_runtime_canonical_refinement_ready

/-- The runtime-canonical contract still does not justify projected runtime/system refinement. -/
theorem runtime_canonical_contract_does_not_claim_runtime_system_refinement_ready :
    ¬ runtimeCanonicalContract.optional.runtimeSystemRefinementReady :=
  FieldAssumptions.runtime_canonical_contract_does_not_claim_runtime_system_refinement_ready

/-- Even the stronger reduced-quality contract remains explicitly non-optimality. -/
theorem reduced_quality_contract_still_does_not_claim_global_optimality_ready :
    ¬ reducedQualityContract.optional.globalOptimalityReady :=
  FieldAssumptions.reduced_quality_contract_still_does_not_claim_global_optimality_ready

/-! ## Explicit Unlocks -/

/-- The support-refinement contract unlocks only support-optimality refinement. -/
theorem support_optimality_contract_unlocks_support_optimality_refinement :
    supportOptimalityContract.optional.supportOptimalityRefinementReady :=
  FieldAssumptions.support_optimality_contract_unlocks_support_optimality_refinement

/-- The canonical-router contract unlocks router-owned support refinement only. -/
theorem canonical_router_contract_unlocks_canonical_router_refinement :
    canonicalRouterContract.optional.canonicalRouterRefinementReady :=
  FieldAssumptions.canonical_router_contract_unlocks_canonical_router_refinement

/-- The runtime-canonical contract unlocks only runtime-to-canonical refinement. -/
theorem runtime_canonical_contract_unlocks_runtime_canonical_refinement :
    runtimeCanonicalContract.optional.runtimeCanonicalRefinementReady :=
  FieldAssumptions.runtime_canonical_contract_unlocks_runtime_canonical_refinement

/-- The runtime-system contract unlocks theorem-driven projected runtime/system refinement. -/
theorem runtime_system_contract_unlocks_runtime_system_refinement :
    runtimeSystemContract.optional.runtimeSystemRefinementReady :=
  FieldAssumptions.runtime_system_contract_unlocks_runtime_system_refinement

/-- The same runtime-system contract also unlocks the runtime-state refinement
surface layered above the older projected-artifact theorems. -/
theorem runtime_system_contract_unlocks_runtime_state_system_canonical_refinement :
    runtimeSystemContract.optional.runtimeSystemRefinementReady :=
  FieldAssumptions.runtime_system_contract_unlocks_runtime_system_refinement

/-- The runtime-system contract also unlocks support-conservativity on the
runtime-state refinement surface. -/
theorem runtime_system_contract_unlocks_runtime_state_support_safety :
    runtimeSystemContract.optional.runtimeSystemRefinementReady :=
  FieldAssumptions.runtime_system_contract_unlocks_runtime_system_refinement

/-- The runtime-system contract also unlocks no-route-from-silence statements
for quiescent runtime states projecting one system state. -/
theorem runtime_system_contract_unlocks_runtime_state_no_route_creation_from_silence :
    runtimeSystemContract.optional.runtimeSystemRefinementReady :=
  FieldAssumptions.runtime_system_contract_unlocks_runtime_system_refinement

/-- The canonical-router contract also unlocks stability of the stronger
router-owned support-then-hop selector. -/
theorem canonical_router_contract_unlocks_stronger_router_selector_stability :
    canonicalRouterContract.optional.canonicalRouterRefinementReady :=
  FieldAssumptions.canonical_router_contract_unlocks_canonical_router_refinement

/-- The runtime-system contract unlocks order-insensitivity results when two
system states induce the same effective projected runtime information. -/
theorem runtime_system_contract_unlocks_canonical_route_order_insensitivity :
    runtimeSystemContract.optional.runtimeSystemRefinementReady :=
  FieldAssumptions.runtime_system_contract_unlocks_runtime_system_refinement

/-- Broader async theorems do not silently replace the existing reliable-
immediate boundary used by the fixed-point and stronger canonical theorems. -/
theorem broader_async_contract_does_not_replace_reliable_immediate_fixed_point_boundary :
    canonicalRouterContract.optional.canonicalRouterRefinementReady :=
  FieldAssumptions.canonical_router_contract_unlocks_canonical_router_refinement

/-! ## Stronger Contracts Remain Non-Optimality -/

/-- Even the support-refinement contract remains explicitly non-optimality. -/
theorem support_optimality_contract_still_does_not_claim_global_optimality_ready :
    ¬ supportOptimalityContract.optional.globalOptimalityReady :=
  FieldAssumptions.support_optimality_contract_still_does_not_claim_global_optimality_ready

/-- Even the canonical-router contract remains explicitly non-optimality. -/
theorem canonical_router_contract_still_does_not_claim_global_optimality_ready :
    ¬ canonicalRouterContract.optional.globalOptimalityReady :=
  FieldAssumptions.canonical_router_contract_still_does_not_claim_global_optimality_ready

/-- Even the runtime-canonical contract remains explicitly non-optimality. -/
theorem runtime_canonical_contract_still_does_not_claim_global_optimality_ready :
    ¬ runtimeCanonicalContract.optional.globalOptimalityReady :=
  FieldAssumptions.runtime_canonical_contract_still_does_not_claim_global_optimality_ready

/-- Even the runtime-system contract remains explicitly non-optimality. -/
theorem runtime_system_contract_still_does_not_claim_global_optimality_ready :
    ¬ runtimeSystemContract.optional.globalOptimalityReady :=
  FieldAssumptions.runtime_system_contract_still_does_not_claim_global_optimality_ready

/-- Even the strongest current runtime-system contract is still not a claim of
full Rust/runtime correctness. -/
theorem runtime_system_contract_still_does_not_claim_full_rust_runtime_correctness_ready :
    ¬ FullRustRuntimeCorrectnessReady :=
  FieldAssumptions.runtime_system_contract_still_does_not_claim_full_rust_runtime_correctness_ready

/-- The current silence/dropout theorem pack is explicitly outside the
dishonest-publication model. -/
theorem silence_dropout_theorems_do_not_extend_to_dishonest_publication :
    ∃ honest dishonest,
      FieldRouterCanonical.CanonicalRouteEligible FieldNetworkAPI.DestinationClass.corridorA honest ∧
        FieldRouterCanonical.CanonicalRouteEligible FieldNetworkAPI.DestinationClass.corridorA dishonest ∧
        honest.candidate.support < dishonest.candidate.support ∧
        FieldRouterCanonical.canonicalBestRoute
          FieldNetworkAPI.DestinationClass.corridorA [honest, dishonest] = some dishonest :=
  FieldAssumptions.silence_dropout_theorems_do_not_extend_to_dishonest_publication

end FieldSystemBoundary
