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
open FieldProtocolConservation

structure SemanticAssumptions where
  normalizedBeliefAvailable : Prop
  observationalProjectionOnly : Prop

structure RuntimeEnvelopeAssumptions where
  admitted : List RuntimeRoundArtifact → Prop

structure ProofContract where
  semantic : SemanticAssumptions
  runtime : RuntimeEnvelopeAssumptions

def defaultSemanticAssumptions : SemanticAssumptions :=
  { normalizedBeliefAvailable := True
    observationalProjectionOnly := True }

def defaultRuntimeEnvelopeAssumptions : RuntimeEnvelopeAssumptions :=
  { admitted := fun artifacts => ∀ artifact ∈ artifacts, RuntimeArtifactAdmitted artifact }

def defaultContract : ProofContract :=
  { semantic := defaultSemanticAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions }

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

end FieldAssumptions
