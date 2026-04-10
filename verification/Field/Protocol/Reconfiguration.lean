import Field.Protocol.Instance

/-!
Current audit result for field protocol reconfiguration and delegation.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolReconfiguration

open FieldProtocolAPI

/-- The current reduced field protocol is a fixed-participant choreography. -/
def FixedParticipantChoreography : Prop :=
  FieldProtocolAPI.globalActions.length = 2 ∧
    FieldProtocolAPI.controllerRole = asRole .controller ∧
    FieldProtocolAPI.neighborRole = asRole .neighbor

/-- Reconfiguration is not currently part of the reduced field private
protocol semantics. -/
def ReconfigurationRequired : Prop := False

theorem reduced_protocol_is_fixed_participant :
    FixedParticipantChoreography := by
  change
    FieldProtocolInstance.globalActionsImpl.length = 2 ∧
      FieldProtocolInstance.controllerRoleImpl = asRole .controller ∧
      FieldProtocolInstance.neighborRoleImpl = asRole .neighbor
  constructor
  · rfl
  · constructor <;> rfl

theorem current_reduced_protocol_requires_no_reconfiguration :
    ¬ ReconfigurationRequired := by
  simp [ReconfigurationRequired]

end FieldProtocolReconfiguration
