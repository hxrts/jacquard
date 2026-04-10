import Field.Adequacy.Canonical
import Field.Adequacy.Instance
import Field.Adequacy.Projection
import Field.Information.Blindness
import Field.Protocol.Conservation
import Field.Quality.Refinement
import Field.System.Canonical

/-!
Reduced packaged assumptions for the growing field proof stack.
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

structure SemanticAssumptions where
  normalizedBeliefAvailable : Prop
  observationalProjectionOnly : Prop

structure ProtocolEnvelopeAssumptions where
  reducedMachineCoherent : MachineSnapshot → Prop
  semanticObjectsObservational : ProtocolTrace → Prop

structure RuntimeEnvelopeAssumptions where
  admitted : List RuntimeRoundArtifact → Prop
  respectsReducedEnvelope :
    ∀ artifacts, admitted artifacts → RuntimeExecutionAdmitted artifacts

structure OptionalStrengtheningAssumptions where
  receiveRefinementEnabled : Prop
  simulationStrengthened : Prop
  reducedQualityComparisonReady : Prop
  supportOptimalityRefinementReady : Prop
  canonicalRouterRefinementReady : Prop
  runtimeCanonicalRefinementReady : Prop
  runtimeSystemRefinementReady : Prop
  globalOptimalityReady : Prop

structure ProofContract where
  semantic : SemanticAssumptions
  protocol : ProtocolEnvelopeAssumptions
  runtime : RuntimeEnvelopeAssumptions
  optional : OptionalStrengtheningAssumptions

def defaultSemanticAssumptions : SemanticAssumptions :=
  { normalizedBeliefAvailable := True
    observationalProjectionOnly := True }

def defaultRuntimeEnvelopeAssumptions : RuntimeEnvelopeAssumptions :=
  { admitted := fun artifacts => ∀ artifact ∈ artifacts, RuntimeArtifactAdmitted artifact
    respectsReducedEnvelope := by
      intro artifacts hAdmitted
      exact hAdmitted }

def defaultProtocolEnvelopeAssumptions : ProtocolEnvelopeAssumptions :=
  { reducedMachineCoherent := MachineCoherent
    semanticObjectsObservational := fun trace => FieldEvidenceConservation trace }

def defaultOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := False
    supportOptimalityRefinementReady := False
    canonicalRouterRefinementReady := False
    runtimeCanonicalRefinementReady := False
    runtimeSystemRefinementReady := False
    globalOptimalityReady := False }

def reducedQualityOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := True
    supportOptimalityRefinementReady := False
    canonicalRouterRefinementReady := False
    runtimeCanonicalRefinementReady := False
    runtimeSystemRefinementReady := False
    globalOptimalityReady := False }

def supportOptimalityOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := True
    supportOptimalityRefinementReady := True
    canonicalRouterRefinementReady := False
    runtimeCanonicalRefinementReady := False
    runtimeSystemRefinementReady := False
    globalOptimalityReady := False }

def canonicalRouterOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := True
    supportOptimalityRefinementReady := True
    canonicalRouterRefinementReady := True
    runtimeCanonicalRefinementReady := False
    runtimeSystemRefinementReady := False
    globalOptimalityReady := False }

def runtimeCanonicalOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := True
    supportOptimalityRefinementReady := True
    canonicalRouterRefinementReady := True
    runtimeCanonicalRefinementReady := True
    runtimeSystemRefinementReady := False
    globalOptimalityReady := False }

def runtimeSystemOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := True
    supportOptimalityRefinementReady := True
    canonicalRouterRefinementReady := True
    runtimeCanonicalRefinementReady := True
    runtimeSystemRefinementReady := True
    globalOptimalityReady := False }

def defaultContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := defaultOptionalStrengtheningAssumptions }

def reducedQualityContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := reducedQualityOptionalStrengtheningAssumptions }

def supportOptimalityContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := supportOptimalityOptionalStrengtheningAssumptions }

def canonicalRouterContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := canonicalRouterOptionalStrengtheningAssumptions }

def runtimeCanonicalContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := runtimeCanonicalOptionalStrengtheningAssumptions }

def runtimeSystemContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := runtimeSystemOptionalStrengtheningAssumptions }

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
      (contract.runtime.respectsReducedEnvelope artifacts hAdmitted)

/-- Packaged simulation witness obtained from the runtime assumption contract. -/
def contract_yields_runtime_trace_simulation
    (contract : ProofContract)
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : contract.runtime.admitted artifacts) :
    RuntimeTraceSimulation artifacts := by
  exact
    FieldAdequacyInstance.admitted_runtime_execution_simulates_reduced_protocol
      artifacts
      (contract.runtime.respectsReducedEnvelope artifacts hAdmitted)

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
    (hAdmitted : contract.runtime.admitted artifacts)
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

theorem default_contract_does_not_claim_support_optimality_refinement_ready :
    ¬ defaultContract.optional.supportOptimalityRefinementReady := by
  simp [defaultContract, defaultOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_does_not_claim_support_optimality_refinement_ready :
    ¬ reducedQualityContract.optional.supportOptimalityRefinementReady := by
  simp [reducedQualityContract, reducedQualityOptionalStrengtheningAssumptions]

theorem default_contract_does_not_claim_canonical_router_refinement_ready :
    ¬ defaultContract.optional.canonicalRouterRefinementReady := by
  simp [defaultContract, defaultOptionalStrengtheningAssumptions]

theorem support_optimality_contract_does_not_claim_canonical_router_refinement_ready :
    ¬ supportOptimalityContract.optional.canonicalRouterRefinementReady := by
  simp [supportOptimalityContract, supportOptimalityOptionalStrengtheningAssumptions]

theorem canonical_router_contract_does_not_claim_runtime_canonical_refinement_ready :
    ¬ canonicalRouterContract.optional.runtimeCanonicalRefinementReady := by
  simp [canonicalRouterContract, canonicalRouterOptionalStrengtheningAssumptions]

theorem runtime_canonical_contract_does_not_claim_runtime_system_refinement_ready :
    ¬ runtimeCanonicalContract.optional.runtimeSystemRefinementReady := by
  simp [runtimeCanonicalContract, runtimeCanonicalOptionalStrengtheningAssumptions]

theorem default_contract_does_not_claim_global_optimality_ready :
    ¬ defaultContract.optional.globalOptimalityReady := by
  simp [defaultContract, defaultOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_unlocks_reduced_quality_comparison :
    reducedQualityContract.optional.reducedQualityComparisonReady := by
  simp [reducedQualityContract, reducedQualityOptionalStrengtheningAssumptions]

theorem support_optimality_contract_unlocks_support_optimality_refinement :
    supportOptimalityContract.optional.supportOptimalityRefinementReady := by
  simp [supportOptimalityContract, supportOptimalityOptionalStrengtheningAssumptions]

theorem canonical_router_contract_unlocks_canonical_router_refinement :
    canonicalRouterContract.optional.canonicalRouterRefinementReady := by
  simp [canonicalRouterContract, canonicalRouterOptionalStrengtheningAssumptions]

theorem runtime_canonical_contract_unlocks_runtime_canonical_refinement :
    runtimeCanonicalContract.optional.runtimeCanonicalRefinementReady := by
  simp [runtimeCanonicalContract, runtimeCanonicalOptionalStrengtheningAssumptions]

theorem runtime_system_contract_unlocks_runtime_system_refinement :
    runtimeSystemContract.optional.runtimeSystemRefinementReady := by
  simp [runtimeSystemContract, runtimeSystemOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_still_does_not_claim_global_optimality_ready :
    ¬ reducedQualityContract.optional.globalOptimalityReady := by
  simp [reducedQualityContract, reducedQualityOptionalStrengtheningAssumptions]

theorem support_optimality_contract_still_does_not_claim_global_optimality_ready :
    ¬ supportOptimalityContract.optional.globalOptimalityReady := by
  simp [supportOptimalityContract, supportOptimalityOptionalStrengtheningAssumptions]

theorem canonical_router_contract_still_does_not_claim_global_optimality_ready :
    ¬ canonicalRouterContract.optional.globalOptimalityReady := by
  simp [canonicalRouterContract, canonicalRouterOptionalStrengtheningAssumptions]

theorem runtime_canonical_contract_still_does_not_claim_global_optimality_ready :
    ¬ runtimeCanonicalContract.optional.globalOptimalityReady := by
  simp [runtimeCanonicalContract, runtimeCanonicalOptionalStrengtheningAssumptions]

theorem runtime_system_contract_still_does_not_claim_global_optimality_ready :
    ¬ runtimeSystemContract.optional.globalOptimalityReady := by
  simp [runtimeSystemContract, runtimeSystemOptionalStrengtheningAssumptions]

end FieldAssumptions
