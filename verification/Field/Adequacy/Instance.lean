import Field.Adequacy.API

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAdequacyInstance

open FieldAdequacyAPI
open FieldBoundary
open FieldModelAPI
open FieldProtocolAPI
open FieldProtocolBridge
open FieldRouterLifecycle

/-- Extract a reduced protocol snapshot from the Rust-facing round artifact. -/
def extractSnapshotImpl
    (artifact : RuntimeRoundArtifact) : MachineSnapshot :=
  { stepBudgetRemaining := artifact.stepBudgetRemaining
    blockedOn := artifact.blockedReceive
    disposition := artifact.disposition
    emittedCount := artifact.emittedCount }

/-- Replay-visible semantic object induced by one admitted runtime round
artifact. -/
def artifactSemanticObjects
    (artifact : RuntimeRoundArtifact) : List ProtocolSemanticObject :=
  if artifact.disposition = HostDisposition.failedClosed || artifact.emittedCount = 0 then
    []
  else
    [ { batch := { summaryCount := artifact.emittedCount }
        disposition := artifact.disposition
        authority := OutputAuthority.observationalOnly } ]

/-- One reduced trace chunk contributed by a single runtime artifact. -/
def artifactTraceChunk
    (artifact : RuntimeRoundArtifact) : ProtocolTrace :=
  let inputEvent :=
    ProtocolTraceEvent.machineInput
      (if artifact.blockedReceive.isSome then
        MachineInput.poll
      else
        MachineInput.receiveSummary)
  inputEvent :: (artifactSemanticObjects artifact).map ProtocolTraceEvent.semanticObject

/-- Extract a reduced protocol trace from a list of runtime round artifacts. -/
def extractTraceImpl
    (artifacts : List RuntimeRoundArtifact) : ProtocolTrace :=
  artifacts.flatMap artifactTraceChunk

/-- Controller evidence computed from the runtime artifact list. -/
def runtimeEvidenceImpl
    (artifacts : List RuntimeRoundArtifact) : List EvidenceInput :=
  controllerEvidenceFromTrace (extractTraceImpl artifacts)

/-- Every semantic object emitted from one runtime artifact remains
observational-only. -/
theorem artifact_semantic_objects_stay_observational
    (artifact : RuntimeRoundArtifact) :
    ∀ object ∈ artifactSemanticObjects artifact,
      object.authority = OutputAuthority.observationalOnly := by
  intro object hObject
  by_cases hSilent :
      artifact.disposition = HostDisposition.failedClosed || artifact.emittedCount = 0
  · simp [artifactSemanticObjects, hSilent] at hObject
  · simp [artifactSemanticObjects, hSilent] at hObject
    simp [hObject]

/-- Semantic-object extraction erases the machine-input prefix from a single
runtime trace chunk and preserves only the replay-visible semantic objects. -/
theorem trace_semantic_objects_artifactTraceChunk
    (artifact : RuntimeRoundArtifact) :
    traceSemanticObjects (artifactTraceChunk artifact) =
      artifactSemanticObjects artifact := by
  by_cases hSilent :
      artifact.disposition = HostDisposition.failedClosed || artifact.emittedCount = 0
  · rw [artifactTraceChunk, artifactSemanticObjects, if_pos hSilent]
    simp [traceSemanticObjects]
  · rw [artifactTraceChunk, artifactSemanticObjects, if_neg hSilent]
    simp [traceSemanticObjects]

/-- Filtering the replay-visible semantic objects from a semantic-object-only
list is the identity. -/
theorem filterMap_semanticObject_artifactSemanticObjects
    (artifact : RuntimeRoundArtifact) :
    List.filterMap
        ((fun event =>
            match event with
            | .machineInput _ => none
            | .semanticObject object => some object) ∘
          ProtocolTraceEvent.semanticObject)
        (artifactSemanticObjects artifact) =
      artifactSemanticObjects artifact := by
  by_cases hSilent :
      artifact.disposition = HostDisposition.failedClosed || artifact.emittedCount = 0
  · rw [artifactSemanticObjects, if_pos hSilent]
    simp
  · rw [artifactSemanticObjects, if_neg hSilent]
    simp

/-- Extracting semantic objects from the reduced runtime trace is equivalent to
flattening the per-artifact semantic objects directly. -/
theorem trace_semantic_objects_extractTraceImpl
    (artifacts : List RuntimeRoundArtifact) :
    traceSemanticObjects (extractTraceImpl artifacts) =
      artifacts.flatMap artifactSemanticObjects := by
  induction artifacts with
  | nil =>
      simp [extractTraceImpl, traceSemanticObjects]
  | cons artifact rest ih =>
      have ih' :
          List.filterMap
              (fun event =>
                match event with
                | .machineInput _ => none
                | .semanticObject object => some object)
              (List.flatMap artifactTraceChunk rest) =
            List.flatMap artifactSemanticObjects rest := by
        simpa [extractTraceImpl, traceSemanticObjects] using ih
      simp [extractTraceImpl, artifactTraceChunk, List.flatMap_cons,
        traceSemanticObjects, List.filterMap_append]
      calc
        List.filterMap
            ((fun event =>
                match event with
                | .machineInput _ => none
                | .semanticObject object => some object) ∘
              ProtocolTraceEvent.semanticObject)
            (artifactSemanticObjects artifact) ++
            List.filterMap
              (fun event =>
                match event with
                | .machineInput _ => none
                | .semanticObject object => some object)
              (List.flatMap artifactTraceChunk rest)
            =
          artifactSemanticObjects artifact ++
            List.filterMap
              (fun event =>
                match event with
                | .machineInput _ => none
                | .semanticObject object => some object)
              (List.flatMap artifactTraceChunk rest) := by
                rw [filterMap_semanticObject_artifactSemanticObjects]
        _ =
          artifactSemanticObjects artifact ++ List.flatMap artifactSemanticObjects rest := by
            rw [ih']

instance fieldAdequacyLaws : FieldAdequacyAPI.Laws where
  extractSnapshot := extractSnapshotImpl
  extractTrace := extractTraceImpl
  runtimeEvidence := runtimeEvidenceImpl
  runtime_admitted_implies_bounded_and_coherent := by
    intro artifact hAdmitted
    rcases hAdmitted with ⟨hBudget, hEmitted, hDone, hBlocked, _hRouter⟩
    constructor
    · exact ⟨hBudget, hEmitted⟩
    · constructor
      · intro hTerminal
        exact hDone hTerminal
      · intro hBlockedState
        exact hBlocked hBlockedState
  runtime_evidence_agrees_with_semantic_trace := by
    intro artifacts
    rfl
  runtime_execution_extracts_to_observational_trace := by
    intro artifacts _hAdmitted object hObject
    change object ∈ traceSemanticObjects (extractTraceImpl artifacts) at hObject
    have hFlat : object ∈ artifacts.flatMap artifactSemanticObjects := by
      simpa [trace_semantic_objects_extractTraceImpl] using hObject
    simp only [List.mem_flatMap] at hFlat
    rcases hFlat with ⟨artifact, hMemArtifact, hMemObject⟩
    exact artifact_semantic_objects_stay_observational artifact object hMemObject

/-- If the Rust-facing artifact stays inside the declared protocol envelope,
its extracted observational trace is admitted by the reduced Lean protocol
model. -/
theorem admitted_runtime_artifact_extracts_to_protocol_snapshot
    (artifact : RuntimeRoundArtifact)
    (hAdmitted : RuntimeArtifactAdmitted artifact) :
    MachineBounded (FieldAdequacyAPI.extractSnapshot artifact) ∧
      MachineCoherent (FieldAdequacyAPI.extractSnapshot artifact) := by
  exact FieldAdequacyAPI.runtime_admitted_implies_bounded_and_coherent artifact hAdmitted

/-- Any admitted runtime router projection remains lifecycle-honest. -/
theorem admitted_runtime_artifact_router_projection_honest
    (artifact : RuntimeRoundArtifact)
    (hAdmitted : RuntimeArtifactAdmitted artifact) :
    RuntimeRouterArtifactAdmitted artifact := by
  exact hAdmitted.2.2.2.2

theorem runtimeLifecycleRoutes_mem_implies_honest
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts)
    (route : LifecycleRoute)
    (hMem : route ∈ FieldAdequacyAPI.runtimeLifecycleRoutes artifacts) :
    LifecycleHonest route := by
  unfold FieldAdequacyAPI.runtimeLifecycleRoutes at hMem
  rcases List.mem_filterMap.1 hMem with ⟨artifact, hArtifactMem, hSome⟩
  have hRouter :=
    admitted_runtime_artifact_router_projection_honest artifact (hAdmitted artifact hArtifactMem)
  cases hProjection : artifact.routerArtifact with
  | none =>
      simp [hProjection] at hSome
  | some projection =>
      simp [hProjection] at hSome
      subst hSome
      simpa [RuntimeRouterArtifactAdmitted, hProjection] using hRouter

/-- The controller evidence batch computed from admitted runtime artifacts
agrees with the evidence batch induced by the extracted semantic trace. -/
theorem runtime_trace_evidence_matches_protocol_trace
    (artifacts : List RuntimeRoundArtifact) :
    FieldAdequacyAPI.runtimeEvidence artifacts =
      controllerEvidenceFromTrace (FieldAdequacyAPI.extractTrace artifacts) := by
  exact FieldAdequacyAPI.runtime_evidence_agrees_with_semantic_trace artifacts

/-- Execution-level adequacy: an admitted Rust runtime execution extracts to a
Lean trace whose semantic objects remain observational-only. -/
theorem admitted_runtime_execution_extracts_to_observational_trace
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts) :
    ∀ object ∈ traceSemanticObjects (FieldAdequacyAPI.extractTrace artifacts),
      object.authority = OutputAuthority.observationalOnly := by
  exact FieldAdequacyAPI.runtime_execution_extracts_to_observational_trace artifacts hAdmitted

/-- Simulation witness for the reduced field adequacy layer: an admitted Rust
artifact list is simulated by the extracted reduced Lean protocol trace, and
that trace stays inside the observational-only envelope. -/
def admitted_runtime_execution_simulates_reduced_protocol
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts) :
    RuntimeTraceSimulation artifacts := by
  refine
    { trace := FieldAdequacyAPI.extractTrace artifacts
      trace_eq_extract := rfl
      trace_admitted := ?_ }
  exact admitted_runtime_execution_extracts_to_observational_trace artifacts hAdmitted

/-- The reduced simulation witness preserves the same controller-visible
evidence batch as the Rust-side artifact extraction. -/
theorem runtime_simulation_preserves_controller_evidence_batch
    (artifacts : List RuntimeRoundArtifact)
    (hAdmitted : RuntimeExecutionAdmitted artifacts) :
    controllerEvidenceFromTrace
        (admitted_runtime_execution_simulates_reduced_protocol artifacts hAdmitted).trace =
      FieldAdequacyAPI.runtimeEvidence artifacts := by
  rw [(admitted_runtime_execution_simulates_reduced_protocol artifacts hAdmitted).trace_eq_extract]
  symm
  exact runtime_trace_evidence_matches_protocol_trace artifacts

theorem artifact_semantic_objects_match_extracted_snapshot
    (artifact : RuntimeRoundArtifact) :
    artifactSemanticObjects artifact =
      FieldProtocolAPI.exportSemanticObjects (extractSnapshotImpl artifact) := by
  rfl

/-- Stronger refinement witness: runtime artifacts refine not only to the
reduced protocol trace but also to the corresponding erased fragment trace. -/
theorem runtime_execution_refines_fragment_trace
    (artifacts : List RuntimeRoundArtifact) :
    traceSemanticObjects (FieldAdequacyAPI.extractTrace artifacts) =
      fragmentTraceSemanticObjects
        (fragmentTraceOfSnapshots (artifacts.map FieldAdequacyAPI.extractSnapshot)) := by
  change traceSemanticObjects (extractTraceImpl artifacts) =
    fragmentTraceSemanticObjects (fragmentTraceOfSnapshots (artifacts.map extractSnapshotImpl))
  rw [trace_semantic_objects_extractTraceImpl]
  have hSnapshot :
      artifacts.flatMap artifactSemanticObjects =
        snapshotTraceSemanticObjects (artifacts.map extractSnapshotImpl) := by
    induction artifacts with
    | nil =>
        simp [snapshotTraceSemanticObjects]
    | cons artifact rest ih =>
        simp [snapshotTraceSemanticObjects, artifact_semantic_objects_match_extracted_snapshot, ih]
  rw [hSnapshot]
  rw [← snapshot_trace_semantic_objects_match_fragment_trace]
theorem runtime_execution_refinement_preserves_fragment_observer_projection
    (artifacts : List RuntimeRoundArtifact) :
    controllerEvidenceFromTrace (FieldAdequacyAPI.extractTrace artifacts) =
      semanticObjectsToEvidence
        (fragmentTraceSemanticObjects
          (fragmentTraceOfSnapshots (artifacts.map FieldAdequacyAPI.extractSnapshot))) := by
  unfold controllerEvidenceFromTrace
  simpa [runtime_execution_refines_fragment_trace artifacts]

end FieldAdequacyInstance
