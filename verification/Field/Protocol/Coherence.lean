import Field.Protocol.Instance

/-!
Reduced coherence lemmas making the field analogue of updated-edge,
incident-edge, and unrelated-edge preservation explicit.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolCoherence

open FieldProtocolAPI

/-- Updated-edge style coherence: receiving a summary keeps the machine on the
active summary/ack frontier with a bounded emitted batch. -/
def UpdatedEdgeStyle (snapshot : MachineSnapshot) : Prop :=
  snapshot.disposition = HostDisposition.running →
    snapshot.blockedOn = some SummaryLabel.antiEntropyAck →
    0 < snapshot.emittedCount

/-- Incident-edge style coherence: blocked machines expose a concrete receive
frontier rather than a silent blocked state. -/
def IncidentEdgeStyle (snapshot : MachineSnapshot) : Prop :=
  snapshot.disposition = HostDisposition.blocked →
    snapshot.blockedOn.isSome

/-- Unrelated-edge style coherence: terminal or complete machines do not keep a
stale blocked frontier. -/
def UnrelatedEdgeStyle (snapshot : MachineSnapshot) : Prop :=
  (snapshot.disposition = HostDisposition.complete ∨
      snapshot.disposition = HostDisposition.failedClosed) →
    snapshot.blockedOn = none

theorem receive_summary_establishes_updated_edge_style
    (snapshot : MachineSnapshot) :
    UpdatedEdgeStyle (FieldProtocolAPI.advanceMachine MachineInput.receiveSummary snapshot) := by
  change UpdatedEdgeStyle (FieldProtocolInstance.advanceMachineImpl MachineInput.receiveSummary snapshot)
  intro hRunning hBlocked
  by_cases hCmp : snapshot.emittedCount + 1 ≤ 8
  · simp [FieldProtocolInstance.advanceMachineImpl, FieldProtocolInstance.clampMachineCount,
      Nat.min_eq_left hCmp]
  · have hGe : 8 ≤ snapshot.emittedCount + 1 := Nat.le_of_not_ge hCmp
    simp [FieldProtocolInstance.advanceMachineImpl, FieldProtocolInstance.clampMachineCount,
      Nat.min_eq_right hGe]

theorem poll_establishes_incident_edge_style
    (snapshot : MachineSnapshot)
    (hNotComplete : snapshot.disposition ≠ HostDisposition.complete) :
    IncidentEdgeStyle (FieldProtocolAPI.advanceMachine MachineInput.poll snapshot) := by
  change IncidentEdgeStyle (FieldProtocolInstance.advanceMachineImpl MachineInput.poll snapshot)
  intro hBlocked
  simp [FieldProtocolInstance.advanceMachineImpl, hNotComplete] at hBlocked ⊢

theorem receive_ack_establishes_unrelated_edge_style
    (snapshot : MachineSnapshot) :
    UnrelatedEdgeStyle (FieldProtocolAPI.advanceMachine MachineInput.receiveAck snapshot) := by
  change UnrelatedEdgeStyle (FieldProtocolInstance.advanceMachineImpl MachineInput.receiveAck snapshot)
  intro hTerminal
  simp [FieldProtocolInstance.advanceMachineImpl] at hTerminal ⊢

theorem cancel_establishes_unrelated_edge_style
    (snapshot : MachineSnapshot) :
    UnrelatedEdgeStyle (FieldProtocolAPI.advanceMachine MachineInput.cancel snapshot) := by
  change UnrelatedEdgeStyle (FieldProtocolInstance.advanceMachineImpl MachineInput.cancel snapshot)
  intro hTerminal
  simp [FieldProtocolInstance.advanceMachineImpl] at hTerminal ⊢

theorem poll_on_complete_machine_is_unrelated_edge_stable
    (snapshot : MachineSnapshot)
    (hComplete : snapshot.disposition = HostDisposition.complete) :
    FieldProtocolAPI.advanceMachine MachineInput.poll snapshot = snapshot := by
  change FieldProtocolInstance.advanceMachineImpl MachineInput.poll snapshot = snapshot
  simp [FieldProtocolInstance.advanceMachineImpl, hComplete]

end FieldProtocolCoherence
