import Field.Adequacy.API

/-
The Problem. The original adequacy layer reasons directly about flat lists of
runtime artifacts. That is enough for an artifact-to-trace bridge, but it does
not yet provide a proof-facing runtime state object or a small execution step
semantics that later refinement theorems can relate to `Field/System`.

Solution Structure.
1. Define a reduced runtime state that tracks pending and completed artifacts.
2. Define a one-step execution relation that consumes one pending artifact.
3. Restate the current artifact-oriented adequacy bridge on top of runtime
   states and prove admission is preserved across runtime steps.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyRuntime

open FieldAdequacyAPI
open FieldBoundary
open FieldModelAPI

def runtimeStateObjectRole : FieldArchitecture.ObjectRole :=
  .semanticCore

/-! ## Runtime State Vocabulary -/

/-- Reduced runtime execution state used by the adequacy layer. It keeps only
the proof-relevant pending and completed runtime artifacts and intentionally
omits all richer host/runtime internals. This is an execution object, not a
publication object or a router-truth object. -/
structure RuntimeState where
  pendingArtifacts : List RuntimeRoundArtifact
  completedArtifacts : List RuntimeRoundArtifact
  deriving Repr, DecidableEq, BEq

/-- The completed artifact prefix extracted from one reduced runtime state. -/
def runtimeArtifactsOfState
    (state : RuntimeState) : List RuntimeRoundArtifact :=
  state.completedArtifacts

/-- Initial reduced runtime state whose pending work is exactly one runtime
artifact list and whose completed prefix is empty. -/
def initialRuntimeState
    (artifacts : List RuntimeRoundArtifact) : RuntimeState :=
  { pendingArtifacts := artifacts
    completedArtifacts := [] }

/-- One reduced runtime step consumes exactly one pending artifact and appends
it to the completed execution prefix. -/
inductive RuntimeStep : RuntimeState → RuntimeState → Prop where
  | consume
      (artifact : RuntimeRoundArtifact)
      (pendingTail completed : List RuntimeRoundArtifact) :
      RuntimeStep
        { pendingArtifacts := artifact :: pendingTail
          completedArtifacts := completed }
        { pendingArtifacts := pendingTail
          completedArtifacts := completed ++ [artifact] }

/-- Runtime-state admission requires both pending and completed runtime
artifacts to stay inside the reduced adequacy envelope. -/
def RuntimeStateAdmitted
    (state : RuntimeState) : Prop :=
  RuntimeExecutionAdmitted state.pendingArtifacts ∧
    RuntimeExecutionAdmitted (runtimeArtifactsOfState state)

/-! ## Artifact Extraction And State Adequacy -/

section LawWrappers

variable [FieldAdequacyAPI.Laws]

/-- Reduced protocol trace extracted from the completed runtime prefix of one
runtime state. -/
def extractTraceOfState
    (state : RuntimeState) : FieldProtocolAPI.ProtocolTrace :=
  extractTrace (runtimeArtifactsOfState state)

/-- Controller-visible evidence extracted from the completed runtime prefix of
one runtime state. -/
def runtimeEvidenceOfState
    (state : RuntimeState) : List EvidenceInput :=
  runtimeEvidence (runtimeArtifactsOfState state)

/-- State-level simulation witness obtained by reusing the existing
artifact-list adequacy bridge on the completed runtime prefix. -/
def admitted_runtime_state_simulates_reduced_protocol
    (state : RuntimeState)
    (hAdmitted : RuntimeStateAdmitted state) :
    RuntimeTraceSimulation (runtimeArtifactsOfState state) := by
  refine
    { trace := extractTraceOfState state
      trace_eq_extract := rfl
      trace_admitted :=
        runtime_execution_extracts_to_observational_trace
          (runtimeArtifactsOfState state)
          hAdmitted.2 }

theorem runtime_state_evidence_agrees_with_semantic_trace
    (state : RuntimeState) :
    runtimeEvidenceOfState state =
      controllerEvidenceFromTrace (extractTraceOfState state) := by
  exact runtime_evidence_agrees_with_semantic_trace (runtimeArtifactsOfState state)

theorem admitted_runtime_state_extracts_to_observational_trace
    (state : RuntimeState)
    (hAdmitted : RuntimeStateAdmitted state) :
    ProtocolTraceAdmitted (extractTraceOfState state) := by
  exact
    runtime_execution_extracts_to_observational_trace
      (runtimeArtifactsOfState state)
      hAdmitted.2

end LawWrappers

/-! ## Step Properties -/

theorem runtimeArtifactsOfState_initialRuntimeState
    (artifacts : List RuntimeRoundArtifact) :
    runtimeArtifactsOfState (initialRuntimeState artifacts) = [] := by
  rfl

theorem initialRuntimeState_admitted
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts) :
    RuntimeStateAdmitted (initialRuntimeState artifacts) := by
  constructor
  · exact hAdmitted
  · intro artifact hMem
    simp [initialRuntimeState, runtimeArtifactsOfState] at hMem

theorem runtime_step_appends_step_artifact
    {source target : RuntimeState}
    (hStep : RuntimeStep source target) :
    ∃ artifact,
      artifact ∈ source.pendingArtifacts ∧
        runtimeArtifactsOfState target =
          runtimeArtifactsOfState source ++ [artifact] := by
  cases hStep with
  | consume artifact pendingTail completed =>
      refine ⟨artifact, ?_, ?_⟩
      · simp
      · rfl

theorem runtime_state_admitted_implies_step_artifact_admitted
    {source target : RuntimeState}
    (hState : RuntimeStateAdmitted source)
    (hStep : RuntimeStep source target) :
    ∃ artifact,
      artifact ∈ source.pendingArtifacts ∧
        RuntimeArtifactAdmitted artifact ∧
        runtimeArtifactsOfState target =
          runtimeArtifactsOfState source ++ [artifact] := by
  rcases runtime_step_appends_step_artifact hStep with ⟨artifact, hMem, hAppend⟩
  refine ⟨artifact, hMem, hState.1 artifact hMem, hAppend⟩

theorem runtime_step_preserves_state_admitted
    {source target : RuntimeState}
    (hState : RuntimeStateAdmitted source)
    (hStep : RuntimeStep source target) :
    RuntimeStateAdmitted target := by
  cases hStep with
  | consume artifact pendingTail completed =>
      constructor
      · intro pendingArtifact hMem
        exact hState.1 pendingArtifact (by simp [hMem])
      · intro completedArtifact hMem
        simp [runtimeArtifactsOfState] at hMem
        rcases hMem with hOld | rfl
        · exact hState.2 completedArtifact hOld
        · exact hState.1 completedArtifact (by simp)

theorem runtime_step_target_trace_extends_source
    [FieldAdequacyAPI.Laws]
    {source target : RuntimeState}
    (hStep : RuntimeStep source target) :
    ∃ artifact,
      artifact ∈ source.pendingArtifacts ∧
        extractTraceOfState target =
          extractTrace (runtimeArtifactsOfState source ++ [artifact]) := by
  rcases runtime_step_appends_step_artifact hStep with ⟨artifact, hMem, hAppend⟩
  refine ⟨artifact, hMem, ?_⟩
  simpa [extractTraceOfState] using congrArg extractTrace hAppend

end FieldAdequacyRuntime
