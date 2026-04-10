import Field.Model.Boundary
import Field.Protocol.Instance

/-!
Minimal receive-refinement hook for the reduced field protocol. This does not
attempt full monitor-level subtype replacement; it introduces the smallest
refined receive objects needed to state a field-side refinement theorem in the
same shape.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolReceiveRefinement

open FieldBoundary
open FieldModelAPI
open FieldProtocolAPI

inductive RefinedReceive
  | summaryDelta (summaryCount : Nat)
  | antiEntropyAck (remainingBudget : Nat)
  deriving Inhabited, Repr, DecidableEq, BEq

def refinedReceiveLabel : RefinedReceive → SummaryLabel
  | .summaryDelta _ => SummaryLabel.summaryDelta
  | .antiEntropyAck _ => SummaryLabel.antiEntropyAck

def refinedReceiveInput : RefinedReceive → MachineInput
  | .summaryDelta _ => MachineInput.receiveSummary
  | .antiEntropyAck _ => MachineInput.receiveAck

/-- Minimal field-side subtype-replacement shape: every refined receive
projects to one of the two existing coarse labels without changing ownership or
authority boundaries. -/
def SubtypeReplacementShape (receive : RefinedReceive) : Prop :=
  refinedReceiveLabel receive = SummaryLabel.summaryDelta ∨
    refinedReceiveLabel receive = SummaryLabel.antiEntropyAck

structure SubtypeReplacementWitness where
  refined : RefinedReceive
  baseLabel : SummaryLabel
  baseInput : MachineInput
  labelMatches : refinedReceiveLabel refined = baseLabel
  inputMatches : refinedReceiveInput refined = baseInput

theorem refined_receive_has_existing_label
    (receive : RefinedReceive) :
    SubtypeReplacementShape receive := by
  cases receive <;> simp [SubtypeReplacementShape, refinedReceiveLabel]

theorem refined_receive_respects_existing_projection_surface
    (receive : RefinedReceive) :
    refinedReceiveInput receive = MachineInput.receiveSummary ∨
      refinedReceiveInput receive = MachineInput.receiveAck := by
  cases receive <;> simp [refinedReceiveInput]

theorem subtype_replacement_style_receive_refinement
    (receive : RefinedReceive) :
    refinedReceiveInput receive = MachineInput.receiveSummary ∨
      refinedReceiveInput receive = MachineInput.receiveAck := by
  exact refined_receive_respects_existing_projection_surface receive

theorem refined_receive_has_subtype_replacement_witness
    (receive : RefinedReceive) :
    ∃ witness : SubtypeReplacementWitness,
      witness.refined = receive := by
  cases receive with
  | summaryDelta summaryCount =>
      refine ⟨SubtypeReplacementWitness.mk (.summaryDelta summaryCount)
        SummaryLabel.summaryDelta MachineInput.receiveSummary rfl rfl, rfl⟩
  | antiEntropyAck remainingBudget =>
      refine ⟨SubtypeReplacementWitness.mk (.antiEntropyAck remainingBudget)
        SummaryLabel.antiEntropyAck MachineInput.receiveAck rfl rfl, rfl⟩

theorem refined_receive_preserves_observational_boundary
    (receive : RefinedReceive)
    (snapshot : MachineSnapshot) :
    ∀ evidence ∈ controllerEvidenceFromSnapshot (FieldProtocolAPI.advanceMachine (refinedReceiveInput receive) snapshot),
      evidence.reachability = ReachabilitySignal.unknown ∨
        evidence.reachability = ReachabilitySignal.corridorOnly := by
  intro evidence hEvidence
  exact FieldBoundary.all_controller_evidence_from_snapshot_stays_observational
    (snapshot := FieldProtocolAPI.advanceMachine (refinedReceiveInput receive) snapshot)
    evidence hEvidence

theorem subtype_replacement_witness_preserves_observational_boundary
    (receive : RefinedReceive)
    (snapshot : MachineSnapshot) :
    ∀ witness : SubtypeReplacementWitness,
      witness.refined = receive →
        ∀ evidence ∈ controllerEvidenceFromSnapshot
            (FieldProtocolAPI.advanceMachine witness.baseInput snapshot),
          evidence.reachability = ReachabilitySignal.unknown ∨
            evidence.reachability = ReachabilitySignal.corridorOnly := by
  intro witness hWitness evidence hEvidence
  subst hWitness
  have hEvidence' :
      evidence ∈ controllerEvidenceFromSnapshot
        (FieldProtocolAPI.advanceMachine (refinedReceiveInput witness.refined) snapshot) := by
    simpa [witness.inputMatches] using hEvidence
  exact refined_receive_preserves_observational_boundary witness.refined snapshot evidence hEvidence'

end FieldProtocolReceiveRefinement
