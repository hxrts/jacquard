import Field.Adequacy.Instance
import Field.Information.Blindness
import Field.Protocol.Conservation

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
  qualityComparisonReady : Prop

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
    qualityComparisonReady := False }

def defaultContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := defaultOptionalStrengtheningAssumptions }

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

end FieldAssumptions
