import Field.Network.API

/-!
Reduced asynchronous network layer above the synchronous publication model.

This layer keeps timing, loss, and retry assumptions explicit while remaining
small enough to connect back to the synchronous round buffer. It is still a
reduced semantics: delivery is represented by in-flight envelopes and a single
step relation over delays rather than by the full production transport stack.
-/

set_option autoImplicit false
set_option relaxedAutoImplicit false

namespace FieldAsyncAPI

open FieldModelAPI
open FieldNetworkAPI

structure AsyncAssumptions where
  maxDelay : Nat
  retryBound : Nat
  lossPossible : Prop

def reliableImmediateAssumptions : AsyncAssumptions :=
  { maxDelay := 0
    retryBound := 0
    lossPossible := False }

structure AsyncEnvelope where
  sender : NodeId
  receiver : NodeId
  destination : DestinationClass
  projection : CorridorEnvelopeProjection
  delay : Nat
  retryCount : Nat
  dropped : Bool
  deriving Repr, DecidableEq, BEq

structure AsyncState where
  network : NetworkState
  assumptions : AsyncAssumptions
  inFlight : List AsyncEnvelope
  tick : Nat

def publicationEnvelope
    (network : NetworkState)
    (assumptions : AsyncAssumptions)
    (sender receiver : NodeId)
    (destination : DestinationClass) : AsyncEnvelope :=
  { sender := sender
    receiver := receiver
    destination := destination
    projection := (publishMessage sender destination (network.localStates sender destination)).projection
    delay := assumptions.maxDelay
    retryCount := 0
    dropped := False }

def readyForDelivery (envelope : AsyncEnvelope) : Bool :=
  envelope.delay = 0 && !envelope.dropped

def stepEnvelope (envelope : AsyncEnvelope) : AsyncEnvelope :=
  if envelope.delay = 0 then
    envelope
  else
    { envelope with delay := envelope.delay - 1 }

def enqueuePublications
    (network : NetworkState)
    (assumptions : AsyncAssumptions) : List AsyncEnvelope :=
  (allNodes.foldr
      (fun sender senderAcc =>
        (allNodes.foldr
            (fun receiver receiverAcc =>
              (allDestinations.filterMap fun destination =>
                  if network.neighbors sender receiver then
                    some (publicationEnvelope network assumptions sender receiver destination)
                  else
                    none) ++ receiverAcc)
            []) ++ senderAcc)
      [])

def asyncStep (state : AsyncState) : AsyncState :=
  { network := state.network
    assumptions := state.assumptions
    inFlight := (state.inFlight.map stepEnvelope) ++ enqueuePublications state.network state.assumptions
    tick := state.tick + 1 }

def readyMessages
    (state : AsyncState)
    (receiver : NodeId)
    (destination : DestinationClass) : List AsyncEnvelope :=
  state.inFlight.filter fun envelope =>
    envelope.receiver = receiver &&
      envelope.destination = destination &&
      readyForDelivery envelope

def drainReadyMessages (state : AsyncState) : AsyncState :=
  { state with
      inFlight := state.inFlight.filter fun envelope => !(readyForDelivery envelope) }

/-- Small observer view for the reduced async layer. -/
structure AsyncObserverView where
  readyCount : Nat
  inFlightCount : Nat
  deriving Repr, DecidableEq, BEq

def observerView (state : AsyncState) : AsyncObserverView :=
  { readyCount := (state.inFlight.filter readyForDelivery).length
    inFlightCount := state.inFlight.length }

end FieldAsyncAPI
