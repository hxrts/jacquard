import Field.Adequacy.Instance
import Field.Information.Blindness
import Field.Protocol.Conservation
import Field.Quality.System

/-!
Reduced packaged assumptions for the growing field proof stack.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAssumptions

open FieldAdequacyAPI
open FieldAdequacyInstance
open FieldBoundary
open FieldInformationBlindness
open FieldProtocolAPI
open FieldProtocolConservation
open FieldQualityAPI
open FieldQualitySystem
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
    globalOptimalityReady := False }

def reducedQualityOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := True
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

theorem default_contract_does_not_claim_global_optimality_ready :
    ¬ defaultContract.optional.globalOptimalityReady := by
  simp [defaultContract, defaultOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_unlocks_reduced_quality_comparison :
    reducedQualityContract.optional.reducedQualityComparisonReady := by
  simp [reducedQualityContract, reducedQualityOptionalStrengtheningAssumptions]

theorem reduced_quality_contract_still_does_not_claim_global_optimality_ready :
    ¬ reducedQualityContract.optional.globalOptimalityReady := by
  simp [reducedQualityContract, reducedQualityOptionalStrengtheningAssumptions]

end FieldAssumptions
