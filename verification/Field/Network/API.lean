import Mathlib.Tactic.DeriveFintype
import Field.Model.API

/-!
Reduced synchronous network layer above the destination-local field model.

This first network object is intentionally round-based rather than asynchronous.
The round buffer stores one publication per sender/destination slot so a later
async semantics can replace that buffer without rewriting the local field
model itself.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldNetworkAPI

open FieldModelAPI

inductive NodeId
  | alpha
  | beta
  | gamma
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

inductive DestinationClass
  | corridorA
  | corridorB
  deriving Inhabited, Repr, DecidableEq, BEq, Fintype

def allNodes : List NodeId :=
  [.alpha, .beta, .gamma]

def allDestinations : List DestinationClass :=
  [.corridorA, .corridorB]

def NeighborRelation := NodeId → NodeId → Bool

/-- Minimal proof-relevant network message: the sender republishes exactly the
current public corridor projection for one destination class. -/
structure NetworkMessage where
  sender : NodeId
  destination : DestinationClass
  projection : CorridorEnvelopeProjection
  deriving Repr, DecidableEq, BEq

/-- Reduced synchronous network state.

`roundBuffer` is intentionally indexed by sender and destination class. This
keeps the first model synchronous while leaving a clean replacement point for a
future async delivery relation. -/
structure NetworkState where
  localStates : NodeId → DestinationClass → LocalState
  neighbors : NeighborRelation
  roundBuffer : NodeId → DestinationClass → NetworkMessage

/-- Every node publishes its current local public corridor projection. -/
def publishMessage
    (sender : NodeId)
    (destination : DestinationClass)
    (localState : LocalState) : NetworkMessage :=
  { sender := sender
    destination := destination
    projection := localState.projection }

/-- Fill the synchronous round buffer from the current local states. -/
def initializeRoundBuffer
    (locals : NodeId → DestinationClass → LocalState) :
    NodeId → DestinationClass → NetworkMessage :=
  fun sender destination =>
    publishMessage sender destination (locals sender destination)

/-- One synchronous network round republishes the current local projections
into the round buffer and leaves local state unchanged. A later async layer can
refine this by replacing only the delivery semantics. -/
def networkRound (state : NetworkState) : NetworkState :=
  { localStates := state.localStates
    neighbors := state.neighbors
    roundBuffer := initializeRoundBuffer state.localStates }

/-- Delivered messages for one receiver and destination in the current reduced
synchronous semantics. Future async semantics should refine this function
rather than rewriting local-state ownership. -/
def deliveredMessages
    (state : NetworkState)
    (receiver : NodeId)
    (destination : DestinationClass) : List NetworkMessage :=
  allNodes.filterMap fun sender =>
    if state.neighbors sender receiver then
      some (state.roundBuffer sender destination)
    else
      none

/-- All node-local states satisfy the current local harmony law. -/
def NetworkLocallyHarmonious (state : NetworkState) : Prop :=
  ∀ node destination, Harmony (state.localStates node destination)

end FieldNetworkAPI
