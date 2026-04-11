import SessionTypes.Core

/-
The Problem. The field cooperative layer needs a proof-facing protocol surface
that is clearly separate from the local field controller and from router-owned
canonical route truth. We need a compact API for protocol projection,
host-visible machine outputs, and observational export, while keeping concrete
choreography and runtime realization isolated in a companion instance module.

Solution Structure.
1. Define protocol roles, message labels, machine inputs, and observational
   outputs.
2. Define an abstract protocol `Model` with projection, stepping, and export.
3. Define law interfaces for projection harmony, bounded execution, fail-closed
   cancellation, and observational-only export.
4. Re-export stable wrappers consumed by downstream proofs.
-/

/-! # FieldProtocolAPI

Abstract API for the private field summary-exchange protocol layer.

This protocol surface is intentionally narrower than the full field engine. It
models only the private cooperative layer that may later be connected to richer
Telltale choreography and protocol-machine proofs.

Projection taxonomy note:

- protocol projection:
  choreography/session structure -> local protocol surface
- local public projection:
  local field semantics -> corridor/public observable surface
- runtime projection:
  runtime artifacts/state -> reduced Lean adequacy surface

This module owns only the first kind.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolAPI

open SessionTypes.Core

/-! ## Protocol Surface -/

inductive ProtocolRole
  | controller
  | neighbor
  deriving Inhabited, Repr, DecidableEq, BEq

inductive SummaryLabel
  | summaryDelta
  | antiEntropyAck
  deriving Inhabited, Repr, DecidableEq, BEq

inductive MachineInput
  | poll
  | receiveSummary
  | receiveAck
  | cancel
  deriving Inhabited, Repr, DecidableEq, BEq

inductive HostDisposition
  | running
  | blocked
  | complete
  | failedClosed
  deriving Inhabited, Repr, DecidableEq, BEq

inductive OutputAuthority
  | observationalOnly
  deriving Inhabited, Repr, DecidableEq, BEq

structure ObservedSummaryBatch where
  summaryCount : Nat
  deriving Repr, DecidableEq, BEq

structure ProtocolOutput where
  batch : ObservedSummaryBatch
  authority : OutputAuthority
  deriving Repr, DecidableEq, BEq

structure ProtocolSemanticObject where
  batch : ObservedSummaryBatch
  disposition : HostDisposition
  authority : OutputAuthority
  deriving Repr, DecidableEq, BEq

inductive ProtocolTraceEvent
  | machineInput (input : MachineInput)
  | semanticObject (object : ProtocolSemanticObject)
  deriving Inhabited, Repr, DecidableEq, BEq

abbrev ProtocolTrace := List ProtocolTraceEvent

structure GlobalChoreography where
  actions : List Action
  deriving Repr, DecidableEq, BEq

structure MachineSnapshot where
  stepBudgetRemaining : Nat
  blockedOn : Option SummaryLabel
  disposition : HostDisposition
  emittedCount : Nat
  deriving Repr, DecidableEq, BEq

def MachineBounded (snapshot : MachineSnapshot) : Prop :=
  snapshot.stepBudgetRemaining ≤ 8 ∧ snapshot.emittedCount ≤ 8

def ObservationalOnly (outputs : List ProtocolOutput) : Prop :=
  ∀ output ∈ outputs, output.authority = OutputAuthority.observationalOnly

def SemanticObjectsObservationalOnly
    (objects : List ProtocolSemanticObject) : Prop :=
  ∀ object ∈ objects, object.authority = OutputAuthority.observationalOnly

def roleName : ProtocolRole → String
  | .controller => "controller"
  | .neighbor => "neighbor"

def asRole (role : ProtocolRole) : Role := { name := roleName role }

def labelName : SummaryLabel → String
  | .summaryDelta => "summaryDelta"
  | .antiEntropyAck => "antiEntropyAck"

def localActionForRole
    (role : Role)
    (action : Action) : Option LocalAction :=
  let (sender, receiver, label) := action
  if sender = role.name then
    some { kind := .send, partner := receiver, label := label }
  else if receiver = role.name then
    some { kind := .recv, partner := sender, label := label }
  else
    none

def projectChoreography
    (global : GlobalChoreography)
    (role : Role) : LocalType :=
  { actions := global.actions.filterMap (localActionForRole role) }

def traceSemanticObjects (trace : ProtocolTrace) : List ProtocolSemanticObject :=
  trace.filterMap fun event =>
    match event with
    | .machineInput _ => none
    | .semanticObject object => some object

def MachineCoherent (snapshot : MachineSnapshot) : Prop :=
  ((snapshot.disposition = HostDisposition.complete ∨
      snapshot.disposition = HostDisposition.failedClosed) →
      snapshot.blockedOn = none) ∧
    (snapshot.disposition = HostDisposition.blocked →
      snapshot.blockedOn.isSome)

/-! ## Abstract Operations -/

class Model where
  globalChoreography : GlobalChoreography
  controllerRole : Role
  neighborRole : Role
  globalActions : List Action
  project : Role → LocalType
  advanceMachine : MachineInput → MachineSnapshot → MachineSnapshot
  exportOutputs : MachineSnapshot → List ProtocolOutput
  exportSemanticObjects : MachineSnapshot → List ProtocolSemanticObject

section Wrappers

variable [Model]

def globalChoreography : GlobalChoreography := Model.globalChoreography

def controllerRole : Role := Model.controllerRole

def neighborRole : Role := Model.neighborRole

def globalActions : List Action := Model.globalActions

def project (role : Role) : LocalType := Model.project role

def advanceMachine
    (input : MachineInput)
    (snapshot : MachineSnapshot) : MachineSnapshot :=
  Model.advanceMachine input snapshot

def exportOutputs (snapshot : MachineSnapshot) : List ProtocolOutput :=
  Model.exportOutputs snapshot

def exportSemanticObjects
    (snapshot : MachineSnapshot) : List ProtocolSemanticObject :=
  Model.exportSemanticObjects snapshot

end Wrappers

/-! ## Law Interfaces -/

abbrev ProjectionHarmony (M : Model) : Prop :=
  @Model.project M (@Model.neighborRole M) =
    LocalType.dual (@Model.project M (@Model.controllerRole M))

abbrev ControllerProjectionFromGlobal (M : Model) : Prop :=
  @Model.project M (@Model.controllerRole M) =
    projectChoreography (@Model.globalChoreography M) (@Model.controllerRole M)

abbrev NeighborProjectionFromGlobal (M : Model) : Prop :=
  @Model.project M (@Model.neighborRole M) =
    LocalType.dual (projectChoreography (@Model.globalChoreography M) (@Model.controllerRole M))

abbrev AdvancePreservesBounds (M : Model) : Prop :=
  ∀ input snapshot,
    MachineBounded snapshot →
      MachineBounded (@Model.advanceMachine M input snapshot)

abbrev CancelFailsClosed (M : Model) : Prop :=
  ∀ snapshot,
    (@Model.advanceMachine M MachineInput.cancel snapshot).disposition =
      HostDisposition.failedClosed

abbrev ExportsRemainObservational (M : Model) : Prop :=
  ∀ snapshot, ObservationalOnly (@Model.exportOutputs M snapshot)

abbrev SemanticExportsRemainObservational (M : Model) : Prop :=
  ∀ snapshot, SemanticObjectsObservationalOnly (@Model.exportSemanticObjects M snapshot)

abbrev AdvancePreservesCoherence (M : Model) : Prop :=
  ∀ input snapshot,
    MachineCoherent snapshot →
      MachineCoherent (@Model.advanceMachine M input snapshot)

class Laws extends Model where
  projection_harmony : ProjectionHarmony toModel
  controller_projection_from_global : ControllerProjectionFromGlobal toModel
  neighbor_projection_from_global : NeighborProjectionFromGlobal toModel
  advance_preserves_bounds : AdvancePreservesBounds toModel
  advance_preserves_coherence : AdvancePreservesCoherence toModel
  cancel_fails_closed : CancelFailsClosed toModel
  exports_remain_observational : ExportsRemainObservational toModel
  semantic_exports_remain_observational :
    SemanticExportsRemainObservational toModel

instance (priority := 100) lawsToModel [Laws] : Model := Laws.toModel

section LawWrappers

variable [Laws]

theorem projection_harmony :
    project neighborRole = LocalType.dual (project controllerRole) :=
  Laws.projection_harmony

theorem controller_projection_from_global :
    project controllerRole =
      projectChoreography globalChoreography controllerRole :=
  Laws.controller_projection_from_global

theorem neighbor_projection_from_global :
    project neighborRole =
      LocalType.dual (projectChoreography globalChoreography controllerRole) :=
  Laws.neighbor_projection_from_global

theorem advance_preserves_bounds
    (input : MachineInput)
    (snapshot : MachineSnapshot)
    (h : MachineBounded snapshot) :
    MachineBounded (advanceMachine input snapshot) :=
  Laws.advance_preserves_bounds input snapshot h

theorem advance_preserves_coherence
    (input : MachineInput)
    (snapshot : MachineSnapshot)
    (h : MachineCoherent snapshot) :
    MachineCoherent (advanceMachine input snapshot) :=
  Laws.advance_preserves_coherence input snapshot h

theorem cancel_fails_closed (snapshot : MachineSnapshot) :
    (advanceMachine MachineInput.cancel snapshot).disposition =
      HostDisposition.failedClosed :=
  Laws.cancel_fails_closed snapshot

theorem exports_remain_observational
    (snapshot : MachineSnapshot) :
    ObservationalOnly (exportOutputs snapshot) :=
  Laws.exports_remain_observational snapshot

theorem semantic_exports_remain_observational
    (snapshot : MachineSnapshot) :
    SemanticObjectsObservationalOnly (exportSemanticObjects snapshot) :=
  Laws.semantic_exports_remain_observational snapshot

end LawWrappers

end FieldProtocolAPI
