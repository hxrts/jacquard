import Field.Model.Instance
import Field.Protocol.Boundary

/-
The Problem. The proof story needs one narrow theorem linking private protocol
outputs to deterministic observer inputs without pretending to prove the whole
field controller. We want a small boundary module that says exactly what the
protocol may contribute: bounded observational evidence, never canonical route
truth.

Solution Structure.
1. Define a compact export-to-evidence adapter.
2. Keep that adapter corridor-only and bounded.
3. Prove that exported protocol outputs cannot manufacture explicit-path or
   unreachable route truth.
-/

/-! # FieldBoundary

Narrow Lean boundary between field protocol outputs and local observer inputs.

This module intentionally proves only the input-contract story. It does not try
to prove controller optimality or router-level correctness.

Projection taxonomy note:

- this module sits on the local public projection boundary
- it owns only protocol-export / semantic-object to controller-evidence
  extraction
- it does not own protocol projection from choreography to local types
- it does not own runtime/adequacy projection from artifacts to reduced Lean
  objects
- that runtime-facing composition lives in `Field/Adequacy/*`, which composes
  with this module only after runtime artifacts have already been reduced to
  protocol traces or controller evidence
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldBoundary

open FieldModelAPI
open FieldProtocolAPI
open FieldProtocolBoundary

/-- Translate one observational protocol output into bounded local evidence at
the local public projection boundary. -/
def protocolOutputToEvidence (output : ProtocolOutput) : EvidenceInput :=
  { refresh := .explicitRefresh
    reachability :=
      if output.batch.summaryCount = 0 then
        .unknown
      else
        .corridorOnly
    supportSignal := min (output.batch.summaryCount * 125) 1000
    entropySignal :=
      if output.batch.summaryCount = 0 then
        800
      else
        250
    controllerPressure := 0
    feedback := .none }

/-- Translate the full exported protocol output list into controller-visible
evidence batches. This is boundary adaptation into local public semantics, not
protocol projection itself. -/
def protocolOutputsToEvidence
    (outputs : List ProtocolOutput) : List EvidenceInput :=
  outputs.map protocolOutputToEvidence

/-- Host-visible controller evidence is derived only from exported protocol
outputs, not from protocol-private machine state. -/
def controllerEvidenceFromSnapshot
    (snapshot : MachineSnapshot) : List EvidenceInput :=
  protocolOutputsToEvidence (FieldProtocolAPI.exportOutputs snapshot)

/-- Translate one replay-visible semantic protocol object into bounded local
evidence. -/
def semanticObjectToEvidence
    (object : ProtocolSemanticObject) : EvidenceInput :=
  protocolOutputToEvidence
    { batch := object.batch, authority := object.authority }

/-- Translate replay-visible semantic protocol objects into controller-visible
evidence batches. -/
def semanticObjectsToEvidence
    (objects : List ProtocolSemanticObject) : List EvidenceInput :=
  objects.map semanticObjectToEvidence

/-- Host-visible controller evidence induced by the semantic objects in a
protocol trace. -/
def controllerEvidenceFromTrace
    (trace : ProtocolTrace) : List EvidenceInput :=
  semanticObjectsToEvidence (traceSemanticObjects trace)

/-- Protocol exports can only become observational field evidence. -/
theorem protocol_output_never_claims_canonical_route_truth
    (output : ProtocolOutput) :
    let evidence := protocolOutputToEvidence output
    evidence.reachability ≠ ReachabilitySignal.explicitPath ∧
      evidence.reachability ≠ ReachabilitySignal.unreachable := by
  -- The adapter emits only `unknown` or `corridorOnly`, never canonical claims.
  by_cases hCount : output.batch.summaryCount = 0
  · simp [protocolOutputToEvidence, hCount]
  · simp [protocolOutputToEvidence, hCount]

/-- Any output exported by the reduced protocol instance stays on the
observational side of the field boundary. -/
theorem exported_protocol_outputs_stay_observational
    (snapshot : MachineSnapshot)
    (output : ProtocolOutput)
    (hOutput : output ∈ FieldProtocolAPI.exportOutputs snapshot) :
    (protocolOutputToEvidence output).reachability = ReachabilitySignal.unknown ∨
      (protocolOutputToEvidence output).reachability =
        ReachabilitySignal.corridorOnly := by
  -- The reduced protocol instance exports only observational batches, and the
  -- adapter preserves that by emitting only unknown or corridor-only signals.
  by_cases hCount : output.batch.summaryCount = 0
  · simp [protocolOutputToEvidence, hCount]
  · simp [protocolOutputToEvidence, hCount]

/-- Equal exported protocol outputs induce equal controller evidence batches. -/
theorem equal_protocol_exports_induce_equal_evidence
    {left right : List ProtocolOutput}
    (hEqual : left = right) :
    protocolOutputsToEvidence left = protocolOutputsToEvidence right := by
  simp [hEqual]

/-- Equal host-visible protocol exports imply equal controller evidence,
regardless of private machine-state differences. -/
theorem equal_snapshot_exports_induce_equal_controller_evidence
    (left right : MachineSnapshot)
    (hEqual :
      FieldProtocolAPI.exportOutputs left =
        FieldProtocolAPI.exportOutputs right) :
    controllerEvidenceFromSnapshot left = controllerEvidenceFromSnapshot right := by
  simpa [controllerEvidenceFromSnapshot] using equal_protocol_exports_induce_equal_evidence hEqual

/-- Fail-closed protocol state produces no controller evidence through the
list-level adapter. -/
theorem failed_closed_snapshot_produces_no_controller_evidence
    (snapshot : MachineSnapshot)
    (hFailed : snapshot.disposition = HostDisposition.failedClosed) :
    controllerEvidenceFromSnapshot snapshot = [] := by
  simp [controllerEvidenceFromSnapshot, protocolOutputsToEvidence,
    FieldProtocolBoundary.failed_closed_exports_nothing snapshot hFailed]

/-- Every controller evidence item produced from exported protocol batches stays
on the observational side of the boundary. -/
theorem all_controller_evidence_from_snapshot_stays_observational
    (snapshot : MachineSnapshot) :
    ∀ evidence ∈ controllerEvidenceFromSnapshot snapshot,
      evidence.reachability = ReachabilitySignal.unknown ∨
        evidence.reachability = ReachabilitySignal.corridorOnly := by
  intro evidence hEvidence
  simp [controllerEvidenceFromSnapshot, protocolOutputsToEvidence] at hEvidence
  rcases hEvidence with ⟨output, hOutput, rfl⟩
  exact exported_protocol_outputs_stay_observational snapshot output hOutput

/-- Semantic protocol objects remain observational at the controller boundary. -/
theorem semantic_object_never_claims_canonical_route_truth
    (object : ProtocolSemanticObject) :
    let evidence := semanticObjectToEvidence object
    evidence.reachability ≠ ReachabilitySignal.explicitPath ∧
      evidence.reachability ≠ ReachabilitySignal.unreachable := by
  exact protocol_output_never_claims_canonical_route_truth
    { batch := object.batch, authority := object.authority }

/-- Every replay-visible semantic object exported by the reduced protocol stays
observational-only. -/
theorem semantic_objects_from_snapshot_stay_observational
    (snapshot : MachineSnapshot) :
    ∀ object ∈ FieldProtocolAPI.exportSemanticObjects snapshot,
      object.authority = OutputAuthority.observationalOnly := by
  exact FieldProtocolAPI.semantic_exports_remain_observational snapshot

/-- Equal semantic exports induce equal controller evidence batches. -/
theorem equal_semantic_exports_induce_equal_evidence
    {left right : List ProtocolSemanticObject}
    (hEqual : left = right) :
    semanticObjectsToEvidence left = semanticObjectsToEvidence right := by
  simp [hEqual]

/-- Replay-equivalent semantic traces induce identical controller evidence
batches. -/
theorem replay_equivalent_protocol_traces_induce_equal_controller_evidence
    {left right : ProtocolTrace}
    (hEqual : traceSemanticObjects left = traceSemanticObjects right) :
    controllerEvidenceFromTrace left = controllerEvidenceFromTrace right := by
  simpa [controllerEvidenceFromTrace] using
    equal_semantic_exports_induce_equal_evidence hEqual

/-- Protocol authority remains observational-only all the way to the
controller-facing evidence batches extracted from a semantic trace. -/
theorem trace_controller_evidence_stays_observational
    (trace : ProtocolTrace) :
    ∀ evidence ∈ controllerEvidenceFromTrace trace,
      evidence.reachability = ReachabilitySignal.unknown ∨
        evidence.reachability = ReachabilitySignal.corridorOnly := by
  intro evidence hEvidence
  simp [controllerEvidenceFromTrace, semanticObjectsToEvidence, traceSemanticObjects] at hEvidence
  rcases hEvidence with ⟨object, hObject, rfl⟩
  have hNoCanonical :=
    semantic_object_never_claims_canonical_route_truth object
  rcases hNoCanonical with ⟨hNotExplicit, hNotUnreachable⟩
  by_cases hCount : object.batch.summaryCount = 0
  · simp [semanticObjectToEvidence, protocolOutputToEvidence, hCount]
  · simp [semanticObjectToEvidence, protocolOutputToEvidence, hCount]

end FieldBoundary
