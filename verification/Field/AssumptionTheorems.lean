import Field.AssumptionCore
import Field.Adequacy.Canonical
import Field.Adequacy.Instance
import Field.Adequacy.Projection
import Field.Information.Blindness
import Field.Protocol.Conservation
import Field.Quality.Refinement
import Field.System.Canonical

/-
The Problem. After the assumption vocabulary is defined, the field proof stack
needs one place that states exactly what each packaged contract unlocks. These
theorems should package lower-layer results without re-owning their logic.

Solution Structure.
1. Re-export runtime, adequacy, and boundary consequences of the packaged
   contracts.
2. Package the quality, canonical, and runtime-refinement consequences.
3. Make contract unlocks and explicit non-claims easy to review in one place.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAssumptions

open FieldAdequacyAPI
open FieldAdequacyCanonical
open FieldAdequacyInstance
open FieldAdequacyProjection
open FieldBoundary
open FieldInformationBlindness
open FieldProtocolAPI
open FieldProtocolConservation
open FieldQualityAPI
open FieldQualityReference
open FieldQualityRefinement
open FieldQualitySystem
open FieldSystemCanonical
open FieldSystemEndToEnd

/-! ## Runtime And Boundary Packaging -/

private theorem runtime_contract_respects_envelope
    (contract : ProofContract)
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : contract.runtime.admitted artifacts) :
    RuntimeExecutionAdmitted artifacts :=
  contract.runtime.respectsReducedEnvelope artifacts hAdmitted

theorem contract_yields_runtime_evidence_agreement
    (contract : ProofContract)
    (artifacts : List RuntimeRoundArtifact)
    (_hAdmitted : contract.runtime.admitted artifacts) :
    FieldAdequacyAPI.runtimeEvidence artifacts =
      controllerEvidenceFromTrace (FieldAdequacyAPI.extractTrace artifacts) := by
  exact FieldAdequacyInstance.runtime_trace_evidence_matches_protocol_trace artifacts

theorem contract_yields_observational_controller_boundary
    (contract : ProofContract)
    (trace : ProtocolTrace) :
    FieldEvidenceConservation trace := by
  exact FieldProtocolConservation.protocol_trace_evidence_conserved trace

theorem contract_yields_protocol_trace_admitted
    (contract : ProofContract)
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : contract.runtime.admitted artifacts) :
    ProtocolTraceAdmitted (FieldAdequacyAPI.extractTrace artifacts) := by
  exact
    FieldAdequacyAPI.runtime_execution_extracts_to_observational_trace
      artifacts
      (runtime_contract_respects_envelope contract artifacts hAdmitted)

/-- Packaged simulation witness obtained from the runtime assumption contract. -/
def contract_yields_runtime_trace_simulation
    (contract : ProofContract)
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : contract.runtime.admitted artifacts) :
    RuntimeTraceSimulation artifacts := by
  exact
    FieldAdequacyInstance.admitted_runtime_execution_simulates_reduced_protocol
      artifacts
      (runtime_contract_respects_envelope contract artifacts hAdmitted)

/-! ## Quality And Canonical Packaging -/

theorem contract_yields_reduced_quality_stability
    (contract : ProofContract)
    (_hReady : contract.optional.reducedQualityComparisonReady)
    (objective : ComparisonObjective)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = FieldAsyncAPI.reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = []) :
    bestSystemRouteView objective destination (systemStep state) =
      bestSystemRouteView objective destination state := by
  exact
    best_system_route_view_stable_under_reliable_immediate_empty
      objective destination state hAssumptions hEmpty

theorem contract_yields_reduced_quality_support_conservativity
    (contract : ProofContract)
    (_hReady : contract.optional.reducedQualityComparisonReady)
    (objective : ComparisonObjective)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = FieldAsyncAPI.reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : FieldNetworkAPI.NetworkLocallyHarmonious state.async.network)
    (winner : RouteComparisonView)
    (hWinner : bestSystemRouteView objective destination state = some winner) :
    winner.support ≤
      (state.async.network.localStates winner.publisher destination).posterior.support := by
  exact
    best_system_route_view_support_conservative
      objective destination state hAssumptions hEmpty hHarmony winner hWinner

theorem contract_yields_explicit_path_quality_observer
    (contract : ProofContract)
    (_hReady : contract.optional.reducedQualityComparisonReady)
    (objective : ComparisonObjective)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hAssumptions : state.async.assumptions = FieldAsyncAPI.reliableImmediateAssumptions)
    (hEmpty : state.async.inFlight = [])
    (hHarmony : FieldNetworkAPI.NetworkLocallyHarmonious state.async.network)
    (winner : RouteComparisonView)
    (hWinner : bestSystemRouteView objective destination state = some winner)
    (hShape : winner.shape = FieldModelAPI.CorridorShape.explicitPath) :
    (state.async.network.localStates winner.publisher destination).posterior.knowledge =
      FieldModelAPI.ReachabilityKnowledge.explicitPath := by
  exact
    best_system_route_view_explicit_path_requires_explicit_sender_knowledge
      objective destination state hAssumptions hEmpty hHarmony winner hWinner hShape

theorem contract_yields_support_optimality_refinement
    (contract : ProofContract)
    (_hReady : contract.optional.supportOptimalityRefinementReady)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (winner : RouteComparisonView)
    (hWinner : bestSystemRouteView .supportDominance destination state = some winner) :
    ReferenceSupportBestRouteView destination (systemStep state).lifecycle winner := by
  exact
    bestSystemRouteView_supportDominance_refines_reference
      destination state winner hWinner

theorem contract_yields_canonical_router_refinement
    (contract : ProofContract)
    (_hReady : contract.optional.canonicalRouterRefinementReady)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) :
    bestSystemRouteView .supportDominance destination state =
      canonicalSystemRouteView destination state := by
  exact bestSystemRouteView_supportDominance_eq_canonicalSystemRouteView destination state

theorem contract_yields_runtime_canonical_refinement
    (contract : ProofContract)
    (_hReady : contract.optional.runtimeCanonicalRefinementReady)
    (artifacts : List RuntimeRoundArtifact)
    (_hAdmitted : contract.runtime.admitted artifacts)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState)
    (hAligned : RuntimeSystemCanonicalAligned artifacts state) :
    runtimeCanonicalRoute destination artifacts =
      canonicalSystemRoute destination state := by
  exact
    runtime_canonical_route_eq_canonicalSystemRoute_of_alignment
      destination artifacts state hAligned

theorem contract_yields_runtime_system_canonical_refinement
    (contract : ProofContract)
    (_hReady : contract.optional.runtimeSystemRefinementReady)
    (destination : FieldNetworkAPI.DestinationClass)
    (state : EndToEndState) :
    runtimeCanonicalRoute destination (projectedRuntimeArtifactsOfState state) =
      canonicalSystemRoute destination state := by
  exact projected_runtime_canonical_route_eq_canonicalSystemRoute destination state

/-! ## Contract Unlocks And Non-Claims -/

theorem default_contract_does_not_claim_support_optimality_refinement_ready :
    ¬ defaultContract.optional.supportOptimalityRefinementReady := by
  simp [defaultContract, mkProofContract, defaultOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_does_not_claim_support_optimality_refinement_ready :
    ¬ reducedQualityContract.optional.supportOptimalityRefinementReady := by
  simp [reducedQualityContract, mkProofContract, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem default_contract_does_not_claim_canonical_router_refinement_ready :
    ¬ defaultContract.optional.canonicalRouterRefinementReady := by
  simp [defaultContract, mkProofContract, defaultOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem support_optimality_contract_does_not_claim_canonical_router_refinement_ready :
    ¬ supportOptimalityContract.optional.canonicalRouterRefinementReady := by
  simp [supportOptimalityContract, mkProofContract, supportOptimalityOptionalStrengtheningAssumptions,
    reducedQualityOptionalStrengtheningAssumptions, baseOptionalStrengtheningAssumptions]

theorem canonical_router_contract_does_not_claim_runtime_canonical_refinement_ready :
    ¬ canonicalRouterContract.optional.runtimeCanonicalRefinementReady := by
  simp [canonicalRouterContract, mkProofContract, canonicalRouterOptionalStrengtheningAssumptions,
    supportOptimalityOptionalStrengtheningAssumptions, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem runtime_canonical_contract_does_not_claim_runtime_system_refinement_ready :
    ¬ runtimeCanonicalContract.optional.runtimeSystemRefinementReady := by
  simp [runtimeCanonicalContract, mkProofContract, runtimeCanonicalOptionalStrengtheningAssumptions,
    canonicalRouterOptionalStrengtheningAssumptions, supportOptimalityOptionalStrengtheningAssumptions,
    reducedQualityOptionalStrengtheningAssumptions, baseOptionalStrengtheningAssumptions]

theorem default_contract_does_not_claim_global_optimality_ready :
    ¬ defaultContract.optional.globalOptimalityReady := by
  simp [defaultContract, mkProofContract, defaultOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_unlocks_reduced_quality_comparison :
    reducedQualityContract.optional.reducedQualityComparisonReady := by
  simp [reducedQualityContract, mkProofContract, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem support_optimality_contract_unlocks_support_optimality_refinement :
    supportOptimalityContract.optional.supportOptimalityRefinementReady := by
  simp [supportOptimalityContract, mkProofContract, supportOptimalityOptionalStrengtheningAssumptions,
    reducedQualityOptionalStrengtheningAssumptions, baseOptionalStrengtheningAssumptions]

theorem canonical_router_contract_unlocks_canonical_router_refinement :
    canonicalRouterContract.optional.canonicalRouterRefinementReady := by
  simp [canonicalRouterContract, mkProofContract, canonicalRouterOptionalStrengtheningAssumptions,
    supportOptimalityOptionalStrengtheningAssumptions, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem runtime_canonical_contract_unlocks_runtime_canonical_refinement :
    runtimeCanonicalContract.optional.runtimeCanonicalRefinementReady := by
  simp [runtimeCanonicalContract, mkProofContract, runtimeCanonicalOptionalStrengtheningAssumptions,
    canonicalRouterOptionalStrengtheningAssumptions, supportOptimalityOptionalStrengtheningAssumptions,
    reducedQualityOptionalStrengtheningAssumptions, baseOptionalStrengtheningAssumptions]

theorem runtime_system_contract_unlocks_runtime_system_refinement :
    runtimeSystemContract.optional.runtimeSystemRefinementReady := by
  simp [runtimeSystemContract, mkProofContract, runtimeSystemOptionalStrengtheningAssumptions,
    runtimeCanonicalOptionalStrengtheningAssumptions, canonicalRouterOptionalStrengtheningAssumptions,
    supportOptimalityOptionalStrengtheningAssumptions, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_still_does_not_claim_global_optimality_ready :
    ¬ reducedQualityContract.optional.globalOptimalityReady := by
  simp [reducedQualityContract, mkProofContract, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem support_optimality_contract_still_does_not_claim_global_optimality_ready :
    ¬ supportOptimalityContract.optional.globalOptimalityReady := by
  simp [supportOptimalityContract, mkProofContract, supportOptimalityOptionalStrengtheningAssumptions,
    reducedQualityOptionalStrengtheningAssumptions, baseOptionalStrengtheningAssumptions]

theorem canonical_router_contract_still_does_not_claim_global_optimality_ready :
    ¬ canonicalRouterContract.optional.globalOptimalityReady := by
  simp [canonicalRouterContract, mkProofContract, canonicalRouterOptionalStrengtheningAssumptions,
    supportOptimalityOptionalStrengtheningAssumptions, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

theorem runtime_canonical_contract_still_does_not_claim_global_optimality_ready :
    ¬ runtimeCanonicalContract.optional.globalOptimalityReady := by
  simp [runtimeCanonicalContract, mkProofContract, runtimeCanonicalOptionalStrengtheningAssumptions,
    canonicalRouterOptionalStrengtheningAssumptions, supportOptimalityOptionalStrengtheningAssumptions,
    reducedQualityOptionalStrengtheningAssumptions, baseOptionalStrengtheningAssumptions]

theorem runtime_system_contract_still_does_not_claim_global_optimality_ready :
    ¬ runtimeSystemContract.optional.globalOptimalityReady := by
  simp [runtimeSystemContract, mkProofContract, runtimeSystemOptionalStrengtheningAssumptions,
    runtimeCanonicalOptionalStrengtheningAssumptions, canonicalRouterOptionalStrengtheningAssumptions,
    supportOptimalityOptionalStrengtheningAssumptions, reducedQualityOptionalStrengtheningAssumptions,
    baseOptionalStrengtheningAssumptions]

end FieldAssumptions
