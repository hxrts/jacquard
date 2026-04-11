import Field.Model.Boundary
import Field.Protocol.Instance
import SessionTypes.Core

/-
The Problem. The reduced field protocol instance needs a bridge to a
Telltale-shaped machine fragment so the proof stack can talk about replay,
fragment traces, and controller-facing evidence without collapsing protocol
semantics into one Jacquard-specific machine state.

Solution Structure.
1. Define one reduced machine fragment that preserves only controller-relevant
   fields.
2. Erase snapshots and traces into that fragment vocabulary.
3. Prove the fragment preserves the same observational semantic objects.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolBridge

open FieldBoundary
open FieldProtocolAPI
open SessionTypes.Core

/-! ## Fragment Vocabulary -/

/-- Reduced fragment of a Telltale protocol-machine state carrying only the
controller-relevant fields preserved by the current field proof boundary. -/
structure TelltaleMachineFragment where
  global : GlobalChoreography
  controllerLocal : LocalType
  neighborLocal : LocalType
  blockedOn : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
  stepBudgetRemaining : Nat
  deriving Repr

/-- Erase a reduced field machine snapshot into the corresponding protocol
machine fragment. -/
def snapshotToFragment
    (snapshot : MachineSnapshot) : TelltaleMachineFragment :=
  { global := FieldProtocolAPI.globalChoreography
    controllerLocal := FieldProtocolAPI.project FieldProtocolAPI.controllerRole
    neighborLocal := FieldProtocolAPI.project FieldProtocolAPI.neighborRole
    blockedOn := snapshot.blockedOn
    disposition := snapshot.disposition
    emittedCount := snapshot.emittedCount
    stepBudgetRemaining := snapshot.stepBudgetRemaining }

/-- The fragment preserves exactly the controller-relevant fields of the
reduced machine snapshot. -/
def SnapshotMatchesFragment
    (snapshot : MachineSnapshot)
    (fragment : TelltaleMachineFragment) : Prop :=
  fragment.global = FieldProtocolAPI.globalChoreography ∧
    fragment.controllerLocal = FieldProtocolAPI.project FieldProtocolAPI.controllerRole ∧
    fragment.neighborLocal = FieldProtocolAPI.project FieldProtocolAPI.neighborRole ∧
    fragment.blockedOn = snapshot.blockedOn ∧
    fragment.disposition = snapshot.disposition ∧
    fragment.emittedCount = snapshot.emittedCount ∧
    fragment.stepBudgetRemaining = snapshot.stepBudgetRemaining

/-- Semantic objects preserved by the reduced machine-fragment erasure. -/
def fragmentSemanticObjects
    (fragment : TelltaleMachineFragment) : List ProtocolSemanticObject :=
  if fragment.disposition = HostDisposition.failedClosed || fragment.emittedCount = 0 then
    []
  else
    [ { batch := { summaryCount := fragment.emittedCount }
        disposition := fragment.disposition
        authority := OutputAuthority.observationalOnly } ]

/-- Reduced fragment trace built from protocol snapshots. -/
abbrev FragmentTrace := List TelltaleMachineFragment

/-- Replay-visible semantic trace extracted from a fragment trace. -/
def fragmentTraceSemanticObjects
    (trace : FragmentTrace) : List ProtocolSemanticObject :=
  trace.flatMap fragmentSemanticObjects

/-- Erase a list of protocol snapshots into a fragment trace. -/
def fragmentTraceOfSnapshots
    (snapshots : List MachineSnapshot) : FragmentTrace :=
  snapshots.map snapshotToFragment

/-- Replay-visible semantic objects extracted directly from a list of protocol
snapshots. -/
def snapshotTraceSemanticObjects
    (snapshots : List MachineSnapshot) : List ProtocolSemanticObject :=
  snapshots.flatMap FieldProtocolAPI.exportSemanticObjects

/-- Trace relation between field protocol traces and erased fragment traces. -/
def TraceRel
    (protocolTrace : ProtocolTrace)
    (fragmentTrace : FragmentTrace) : Prop :=
  traceSemanticObjects protocolTrace = fragmentTraceSemanticObjects fragmentTrace

/-- The erasure from snapshots to fragments preserves the current observational
surface exactly. -/
theorem snapshot_to_fragment_matches
    (snapshot : MachineSnapshot) :
    SnapshotMatchesFragment snapshot (snapshotToFragment snapshot) := by
  simp [SnapshotMatchesFragment, snapshotToFragment]

/-- The fragment preserves the same replay-visible semantic objects exported by
the reduced machine snapshot. -/
theorem fragment_erasure_preserves_semantic_objects
    (snapshot : MachineSnapshot) :
    fragmentSemanticObjects (snapshotToFragment snapshot) =
      FieldProtocolAPI.exportSemanticObjects snapshot := by
  rfl

theorem snapshot_trace_semantic_objects_match_fragment_trace
    (snapshots : List MachineSnapshot) :
    snapshotTraceSemanticObjects snapshots =
      fragmentTraceSemanticObjects (fragmentTraceOfSnapshots snapshots) := by
  induction snapshots with
  | nil =>
      simp [snapshotTraceSemanticObjects, fragmentTraceOfSnapshots, fragmentTraceSemanticObjects]
  | cons snapshot rest ih =>
      calc
        snapshotTraceSemanticObjects (snapshot :: rest)
            = FieldProtocolAPI.exportSemanticObjects snapshot ++ snapshotTraceSemanticObjects rest := by
                simp [snapshotTraceSemanticObjects]
        _ = fragmentSemanticObjects (snapshotToFragment snapshot) ++
              fragmentTraceSemanticObjects (fragmentTraceOfSnapshots rest) := by
                rw [fragment_erasure_preserves_semantic_objects, ih]
        _ = fragmentTraceSemanticObjects (fragmentTraceOfSnapshots (snapshot :: rest)) := by
                simp [fragmentTraceSemanticObjects, fragmentTraceOfSnapshots]

/-- One reduced field machine step corresponds to one reduced fragment step
with observationally equivalent semantic export. -/
theorem advance_machine_simulates_fragment_step
    (input : MachineInput)
    (snapshot : MachineSnapshot) :
    let nextSnapshot := FieldProtocolAPI.advanceMachine input snapshot
    let nextFragment := snapshotToFragment nextSnapshot
    SnapshotMatchesFragment nextSnapshot nextFragment ∧
      fragmentSemanticObjects nextFragment =
        FieldProtocolAPI.exportSemanticObjects nextSnapshot := by
  simp [snapshot_to_fragment_matches, fragment_erasure_preserves_semantic_objects]

/-- Replay-equivalent fragment traces induce the same controller-facing evidence
batches as replay-equivalent protocol traces. -/
theorem replay_equivalent_fragment_traces_induce_equal_controller_evidence
    {left right : FragmentTrace}
    (hEqual : fragmentTraceSemanticObjects left = fragmentTraceSemanticObjects right) :
    semanticObjectsToEvidence (fragmentTraceSemanticObjects left) =
      semanticObjectsToEvidence (fragmentTraceSemanticObjects right) := by
  simp [hEqual]

theorem snapshot_trace_observer_projection_matches_fragment_trace
    (snapshots : List MachineSnapshot) :
    semanticObjectsToEvidence (snapshotTraceSemanticObjects snapshots) =
      semanticObjectsToEvidence
        (fragmentTraceSemanticObjects (fragmentTraceOfSnapshots snapshots)) := by
  simp [snapshot_trace_semantic_objects_match_fragment_trace]

/-- The current fragment erasure preserves only observational authority. -/
theorem fragment_semantic_objects_stay_observational
    (trace : FragmentTrace) :
    ∀ object ∈ fragmentTraceSemanticObjects trace,
      object.authority = OutputAuthority.observationalOnly := by
  intro object hObject
  induction trace with
  | nil =>
      simp [fragmentTraceSemanticObjects] at hObject
  | cons fragment rest ih =>
      simp [fragmentTraceSemanticObjects, fragmentSemanticObjects] at hObject
      rcases hObject with hHere | hThere
      · rcases hHere with ⟨_hVisible, hEq⟩
        simp [hEq]
      · rcases hThere with ⟨fragment', _hMem, _hGuard, hEq⟩
        simp [hEq]

end FieldProtocolBridge
