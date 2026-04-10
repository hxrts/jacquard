import Field.Model.API
import Field.Protocol.Bridge

/-!
Minimal adequacy-facing boundary between Rust-visible runtime artifacts and the
reduced Lean private protocol object.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyAPI

open FieldBoundary
open FieldModelAPI
open FieldProtocolAPI

/-- Narrowest Rust-facing round artifact currently worth relating to the Lean
protocol object. This mirrors the controller-relevant fields of
`FieldChoreographyRoundResult`. -/
structure RuntimeRoundArtifact where
  blockedReceive : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
  stepBudgetRemaining : Nat
  deriving Repr, DecidableEq, BEq

/-- Envelope expected from the Rust private runtime before we claim any
adequacy bridge. -/
def RuntimeArtifactAdmitted (artifact : RuntimeRoundArtifact) : Prop :=
  artifact.stepBudgetRemaining ≤ 8 ∧
    artifact.emittedCount ≤ 8 ∧
    ((artifact.disposition = HostDisposition.complete ∨
        artifact.disposition = HostDisposition.failedClosed) →
        artifact.blockedReceive = none) ∧
    (artifact.disposition = HostDisposition.blocked →
      artifact.blockedReceive.isSome)

/-- Execution-level admission: every runtime artifact stays inside the reduced
private protocol envelope. -/
def RuntimeExecutionAdmitted
    (artifacts : List RuntimeRoundArtifact) : Prop :=
  ∀ artifact ∈ artifacts, RuntimeArtifactAdmitted artifact

class Model where
  extractSnapshot : RuntimeRoundArtifact → MachineSnapshot
  extractTrace : List RuntimeRoundArtifact → ProtocolTrace
  runtimeEvidence : List RuntimeRoundArtifact → List EvidenceInput

section Wrappers

variable [Model]

def extractSnapshot (artifact : RuntimeRoundArtifact) : MachineSnapshot :=
  Model.extractSnapshot artifact

def extractTrace (artifacts : List RuntimeRoundArtifact) : ProtocolTrace :=
  Model.extractTrace artifacts

def runtimeEvidence (artifacts : List RuntimeRoundArtifact) : List EvidenceInput :=
  Model.runtimeEvidence artifacts

end Wrappers

abbrev RuntimeAdmittedImpliesBoundedAndCoherent (M : Model) : Prop :=
  ∀ artifact,
    RuntimeArtifactAdmitted artifact →
      MachineBounded (@Model.extractSnapshot M artifact) ∧
        MachineCoherent (@Model.extractSnapshot M artifact)

abbrev RuntimeEvidenceAgreesWithSemanticTrace (M : Model) : Prop :=
  ∀ artifacts,
    @Model.runtimeEvidence M artifacts =
      controllerEvidenceFromTrace (@Model.extractTrace M artifacts)

/-- Execution-level observational adequacy for extracted traces. -/
abbrev RuntimeExecutionExtractsToObservationalTrace (M : Model) : Prop :=
  ∀ artifacts,
    RuntimeExecutionAdmitted artifacts →
      ∀ object ∈ traceSemanticObjects (@Model.extractTrace M artifacts),
        object.authority = OutputAuthority.observationalOnly

class Laws extends Model where
  runtime_admitted_implies_bounded_and_coherent :
    RuntimeAdmittedImpliesBoundedAndCoherent toModel
  runtime_evidence_agrees_with_semantic_trace :
    RuntimeEvidenceAgreesWithSemanticTrace toModel
  runtime_execution_extracts_to_observational_trace :
    RuntimeExecutionExtractsToObservationalTrace toModel

instance (priority := 100) lawsToModel [Laws] : Model := Laws.toModel

section LawWrappers

variable [Laws]

theorem runtime_admitted_implies_bounded_and_coherent
    (artifact : RuntimeRoundArtifact)
    (hAdmitted : RuntimeArtifactAdmitted artifact) :
    MachineBounded (extractSnapshot artifact) ∧
      MachineCoherent (extractSnapshot artifact) :=
  Laws.runtime_admitted_implies_bounded_and_coherent artifact hAdmitted

theorem runtime_evidence_agrees_with_semantic_trace
    (artifacts : List RuntimeRoundArtifact) :
    runtimeEvidence artifacts =
      controllerEvidenceFromTrace (extractTrace artifacts) :=
  Laws.runtime_evidence_agrees_with_semantic_trace artifacts

theorem runtime_execution_extracts_to_observational_trace
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts) :
    ∀ object ∈ traceSemanticObjects (extractTrace artifacts),
      object.authority = OutputAuthority.observationalOnly :=
  Laws.runtime_execution_extracts_to_observational_trace artifacts hAdmitted

end LawWrappers

end FieldAdequacyAPI
