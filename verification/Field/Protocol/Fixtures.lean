import Field.Protocol.Bridge
import Field.Protocol.Conservation
import Field.Protocol.Instance
import Field.Protocol.ReceiveRefinement
import Field.Protocol.Reconfiguration
import Field.Model.Boundary

/- 
The Problem. The reduced field protocol boundary now has a final closure file,
but we also need concrete proof-facing fixtures so the closure story is pinned
to real summary/ack exchanges instead of only bundled propositions.

Solution Structure.
1. Define one representative reduced summary/ack exchange trace.
2. Reuse the fragment-trace and observer-projection bridge on that trace.
3. Package concrete receive-refinement and no-reconfiguration witnesses.
-/

/-!
 # FieldProtocolFixtures

Concrete proof-facing fixtures for the reduced field private protocol.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolFixtures

open FieldProtocolAPI
open FieldProtocolBridge
open FieldProtocolConservation
open FieldProtocolReceiveRefinement
open FieldProtocolReconfiguration
open FieldBoundary

/-! ## Representative Reduced Summary Exchange -/

def afterSummaryReceive : MachineSnapshot :=
  FieldProtocolAPI.advanceMachine
    MachineInput.receiveSummary
    FieldProtocolInstance.initialSnapshot

def afterAckReceive : MachineSnapshot :=
  FieldProtocolAPI.advanceMachine
    MachineInput.receiveAck
    afterSummaryReceive

def summaryExchangeSnapshots : List MachineSnapshot :=
  [FieldProtocolInstance.initialSnapshot, afterSummaryReceive, afterAckReceive]

theorem summary_exchange_fragment_trace_matches_snapshot_trace :
    snapshotTraceSemanticObjects summaryExchangeSnapshots =
      fragmentTraceSemanticObjects
        (fragmentTraceOfSnapshots summaryExchangeSnapshots) := by
  exact snapshot_trace_semantic_objects_match_fragment_trace summaryExchangeSnapshots

theorem summary_exchange_observer_projection_matches_fragment_trace :
    semanticObjectsToEvidence (snapshotTraceSemanticObjects summaryExchangeSnapshots) =
      semanticObjectsToEvidence
        (fragmentTraceSemanticObjects
          (fragmentTraceOfSnapshots summaryExchangeSnapshots)) := by
  exact
    snapshot_lists_preserve_observer_projection_under_fragment_erasure
      summaryExchangeSnapshots

theorem summary_exchange_fragment_trace_stays_observational :
    ∀ object ∈ fragmentTraceSemanticObjects (fragmentTraceOfSnapshots summaryExchangeSnapshots),
      object.authority = OutputAuthority.observationalOnly := by
  exact
    fragment_trace_authority_conserved
      (fragmentTraceOfSnapshots summaryExchangeSnapshots)

theorem summary_receive_fixture_has_subtype_replacement_witness :
    ∃ witness : SubtypeReplacementWitness,
      witness.refined = RefinedReceive.summaryDelta 1 := by
  exact refined_receive_has_subtype_replacement_witness (.summaryDelta 1)

theorem ack_receive_fixture_has_subtype_replacement_witness :
    ∃ witness : SubtypeReplacementWitness,
      witness.refined = RefinedReceive.antiEntropyAck 7 := by
  exact refined_receive_has_subtype_replacement_witness (.antiEntropyAck 7)

theorem summary_exchange_fixture_is_fixed_participant :
    FixedParticipantChoreography := by
  exact reduced_protocol_is_fixed_participant

theorem summary_exchange_fixture_requires_no_reconfiguration :
    ¬ ReconfigurationRequired := by
  exact current_reduced_protocol_requires_no_reconfiguration

end FieldProtocolFixtures
