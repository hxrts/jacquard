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
  | antiEntropyDigest
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

/-! ## Abstract Operations -/

class Model where
  controllerRole : Role
  neighborRole : Role
  globalActions : List Action
  project : Role → LocalType
  advanceMachine : MachineInput → MachineSnapshot → MachineSnapshot
  exportOutputs : MachineSnapshot → List ProtocolOutput

section Wrappers

variable [Model]

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

end Wrappers

/-! ## Law Interfaces -/

abbrev ProjectionHarmony (M : Model) : Prop :=
  @Model.project M (@Model.neighborRole M) =
    LocalType.dual (@Model.project M (@Model.controllerRole M))

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

class Laws extends Model where
  projection_harmony : ProjectionHarmony toModel
  advance_preserves_bounds : AdvancePreservesBounds toModel
  cancel_fails_closed : CancelFailsClosed toModel
  exports_remain_observational : ExportsRemainObservational toModel

instance (priority := 100) lawsToModel [Laws] : Model := Laws.toModel

section LawWrappers

variable [Laws]

theorem projection_harmony :
    project neighborRole = LocalType.dual (project controllerRole) :=
  Laws.projection_harmony

theorem advance_preserves_bounds
    (input : MachineInput)
    (snapshot : MachineSnapshot)
    (h : MachineBounded snapshot) :
    MachineBounded (advanceMachine input snapshot) :=
  Laws.advance_preserves_bounds input snapshot h

theorem cancel_fails_closed (snapshot : MachineSnapshot) :
    (advanceMachine MachineInput.cancel snapshot).disposition =
      HostDisposition.failedClosed :=
  Laws.cancel_fails_closed snapshot

theorem exports_remain_observational
    (snapshot : MachineSnapshot) :
    ObservationalOnly (exportOutputs snapshot) :=
  Laws.exports_remain_observational snapshot

end LawWrappers

end FieldProtocolAPI
