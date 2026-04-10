import Field.Adequacy.API
import Field.Protocol.Conservation

/-
The Problem. The field proof stack needs one place that owns the packaged
assumption vocabulary and contract presets used by higher-layer theorem packs.
Before cleanup, `Field/Assumptions.lean` mixed this contract vocabulary with
every exported theorem, which made ownership and review harder.

Solution Structure.
1. Define the semantic, protocol, runtime, and optional-strengthening records.
2. Define one default contract builder and derive stronger presets from it.
3. Leave theorem-packaging and contract consequences to a separate file.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAssumptions

open FieldAdequacyAPI
open FieldProtocolAPI
open FieldProtocolConservation

/-! ## Assumption Records -/

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

/-! ## Default Envelopes -/

def defaultSemanticAssumptions : SemanticAssumptions :=
  { normalizedBeliefAvailable := True
    observationalProjectionOnly := True }

def defaultRuntimeEnvelopeAssumptions : RuntimeEnvelopeAssumptions :=
  { admitted := fun artifacts => ∀ artifact ∈ artifacts, RuntimeArtifactAdmitted artifact
    respectsReducedEnvelope := by
      -- The default runtime contract does not strengthen admission beyond the
      -- reduced adequacy envelope, so the proof is immediate.
      intro artifacts hAdmitted
      exact hAdmitted }

def defaultProtocolEnvelopeAssumptions : ProtocolEnvelopeAssumptions :=
  { reducedMachineCoherent := MachineCoherent
    semanticObjectsObservational := fun trace => FieldEvidenceConservation trace }

/-! ## Optional Strengthening Presets -/

def baseOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { receiveRefinementEnabled := True
    simulationStrengthened := True
    reducedQualityComparisonReady := False
    supportOptimalityRefinementReady := False
    canonicalRouterRefinementReady := False
    runtimeCanonicalRefinementReady := False
    runtimeSystemRefinementReady := False
    globalOptimalityReady := False }

def defaultOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  baseOptionalStrengtheningAssumptions

def reducedQualityOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { baseOptionalStrengtheningAssumptions with
    reducedQualityComparisonReady := True }

def supportOptimalityOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { reducedQualityOptionalStrengtheningAssumptions with
    supportOptimalityRefinementReady := True }

def canonicalRouterOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { supportOptimalityOptionalStrengtheningAssumptions with
    canonicalRouterRefinementReady := True }

def runtimeCanonicalOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { canonicalRouterOptionalStrengtheningAssumptions with
    runtimeCanonicalRefinementReady := True }

def runtimeSystemOptionalStrengtheningAssumptions : OptionalStrengtheningAssumptions :=
  { runtimeCanonicalOptionalStrengtheningAssumptions with
    runtimeSystemRefinementReady := True }

/-! ## Contract Presets -/

def mkProofContract
    (optional : OptionalStrengtheningAssumptions) : ProofContract :=
  { semantic := defaultSemanticAssumptions
    protocol := defaultProtocolEnvelopeAssumptions
    runtime := defaultRuntimeEnvelopeAssumptions
    optional := optional }

def defaultContract : ProofContract :=
  mkProofContract defaultOptionalStrengtheningAssumptions

def reducedQualityContract : ProofContract :=
  mkProofContract reducedQualityOptionalStrengtheningAssumptions

def supportOptimalityContract : ProofContract :=
  mkProofContract supportOptimalityOptionalStrengtheningAssumptions

def canonicalRouterContract : ProofContract :=
  mkProofContract canonicalRouterOptionalStrengtheningAssumptions

def runtimeCanonicalContract : ProofContract :=
  mkProofContract runtimeCanonicalOptionalStrengtheningAssumptions

def runtimeSystemContract : ProofContract :=
  mkProofContract runtimeSystemOptionalStrengtheningAssumptions

end FieldAssumptions
