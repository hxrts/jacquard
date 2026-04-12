import Field.Protocol.Instance
import Field.Network.API

/-!
Reduced protocol reconfiguration vocabulary for Field.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldProtocolReconfiguration

open FieldProtocolAPI
open FieldNetworkAPI

/-- The reduced field choreography keeps the participant roles fixed even when
session ownership or checkpoints move. -/
def FixedParticipantChoreography : Prop :=
  FieldProtocolAPI.globalActions.length = 2 ∧
    FieldProtocolAPI.controllerRole = asRole .controller ∧
    FieldProtocolAPI.neighborRole = asRole .neighbor

inductive ProtocolClass
  | summaryDissemination
  | antiEntropy
  | retentionReplay
  | explicitCoordination
  deriving Inhabited, Repr, DecidableEq, BEq

inductive ReconfigurationCause
  | ownerTransfer
  | checkpointRestore
  | continuationShift
  deriving Inhabited, Repr, DecidableEq, BEq

structure SessionIdentity where
  protocol : ProtocolClass
  routeBinding : Option Nat
  destination : Option DestinationClass
  generation : Nat
  deriving Inhabited, Repr, DecidableEq, BEq

structure ReducedReconfiguration where
  priorSession : SessionIdentity
  nextSession : SessionIdentity
  priorOwner : Nat
  nextOwner : Nat
  cause : ReconfigurationCause
  participantSetChanged : Bool
  deriving Inhabited, Repr, DecidableEq, BEq

def ReconfigurationAdmitted
    (step : ReducedReconfiguration) : Prop :=
  step.participantSetChanged = false ∧
    step.priorSession.protocol = step.nextSession.protocol ∧
    step.priorSession.destination = step.nextSession.destination ∧
    step.nextSession.generation ≥ step.priorSession.generation

def ReconfigurationObservationalOnly
    (_step : ReducedReconfiguration) : Prop := True

def ReconfigurationDoesNotOwnRouteTruth
    (_step : ReducedReconfiguration) : Prop := True

def ReconfiguringProtocolBoundary : Prop :=
  FixedParticipantChoreography ∧
    ∀ step, ReconfigurationAdmitted step →
      ReconfigurationObservationalOnly step ∧
        ReconfigurationDoesNotOwnRouteTruth step

theorem reduced_protocol_is_fixed_participant :
    FixedParticipantChoreography := by
  change
    FieldProtocolInstance.globalActionsImpl.length = 2 ∧
      FieldProtocolInstance.controllerRoleImpl = asRole .controller ∧
      FieldProtocolInstance.neighborRoleImpl = asRole .neighbor
  constructor
  · rfl
  · constructor <;> rfl

theorem admitted_reconfiguration_keeps_participant_set_fixed
    (step : ReducedReconfiguration)
    (hAdmitted : ReconfigurationAdmitted step) :
    step.participantSetChanged = false := by
  exact hAdmitted.1

theorem admitted_reconfiguration_preserves_destination_scope
    (step : ReducedReconfiguration)
    (hAdmitted : ReconfigurationAdmitted step) :
    step.priorSession.destination = step.nextSession.destination := by
  exact hAdmitted.2.2.1

theorem admitted_reconfiguration_is_observational_only
    (step : ReducedReconfiguration)
    (_hAdmitted : ReconfigurationAdmitted step) :
    ReconfigurationObservationalOnly step := by
  trivial

theorem admitted_reconfiguration_does_not_own_route_truth
    (step : ReducedReconfiguration)
    (_hAdmitted : ReconfigurationAdmitted step) :
    ReconfigurationDoesNotOwnRouteTruth step := by
  trivial

theorem reduced_protocol_boundary_is_reconfiguring_by_design :
    ReconfiguringProtocolBoundary := by
  constructor
  · exact reduced_protocol_is_fixed_participant
  · intro step hAdmitted
    exact ⟨admitted_reconfiguration_is_observational_only step hAdmitted,
      admitted_reconfiguration_does_not_own_route_truth step hAdmitted⟩

end FieldProtocolReconfiguration
