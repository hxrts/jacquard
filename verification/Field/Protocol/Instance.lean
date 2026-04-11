import Field.Protocol.API

/-
The Problem. The field protocol layer needs one concrete reduced protocol
instance so the proof boundary is exercised against a real choreography-shaped
object instead of prose alone. The instance should stay narrow: summary
exchange, acknowledgement, bounded retries, and fail-closed cancellation.

Solution Structure.
1. Define one controller/neighbor summary-exchange choreography.
2. Define a small bounded host-visible machine state.
3. Prove projection harmony, bounded stepping, and observational-only export.
4. Expose a few executable lemmas that exercise the reduced machine.
-/

/-! # FieldProtocolInstance

First reduced realization of the private field summary-exchange protocol.

This instance is intentionally smaller than the Rust runtime in
`crates/field/src/choreography.rs`. It captures the proof-relevant boundary:
roles, local projections, bounded machine stepping, fail-closed cancellation,
and observational export.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolInstance

open FieldProtocolAPI
open SessionTypes.Core

/-! ## Protocol Skeleton -/

/-- Local controller role for the reduced summary-exchange protocol. -/
def controllerRoleImpl : Role := asRole .controller

/-- Local neighbor role for the reduced summary-exchange protocol. -/
def neighborRoleImpl : Role := asRole .neighbor

/-- Reduced global field choreography object. -/
def globalChoreographyImpl : GlobalChoreography :=
  { actions :=
      [ (roleName .controller, roleName .neighbor, labelName .summaryDelta)
      , (roleName .neighbor, roleName .controller, labelName .antiEntropyAck)
      ] }

/-- Reduced global summary-exchange action list. -/
def globalActionsImpl : List Action := globalChoreographyImpl.actions

/-- Controller-local projection of the reduced global choreography. -/
def controllerLocalType : LocalType :=
  projectChoreography globalChoreographyImpl controllerRoleImpl

/-- Neighbor-local projection is the dual of the controller view. -/
def neighborLocalType : LocalType := LocalType.dual controllerLocalType

/-- Projection for the two protocol roles used by the reduced instance. -/
def projectImpl (role : Role) : LocalType :=
  if role = controllerRoleImpl then
    controllerLocalType
  else if role = neighborRoleImpl then
    neighborLocalType
  else
    { actions := [] }

/-! ## Bounded Host Machine -/

/-- Clamp the emitted summary count into the shared step budget. -/
def clampMachineCount (value : Nat) : Nat := min value 8

/-- Clamp the remaining step budget into the shared machine budget. -/
def clampMachineBudget (value : Nat) : Nat := min value 8

/-- One bounded machine step for the reduced summary protocol. -/
def advanceMachineImpl
    (input : MachineInput)
    (snapshot : MachineSnapshot) : MachineSnapshot :=
  match input with
  | .poll =>
      if snapshot.disposition = HostDisposition.complete then
        snapshot
      else
        { snapshot with
          blockedOn := some SummaryLabel.summaryDelta
          disposition := HostDisposition.blocked }
  | .receiveSummary =>
      { stepBudgetRemaining := clampMachineBudget (snapshot.stepBudgetRemaining - 1)
        blockedOn := some SummaryLabel.antiEntropyAck
        disposition := HostDisposition.running
        emittedCount := clampMachineCount (snapshot.emittedCount + 1) }
  | .receiveAck =>
      { stepBudgetRemaining := clampMachineBudget (snapshot.stepBudgetRemaining - 1)
        blockedOn := none
        disposition := HostDisposition.complete
        emittedCount := snapshot.emittedCount }
  | .cancel =>
      { stepBudgetRemaining := snapshot.stepBudgetRemaining
        blockedOn := none
        disposition := HostDisposition.failedClosed
        emittedCount := snapshot.emittedCount }

/-- Export only observational summary batches to the host controller. -/
def exportOutputsImpl (snapshot : MachineSnapshot) : List ProtocolOutput :=
  if snapshot.disposition = HostDisposition.failedClosed
      || snapshot.emittedCount = 0 then
    []
  else
    [ { batch := { summaryCount := snapshot.emittedCount }
        authority := OutputAuthority.observationalOnly } ]

/-- Replay-visible semantic objects exported by the reduced protocol instance. -/
def exportSemanticObjectsImpl
    (snapshot : MachineSnapshot) : List ProtocolSemanticObject :=
  if snapshot.disposition = HostDisposition.failedClosed
      || snapshot.emittedCount = 0 then
    []
  else
    [ { batch := { summaryCount := snapshot.emittedCount }
        disposition := snapshot.disposition
        authority := OutputAuthority.observationalOnly } ]

/-! ## API Instance -/

-- long-block-exception: the reduced protocol laws are intentionally presented
-- as one executable law bundle so the whole boundary contract is reviewable in
-- one place against the concrete machine step.
instance fieldProtocolLaws : FieldProtocolAPI.Laws where
  globalChoreography := globalChoreographyImpl
  controllerRole := controllerRoleImpl
  neighborRole := neighborRoleImpl
  globalActions := globalActionsImpl
  project := projectImpl
  advanceMachine := advanceMachineImpl
  exportOutputs := exportOutputsImpl
  exportSemanticObjects := exportSemanticObjectsImpl
  projection_harmony := by
    -- The controller role is projected from the global choreography and the neighbor
    -- role is the dual local view.
    simp [ProjectionHarmony, projectImpl, controllerRoleImpl, neighborRoleImpl,
      neighborLocalType, asRole, roleName]
  controller_projection_from_global := by
    simp [ControllerProjectionFromGlobal, projectImpl, controllerRoleImpl,
      controllerLocalType, globalChoreographyImpl, asRole, roleName]
  neighbor_projection_from_global := by
    simp [NeighborProjectionFromGlobal, projectImpl, controllerRoleImpl, neighborRoleImpl,
      neighborLocalType, controllerLocalType, globalChoreographyImpl, asRole, roleName]
  advance_preserves_bounds := by
    intro input snapshot hBounded
    -- Every machine step either preserves the bounded counters or reclamps them.
    rcases hBounded with ⟨hBudget, hEmitted⟩
    cases input
    · by_cases hComplete : snapshot.disposition = HostDisposition.complete
      · simp [advanceMachineImpl, MachineBounded, hComplete, hBudget, hEmitted]
      · simp [advanceMachineImpl, MachineBounded, hComplete, hBudget, hEmitted]
    · constructor
      · exact Nat.min_le_right (snapshot.stepBudgetRemaining - 1) 8
      · exact Nat.min_le_right (snapshot.emittedCount + 1) 8
    · constructor
      · exact Nat.min_le_right (snapshot.stepBudgetRemaining - 1) 8
      · exact hEmitted
    · exact ⟨hBudget, hEmitted⟩
  advance_preserves_coherence := by
    intro input snapshot hCoherent
    cases input
    · by_cases hComplete : snapshot.disposition = HostDisposition.complete
      · simpa [advanceMachineImpl, hComplete] using hCoherent
      · constructor
        · intro hDone
          simp [advanceMachineImpl, hComplete] at hDone
        · intro hBlockedState
          simp [advanceMachineImpl, hComplete]
    · constructor
      · intro hDone
        simp [advanceMachineImpl] at hDone
      · intro hBlockedState
        simp [advanceMachineImpl] at hBlockedState
    · constructor
      · intro _hDone
        simp [advanceMachineImpl]
      · intro hBlockedState
        simp [advanceMachineImpl] at hBlockedState
    · constructor
      · intro _hDone
        simp [advanceMachineImpl]
      · intro hBlockedState
        simp [advanceMachineImpl] at hBlockedState
  cancel_fails_closed := by
    intro snapshot
    -- Cancellation is the only transition that can force failed-closed termination.
    simp [advanceMachineImpl]
  exports_remain_observational := by
    intro snapshot output hOutput
    -- Exported batches carry only observational authority by construction.
    simp [exportOutputsImpl] at hOutput
    simp [hOutput]
  semantic_exports_remain_observational := by
    intro snapshot object hObject
    -- Replay-visible semantic objects stay observational-only by construction.
    simp [exportSemanticObjectsImpl] at hObject
    simp [hObject]
  failed_closed_exports_nothing := by
    intro snapshot hFailed
    simp [exportOutputsImpl, hFailed]

/-! ## Representative Machine Lemmas -/

/-- Empty machine snapshot used by the reduced protocol examples. -/
def initialSnapshot : MachineSnapshot :=
  { stepBudgetRemaining := 8
    blockedOn := none
    disposition := HostDisposition.running
    emittedCount := 0 }

/-- Receiving a summary emits one observational batch and blocks for the ack. -/
theorem receive_summary_emits_observational_batch :
    let next := FieldProtocolAPI.advanceMachine MachineInput.receiveSummary initialSnapshot
    next.blockedOn = some SummaryLabel.antiEntropyAck ∧
      FieldProtocolAPI.exportOutputs next =
        [ { batch := { summaryCount := 1 }
            authority := OutputAuthority.observationalOnly } ] := by
  -- One received summary consumes one step, emits one batch, and waits for the ack.
  change
    (advanceMachineImpl MachineInput.receiveSummary initialSnapshot).blockedOn =
      some SummaryLabel.antiEntropyAck ∧
      exportOutputsImpl (advanceMachineImpl MachineInput.receiveSummary initialSnapshot) =
        [ { batch := { summaryCount := 1 }
            authority := OutputAuthority.observationalOnly } ]
  simp [advanceMachineImpl, exportOutputsImpl, initialSnapshot, clampMachineBudget,
    clampMachineCount]

/-- The instance projection really is induced from the reduced global
choreography object. -/
theorem controller_projection_matches_global_choreography :
    projectImpl controllerRoleImpl =
      projectChoreography globalChoreographyImpl controllerRoleImpl := by
  rfl

/-- The instance projection really is induced from the reduced global
choreography object. -/
theorem neighbor_projection_matches_global_choreography :
    projectImpl neighborRoleImpl = LocalType.dual controllerLocalType := by
  simp [projectImpl, neighborRoleImpl, controllerRoleImpl, neighborLocalType,
    asRole, roleName]

/-- Receiving a summary emits one replay-visible semantic object and waits for
the ack. -/
theorem receive_summary_emits_semantic_object :
    let next := FieldProtocolAPI.advanceMachine MachineInput.receiveSummary initialSnapshot
    FieldProtocolAPI.exportSemanticObjects next =
      [ { batch := { summaryCount := 1 }
          disposition := HostDisposition.running
          authority := OutputAuthority.observationalOnly } ] := by
  change
    exportSemanticObjectsImpl (advanceMachineImpl MachineInput.receiveSummary initialSnapshot) =
      [ { batch := { summaryCount := 1 }
          disposition := HostDisposition.running
          authority := OutputAuthority.observationalOnly } ]
  simp [advanceMachineImpl, exportSemanticObjectsImpl, initialSnapshot, clampMachineBudget,
    clampMachineCount]

/-- Polling a live machine blocks on the next summary input. -/
theorem poll_blocks_for_summary :
    (FieldProtocolAPI.advanceMachine MachineInput.poll initialSnapshot).blockedOn =
      some SummaryLabel.summaryDelta := by
  -- Polling asks the host bridge for the next summary message.
  change (advanceMachineImpl MachineInput.poll initialSnapshot).blockedOn =
    some SummaryLabel.summaryDelta
  simp [advanceMachineImpl, initialSnapshot]

/-- Fail-closed cancellation produces no host-visible outputs. -/
theorem cancelled_machine_exports_nothing :
    FieldProtocolAPI.exportOutputs
        (FieldProtocolAPI.advanceMachine MachineInput.cancel initialSnapshot) = [] := by
  -- Once the machine fails closed, the protocol must stop exporting batches.
  change exportOutputsImpl (advanceMachineImpl MachineInput.cancel initialSnapshot) = []
  simp [advanceMachineImpl, exportOutputsImpl, initialSnapshot]

end FieldProtocolInstance
